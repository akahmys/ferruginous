use kurbo::{BezPath, Point, Affine};
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::raw::{FileRef, TableProvider};
use skrifa::{GlyphId, MetadataProvider};
use std::collections::BTreeMap;

pub struct KurboPen {
    path: BezPath,
}

impl Default for KurboPen {
    fn default() -> Self {
        Self::new()
    }
}

impl KurboPen {
    pub fn new() -> Self {
        Self { path: BezPath::new() }
    }
    pub fn finish(self) -> BezPath {
        self.path
    }
}

impl OutlinePen for KurboPen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.path.move_to(Point::new(x as f64, y as f64));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.path.line_to(Point::new(x as f64, y as f64));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.path.quad_to(Point::new(x1 as f64, y1 as f64), Point::new(x as f64, y as f64));
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.path.curve_to(
            Point::new(x1 as f64, y1 as f64),
            Point::new(x2 as f64, y2 as f64),
            Point::new(x as f64, y as f64),
        );
    }
    fn close(&mut self) {
        self.path.close_path();
    }
}

pub struct TextLayoutOptions {
    pub font_size: f32,
    pub char_spacing: f32,
    pub word_spacing: f32,
    pub horizontal_scaling: f32, // Percentage (100.0)
}

impl Default for TextLayoutOptions {
    fn default() -> Self {
        Self { font_size: 1.0, char_spacing: 0.0, word_spacing: 0.0, horizontal_scaling: 100.0 }
    }
}

pub struct SkrifaBridge {
    pub primary_system_font: Option<Vec<u8>>,
    glyph_cache: BTreeMap<(u32, u32), BezPath>,
}

impl SkrifaBridge {
    pub fn new(primary_system_font: Option<Vec<u8>>) -> Self {
        Self {
            primary_system_font,
            glyph_cache: BTreeMap::new(),
        }
    }

    pub fn get_units_per_em(&self, data: &[u8]) -> Option<u16> {
        let sfnt_data = ensure_sfnt(data);
        let data_ref = sfnt_data.as_deref().unwrap_or(data);
        if let Ok(file) = FileRef::new(data_ref) {
            let font_opt = match file {
                FileRef::Font(f) => Some(f),
                FileRef::Collection(c) => c.get(0).ok(),
            };
            if let Some(font) = font_opt {
                return Some(font.head().ok()?.units_per_em());
            }
        }
        None
    }

    #[allow(clippy::too_many_arguments)]
    pub fn extract_path(
        &mut self,
        data: &[u8],
        gid: u32,
        char_code: u32,
        cid_to_gid_map: Option<&[u16]>,
        is_vertical: bool,
        unicode_fallback: Option<char>,
        is_japanese: bool,
        force_system_fallback: bool,
        system_font: Option<&skrifa::FontRef>,
        primary_font: Option<&skrifa::FontRef>,
    ) -> Option<BezPath> {
        if let Some(path) = self.glyph_cache.get(&(gid, char_code)) {
            return Some(path.clone());
        }

        let res = self.extract_path_inner(
            data,
            gid,
            char_code,
            cid_to_gid_map,
            is_vertical,
            unicode_fallback,
            is_japanese,
            force_system_fallback,
            system_font,
            primary_font,
        );

        if let Some(ref path) = res {
            self.glyph_cache.insert((gid, char_code), path.clone());
        }
        res
    }

    #[allow(clippy::too_many_arguments)]
    fn extract_path_inner(
        &self,
        data: &[u8],
        gid: u32,
        char_code: u32,
        cid_to_gid_map: Option<&[u16]>,
        _is_vertical: bool,
        unicode_fallback: Option<char>,
        is_japanese: bool,
        force_system_fallback: bool,
        system_font: Option<&skrifa::FontRef>,
        primary_font: Option<&skrifa::FontRef>,
    ) -> Option<BezPath> {
        if !force_system_fallback
            && let Some(f) = primary_font {
            let mut target_gid = gid;
            if let Some(map) = cid_to_gid_map
                && (gid as usize) < map.len() {
                target_gid = map[gid as usize] as u32;
            }
            
            let sfnt_data = ensure_sfnt(data);
            let sfnt = sfnt_data.as_deref().unwrap_or(data);
            
            // Special handling for CID fonts (wrapped CFF or standard OTTO)
            if ((sfnt.len() >= 2 && sfnt[0] == 0x01 && sfnt[1] == 0x00)
                || (sfnt.len() >= 4 && &sfnt[0..4] == b"OTTO"))
                && let Some(cff_data) = extract_cff_from_sfnt(sfnt)
                && let Some(resolved) = cff_get_gid_for_cid(cff_data, gid as u16) {
                target_gid = resolved as u32;
            }

            if let Some(glyph) = f.outline_glyphs().get(skrifa::GlyphId::new(target_gid)) {
                let mut pen = KurboPen::new();
                if glyph.draw(DrawSettings::unhinted(Size::new(1000.0), LocationRef::default()), &mut pen).is_ok() {
                    return Some(pen.finish());
                }
            }
        }

        // Stage 3: System Fallback
        if let Some(mut uch) = unicode_fallback.or_else(|| std::char::from_u32(char_code)) {
            // Special case: If we have a Plane-15 PUA from our Identity fallback,
            // recover the original CID.
            let mut recovered_cid = None;
            let u_val = uch as u32;
            if u_val >= 0xF0000 {
                let cid = u_val - 0xF0000;
                recovered_cid = Some(cid);
                
                // Try to resolve Unicode from CID using Adobe-Japan1-UCS2 if it was a Japanese font
                if is_japanese
                    && let Some(ucs2) = ferruginous_core::font::cmap::CMap::load_named("Adobe-Japan1-UCS2") {
                    let cid_bytes = vec![(cid >> 8) as u8, (cid & 0xFF) as u8];
                    if let Some(s) = ucs2.map(&cid_bytes)
                        && let Some(c) = s.chars().next() {
                        uch = c;
                    }
                }
            }

            if let Some(font) = system_font {
                let mut final_gid = font.charmap().map(uch);
                
                // If Unicode mapping failed but we have a CID and it's a Japanese font,
                // try direct CID-to-GID mapping if the system font is AJ1-compatible.
                if final_gid.is_none() && is_japanese && let Some(cid) = recovered_cid {
                    // Heuristic: For many Japanese system fonts (Hiragino), GID 0-8720 roughly match AJ1 CIDs.
                    // This is a last resort but better than tofu.
                    final_gid = Some(skrifa::GlyphId::new(cid));
                }

                if let Some(gid) = final_gid
                    && let Some(glyph) = font.outline_glyphs().get(gid) {
                    let mut pen = KurboPen::new();
                    if glyph.draw(DrawSettings::unhinted(Size::new(1000.0), LocationRef::default()), &mut pen).is_ok() {
                        return Some(pen.finish());
                    }
                }
            }
        }

        // Stage 4: Final Tofu Fallback
        if let Some(font) = system_font
            && let Some(glyph) = font.outline_glyphs().get(GlyphId::new(0)) {
            let mut pen = KurboPen::new();
            if glyph.draw(DrawSettings::unhinted(Size::new(1000.0), LocationRef::default()), &mut pen).is_ok() {
                return Some(pen.finish());
            }
        }

        None
    }

    pub fn render_glyphs(
        &mut self,
        font_data: &[u8],
        glyphs: &[(u32, f32)],
        options: &TextLayoutOptions,
    ) -> BezPath {
        let mut combined_path = BezPath::new();
        let mut x_offset = 0.0;
        let scale = options.font_size / 1000.0;
        let h_scale = options.horizontal_scaling / 100.0;

        for (gid, width) in glyphs {
            if let Some(mut path) = self.extract_path(font_data, *gid, *gid, None, false, None, false, false, None, None) {
                let transform = Affine::translate((x_offset, 0.0))
                    * Affine::scale_non_uniform(scale as f64 * h_scale as f64, scale as f64);
                path.apply_affine(transform);
                combined_path.extend(path);
            }
            x_offset += *width as f64 * scale as f64 * h_scale as f64;
        }
        combined_path
    }
}

impl Default for SkrifaBridge {
    fn default() -> Self {
        Self::new(None)
    }
}

pub fn ensure_sfnt(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 4 {
        return None;
    }
    
    let tag = &data[0..4];
    if tag == b"OTTO" || tag == [0, 1, 0, 0] || tag == b"true" || tag == b"typ1" {
        return Some(data.to_vec());
    }

    if data[0] == 0x01 && data[1] == 0x00 {
        let cff_len = data.len() as u32;
        let cmap_len = 262u32 + 12u32;
        let head_len = 54u32;
        let hhea_len = 36u32;
        let hmtx_len = 4u32;
        let maxp_len = 6u32;
        let num_glyphs = cff_get_num_glyphs(data);

        let mut sfnt = Vec::with_capacity(data.len() + 1024);
        let offsets = write_sfnt_directory(
            &mut sfnt, cff_len, cmap_len, head_len, hhea_len, hmtx_len, maxp_len,
        );

        sfnt.resize(offsets.total_len as usize, 0);
        sfnt[offsets.cff as usize..offsets.cff as usize + data.len()].copy_from_slice(data);

        write_cmap_table_v0(&mut sfnt, offsets.cmap);
        write_head_table(&mut sfnt, offsets.head);
        write_hhea_table(&mut sfnt, offsets.hhea, num_glyphs);
        write_hmtx_table(&mut sfnt, offsets.hmtx);
        write_maxp_table(&mut sfnt, offsets.maxp, num_glyphs);

        // Update checksums in the directory
        update_table_checksums(&mut sfnt, 6);

        return Some(sfnt);
    }
    None
}

struct TableOffsets {
    cff: u32,
    cmap: u32,
    head: u32,
    hhea: u32,
    hmtx: u32,
    maxp: u32,
    total_len: u32,
}

fn write_sfnt_directory(
    sfnt: &mut Vec<u8>,
    cff_len: u32,
    cmap_len: u32,
    head_len: u32,
    hhea_len: u32,
    hmtx_len: u32,
    maxp_len: u32,
) -> TableOffsets {
    // SFNT Header
    sfnt.extend_from_slice(b"OTTO");
    sfnt.extend_from_slice(&6u16.to_be_bytes()); // numTables
    sfnt.extend_from_slice(&64u16.to_be_bytes()); // searchRange
    sfnt.extend_from_slice(&2u16.to_be_bytes()); // entrySelector
    sfnt.extend_from_slice(&32u16.to_be_bytes()); // rangeShift

    let mut current_offset: u32 = 12 + 6 * 16;
    
    // Helper to add table entry
    let mut add_table = |sfnt: &mut Vec<u8>, tag: &[u8; 4], len: u32| -> u32 {
        let offset = current_offset;
        sfnt.extend_from_slice(tag);
        sfnt.extend_from_slice(&0u32.to_be_bytes()); // Checksum (placeholder)
        sfnt.extend_from_slice(&offset.to_be_bytes());
        sfnt.extend_from_slice(&len.to_be_bytes());
        current_offset = (current_offset + len + 3) & !3; // Align to 4 bytes
        offset
    };

    let cff = add_table(sfnt, b"CFF ", cff_len);
    let cmap = add_table(sfnt, b"cmap", cmap_len);
    let head = add_table(sfnt, b"head", head_len);
    let hhea = add_table(sfnt, b"hhea", hhea_len);
    let hmtx = add_table(sfnt, b"hmtx", hmtx_len);
    let maxp = add_table(sfnt, b"maxp", maxp_len);

    TableOffsets {
        cff,
        cmap,
        head,
        hhea,
        hmtx,
        maxp,
        total_len: current_offset,
    }
}

fn write_cmap_table_v0(sfnt: &mut [u8], offset: u32) {
    let o = offset as usize;
    sfnt[o..o+2].copy_from_slice(&0u16.to_be_bytes()); // version
    sfnt[o+2..o+4].copy_from_slice(&1u16.to_be_bytes()); // numTables
    sfnt[o+4..o+6].copy_from_slice(&1u16.to_be_bytes()); // platformID (Macintosh)
    sfnt[o+6..o+8].copy_from_slice(&0u16.to_be_bytes()); // encodingID (Roman)
    sfnt[o+8..o+12].copy_from_slice(&12u32.to_be_bytes()); // subtableOffset

    let so = o + 12;
    sfnt[so..so+2].copy_from_slice(&0u16.to_be_bytes()); // format (Mac Roman)
    sfnt[so+2..so+4].copy_from_slice(&262u16.to_be_bytes()); // length
    sfnt[so+4..so+6].copy_from_slice(&0u16.to_be_bytes()); // language
    for i in 0..256 {
        sfnt[so+6+i] = i as u8; // identity map
    }
}

fn write_head_table(sfnt: &mut [u8], offset: u32) {
    let o = offset as usize;
    sfnt[o..o+4].copy_from_slice(&0x00010000u32.to_be_bytes()); // version
    sfnt[o+4..o+8].copy_from_slice(&0x00010000u32.to_be_bytes()); // fontRevision
    sfnt[o+8..o+12].copy_from_slice(&0u32.to_be_bytes()); // checkSumAdjustment
    sfnt[o+12..o+16].copy_from_slice(&0x5F0F3CF5u32.to_be_bytes()); // magicNumber
    sfnt[o+16..o+18].copy_from_slice(&0u16.to_be_bytes()); // flags
    sfnt[o+18..o+20].copy_from_slice(&1000u16.to_be_bytes()); // unitsPerEm
    // Skip creation/modification times
    sfnt[o+36..o+38].copy_from_slice(&(-1000i16).to_be_bytes()); // xMin
    sfnt[o+38..o+40].copy_from_slice(&(-1000i16).to_be_bytes()); // yMin
    sfnt[o+40..o+42].copy_from_slice(&2000i16.to_be_bytes()); // xMax
    sfnt[o+42..o+44].copy_from_slice(&2000i16.to_be_bytes()); // yMax
    sfnt[o+44..o+46].copy_from_slice(&0u16.to_be_bytes()); // macStyle
    sfnt[o+46..o+48].copy_from_slice(&0u16.to_be_bytes()); // lowestRecPPEM
    sfnt[o+48..o+50].copy_from_slice(&2u16.to_be_bytes()); // fontDirectionHint
    sfnt[o+50..o+52].copy_from_slice(&0u16.to_be_bytes()); // indexToLocFormat
    sfnt[o+52..o+54].copy_from_slice(&0u16.to_be_bytes()); // glyphDataFormat
}

fn write_hhea_table(sfnt: &mut [u8], offset: u32, num_glyphs: u16) {
    let o = offset as usize;
    sfnt[o..o+4].copy_from_slice(&0x00010000u32.to_be_bytes()); // version
    sfnt[o+4..o+6].copy_from_slice(&800i16.to_be_bytes()); // ascender
    sfnt[o+6..o+8].copy_from_slice(&(-200i16).to_be_bytes()); // descender
    sfnt[o+8..o+10].copy_from_slice(&0i16.to_be_bytes()); // lineGap
    sfnt[o+10..o+12].copy_from_slice(&1000u16.to_be_bytes()); // advanceWidthMax
    sfnt[o+34..o+36].copy_from_slice(&num_glyphs.to_be_bytes()); // numberOfHMetrics
}

fn write_hmtx_table(sfnt: &mut [u8], offset: u32) {
    let o = offset as usize;
    sfnt[o..o+2].copy_from_slice(&1000u16.to_be_bytes()); // advanceWidth
    sfnt[o+2..o+4].copy_from_slice(&0i16.to_be_bytes()); // lsb
}

fn write_maxp_table(sfnt: &mut [u8], offset: u32, num_glyphs: u16) {
    let o = offset as usize;
    sfnt[o..o+4].copy_from_slice(&0x00005000u32.to_be_bytes()); // version
    sfnt[o+4..o+6].copy_from_slice(&num_glyphs.to_be_bytes()); // numGlyphs
}

fn update_table_checksums(sfnt: &mut [u8], num_tables: usize) {
    for i in 0..num_tables {
        let p = 12 + i * 16;
        if p + 16 > sfnt.len() { break; }
        let offset = u32::from_be_bytes([sfnt[p+8], sfnt[p+9], sfnt[p+10], sfnt[p+11]]) as usize;
        let length = u32::from_be_bytes([sfnt[p+12], sfnt[p+13], sfnt[p+14], sfnt[p+15]]) as usize;
        if offset + length <= sfnt.len() {
            let checksum = calc_checksum(&sfnt[offset..offset + length]);
            sfnt[p+4..p+8].copy_from_slice(&checksum.to_be_bytes());
        }
    }
}

fn calc_checksum(data: &[u8]) -> u32 {
    let mut sum = 0u32;
    let mut i = 0;
    while i + 4 <= data.len() {
        sum = sum.wrapping_add(u32::from_be_bytes([data[i], data[i+1], data[i+2], data[i+3]]));
        i += 4;
    }
    if i < data.len() {
        let mut last = [0u8; 4];
        last[0..data.len() - i].copy_from_slice(&data[i..]);
        sum = sum.wrapping_add(u32::from_be_bytes(last));
    }
    sum
}

fn cff_get_num_glyphs(data: &[u8]) -> u16 {
    if data.len() < 4 { return 0; }
    let charstrings_idx_pos = match cff_find_charstrings_pos(data) {
        Some(p) => p,
        None => return 0,
    };
    if charstrings_idx_pos + 2 > data.len() { return 0; }
    u16::from_be_bytes([data[charstrings_idx_pos], data[charstrings_idx_pos + 1]])
}

fn cff_find_charstrings_pos(data: &[u8]) -> Option<usize> {
    if data.len() < 4 { return None; }
    let _header_len = data[2] as usize;
    let name_idx_pos = 4;
    let top_dict_idx_pos = cff_skip_index(data, name_idx_pos)?;
    
    // Top Dict Index should contain exactly one dict for simple fonts
    let pos = top_dict_idx_pos;
    let count = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
    if count == 0 { return None; }
    let off_size = data[pos + 2] as usize;
    let offset1 = cff_get_offset(&data[pos + 3..], off_size)?;
    let offset2 = cff_get_offset(&data[pos + 3 + off_size..], off_size)?;
    let dict_data = &data[pos + 3 + off_size * (count + 1) + offset1 as usize - 1 .. pos + 3 + off_size * (count + 1) + offset2 as usize - 1];

    let mut i = 0;
    while i < dict_data.len() {
        let b = dict_data[i];
        if b <= 21 {
            if b == 17 { // CharStrings
                // The operand should have been pushed before
                return Some(0); // Dummy, need real operand parsing
            }
            i += 1;
        } else if (32..=246).contains(&b) {
            i += 1;
        } else if (247..=250).contains(&b) {
            i += 2;
        } else if (251..=254).contains(&b) || b == 28 {
            i += 3;
        } else if b == 29 {
            i += 5;
        } else if b == 30 {
            i += 1;
            while i < dict_data.len() && (dict_data[i] & 0x0F) != 0x0F && (dict_data[i] >> 4) != 0x0F {
                i += 1;
            }
            i += 1;
        } else {
            i += 1;
        }
    }
    
    // Fallback: search for the CharStrings operator (17) manually
    let mut i = 0;
    while i < dict_data.len() {
        if dict_data[i] == 17 {
            // Find the operand before it
            let mut j = i as i32 - 1;
            while j >= 0 {
                let b = dict_data[j as usize];
                if b >= 32 { // It's an operand or part of one
                    // Simplification: assume 1-byte or 2-byte operand
                    if (32..=246).contains(&b) {
                        let val = (b as i32) - 139;
                        return Some(val as usize); // Still dummy, need full data ref
                    }
                }
                j -= 1;
            }
        }
        i += 1;
    }

    None
}

fn cff_skip_index(data: &[u8], pos: usize) -> Option<usize> {
    if pos + 2 > data.len() { return None; }
    let count = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
    if count == 0 { return Some(pos + 2); }
    let off_size = data[pos + 2] as usize;
    let total_off_size = off_size * (count + 1);
    let last_offset = cff_get_offset(&data[pos + 3 + off_size * count ..], off_size)?;
    Some(pos + 3 + total_off_size + last_offset as usize - 1)
}

fn cff_get_offset(data: &[u8], size: usize) -> Option<u32> {
    if data.len() < size { return None; }
    match size {
        1 => Some(data[0] as u32),
        2 => Some(u16::from_be_bytes([data[0], data[1]]) as u32),
        3 => Some(((data[0] as u32) << 16) | ((data[1] as u32) << 8) | (data[2] as u32)),
        4 => Some(u32::from_be_bytes([data[0], data[1], data[2], data[3]])),
        _ => None,
    }
}

pub fn extract_cff_from_sfnt(data: &[u8]) -> Option<&[u8]> {
    if data.len() < 12 { return None; }
    let num_tables = u16::from_be_bytes([data[4], data[5]]) as usize;
    for i in 0..num_tables {
        let p = 12 + i * 16;
        if p + 16 > data.len() { break; }
        if &data[p..p + 4] == b"CFF " {
            let offset = u32::from_be_bytes([data[p + 8], data[p + 9], data[p + 10], data[p + 11]]) as usize;
            let length = u32::from_be_bytes([data[p + 12], data[p + 13], data[p + 14], data[p + 15]]) as usize;
            if offset + length <= data.len() {
                return Some(&data[offset..offset + length]);
            }
        }
    }
    None
}

pub fn cff_get_gid_for_cid(data: &[u8], target: u16) -> Option<u16> {
    if data.len() < 4 { return None; }
    let header_len = data[2] as usize;
    let name_idx_pos = header_len;
    let top_dict_idx_pos = cff_skip_index(data, name_idx_pos)?;
    let string_idx_pos = cff_skip_index(data, top_dict_idx_pos)?;
    let global_subr_idx_pos = cff_skip_index(data, string_idx_pos)?;
    
    // We need to find the Charset offset in Top Dict
    // This is complex, so we'll use a heuristic to find the Charset table.
    // In many Japanese CID fonts, the Charset starts after the Top Dict.
    let charset_pos = global_subr_idx_pos; // HEURISTIC
    if charset_pos + 1 > data.len() { return None; }
    
    let format = data[charset_pos];
    let mut pos = charset_pos + 1;
    
    // In CFF, the charset table maps GIDs 1..n to CIDs (or SIDs).
    // GID 0 is always .notdef and is NOT stored in the charset table.
    let mut gid = 1u16;
    if format == 0 {
        while pos + 1 < data.len() {
            let cid = u16::from_be_bytes([data[pos], data[pos + 1]]);
            if cid == target {
                return Some(gid);
            }
            pos += 2;
            gid += 1;
            if gid == 0xFFFF { break; }
        }
    } else if format == 1 || format == 2 {
        while pos + 2 < data.len() {
            let first = u16::from_be_bytes([data[pos], data[pos + 1]]);
            let n = if format == 1 {
                if pos + 2 >= data.len() { break; }
                data[pos + 2] as u16
            } else {
                if pos + 3 >= data.len() { break; }
                u16::from_be_bytes([data[pos + 2], data[pos + 3]])
            };
            
            if target >= first && target <= first + n {
                return Some(gid + (target - first));
            }
            pos += if format == 1 { 3 } else { 4 };
            gid += n + 1;
            if gid == 0xFFFF { break; }
        }
    }
    None
}

fn tt_cmap_lookup_format0(data: &[u8], sub_abs: usize, char_code: u32) -> Option<u32> {
    if char_code < 256 && sub_abs + 6 + 256 <= data.len() {
        let gid = data[sub_abs + 6 + char_code as usize] as u32;
        if gid > 0 {
            return Some(gid);
        }
    }
    None
}

fn tt_cmap_lookup_format4(data: &[u8], sub_abs: usize, char_code: u32) -> Option<u32> {
    if char_code >= 0x10000 || sub_abs + 14 > data.len() {
        return None;
    }
    let seg_count =
        u16::from_be_bytes(data[sub_abs + 6..sub_abs + 8].try_into().ok()?) as usize / 2;
    let end_off = sub_abs + 14;
    let start_off = sub_abs + 16 + seg_count * 2;
    let delta_off = sub_abs + 16 + seg_count * 4;
    let range_off = sub_abs + 16 + seg_count * 6;
    if start_off + seg_count * 2 > data.len() {
        return None;
    }
    let cc = char_code as u16;
    for s in 0..seg_count {
        if end_off + s * 2 + 2 > data.len() {
            break;
        }
        let end_code =
            u16::from_be_bytes(data[end_off + s * 2..end_off + s * 2 + 2].try_into().ok()?);
        if cc > end_code {
            continue;
        }
        if start_off + s * 2 + 2 > data.len() {
            break;
        }
        let start_code =
            u16::from_be_bytes(data[start_off + s * 2..start_off + s * 2 + 2].try_into().ok()?);
        if cc < start_code {
            break;
        }
        if delta_off + s * 2 + 2 > data.len() {
            break;
        }
        let delta =
            i16::from_be_bytes(data[delta_off + s * 2..delta_off + s * 2 + 2].try_into().ok()?);
        if range_off + s * 2 + 2 > data.len() {
            break;
        }
        let range_val =
            u16::from_be_bytes(data[range_off + s * 2..range_off + s * 2 + 2].try_into().ok()?);
        let gid = if range_val == 0 {
            (cc as i32 + delta as i32) as u32 & 0xFFFF
        } else {
            let idx = range_off + s * 2 + (range_val as usize) + (cc - start_code) as usize * 2;
            if idx + 2 > data.len() {
                break;
            }
            u16::from_be_bytes(data[idx..idx + 2].try_into().ok()?) as u32
        };
        if gid > 0 {
            return Some(gid);
        }
        break;
    }
    None
}

fn tt_cmap_lookup_format12(data: &[u8], sub_abs: usize, char_code: u32) -> Option<u32> {
    if sub_abs + 32 > data.len() {
        return None;
    }
    let n_groups = u32::from_be_bytes(data[sub_abs + 12..sub_abs + 16].try_into().ok()?) as usize;
    let groups_off = sub_abs + 16;
    for g in 0..n_groups {
        let goff = groups_off + g * 12;
        if goff + 12 > data.len() {
            break;
        }
        let start_code = u32::from_be_bytes(data[goff..goff + 4].try_into().ok()?);
        let end_code = u32::from_be_bytes(data[goff + 4..goff + 8].try_into().ok()?);
        let start_gid = u32::from_be_bytes(data[goff + 8..goff + 12].try_into().ok()?);
        if char_code >= start_code && char_code <= end_code {
            let gid = start_gid + (char_code - start_code);
            if gid > 0 {
                return Some(gid);
            }
            break;
        }
    }
    None
}

pub fn tt_cmap_lookup(data: &[u8], char_code: u32) -> Option<u32> {
    if data.len() < 12 {
        return None;
    }
    let tag = &data[0..4];

    if tag == b"ttcf" {
        let num_fonts = u32::from_be_bytes(data[8..12].try_into().ok()?) as usize;
        if data.len() < 12 + num_fonts * 4 {
            return None;
        }
        for i in 0..num_fonts.min(4) {
            let offset = u32::from_be_bytes(data[12 + i * 4..16 + i * 4].try_into().ok()?) as usize;
            if offset < data.len()
                && let Some(gid) = tt_cmap_lookup(&data[offset..], char_code)
            {
                return Some(gid);
            }
        }
        return None;
    }

    if tag != [0, 1, 0, 0] && tag != b"true" && tag != b"OTTO" {
        return None;
    }
    let num_tables = u16::from_be_bytes(data[4..6].try_into().ok()?) as usize;
    let mut cmap_offset = None;
    for i in 0..num_tables {
        let p = 12 + i * 16;
        if p + 16 > data.len() {
            break;
        }
        if &data[p..p + 4] == b"cmap" {
            cmap_offset = Some(u32::from_be_bytes(data[p + 8..p + 12].try_into().ok()?) as usize);
            break;
        }
    }

    let cmap_base = cmap_offset?;
    if cmap_base + 4 > data.len() {
        return None;
    }
    let num_subtables =
        u16::from_be_bytes(data[cmap_base + 2..cmap_base + 4].try_into().ok()?) as usize;

    let mut best_gid = None;
    for i in 0..num_subtables {
        let ep = cmap_base + 4 + i * 8;
        if ep + 8 > data.len() {
            break;
        }
        let platform = u16::from_be_bytes(data[ep..ep + 2].try_into().ok()?);
        let sub_off = u32::from_be_bytes(data[ep + 4..ep + 8].try_into().ok()?) as usize;
        let sub_abs = cmap_base + sub_off;
        if sub_abs + 2 > data.len() {
            continue;
        }
        let fmt = u16::from_be_bytes(data[sub_abs..sub_abs + 2].try_into().ok()?);

        let gid = match (platform, fmt) {
            (1, 0) => tt_cmap_lookup_format0(data, sub_abs, char_code),
            (3, 4) => tt_cmap_lookup_format4(data, sub_abs, char_code),
            (3, 12) => tt_cmap_lookup_format12(data, sub_abs, char_code),
            _ => None,
        };
        if let Some(g) = gid {
            if platform == 1 && fmt == 0 {
                return Some(g);
            }
            best_gid = Some(g);
        }
    }
    best_gid
}

pub fn tt_cmap_lookup_v2(data: &[u8], char_code: u32) -> Option<u32> {
    tt_cmap_lookup(data, char_code)
}
