use crate::font::FontResource;
use crate::{PdfError, PdfResult};
use std::collections::BTreeMap;

/// A surgical patcher for SFNT binaries.
pub struct FontReconstructor;

struct DisassembledSfnt {
    magic: [u8; 4],
    tables: Vec<([u8; 4], Vec<u8>)>,
}

impl FontReconstructor {
    /// Reconstructs a font by injecting PDF metrics into the provided SFNT data.
    /// Returns the patched data and any discovered CID-to-GID mapping.
    pub fn reconstruct(
        resource: &FontResource,
        raw_data: &[u8],
    ) -> PdfResult<(Vec<u8>, Option<Vec<u16>>)> {
        let is_sfnt = raw_data.len() >= 4
            && (raw_data.starts_with(b"OTTO")
                || raw_data.starts_with(&[0, 1, 0, 0])
                || raw_data.starts_with(b"ttcf")
                || raw_data.starts_with(b"true")
                || raw_data.starts_with(b"typ1"));
        let is_naked_cff1 = !is_sfnt && raw_data.len() >= 4 && raw_data[0] == 1 && raw_data[1] == 0;
        let is_naked_cff2 = !is_sfnt && raw_data.len() >= 4 && raw_data[0] == 2 && raw_data[1] == 0;

        let mut discovered_map = None;

        let mut sfnt = if is_sfnt {
            let (_, map) = Self::inspect_cff(raw_data);
            discovered_map = map;
            raw_data.to_vec()
        } else if is_naked_cff1 {
            let (data, map) = Self::wrap_cff(raw_data, resource)?;
            discovered_map = map;
            data
        } else if is_naked_cff2 {
            return Err(PdfError::Other("Naked CFF2 wrapping not yet implemented".into()));
        } else {
            raw_data.to_vec()
        };

        if raw_data.starts_with(b"ttcf") {
            return Ok((raw_data.to_vec(), None));
        }

        if let Ok(mut sfnt_dis) = Self::disassemble_sfnt(&sfnt) {
            // Patch hmtx (Glyph Widths)
            Self::patch_hmtx_direct(&mut sfnt_dis.tables, resource);

            // Patch/Inject cmap (Character Mapping)
            if let Some(cmap_data) = Self::synthesize_bridged_cmap(resource, &sfnt) {
                if let Some(idx) = sfnt_dis.tables.iter().position(|(t, _)| t == b"cmap") {
                    sfnt_dis.tables[idx].1 = cmap_data;
                } else {
                    sfnt_dis.tables.push((*b"cmap", cmap_data));
                }
                if let Ok(new_data) = Self::assemble_sfnt(&sfnt_dis.magic, &sfnt_dis.tables) {
                    sfnt = new_data;
                }
            }
        }

        Ok((sfnt, discovered_map))
    }

    fn wrap_cff(
        cff_data: &[u8],
        resource: &FontResource,
    ) -> PdfResult<(Vec<u8>, Option<Vec<u16>>)> {
        let mut tables = Vec::new();
        tables.push((*b"CFF ", cff_data.to_vec()));
        let (num_glyphs, cid_map) = Self::inspect_cff(cff_data);

        let mut head = vec![0u8; 54];
        head[0..4].copy_from_slice(&[0, 1, 0, 0]);
        head[12..16].copy_from_slice(&0x5F0F3CF5u32.to_be_bytes());
        head[18..20].copy_from_slice(&1000u16.to_be_bytes());
        tables.push((*b"head", head));

        let mut hhea = vec![0u8; 36];
        hhea[0..4].copy_from_slice(&[0, 1, 0, 0]);
        hhea[34..36].copy_from_slice(&num_glyphs.to_be_bytes());
        tables.push((*b"hhea", hhea));

        let mut maxp = vec![0u8; 32];
        maxp[0..4].copy_from_slice(&[0, 0, 0x50, 0]);
        maxp[4..6].copy_from_slice(&num_glyphs.to_be_bytes());
        tables.push((*b"maxp", maxp));

        let mut hmtx = Vec::with_capacity(num_glyphs as usize * 4);
        for gid in 0..num_glyphs {
            let width = resource.glyph_width_by_cid(gid as u32);
            hmtx.extend_from_slice(&(width as i16).to_be_bytes());
            hmtx.extend_from_slice(&0i16.to_be_bytes());
        }
        tables.push((*b"hmtx", hmtx));

        if let Some(cmap) = Self::synthesize_cmap_direct(resource) {
            tables.push((*b"cmap", cmap));
        }

        let data = Self::assemble_sfnt(b"OTTO", &tables)?;
        Ok((data, cid_map))
    }

    fn synthesize_bridged_cmap(resource: &FontResource, raw_data: &[u8]) -> Option<Vec<u8>> {
        if resource.unicode_to_gid.is_empty() {
            return None;
        }

        let mut actual_gid_map = BTreeMap::new();

        // GID Rescue: Build physical CID -> GID map by scanning all internal cmap tables
        if let Ok(face) = ttf_parser::Face::parse(raw_data, 0) {
            for table in face.tables().cmap.iter().flat_map(|t| t.subtables) {
                table.codepoints(|cp| {
                    if let Some(gid) = table.glyph_index(cp) {
                        actual_gid_map.insert(cp, gid.0 as u32);
                    }
                });
            }
        }

        // Manual scan fallback for deeply legacy subset fonts
        if actual_gid_map.is_empty() && raw_data.len() > 12 {
            let num_tables = u16::from_be_bytes([raw_data[4], raw_data[5]]) as usize;
            for i in 0..num_tables {
                let entry_off = 12 + i * 16;
                if entry_off + 16 > raw_data.len() {
                    break;
                }
                let tag = &raw_data[entry_off..entry_off + 4];
                if tag == b"cmap" {
                    let offset = u32::from_be_bytes([
                        raw_data[entry_off + 8],
                        raw_data[entry_off + 9],
                        raw_data[entry_off + 10],
                        raw_data[entry_off + 11],
                    ]) as usize;
                    let length = u32::from_be_bytes([
                        raw_data[entry_off + 12],
                        raw_data[entry_off + 13],
                        raw_data[entry_off + 14],
                        raw_data[entry_off + 15],
                    ]) as usize;
                    if offset + length <= raw_data.len() {
                        Self::manual_extract_cmap(
                            &raw_data[offset..offset + length],
                            &mut actual_gid_map,
                        );
                    }
                }
            }
        }

        let mut mappings = Vec::new();
        for (&c, &cid) in resource.unicode_to_gid.iter() {
            let actual_gid = if let Some(&gid) = actual_gid_map.get(&(cid as u32)) {
                gid
            } else if let Some(ref map) = resource.cid_to_gid_map {
                map.get(cid as usize).copied().unwrap_or(cid as u16) as u32
            } else {
                cid as u32
            };
            mappings.push((c as u32, actual_gid));
        }

        Self::assemble_cmap_table(&mappings)
    }

    fn synthesize_cmap_direct(resource: &FontResource) -> Option<Vec<u8>> {
        let mut mappings = Vec::new();
        for (&c, &cid) in resource.unicode_to_gid.iter() {
            mappings.push((c as u32, cid));
        }
        Self::assemble_cmap_table(&mappings)
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
        let num_tables = u16::from_be_bytes([sfnt[4], sfnt[5]]) as usize;
        let mut tables = Vec::new();
        for i in 0..num_tables {
            let entry = 12 + i * 16;
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
        let mut magic = [0u8; 4];
        magic.copy_from_slice(&sfnt[0..4]);
        Ok(DisassembledSfnt { magic, tables })
    }

    fn manual_extract_cmap(data: &[u8], map: &mut std::collections::BTreeMap<u32, u32>) {
        if data.len() < 4 {
            return;
        }
        let num_tables = u16::from_be_bytes([data[2], data[3]]) as usize;
        for i in 0..num_tables {
            let off = 4 + i * 8;
            if off + 8 > data.len() {
                break;
            }
            let sub_off =
                u32::from_be_bytes([data[off + 4], data[off + 5], data[off + 6], data[off + 7]])
                    as usize;
            if sub_off + 2 > data.len() {
                continue;
            }
            let format = u16::from_be_bytes([data[sub_off], data[sub_off + 1]]);
            if format == 4 {
                Self::parse_cmap_format_4(&data[sub_off..], map);
            } else if format == 12 {
                Self::parse_cmap_format_12(&data[sub_off..], map);
            }
        }
    }

    fn parse_cmap_format_4(data: &[u8], map: &mut std::collections::BTreeMap<u32, u32>) {
        if data.len() < 14 {
            return;
        }
        let seg_count = (u16::from_be_bytes([data[6], data[7]]) / 2) as usize;
        let end_codes = &data[14..14 + seg_count * 2];
        let start_codes = &data[16 + seg_count * 2..16 + seg_count * 4];
        let id_deltas = &data[16 + seg_count * 4..16 + seg_count * 6];
        let id_range_offsets = &data[16 + seg_count * 6..16 + seg_count * 8];

        for i in 0..seg_count {
            let start = u16::from_be_bytes([start_codes[i * 2], start_codes[i * 2 + 1]]) as u32;
            let end = u16::from_be_bytes([end_codes[i * 2], end_codes[i * 2 + 1]]) as u32;
            let delta = u16::from_be_bytes([id_deltas[i * 2], id_deltas[i * 2 + 1]]) as i32;
            let range_offset =
                u16::from_be_bytes([id_range_offsets[i * 2], id_range_offsets[i * 2 + 1]]) as usize;

            if start == 0xFFFF {
                continue;
            }

            for cp in start..=end {
                if range_offset == 0 {
                    let gid = (cp as i32 + delta) as u16 as u32;
                    map.insert(cp, gid);
                } else {
                    // Complexity: idRangeOffset points into glyphIdArray
                }
            }
        }
    }

    fn parse_cmap_format_12(data: &[u8], map: &mut std::collections::BTreeMap<u32, u32>) {
        if data.len() < 16 {
            return;
        }
        let num_groups = u32::from_be_bytes([data[12], data[13], data[14], data[15]]) as usize;
        for i in 0..num_groups {
            let off = 16 + i * 12;
            if off + 12 > data.len() {
                break;
            }
            let start =
                u32::from_be_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]]);
            let end =
                u32::from_be_bytes([data[off + 4], data[off + 5], data[off + 6], data[off + 7]]);
            let mut gid =
                u32::from_be_bytes([data[off + 8], data[off + 9], data[off + 10], data[off + 11]]);
            for cp in start..=end {
                map.insert(cp, gid);
                gid += 1;
            }
        }
    }

    fn assemble_sfnt(magic: &[u8; 4], tables: &[([u8; 4], Vec<u8>)]) -> PdfResult<Vec<u8>> {
        let mut tables = tables.to_vec();
        tables.sort_by_key(|t| t.0);
        let mut output = Vec::new();
        output.extend_from_slice(magic);
        output.extend_from_slice(&(tables.len() as u16).to_be_bytes());
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

    fn inspect_cff(data: &[u8]) -> (u16, Option<Vec<u16>>) {
        let is_sfnt =
            data.len() >= 4 && (data.starts_with(b"OTTO") || data.starts_with(&[0, 1, 0, 0]));
        let cff_data = if is_sfnt {
            if let Some((o, e)) = find_table_range(data, b"CFF ") {
                &data[o..e]
            } else {
                return (1, None);
            }
        } else {
            data
        };

        if cff_data.len() < 10 {
            return (1, None);
        }
        let mut pos = cff_data[2] as usize;
        pos = skip_index(cff_data, pos); // Name
        let ts = pos;
        let tc = if pos + 2 <= cff_data.len() {
            u16::from_be_bytes([cff_data[pos], cff_data[pos + 1]])
        } else {
            0
        };
        pos = skip_index(cff_data, pos); // Top DICT
        pos = skip_index(cff_data, pos); // String
        let _ = skip_index(cff_data, pos); // Global Subr
        let (cso, cso2, is_cid) = Self::parse_cff_top_dict(cff_data, ts, tc);
        let ng = if let Some(o) = cso
            && o < cff_data.len()
        {
            if o + 2 <= cff_data.len() {
                u16::from_be_bytes([cff_data[o], cff_data[o + 1]])
            } else {
                1024
            }
        } else {
            1024
        };
        let mut map = None;
        if is_cid
            && let Some(o) = cso2
            && o < cff_data.len()
        {
            map = Self::parse_cff_charset(cff_data, o, ng);
        }
        (ng, map)
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
                    match op {
                        17 => cso = ops.last().copied().map(|v| v as usize),
                        15 => cso2 = ops.last().copied().map(|v| v as usize),
                        0x0C1E => is_cid = true,
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

    fn parse_cff_charset(data: &[u8], off: usize, num_glyphs: u16) -> Option<Vec<u16>> {
        let mut map = vec![0u16; 65536];
        let format = data[off];
        let mut cpos = off + 1;
        if format == 0 {
            for gid in 1..num_glyphs {
                if cpos + 2 > data.len() {
                    break;
                }
                let cid = u16::from_be_bytes([data[cpos], data[cpos + 1]]);
                map[cid as usize] = gid;
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
                    if (fc as usize + i as usize) < 65536 {
                        map[fc as usize + i as usize] = gid;
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
    if b0 == 28 {
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
