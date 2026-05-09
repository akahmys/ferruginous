use crate::font::FontResource;
use crate::{PdfError, PdfResult};
use std::collections::BTreeMap;

/// A surgical patcher for SFNT binaries.
pub struct FontReconstructor;

struct DisassembledSfnt {
    magic: [u8; 4],
    tables: Vec<([u8; 4], Vec<u8>)>,
}

/// The result of a font reconstruction operation.
#[derive(Debug, Clone)]
pub struct ReconstructedFont {
    /// The patched SFNT binary data.
    pub data: Vec<u8>,
    /// Whether the font is CID-keyed.
    pub is_cid: bool,
    /// Discovered CID-to-GID mapping (Authoritative).
    pub cid_to_gid_map: Option<BTreeMap<u32, u32>>,
    /// Discovered Glyph Name to GID mapping.
    pub name_to_gid_map: Option<BTreeMap<String, u32>>,
    /// Discovered CFF SID to GID mapping.
    pub sid_to_gid_map: Option<BTreeMap<u32, u32>>,
}

/// Standard CFF SID for the last predefined standard string.
const CFF_LAST_STANDARD_SID: u32 = 390;

/// Font format identified from binary data signatures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontFormat {
    /// SFNT container (OpenType/TrueType).
    Sfnt,
    /// Naked CFF version 1.0.
    Cff1,
    /// Naked CFF version 2.0.
    Cff2,
    /// Adobe Type 1 Binary (PFB).
    Type1Pfb,
    /// Adobe Type 1 ASCII (PFA).
    Type1Pfa,
    /// Unrecognized format.
    Unknown,
}

impl FontFormat {
    /// Detects the font format from raw binary data.
    pub fn detect(data: &[u8]) -> Self {
        if data.len() < 2 {
            return FontFormat::Unknown;
        }

        // 1. SFNT Signatures (OTTO, 0x00 0x01 0x00 0x00, ttcf, true)
        if data.len() >= 4 && (
            data.starts_with(b"OTTO")
                || data.starts_with(&[0, 1, 0, 0])
                || data.starts_with(b"ttcf")
                || data.starts_with(b"true")
        ) {
            return FontFormat::Sfnt;
        }

        // 2. CFF Signatures
        // CFF1: major=1, minor=0
        if data.len() >= 2 && data[0] == 1 && data[1] == 0 {
            return FontFormat::Cff1;
        }
        // CFF2: major=2
        if !data.is_empty() && data[0] == 2 {
            return FontFormat::Cff2;
        }

        // 3. Type 1 Signatures
        // PFB: 0x80 0x01 (Segment start)
        if data.starts_with(&[0x80, 0x01]) {
            return FontFormat::Type1Pfb;
        }
        // PFA: %! (PostScript)
        if data.starts_with(b"%!") {
            return FontFormat::Type1Pfa;
        }

        FontFormat::Unknown
    }
}

impl FontReconstructor {
    /// Reconstructs a font by injecting PDF metrics into the provided SFNT data.
    /// 
    /// This method performs surgical patching of font tables (hmtx, cmap) to align
    /// the physical font file with the metrics declared in the PDF document.
    pub fn reconstruct(
        resource: &FontResource,
        raw_data: &[u8],
    ) -> PdfResult<ReconstructedFont> {
        let format = FontFormat::detect(raw_data);
        log::debug!("[RECONSTRUCT] Starting reconstruction for {} (format: {:?}, size: {} bytes)", 
            resource.base_font.as_str(), format, raw_data.len());
        
        // Phase 1: Normalization (Convert any format to a Virtual SFNT)
        let normalized = Self::normalize_to_sfnt(format, raw_data, resource)?;
        let mut sfnt = normalized.data;
        let is_cid_font = normalized.is_cid;
        let discovered_map_bt = normalized.sid_to_gid_map.clone();
        let discovered_sid_map = normalized.sid_to_gid_map;
        let discovered_name_map = normalized.name_to_gid_map;

        // Phase 2: Surgical Patching (Apply PDF metrics and mappings to the SFNT)
        if let Ok(mut sfnt_dis) = Self::disassemble_sfnt(&sfnt) {
            // Patch hmtx (Glyph Widths)
            Self::patch_hmtx_direct(&mut sfnt_dis.tables, resource);

            // Patch/Inject cmap (Character Mapping)
            let (cmap_data_opt, synthesized_cid_map) =
                Self::synthesize_bridged_cmap(resource, &sfnt, discovered_map_bt.as_ref(), discovered_name_map.as_ref(), is_cid_font);

            if let Some(cmap_data) = cmap_data_opt {
                if let Some(idx) = sfnt_dis.tables.iter().position(|(t, _)| t == b"cmap") {
                    sfnt_dis.tables[idx].1 = cmap_data;
                } else {
                    sfnt_dis.tables.push((*b"cmap", cmap_data));
                }

                log::debug!("[RECONSTRUCT] Synthesized SFNT with {} tables", sfnt_dis.tables.len());
                for (tag, data) in &sfnt_dis.tables {
                    let tag_str = String::from_utf8_lossy(tag);
                    log::debug!("[RECONSTRUCT] Table {}: size {} bytes", tag_str, data.len());
                }

                if let Ok(new_data) = Self::assemble_sfnt(&sfnt_dis.magic, &sfnt_dis.tables) {
                    sfnt = new_data;
                }
            }
            
            let mut final_cid_map = normalized.cid_to_gid_map;
            if !synthesized_cid_map.is_empty() {
                final_cid_map = Some(synthesized_cid_map);
            }

            return Ok(ReconstructedFont {
                data: sfnt,
                is_cid: is_cid_font,
                cid_to_gid_map: final_cid_map,
                name_to_gid_map: discovered_name_map,
                sid_to_gid_map: discovered_sid_map,
            });
        }

        Ok(ReconstructedFont {
            data: sfnt,
            is_cid: is_cid_font,
            cid_to_gid_map: discovered_map_bt,
            name_to_gid_map: discovered_name_map,
            sid_to_gid_map: discovered_sid_map,
        })
    }

    /// Attempts to rescue the CID-to-GID mapping by scanning internal SFNT cmap tables 
    /// and bridging them via the PDF's ToUnicode map if available.
    /// Also attempts to rescue Glyph Name to GID mappings from the 'post' table.
    fn rescue_sid_map_from_sfnt(data: &[u8], resource: &FontResource, name_to_gid: &mut BTreeMap<String, u32>) -> Option<BTreeMap<u32, u32>> {
        let mut map = BTreeMap::new();
        if let Ok(face) = ttf_parser::Face::parse(data, 0) {
            // 1. Build an internal Unicode -> GID map from the font's own cmap tables
            let mut internal_u2g = BTreeMap::new();
            
            // 1.1 Extract Glyph Names if available (Essential for subsetted fonts)
            for gid in 0..face.number_of_glyphs() {
                if let Some(name) = face.glyph_name(ttf_parser::GlyphId(gid)) {
                    log::debug!("[RECONSTRUCT] Found glyph name: /{} -> GID {}", name, gid);
                    name_to_gid.insert(name.to_string(), gid as u32);
                }
            }

            for table in face.tables().cmap.iter().flat_map(|t| t.subtables) {
                if table.is_unicode() {
                    table.codepoints(|cp| {
                        if let Some(gid) = table.glyph_index(cp) {
                            internal_u2g.insert(cp, gid.0 as u32);
                        }
                    });
                } else {
                    table.codepoints(|cp| {
                        if let Some(gid) = table.glyph_index(cp) {
                            log::debug!("[RECONSTRUCT] Found non-Unicode mapping: {} -> GID {}", cp, gid.0);
                            map.insert(cp, gid.0 as u32);
                        }
                    });
                }
            }
            
            // 2. Bridge via unified_map (ToUnicode): maps logical Unicode back to character codes.
            // This allows us to find the correct GID for a character code even if the font's 
            // internal cmap is strictly Unicode or has lying Identity entries.
            // Priority: PDF metrics (ToUnicode) ALWAYS override font internal cmaps.
            for (uni_str, &code) in &resource.unified_map {
                if let Some(c) = uni_str.chars().next()
                    && let Some(&gid) = internal_u2g.get(&(c as u32)) {
                        map.insert(code, gid);
                    }
            }
        }
        if map.is_empty() { None } else { Some(map) }
    }

    fn normalize_to_sfnt(
        format: FontFormat,
        data: &[u8],
        resource: &FontResource,
    ) -> PdfResult<ReconstructedFont> {
        match format {
            FontFormat::Sfnt => {
                let mut info = Self::inspect_cff(data).unwrap_or(CffInfo::empty());
                
                // If sid_to_gid is still missing (e.g. TrueType font or CFF parse failed),
                // attempt to rescue mappings from the internal SFNT cmap table.
                if info.sid_to_gid.is_none() {
                    info.sid_to_gid = Self::rescue_sid_map_from_sfnt(data, resource, info.name_to_gid.get_or_insert_with(BTreeMap::new));
                }

                // Synthesize authoritative mappings by bridging PDF metrics with physical charset
                let (synthesized_cmap, synthesized_cid_map) = 
                    Self::synthesize_bridged_cmap(resource, data, info.sid_to_gid.as_ref(), info.name_to_gid.as_ref(), info.is_cid);

                let mut final_data = data.to_vec();
                
                // Patch the existing SFNT with the authoritative synthesized cmap
                if let Some(new_cmap_data) = synthesized_cmap
                    && let Ok(mut sfnt_dis) = Self::disassemble_sfnt(data) {
                        log::debug!("[RECONSTRUCT] Patching SFNT cmap table for {}", resource.base_font.as_str());
                        if let Some(idx) = sfnt_dis.tables.iter().position(|(t, _)| t == b"cmap") {
                            sfnt_dis.tables[idx].1 = new_cmap_data;
                        } else {
                            sfnt_dis.tables.push((*b"cmap", new_cmap_data));
                        }
                        
                        if let Ok(patched_data) = Self::assemble_sfnt(&sfnt_dis.magic, &sfnt_dis.tables) {
                            final_data = patched_data;
                        }
                    }

                let mut cid_to_gid_map = Self::build_cid_to_gid_map(&info);
                if !synthesized_cid_map.is_empty() {
                    cid_to_gid_map = Some(synthesized_cid_map);
                }

                Ok(ReconstructedFont {
                    data: final_data,
                    is_cid: info.is_cid,
                    cid_to_gid_map,
                    name_to_gid_map: info.name_to_gid,
                    sid_to_gid_map: info.sid_to_gid,
                })
            }
            FontFormat::Cff1 | FontFormat::Cff2 => {
                let tag = if format == FontFormat::Cff1 { *b"CFF " } else { *b"CFF2" };
                Self::wrap_naked_outline(tag, data, resource)
            }
            FontFormat::Type1Pfb | FontFormat::Type1Pfa => {
                Self::transcode_type1_to_cff(data, resource)
            }
            FontFormat::Unknown => {
                log::debug!("[RECONSTRUCT] Unrecognized font format, using raw data as placeholder SFNT");
                Ok(ReconstructedFont {
                    data: data.to_vec(),
                    is_cid: false,
                    cid_to_gid_map: None,
                    name_to_gid_map: None,
                    sid_to_gid_map: None,
                })
            }
        }
    }

    fn transcode_type1_to_cff(data: &[u8], resource: &FontResource) -> PdfResult<ReconstructedFont> {
        if let (Some(l1), Some(l2), Some(l3)) = (resource.length1, resource.length2, resource.length3) {
            log::info!("[RECONSTRUCT] Type 1 segments detected: L1={}, L2={}, L3={}. Length match: {}", 
                l1, l2, l3, (l1 + l2 + l3) as usize == data.len());
            // Future: Implement PFB reconstruction and CFF transcoding here.
        }
        
        // For now, we still return an error as Type 1 cannot be directly wrapped in SFNT for modern renderers.
        Err(PdfError::Other("Type 1 font transcoding to Virtual OpenType not yet implemented".into()))
    }

    fn wrap_naked_outline(
        tag: [u8; 4],
        outline_data: &[u8],
        resource: &FontResource,
    ) -> PdfResult<ReconstructedFont> {
        let mut tables = Vec::new();
        tables.push((tag, outline_data.to_vec()));
        
        let info = if tag == *b"CFF " || tag == *b"CFF2" {
            Self::inspect_cff(outline_data).unwrap_or(CffInfo::empty())
        } else {
            CffInfo::empty()
        };
        
        let num_glyphs = info.num_glyphs;

        let mut head = vec![0u8; 54];
        head[0..4].copy_from_slice(b"OTTO"); // version for CFF-based OpenType
        head[12..16].copy_from_slice(&0x5F0F3CF5u32.to_be_bytes()); // magic
        head[18..20].copy_from_slice(&1000u16.to_be_bytes()); // unitsPerEm
        tables.push((*b"head", head));

        let mut hhea = vec![0u8; 36];
        hhea[0..4].copy_from_slice(&[0, 1, 0, 0]);
        hhea[34..36].copy_from_slice(&(num_glyphs as u16).to_be_bytes());
        tables.push((*b"hhea", hhea));

        let mut maxp = vec![0u8; 32];
        maxp[0..4].copy_from_slice(&[0, 0, 0x50, 0]); // Version 0.5
        maxp[4..6].copy_from_slice(&(num_glyphs as u16).to_be_bytes());
        tables.push((*b"maxp", maxp));

        let mut hmtx = Vec::with_capacity(num_glyphs * 4);
        for gid in 0..num_glyphs {
            let width = resource.glyph_width_by_cid(gid as u32);
            hmtx.extend_from_slice(&(width as i16).to_be_bytes());
            hmtx.extend_from_slice(&0i16.to_be_bytes());
        }
        tables.push((*b"hmtx", hmtx));

        // Synthesize a minimal OS/2 table (Required for OpenType)
        let mut os2 = vec![0u8; 96];
        os2[0..2].copy_from_slice(&3u16.to_be_bytes()); // version
        os2[64..66].copy_from_slice(&400u16.to_be_bytes()); // usWeightClass (Normal)
        os2[66..68].copy_from_slice(&5u16.to_be_bytes()); // usWidthClass (Medium)
        tables.push((*b"OS/2", os2));

        // Synthesize a minimal name table
        let font_name = resource.base_font.as_str().as_bytes();
        let name_count = 1;
        let mut name_table = vec![0u8; 6 + 12 * name_count + font_name.len()];
        name_table[2..4].copy_from_slice(&(name_count as u16).to_be_bytes()); // count
        name_table[4..6].copy_from_slice(&(6 + 12 * name_count as u16).to_be_bytes()); // storage offset
        
        // Record 1: Full Name (ID 4)
        name_table[6..8].copy_from_slice(&3u16.to_be_bytes()); // platform Windows
        name_table[8..10].copy_from_slice(&1u16.to_be_bytes()); // encoding Unicode
        name_table[10..12].copy_from_slice(&0u16.to_be_bytes()); // language 0
        name_table[12..14].copy_from_slice(&4u16.to_be_bytes()); // nameID 4
        name_table[14..16].copy_from_slice(&(font_name.len() as u16).to_be_bytes()); // length
        name_table[16..18].copy_from_slice(&0u16.to_be_bytes()); // offset
        name_table[18 + 12 * (name_count - 1)..18 + 12 * (name_count - 1) + font_name.len()].copy_from_slice(font_name);
        tables.push((*b"name", name_table));

        // Synthesize a minimal post table (Version 3.0)
        let mut post = vec![0u8; 32];
        post[0..4].copy_from_slice(&[0, 0, 3, 0]); // version 3.0
        tables.push((*b"post", post));

        // Synthesize a bridged cmap table
        let (cmap_data_opt, synthesized_cid_map) = 
            Self::synthesize_bridged_cmap(resource, outline_data, info.sid_to_gid.as_ref(), info.name_to_gid.as_ref(), info.is_cid);
            
        if let Some(cmap_data) = cmap_data_opt {
            tables.push((*b"cmap", cmap_data));
        }

        let sfnt_data = if let Ok(new_data) = Self::assemble_sfnt(b"OTTO", &tables) {
            new_data
        } else {
            outline_data.to_vec()
        };

        let mut cid_to_gid_map = Self::build_cid_to_gid_map(&info);
        if !synthesized_cid_map.is_empty() {
            cid_to_gid_map = Some(synthesized_cid_map);
        }

        Ok(ReconstructedFont {
            data: sfnt_data,
            is_cid: info.is_cid,
            cid_to_gid_map,
            name_to_gid_map: info.name_to_gid,
            sid_to_gid_map: info.sid_to_gid,
        })
    }

    fn synthesize_bridged_cmap(
        resource: &FontResource,
        raw_data: &[u8],
        discovered_map: Option<&BTreeMap<u32, u32>>,
        name_to_gid: Option<&BTreeMap<String, u32>>,
        _is_cid: bool,
    ) -> (Option<Vec<u8>>, BTreeMap<u32, u32>) {
        if resource.unified_map.is_empty() {
            return (None, BTreeMap::new());
        }

        let mut internal_unicode_map = BTreeMap::new();
        let mut internal_code_map = BTreeMap::new();
        let mut cid_to_gid_map = BTreeMap::new();

        // GID Rescue: Build physical CID -> GID map by scanning all internal cmap tables
        if let Ok(face) = ttf_parser::Face::parse(raw_data, 0) {
            for table in face.tables().cmap.iter().flat_map(|t| t.subtables) {
                let is_unicode = table.is_unicode();
                table.codepoints(|cp| {
                    if let Some(gid) = table.glyph_index(cp) {
                        if is_unicode {
                            internal_unicode_map.insert(cp, gid.0 as u32);
                        } else {
                            internal_code_map.insert(cp, gid.0 as u32);
                        }
                    }
                });
            }
        }

        let mut mappings = Vec::new();
        for (uni_str, &cid) in resource.unified_map.iter() {
            let Some(c) = uni_str.chars().next() else {
                continue;
            };

            let mut actual_gid = None;
            
            // Priority -1: Identity Mapping (Authoritative for Western subsetted "Lying Identity" fonts)
            // If the font is CID-keyed, ordering is Identity, and it's not CJK, the CID is the truth.
            let is_identity = resource.cid_ordering.as_deref().is_none_or(|o| o == "Identity");
            let is_lying_identity = resource.is_cid_keyed && is_identity && !resource.is_cjk();
            
            if is_lying_identity && cid != 0 {
                actual_gid = Some(cid);
            }

            // Priority 0: Name-to-GID bridge (Highest trust for CFF fonts with custom names)
            if let Some(nmap) = name_to_gid
                && let Some(glyph_name) = resource.encoding.as_ref().and_then(|e| e.map(&[cid as u8])) {
                    let name = if glyph_name.starts_with('/') { &glyph_name[1..] } else { &glyph_name };
                    actual_gid = nmap.get(name).copied();
                }

            // Priority 1: SID/CID-to-GID map from font parsing
            if actual_gid.is_none() && let Some(map) = discovered_map {
                if resource.is_cid_keyed {
                    // For CID fonts, cid in unified_map is the actual CID
                    actual_gid = map.get(&{ cid }).copied();
                } else {
                    // For simple fonts, cid is the character code.
                    actual_gid = map.get(&{ cid }).copied();
                }
            }

            // Priority 2: Original font cmap tables (via ttf-parser)
            if actual_gid.is_none() {
                // First try Unicode lookup (standard)
                actual_gid = internal_unicode_map.get(&(c as u32)).copied();
                
                // Then try Code/CID lookup (essential for subset TrueType with custom cmaps)
                if actual_gid.is_none() {
                    actual_gid = internal_code_map.get(&{ cid }).copied();
                }
            }

            // Priority 3: Glyph name fallback (Effective for subsets that have names but broken cmaps)
            if actual_gid.is_none() && let Some(nmap) = name_to_gid {
                // Try literal names like "e", "uni0065", etc.
                if let Some(gid) = nmap.get(&c.to_string()) {
                    actual_gid = Some(*gid);
                } else {
                    let hex_name = format!("uni{:04X}", c as u32);
                    if let Some(gid) = nmap.get(&hex_name) {
                        actual_gid = Some(*gid);
                    }
                }
            }

            // Final fallback: Resource's own mapping (Identity fallback)
            // Note: We only use the Identity fallback if we are SURE it's a CID-keyed font 
            // where CID == GID is the intended path. For subsetted Western fonts,
            // it's better to return 0 (missing) here and let resolve_gid's Unicode path take over.
            let final_gid = if let Some(gid) = actual_gid {
                gid
            } else if resource.is_cid_keyed && resource.cid_to_gid_map.is_some() {
                // If the PDF provided an explicit CIDToGIDMap, we trust it.
                resource.to_gid(cid, None)
            } else if resource.is_cid_keyed && !resource.base_font.as_str().contains('+') {
                // If it's a standard (non-subset) CID font, Identity is usually safe.
                resource.to_gid(cid, None)
            } else {
                // For subsetted fonts (+ in name), Identity is often lying. 
                // Return 0 and let Unicode path (Priority 2) handle it.
                0
            };
            
            mappings.push((c as u32, final_gid));
            cid_to_gid_map.insert(cid, final_gid);
        }

        (Self::assemble_cmap_table(&mappings), cid_to_gid_map)
    }

    fn assemble_cmap_table(mappings: &[(u32, u32)]) -> Option<Vec<u8>> {
        if mappings.is_empty() {
            return None;
        }
        let mut m = mappings.to_vec();
        m.sort_by_key(|v| v.0);
        m.dedup_by_key(|v| v.0);

        let mut cmap = Vec::new();
        cmap.extend_from_slice(&0u16.to_be_bytes()); // version
        cmap.extend_from_slice(&1u16.to_be_bytes()); // numTables
        cmap.extend_from_slice(&3u16.to_be_bytes()); // Windows
        cmap.extend_from_slice(&10u16.to_be_bytes()); // UCS-4
        cmap.extend_from_slice(&12u32.to_be_bytes()); // offset

        let mut groups = Vec::new();
        let mut cur_start = m[0].0;
        let mut cur_gid = m[0].1;
        let mut cur_len = 1;
        for &(cv, gv) in m.iter().skip(1) {
            if cv == cur_start + cur_len && gv == cur_gid + cur_len {
                cur_len += 1;
            } else {
                groups.push((cur_start, cur_start + cur_len - 1, cur_gid));
                cur_start = cv;
                cur_gid = gv;
                cur_len = 1;
            }
        }
        groups.push((cur_start, cur_start + cur_len - 1, cur_gid));

        let sub_len = 16 + (groups.len() as u32) * 12;
        cmap.extend_from_slice(&12u16.to_be_bytes()); // Format 12
        cmap.extend_from_slice(&0u16.to_be_bytes());
        cmap.extend_from_slice(&sub_len.to_be_bytes());
        cmap.extend_from_slice(&0u32.to_be_bytes());
        cmap.extend_from_slice(&(groups.len() as u32).to_be_bytes());
        for (s, e, g) in groups {
            cmap.extend_from_slice(&s.to_be_bytes());
            cmap.extend_from_slice(&e.to_be_bytes());
            cmap.extend_from_slice(&g.to_be_bytes());
        }
        Some(cmap)
    }

    fn disassemble_sfnt(sfnt: &[u8]) -> PdfResult<DisassembledSfnt> {
        if sfnt.len() < 12 {
            return Err(PdfError::Internal("SFNT too short".into()));
        }

        let mut base_offset = 0;
        let mut magic = [0u8; 4];
        magic.copy_from_slice(&sfnt[0..4]);

        if &magic == b"ttcf" {
            if sfnt.len() < 12 {
                return Err(PdfError::Internal("TTC header too short".into()));
            }
            let num_fonts = u32::from_be_bytes([sfnt[8], sfnt[9], sfnt[10], sfnt[11]]) as usize;
            if num_fonts == 0 {
                return Err(PdfError::Internal("TTC contains no fonts".into()));
            }
            base_offset = u32::from_be_bytes([sfnt[12], sfnt[13], sfnt[14], sfnt[15]]) as usize;
            if base_offset + 12 > sfnt.len() {
                return Err(PdfError::Internal("TTC offset out of bounds".into()));
            }
            magic.copy_from_slice(&sfnt[base_offset..base_offset + 4]);
        }

        let num_tables = u16::from_be_bytes([sfnt[base_offset + 4], sfnt[base_offset + 5]]) as usize;
        let mut tables = Vec::new();
        for i in 0..num_tables {
            let entry = base_offset + 12 + i * 16;
            if entry + 16 > sfnt.len() {
                break;
            }
            let mut tag = [0; 4];
            tag.copy_from_slice(&sfnt[entry..entry + 4]);
            let offset = u32::from_be_bytes([
                sfnt[entry + 8],
                sfnt[entry + 9],
                sfnt[entry + 10],
                sfnt[entry + 11],
            ]) as usize;
            let length = u32::from_be_bytes([
                sfnt[entry + 12],
                sfnt[entry + 13],
                sfnt[entry + 14],
                sfnt[entry + 15],
            ]) as usize;
            if offset + length <= sfnt.len() {
                tables.push((tag, sfnt[offset..offset + length].to_vec()));
            }
        }
        Ok(DisassembledSfnt { magic, tables })
    }

    fn assemble_sfnt(magic: &[u8; 4], tables: &[([u8; 4], Vec<u8>)]) -> PdfResult<Vec<u8>> {
        let mut output = Vec::new();
        output.extend_from_slice(magic);
        
        let mut tables = tables.to_vec();
        tables.sort_by_key(|t| t.0);
        output.extend_from_slice(&(tables.len() as u16).to_be_bytes());
        log::debug!("[RECONSTRUCT] Assembling SFNT: tables={}, sig={:02x}{:02x}{:02x}{:02x}", 
            tables.len(), output[0], output[1], output[2], output[3]);
        let search_range = (tables.len() as f64).log2().floor().exp2() as u16 * 16;
        output.extend_from_slice(&search_range.to_be_bytes());
        output.extend_from_slice(&((tables.len() as f64).log2().floor() as u16).to_be_bytes());
        output.extend_from_slice(&(tables.len() as u16 * 16 - search_range).to_be_bytes());
        let mut offset = 12 + tables.len() * 16;
        for (tag, data) in &tables {
            output.extend_from_slice(tag);
            output.extend_from_slice(&Self::calc_checksum(data).to_be_bytes());
            output.extend_from_slice(&(offset as u32).to_be_bytes());
            output.extend_from_slice(&(data.len() as u32).to_be_bytes());
            offset += (data.len() + 3) & !3;
        }
        for (_tag, data) in &tables {
            output.extend_from_slice(data);
            let padding = (4 - (data.len() % 4)) % 4;
            output.extend(std::iter::repeat_n(0, padding));
        }
        if let Some(h_off) = find_table_range(&output, b"head") {
            let adj = h_off.0 + 8;
            if adj + 4 <= output.len() {
                output[adj..adj + 4].copy_from_slice(&[0, 0, 0, 0]);
                let sum = 0xB1B0AFBAu32.wrapping_sub(Self::calc_checksum(&output));
                output[adj..adj + 4].copy_from_slice(&sum.to_be_bytes());
            }
        }
        
        log::debug!("[RECONSTRUCT] Final SFNT Header (16 bytes): {:02x}{:02x}{:02x}{:02x} {:02x}{:02x}{:02x}{:02x} {:02x}{:02x}{:02x}{:02x} {:02x}{:02x}{:02x}{:02x}",
            output[0], output[1], output[2], output[3], output[4], output[5], output[6], output[7],
            output[8], output[9], output[10], output[11], output[12], output[13], output[14], output[15]);
            
        Ok(output)
    }

    fn calc_checksum(data: &[u8]) -> u32 {
        let mut sum: u32 = 0;
        let chunks = data.chunks_exact(4);
        let remainder = chunks.remainder();
        for chunk in chunks {
            sum = sum.wrapping_add(u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
        }
        if !remainder.is_empty() {
            let mut padded = [0u8; 4];
            padded[..remainder.len()].copy_from_slice(remainder);
            sum = sum.wrapping_add(u32::from_be_bytes(padded));
        }
        sum
    }

    fn patch_hmtx_direct(tables: &mut [([u8; 4], Vec<u8>)], resource: &FontResource) {
        if let Some(idx) = tables.iter().position(|(t, _)| t == b"hmtx") {
            let hmtx = &mut tables[idx].1;
            let n = hmtx.len() / 4;
            for gid in 0..n {
                let w = resource.glyph_width_by_cid(gid as u32);
                let off = gid * 4;
                if off + 2 <= hmtx.len() {
                    let b = (w as i16).to_be_bytes();
                    hmtx[off] = b[0];
                    hmtx[off + 1] = b[1];
                }
            }
        }
    }
}

pub struct CffInfo {
    pub num_glyphs: usize,
    pub sid_to_gid: Option<BTreeMap<u32, u32>>,
    pub name_to_gid: Option<BTreeMap<String, u32>>,
    pub is_cid: bool,
    pub string_index: Vec<String>,
}

impl CffInfo {
    pub fn empty() -> Self {
        Self {
            num_glyphs: 1,
            sid_to_gid: None,
            name_to_gid: None,
            is_cid: false,
            string_index: Vec::new(),
        }
    }
}

impl FontReconstructor {
    pub fn inspect_cff(data: &[u8]) -> Result<CffInfo, Box<dyn std::error::Error>> {
        let cff_data = Self::extract_cff_stream(data)?;
        if cff_data.len() < 10 {
            return Ok(CffInfo::empty());
        }

        // 1. Parse Header and Basic Indices
        let mut pos = cff_data[2] as usize;
        pos = skip_index(cff_data, pos); // Name INDEX
        let top_dict_pos = pos;
        let tc = if pos + 2 <= cff_data.len() {
            u16::from_be_bytes([cff_data[pos], cff_data[pos + 1]])
        } else {
            0
        };
        pos = skip_index(cff_data, pos); // Top DICT INDEX
        let string_idx_pos = pos;
        // 2. Parse String INDEX (Custom Glyph Names)
        let string_index = Self::parse_string_index(cff_data, string_idx_pos);
        
        // 2.5 Identify Global Subrs INDEX
        let gsubr_pos = skip_index(cff_data, string_idx_pos);
        let gsubr_count = if gsubr_pos + 2 <= cff_data.len() {
            u16::from_be_bytes([cff_data[gsubr_pos], cff_data[gsubr_pos + 1]])
        } else {
            0
        };
        log::debug!("[RECONSTRUCT] Global Subrs INDEX at {}, count: {}", gsubr_pos, gsubr_count);

        // 3. Parse Top DICT for Charset and CharStrings offsets
        let (cso, cso2, is_cid) = Self::parse_cff_top_dict(cff_data, top_dict_pos, tc);

        // 4. Determine Glyph Count and check CharStrings
        let ng = Self::determine_glyph_count(cff_data, cso);
        if let Some(o) = cso {
            log::debug!("[RECONSTRUCT] CharStrings INDEX at {}, count: {}", o, ng);
            for gid in 0..ng {
                let (off, len) = Self::get_charstring_range(cff_data, o, gid);
                if len > 0 {
                    let end = std::cmp::min(off + len, off + 32);
                    let hex = cff_data[off..end].iter().map(|b| format!("{:02X}", b)).collect::<String>();
                    log::debug!("[RECONSTRUCT] GID {} CharString: offset {}, size {} bytes, hex: {}", gid, off, len, hex);
                }
            }
        }
        log::debug!("[RECONSTRUCT] CFF glyph count (ng): {}, Top DICT count: {}, is_cid: {}", ng, tc, is_cid);

        // 5. Parse Charset (SID to GID mapping)
        let mut sid_map = BTreeMap::new();
        if let Some(o) = cso2 {
            log::debug!("[RECONSTRUCT] Found Charset offset: {}, format: {}", o, cff_data.get(o).unwrap_or(&255));
            if o < cff_data.len() && o > 2 {
                if let Some(m) = Self::parse_cff_charset(cff_data, o, ng) {
                    sid_map = m;
                }
            } else if o <= 2 {
                // Predefined Charsets (0: ISOAdobe, 1: Expert, 2: ExpertSubset)
                if o == 0 {
                    for gid in 1..std::cmp::min(ng, 229) {
                        sid_map.insert(gid as u32, gid as u32);
                    }
                }
            }
        } else {
            // Default charset handling
            Self::apply_default_charset(&mut sid_map, is_cid, ng);
        }

        // 6. Derive Glyph Name Map
        let name_to_gid = if !sid_map.is_empty() {
            Some(Self::derive_name_map(cff_data, &sid_map, string_idx_pos))
        } else {
            None
        };

        Ok(CffInfo {
            num_glyphs: ng as usize,
            sid_to_gid: Some(sid_map),
            name_to_gid,
            is_cid,
            string_index,
        })
    }

    fn build_cid_to_gid_map(info: &CffInfo) -> Option<BTreeMap<u32, u32>> {
        if info.is_cid {
            info.sid_to_gid.clone()
        } else {
            None
        }
    }

    fn extract_cff_stream(data: &[u8]) -> Result<&[u8], Box<dyn std::error::Error>> {
        let is_sfnt =
            data.len() >= 4 && (data.starts_with(b"OTTO") || data.starts_with(&[0, 1, 0, 0]));
        if is_sfnt {
            if let Some((o, e)) = find_table_range(data, b"CFF ") {
                log::debug!("[RECONSTRUCT] Found CFF table at {}-{} (size: {})", o, e, e - o);
                Ok(&data[o..e])
            } else if let Some((o, e)) = find_table_range(data, b"CFF2") {
                log::debug!("[RECONSTRUCT] Found CFF2 table at {}-{} (size: {})", o, e, e - o);
                Ok(&data[o..e])
            } else {
                log::warn!("[RECONSTRUCT] CFF table not found in SFNT container");
                Err("CFF table not found in SFNT container".into())
            }
        } else {
            Ok(data)
        }
    }

    fn parse_string_index(data: &[u8], pos: usize) -> Vec<String> {
        let mut string_index = Vec::new();
        let str_count = if pos + 2 <= data.len() {
            u16::from_be_bytes([data[pos], data[pos + 1]]) as usize
        } else {
            0
        };
        for i in 0..str_count {
            if let Some(item) = get_index_item(data, pos, i) {
                string_index.push(String::from_utf8_lossy(&item).to_string());
            }
        }
        string_index
    }

    fn determine_glyph_count(data: &[u8], char_strings_offset: Option<usize>) -> u16 {
        if let Some(o) = char_strings_offset {
            if o + 2 <= data.len() {
                u16::from_be_bytes([data[o], data[o + 1]])
            } else {
                1024
            }
        } else {
            1024
        }
    }

    fn get_charstring_range(data: &[u8], index_offset: usize, gid: u16) -> (usize, usize) {
        if index_offset + 2 > data.len() {
            return (0, 0);
        }
        let count = u16::from_be_bytes([data[index_offset], data[index_offset + 1]]) as usize;
        if gid as usize >= count {
            return (0, 0);
        }

        let off_size = data[index_offset + 2] as usize;
        let offsets_pos = index_offset + 3;
        
        let get_offset = |idx: usize| -> usize {
            let p = offsets_pos + idx * off_size;
            let mut val = 0usize;
            for j in 0..off_size {
                if p + j < data.len() {
                    val = (val << 8) | data[p + j] as usize;
                }
            }
            val
        };

        let start = get_offset(gid as usize);
        let end = get_offset(gid as usize + 1);
        
        // Data starts after the offsets and the 2-byte count + 1-byte offSize
        // Actually, CFF INDEX data starts after count (2), offSize (1), and (count+1)*offSize offsets.
        let data_start_base = index_offset + 3 + (count + 1) * off_size - 1; 
        // Wait! CFF offsets in INDEX are relative to the byte BEFORE the first data byte.
        // So offset 1 is the first byte of data.
        
        if start == 0 || end < start {
            (0, 0)
        } else {
            (data_start_base + start, end - start)
        }
    }

    fn apply_default_charset(map: &mut BTreeMap<u32, u32>, is_cid: bool, num_glyphs: u16) {
        if is_cid {
            for gid in 0..num_glyphs {
                map.insert(gid as u32, gid as u32);
            }
        } else {
            // ISOAdobe fallback for simple fonts
            for gid in 1..std::cmp::min(num_glyphs, 229) {
                map.insert(gid as u32, gid as u32);
            }
        }
    }

    fn derive_name_map(
        data: &[u8],
        sid_map: &BTreeMap<u32, u32>,
        string_idx_pos: usize,
    ) -> BTreeMap<String, u32> {
        let mut nm = BTreeMap::new();
        use crate::font::cff_standard::CFF_STANDARD_STRINGS;

        for (&sid, &gid) in sid_map {
            let name = if sid <= CFF_LAST_STANDARD_SID {
                CFF_STANDARD_STRINGS[sid as usize].to_string()
            } else {
                let custom_idx = (sid - (CFF_LAST_STANDARD_SID + 1)) as usize;
                if let Some(item) = get_index_item(data, string_idx_pos, custom_idx) {
                    String::from_utf8_lossy(&item).to_string()
                } else {
                    format!("c{:03}", sid)
                }
            };
            nm.insert(name, gid);
        }
        nm
    }

    fn parse_cff_top_dict(
        data: &[u8],
        start: usize,
        count: u16,
    ) -> (Option<usize>, Option<usize>, bool) {
        let mut cso = None;
        let mut cso2 = None;
        let mut is_cid = false;
        if count > 0
            && let Some(dd) = get_index_item(data, start, 0)
        {
            let mut dpos = 0;
            let mut ops = Vec::new();
            while dpos < dd.len() {
                let b0 = dd[dpos];
                if b0 <= 21 {
                    let mut op = b0 as u16;
                    dpos += 1;
                    if op == 12 && dpos < dd.len() {
                        op = (op << 8) | dd[dpos] as u16;
                        dpos += 1;
                    }
                    log::debug!("[RECONSTRUCT] Top DICT Op: 0x{:04X}, values: {:?}", op, ops);
                    match op {
                        17 => cso = ops.last().copied().map(|v| v as usize),
                        18 => {
                            if ops.len() >= 2 {
                                let size = ops[ops.len()-2] as usize;
                                let offset = ops[ops.len()-1] as usize;
                                log::debug!("[RECONSTRUCT] Private DICT: offset {}, size {}", offset, size);
                            }
                        }
                        0x0C24 => {
                            let offset = ops.last().copied().unwrap_or(0) as usize;
                            log::debug!("[RECONSTRUCT] FDArray: offset {}", offset);
                            if offset > 0 && offset < data.len() {
                                let (_, count, _) = Self::parse_index_header(data, offset).unwrap_or((0, 0, 0));
                                for i in 0..count {
                                    if let Some(fd) = get_index_item(data, offset, i.into()) {
                                        let mut fdp = 0;
                                        let mut fdops = Vec::new();
                                        while fdp < fd.len() {
                                            let b0 = fd[fdp];
                                            if b0 <= 21 {
                                                let mut op = b0 as u16;
                                                fdp += 1;
                                                if op == 12 && fdp < fd.len() {
                                                    op = (op << 8) | fd[fdp] as u16;
                                                    fdp += 1;
                                                }
                                                if op == 18 && fdops.len() >= 2 {
                                                    let size = fdops[fdops.len()-2] as usize;
                                                    let off = fdops[fdops.len()-1] as usize;
                                                    log::debug!("[RECONSTRUCT] Font DICT {}: Private DICT at {}, size {}", i, off, size);
                                                    // Check Private DICT for Local Subrs (Op 19)
                                                    if off + size <= data.len() {
                                                        let priv_data = &data[off..off+size];
                                                        let mut pp = 0;
                                                        let mut pops = Vec::new();
                                                        while pp < priv_data.len() {
                                                            let pb0 = priv_data[pp];
                                                            if pb0 <= 21 {
                                                                let pop = pb0 as u16;
                                                                pp += 1;
                                                                log::debug!("[RECONSTRUCT]   Private DICT Op: {}, values: {:?}", pop, pops);
                                                                if pop == 19 {
                                                                    let sub_off = pops.last().copied().unwrap_or(0) as i64;
                                                                    log::debug!("[RECONSTRUCT]   Local Subrs at {} (relative {} from Private DICT at {})", (off as i64 + sub_off), sub_off, off);
                                                                }
                                                                pops.clear();
                                                            } else {
                                                                let (v, l) = parse_dict_number(&priv_data[pp..]);
                                                                pops.push(v);
                                                                pp += l;
                                                            }
                                                        }
                                                    }
                                                }
                                                fdops.clear();
                                            } else {
                                                let (v, l) = parse_dict_number(&fd[fdp..]);
                                                fdops.push(v);
                                                fdp += l;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        15 => cso2 = ops.last().copied().map(|v| v as usize),
                        0x0C1E | 0x0C1F | 0x0C22 | 0x0C23 => is_cid = true,
                        _ => {}
                    }
                    ops.clear();
                } else {
                    let (v, l) = parse_dict_number(&dd[dpos..]);
                    ops.push(v);
                    dpos += l;
                }
            }
        }
        (cso, cso2, is_cid)
    }

    fn parse_cff_charset(data: &[u8], off: usize, num_glyphs: u16) -> Option<BTreeMap<u32, u32>> {
        let mut map = BTreeMap::new();
        let format = data[off];
        let mut cpos = off + 1;
        if format == 0 {
            for gid in 1..num_glyphs {
                if cpos + 2 > data.len() {
                    break;
                }
                let cid = u16::from_be_bytes([data[cpos], data[cpos + 1]]);
                if (880..=890).contains(&gid) || gid <= 10 {
                    log::debug!("[RECONSTRUCT] GID {} -> CID {}", gid, cid);
                }
                if cid == 887 {
                    log::debug!("[RECONSTRUCT] Found CID 887 (Format 0) -> GID {}", gid);
                }
                map.insert(cid as u32, gid as u32);
                cpos += 2;
            }
        } else if format == 1 || format == 2 {
            let mut gid = 1;
            while gid < num_glyphs {
                let sz = if format == 1 { 3 } else { 4 };
                if cpos + sz > data.len() {
                    break;
                }
                let fc = u16::from_be_bytes([data[cpos], data[cpos + 1]]);
                let nl = if format == 1 {
                    data[cpos + 2] as u16
                } else {
                    u16::from_be_bytes([data[cpos + 2], data[cpos + 3]])
                };
                cpos += sz;
                for i in 0..=nl {
                    if (fc as u32 + i as u32) < 65536 {
                        let cid = fc as u32 + i as u32;
                        if cid == 887 {
                            log::debug!("[RECONSTRUCT] Found CID 887 -> GID {}", gid);
                        }
                        map.insert(cid, gid as u32);
                    }
                    gid += 1;
                    if gid >= num_glyphs {
                        break;
                    }
                }
            }
        }
        Some(map)
    }

    fn parse_index_header(data: &[u8], pos: usize) -> Option<(usize, u16, usize)> {
        if pos + 2 > data.len() {
            return None;
        }
        let count = u16::from_be_bytes([data[pos], data[pos + 1]]);
        if count == 0 {
            return Some((pos + 2, 0, 0));
        }
        if pos + 3 > data.len() {
            return None;
        }
        let off_size = data[pos + 2] as usize;
        Some((pos + 3, count, off_size))
    }
}

fn skip_index(data: &[u8], pos: usize) -> usize {
    if pos + 2 > data.len() {
        return pos;
    }
    let count = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
    if count == 0 {
        return pos + 2;
    }
    let os = data[pos + 2] as usize;
    let is = 2 + 1 + (count + 1) * os;
    if pos + is > data.len() {
        return pos;
    }
    let lo = pos + 3 + count * os;
    let mut off = 0;
    for j in 0..os {
        off = (off << 8) | data[lo + j] as usize;
    }
    pos + is + off - 1
}

fn get_index_item(data: &[u8], ip: usize, i: usize) -> Option<Vec<u8>> {
    let count = u16::from_be_bytes([data[ip], data[ip + 1]]) as usize;
    if i >= count {
        return None;
    }
    let os = data[ip + 2] as usize;
    let mut s = 0;
    let mut e = 0;
    let sp = ip + 3 + i * os;
    let ep = sp + os;
    for j in 0..os {
        s = (s << 8) | data[sp + j] as usize;
    }
    for j in 0..os {
        e = (e << 8) | data[ep + j] as usize;
    }
    let ds = ip + 3 + (count + 1) * os + s - 1;
    let de = ip + 3 + (count + 1) * os + e - 1;
    if de <= data.len() { Some(data[ds..de].to_vec()) } else { None }
}


fn parse_dict_number(d: &[u8]) -> (i32, usize) {
    let b0 = d[0];
    if b0 == 30 {
        let mut len = 1;
        while len < d.len() {
            let b = d[len];
            len += 1;
            if (b & 0x0F) == 0x0F || (b >> 4) == 0x0F {
                break;
            }
        }
        (0, len)
    } else if b0 == 28 {
        (u16::from_be_bytes([d[1], d[2]]) as i16 as i32, 3)
    } else if b0 == 29 {
        (i32::from_be_bytes([d[1], d[2], d[3], d[4]]), 5)
    } else if (32..=246).contains(&b0) {
        (b0 as i32 - 139, 1)
    } else if (247..=250).contains(&b0) {
        ((b0 as i32 - 247) * 256 + d[1] as i32 + 108, 2)
    } else if (251..=254).contains(&b0) {
        (-(b0 as i32 - 251) * 256 - d[1] as i32 - 108, 2)
    } else {
        (0, 1)
    }
}

fn find_table_range(s: &[u8], t: &[u8; 4]) -> Option<(usize, usize)> {
    if s.len() < 12 {
        return None;
    }
    let nt = u16::from_be_bytes([s[4], s[5]]) as usize;
    for i in 0..nt {
        let e = 12 + i * 16;
        if e + 16 > s.len() {
            break;
        }
        if &s[e..e + 4] == t {
            let o = u32::from_be_bytes([s[e + 8], s[e + 9], s[e + 10], s[e + 11]]) as usize;
            let l = u32::from_be_bytes([s[e + 12], s[e + 13], s[e + 14], s[e + 15]]) as usize;
            return Some((o, o + l));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_format_detection() {
        // SFNT
        assert_eq!(FontFormat::detect(b"OTTO"), FontFormat::Sfnt);
        assert_eq!(FontFormat::detect(&[0, 1, 0, 0]), FontFormat::Sfnt);
        assert_eq!(FontFormat::detect(b"ttcf"), FontFormat::Sfnt);
        assert_eq!(FontFormat::detect(b"true"), FontFormat::Sfnt);

        // CFF
        assert_eq!(FontFormat::detect(&[1, 0, 4, 1]), FontFormat::Cff1);
        assert_eq!(FontFormat::detect(&[2, 0, 5]), FontFormat::Cff2);

        // Type 1
        assert_eq!(FontFormat::detect(&[0x80, 0x01, 0x01]), FontFormat::Type1Pfb);
        assert_eq!(FontFormat::detect(b"%!PS-AdobeFont"), FontFormat::Type1Pfa);
        assert_eq!(FontFormat::detect(b"%!FontType1"), FontFormat::Type1Pfa);

        // Unknown
        assert_eq!(FontFormat::detect(b"abc"), FontFormat::Unknown);
        assert_eq!(FontFormat::detect(&[0x12, 0x34]), FontFormat::Unknown);
    }

    #[test]
    fn test_cff2_wrapping() {
        let dummy_cff2 = vec![2, 0, 5, 1, 2, 3, 4, 5];
        let resource = FontResource::new_test();
        
        let res = FontReconstructor::wrap_naked_outline(*b"CFF2", &dummy_cff2, &resource).unwrap();
        assert_eq!(FontFormat::detect(&res.data), FontFormat::Sfnt);
        
        let dis = FontReconstructor::disassemble_sfnt(&res.data).unwrap();
        assert!(dis.tables.iter().any(|(t, _)| t == b"CFF2"));
    }

    #[test]
    fn test_ttcf_disassembly() {
        let mut ttcf = vec![0; 64];
        ttcf[0..4].copy_from_slice(b"ttcf");
        ttcf[8..12].copy_from_slice(&1u32.to_be_bytes()); // numFonts
        ttcf[12..16].copy_from_slice(&32u32.to_be_bytes()); // offset to first font at 32
        
        // First font header at offset 32
        ttcf[32..36].copy_from_slice(b"OTTO");
        ttcf[36..38].copy_from_slice(&1u16.to_be_bytes()); // numTables = 1
        
        // Table entry at offset 32 + 12 = 44
        ttcf[44..48].copy_from_slice(b"TEST");
        ttcf[52..56].copy_from_slice(&60u32.to_be_bytes()); // offset to data at 60
        ttcf[56..60].copy_from_slice(&4u32.to_be_bytes()); // length
        
        // Table data at offset 60
        ttcf[60..64].copy_from_slice(b"DATA");

        let dis = FontReconstructor::disassemble_sfnt(&ttcf).expect("Failed to disassemble TTCF");
        assert_eq!(dis.magic, *b"OTTO");
        assert_eq!(dis.tables.len(), 1);
        assert_eq!(dis.tables[0].0, *b"TEST");
        assert_eq!(dis.tables[0].1, b"DATA");
    }

    #[test]
    #[ignore]
    fn test_reconstructed_font_parsing() {
        let path = "exports/font-0003.otf";
        if let Ok(data) = std::fs::read(path) {
            println!("Testing font: {} ({} bytes)", path, data.len());
            match ttf_parser::Face::parse(&data, 0) {
                Ok(face) => {
                    println!("Success! Glyphs: {}", face.number_of_glyphs());
                    assert!(face.tables().cff.is_some() || face.tables().cff2.is_some());
                    
                    // Try to draw GID 1
                    let mut builder = NoopBuilder;
                    if let Some(_rect) = face.outline_glyph(ttf_parser::GlyphId(1), &mut builder) {
                        println!("GID 1 outline extraction SUCCEEDED");
                    } else {
                        panic!("GID 1 outline extraction FAILED");
                    }
                }
                Err(e) => {
                    panic!("Failed to parse reconstructed font: {:?}", e);
                }
            }
        }
    }

    struct NoopBuilder;
    impl ttf_parser::OutlineBuilder for NoopBuilder {
        fn move_to(&mut self, _: f32, _: f32) {}
        fn line_to(&mut self, _: f32, _: f32) {}
        fn quad_to(&mut self, _: f32, _: f32, _: f32, _: f32) {}
        fn curve_to(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: f32) {}
        fn close(&mut self) {}
    }
}
