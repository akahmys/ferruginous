use kurbo::{BezPath, Point};
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::raw::{FileRef, TableProvider};
use skrifa::{GlyphId, MetadataProvider};

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

pub struct SkrifaBridge {}

impl SkrifaBridge {
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
}

impl Default for SkrifaBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// Ensures the font data is in an SFNT container.
/// If it's a raw CFF stream, wraps it in a minimal OpenType container.
pub fn ensure_sfnt(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 4 {
        return None;
    }

    let tag = &data[0..4];
    if tag == b"OTTO" || tag == [0, 1, 0, 0] || tag == b"true" || tag == b"typ1" {
        return None;
    }

    if data[0] == 0x01 && data[1] == 0x00 {
        let mut sfnt = Vec::with_capacity(data.len() + 512);
        let _num_tables = 6u16;
        let cff_len = data.len() as u32;
        let cmap_len = 262u32 + 12u32;
        let head_len = 54u32;
        let hhea_len = 36u32;
        let hmtx_len = 4u32;
        let maxp_len = 6u32;

        let offsets = write_sfnt_directory(&mut sfnt, cff_len, cmap_len, head_len, hhea_len, hmtx_len, maxp_len);
        
        sfnt.resize(offsets.total_len as usize, 0);
        sfnt[offsets.cff as usize..offsets.cff as usize + data.len()].copy_from_slice(data);
        
        write_cmap_table(&mut sfnt, offsets.cmap);
        write_head_table(&mut sfnt, offsets.head);
        write_hhea_table(&mut sfnt, offsets.hhea);
        write_hmtx_table(&mut sfnt, offsets.hmtx);
        write_maxp_table(&mut sfnt, offsets.maxp);

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

fn write_sfnt_directory(sfnt: &mut Vec<u8>, cff_len: u32, cmap_len: u32, head_len: u32, hhea_len: u32, hmtx_len: u32, maxp_len: u32) -> TableOffsets {
    sfnt.extend_from_slice(b"OTTO");
    sfnt.extend_from_slice(&6u16.to_be_bytes()); // numTables
    sfnt.extend_from_slice(&64u16.to_be_bytes()); // searchRange
    sfnt.extend_from_slice(&2u16.to_be_bytes()); // entrySelector
    sfnt.extend_from_slice(&32u16.to_be_bytes()); // rangeShift

    let dir_len = 12 + (6 * 16);
    let mut offset = dir_len;

    let cff_off = write_table_record(sfnt, b"CFF ", offset, cff_len);
    offset += (cff_len + 3) & !3;
    let cmap_off = write_table_record(sfnt, b"cmap", offset, cmap_len);
    offset += (cmap_len + 3) & !3;
    let head_off = write_table_record(sfnt, b"head", offset, head_len);
    offset += (head_len + 3) & !3;
    let hhea_off = write_table_record(sfnt, b"hhea", offset, hhea_len);
    offset += (hhea_len + 3) & !3;
    let hmtx_off = write_table_record(sfnt, b"hmtx", offset, hmtx_len);
    offset += (hmtx_len + 3) & !3;
    let maxp_off = write_table_record(sfnt, b"maxp", offset, maxp_len);
    offset += (maxp_len + 3) & !3;

    TableOffsets { cff: cff_off, cmap: cmap_off, head: head_off, hhea: hhea_off, hmtx: hmtx_off, maxp: maxp_off, total_len: offset }
}

fn write_table_record(sfnt: &mut Vec<u8>, tag: &[u8; 4], offset: u32, length: u32) -> u32 {
    sfnt.extend_from_slice(tag);
    sfnt.extend_from_slice(&0u32.to_be_bytes()); // checksum
    sfnt.extend_from_slice(&offset.to_be_bytes());
    sfnt.extend_from_slice(&length.to_be_bytes());
    offset
}

fn write_cmap_table(sfnt: &mut [u8], offset: u32) {
    let start = offset as usize;
    sfnt[start..start + 2].copy_from_slice(&0u16.to_be_bytes()); 
    sfnt[start + 2..start + 4].copy_from_slice(&1u16.to_be_bytes());
    sfnt[start + 4..start + 6].copy_from_slice(&3u16.to_be_bytes());
    sfnt[start + 6..start + 8].copy_from_slice(&1u16.to_be_bytes());
    sfnt[start + 8..start + 12].copy_from_slice(&(12u32).to_be_bytes());
    let sub = start + 12;
    sfnt[sub..sub + 2].copy_from_slice(&0u16.to_be_bytes());
    sfnt[sub + 2..sub + 4].copy_from_slice(&262u16.to_be_bytes());
    for b in 0..256 { sfnt[sub + 6 + b] = b as u8; }
}

fn write_head_table(sfnt: &mut [u8], offset: u32) {
    let start = offset as usize;
    sfnt[start..start + 4].copy_from_slice(&0x00010000u32.to_be_bytes());
    sfnt[start + 12..start + 16].copy_from_slice(&0x5F0F3CF5u32.to_be_bytes());
    sfnt[start + 18..start + 20].copy_from_slice(&1000u16.to_be_bytes());
}

fn write_hhea_table(sfnt: &mut [u8], offset: u32) {
    let start = offset as usize;
    sfnt[start..start + 4].copy_from_slice(&0x00010000u32.to_be_bytes());
    sfnt[start + 4..start + 6].copy_from_slice(&1000i16.to_be_bytes());
    sfnt[start + 6..start + 8].copy_from_slice(&(-200i16).to_be_bytes());
    sfnt[start + 34..start + 36].copy_from_slice(&1u16.to_be_bytes());
}

fn write_hmtx_table(sfnt: &mut [u8], offset: u32) {
    let start = offset as usize;
    sfnt[start..start + 2].copy_from_slice(&1000u16.to_be_bytes());
    sfnt[start + 2..start + 4].copy_from_slice(&0i16.to_be_bytes());
}

fn write_maxp_table(sfnt: &mut [u8], offset: u32) {
    let start = offset as usize;
    sfnt[start..start + 4].copy_from_slice(&0x00005000u32.to_be_bytes());
    sfnt[start + 4..start + 6].copy_from_slice(&65535u16.to_be_bytes());
}

#[allow(unused_parens)]
#[allow(clippy::collapsible_if)]
fn cff_skip_index(data: &[u8], pos: usize) -> Option<usize> {
    if pos + 2 > data.len() {
        return None;
    }
    let count = ((data[pos] as usize) << 8) | (data[pos + 1] as usize);
    if count == 0 {
        return Some(pos + 2);
    }
    if pos + 3 > data.len() {
        return None;
    }
    let off_size = data[pos + 2] as usize;
    let end_pos = pos + 3 + (count + 1) * off_size;
    if end_pos > data.len() {
        return None;
    }

    // Read the last offset to find the length of the string data
    let mut last_off = 0;
    for i in 0..off_size {
        last_off = (last_off << 8) | (data[pos + 3 + count * off_size + i] as usize);
    }
    Some(pos + 3 + (count + 1) * off_size + last_off - 1)
}

fn cff_get_index_item(data: &[u8], idx_pos: usize, index: usize) -> Option<&[u8]> {
    if idx_pos + 2 > data.len() {
        return None;
    }
    let count = ((data[idx_pos] as usize) << 8) | (data[idx_pos + 1] as usize);
    if index >= count {
        return None;
    }
    let off_size = data[idx_pos + 2] as usize;

    let off_pos1 = idx_pos + 3 + index * off_size;
    let off_pos2 = idx_pos + 3 + (index + 1) * off_size;
    if off_pos2 > data.len() {
        return None;
    }

    let mut off1 = 0;
    for i in 0..off_size {
        off1 = (off1 << 8) | (data[off_pos1 + i] as usize);
    }
    let mut off2 = 0;
    for i in 0..off_size {
        off2 = (off2 << 8) | (data[off_pos2 + i] as usize);
    }

    let data_start = idx_pos + 3 + (count + 1) * off_size;
    Some(&data[data_start + off1 - 1..data_start + off2 - 1])
}

fn cff_parse_dict_for_op(dict: &[u8], target_op: u8, is_escaped: bool) -> Option<usize> {
    let mut pos = 0;
    let mut last_operand = None;
    while pos < dict.len() {
        let b = dict[pos];
        if b <= 21 {
            pos += 1;
            if b == 12 {
                if pos < dict.len() {
                    let b2 = dict[pos];
                    pos += 1;
                    if is_escaped && b2 == target_op {
                        return last_operand.map(|v| v as usize);
                    }
                }
            } else if !is_escaped && b == target_op {
                return last_operand.map(|v| v as usize);
            }
        } else {
            let (val, consumed) = cff_consume_operand(&dict[pos..])?;
            last_operand = Some(val);
            pos += consumed;
        }
    }
    None
}

fn cff_consume_operand(data: &[u8]) -> Option<(i32, usize)> {
    let b = data[0];
    if b == 28 {
        if data.len() < 3 { return None; }
        Some(((((data[1] as i16) << 8) | (data[2] as i16)) as i32, 3))
    } else if b == 29 {
        if data.len() < 5 { return None; }
        let val = ((data[1] as i32) << 24) | ((data[2] as i32) << 16) | ((data[3] as i32) << 8) | (data[4] as i32);
        Some((val, 5))
    } else if (32..=246).contains(&b) {
        Some((b as i32 - 139, 1))
    } else if (247..=250).contains(&b) {
        if data.len() < 2 { return None; }
        Some(((b as i32 - 247) * 256 + data[1] as i32 + 108, 2))
    } else if (251..=254).contains(&b) {
        if data.len() < 2 { return None; }
        Some((-(b as i32 - 251) * 256 - data[1] as i32 - 108, 2))
    } else if b == 30 {
        let mut p = 1;
        while p < data.len() {
            let n = data[p]; p += 1;
            if (n >> 4) == 0xF || (n & 0xF) == 0xF { break; }
        }
        Some((0, p))
    } else {
        Some((0, 1))
    }
}

pub fn cff_get_gid_for_cid(data: &[u8], target_cid: u16) -> Option<u16> {
    let cff_data = extract_cff_from_sfnt(data).unwrap_or(data);
    if cff_data.len() < 4 || cff_data[0] != 1 { return None; }
    
    let hdr_size = cff_data[2] as usize;
    let pos = cff_skip_index(cff_data, hdr_size)?;
    let top_dict_data = cff_get_index_item(cff_data, pos, 0)?;

    if let Some(charset_offset) = cff_parse_dict_for_op(top_dict_data, 15, false)
        && charset_offset < cff_data.len() {
            let format = cff_data[charset_offset];
            let charset_ptr = charset_offset + 1;
            if let Some(gid) = cff_lookup_charset(cff_data, format, charset_ptr, target_cid) {
                return Some(gid);
            }
        }

    // Check if it's a CIDFont (ROS operator 12 30 exists)
    // If it's a CIDFont and no custom charset is provided, CID == GID is the default.
    if cff_parse_dict_for_op(top_dict_data, 30, true).is_some() {
        return Some(target_cid);
    }

    None
}

fn extract_cff_from_sfnt(data: &[u8]) -> Option<&[u8]> {
    if data.len() < 12 || &data[0..4] != b"OTTO" { return None; }
    let num_tables = ((data[4] as u16) << 8) | (data[5] as u16);
    let mut p = 12;
    for _ in 0..num_tables {
        if p + 16 > data.len() { break; }
        if &data[p..p + 4] == b"CFF " {
            let offset = u32::from_be_bytes(data[p + 8..p + 12].try_into().ok()?) as usize;
            let length = u32::from_be_bytes(data[p + 12..p + 16].try_into().ok()?) as usize;
            if offset + length <= data.len() {
                return Some(&data[offset..offset + length]);
            }
        }
        p += 16;
    }
    None
}

fn cff_lookup_charset(data: &[u8], format: u8, mut pos: usize, target: u16) -> Option<u16> {
    let mut gid = 1u16;
    if format == 0 {
        while pos + 1 < data.len() {
            let cid = u16::from_be_bytes(data[pos..pos + 2].try_into().ok()?);
            if cid == target { return Some(gid); }
            pos += 2; gid += 1;
        }
    } else if format == 1 || format == 2 {
        while pos + 2 < data.len() {
            let first = u16::from_be_bytes(data[pos..pos + 2].try_into().ok()?);
            let n = if format == 1 { data[pos + 2] as u16 } else { u16::from_be_bytes(data[pos + 2..pos + 4].try_into().ok()?) };
            if target >= first && target <= first + n { return Some(gid + (target - first)); }
            pos += if format == 1 { 3 } else { 4 };
            gid += n + 1;
        }
    }
    None
}

/// TrueTypeフォントのcmapテーブルを走査してchar_code → GIDを解決する。
/// Platform 1 Format 0 (Mac Roman) と Platform 3 Format 4/12 (Windows Unicode) に対応。
fn tt_cmap_lookup_format0(data: &[u8], sub_abs: usize, char_code: u32) -> Option<u32> {
    if char_code < 256 && sub_abs + 6 + 256 <= data.len() {
        let gid = data[sub_abs + 6 + char_code as usize] as u32;
        if gid > 0 { return Some(gid); }
    }
    None
}

fn tt_cmap_lookup_format4(data: &[u8], sub_abs: usize, char_code: u32) -> Option<u32> {
    if char_code >= 0x10000 || sub_abs + 14 > data.len() { return None; }
    let seg_count = u16::from_be_bytes(data[sub_abs+6..sub_abs+8].try_into().ok()?) as usize / 2;
    let end_off = sub_abs + 14;
    let start_off = sub_abs + 16 + seg_count * 2;
    let delta_off = sub_abs + 16 + seg_count * 4;
    let range_off = sub_abs + 16 + seg_count * 6;
    if start_off + seg_count * 2 > data.len() { return None; }
    let cc = char_code as u16;
    for s in 0..seg_count {
        if end_off + s*2 + 2 > data.len() { break; }
        let end_code = u16::from_be_bytes(data[end_off+s*2..end_off+s*2+2].try_into().ok()?);
        if cc > end_code { continue; }
        if start_off + s*2 + 2 > data.len() { break; }
        let start_code = u16::from_be_bytes(data[start_off+s*2..start_off+s*2+2].try_into().ok()?);
        if cc < start_code { break; }
        if delta_off + s*2 + 2 > data.len() { break; }
        let delta = i16::from_be_bytes(data[delta_off+s*2..delta_off+s*2+2].try_into().ok()?);
        if range_off + s*2 + 2 > data.len() { break; }
        let range_val = u16::from_be_bytes(data[range_off+s*2..range_off+s*2+2].try_into().ok()?);
        let gid = if range_val == 0 {
            (cc as i32 + delta as i32) as u32 & 0xFFFF
        } else {
            let idx = range_off + s*2 + (range_val as usize) + (cc - start_code) as usize * 2;
            if idx + 2 > data.len() { break; }
            u16::from_be_bytes(data[idx..idx+2].try_into().ok()?) as u32
        };
        if gid > 0 { return Some(gid); }
        break;
    }
    None
}

fn tt_cmap_lookup_format12(data: &[u8], sub_abs: usize, char_code: u32) -> Option<u32> {
    if sub_abs + 32 > data.len() { return None; }
    let n_groups = u32::from_be_bytes(data[sub_abs+12..sub_abs+16].try_into().ok()?) as usize;
    let groups_off = sub_abs + 16;
    for g in 0..n_groups {
        let goff = groups_off + g * 12;
        if goff + 12 > data.len() { break; }
        let start_code = u32::from_be_bytes(data[goff..goff+4].try_into().ok()?);
        let end_code = u32::from_be_bytes(data[goff+4..goff+8].try_into().ok()?);
        let start_gid = u32::from_be_bytes(data[goff+8..goff+12].try_into().ok()?);
        if char_code >= start_code && char_code <= end_code {
            let gid = start_gid + (char_code - start_code);
            if gid > 0 { return Some(gid); }
            break;
        }
    }
    None
}

/// TrueTypeフォントのcmapテーブルを走査してchar_code → GIDを解決する。
/// Platform 1 Format 0 (Mac Roman) と Platform 3 Format 4/12 (Windows Unicode) に対応。
pub fn tt_cmap_lookup(data: &[u8], char_code: u32) -> Option<u32> {
    if data.len() < 12 { return None; }
    let tag = &data[0..4];

    if tag == b"ttcf" {
        let num_fonts = u32::from_be_bytes(data[8..12].try_into().ok()?) as usize;
        if data.len() < 12 + num_fonts * 4 { return None; }
        for i in 0..num_fonts.min(4) {
            let offset = u32::from_be_bytes(data[12 + i * 4..16 + i * 4].try_into().ok()?) as usize;
            if offset < data.len()
                && let Some(gid) = tt_cmap_lookup(&data[offset..], char_code) { return Some(gid); }
        }
        return None;
    }

    if tag != [0,1,0,0] && tag != b"true" && tag != b"OTTO" { return None; }
    let num_tables = u16::from_be_bytes(data[4..6].try_into().ok()?) as usize;
    let mut cmap_offset = None;
    for i in 0..num_tables {
        let p = 12 + i * 16;
        if p + 16 > data.len() { break; }
        if &data[p..p+4] == b"cmap" {
            cmap_offset = Some(u32::from_be_bytes(data[p+8..p+12].try_into().ok()?) as usize);
            break;
        }
    }
    
    let cmap_base = cmap_offset?;
    if cmap_base + 4 > data.len() { return None; }
    let num_subtables = u16::from_be_bytes(data[cmap_base+2..cmap_base+4].try_into().ok()?) as usize;

    let mut best_gid = None;
    for i in 0..num_subtables {
        let ep = cmap_base + 4 + i * 8;
        if ep + 8 > data.len() { break; }
        let platform = u16::from_be_bytes(data[ep..ep+2].try_into().ok()?);
        let sub_off = u32::from_be_bytes(data[ep+4..ep+8].try_into().ok()?) as usize;
        let sub_abs = cmap_base + sub_off;
        if sub_abs + 2 > data.len() { continue; }
        let fmt = u16::from_be_bytes(data[sub_abs..sub_abs+2].try_into().ok()?);

        let gid = match (platform, fmt) {
            (1, 0) => tt_cmap_lookup_format0(data, sub_abs, char_code),
            (3, 4) => tt_cmap_lookup_format4(data, sub_abs, char_code),
            (3, 12) => tt_cmap_lookup_format12(data, sub_abs, char_code),
            _ => None,
        };
        if let Some(g) = gid {
            if platform == 1 && fmt == 0 { return Some(g); }
            best_gid = Some(g);
        }
    }
    best_gid
}

impl SkrifaBridge {
    pub fn new() -> Self {
        Self {}
    }

    pub fn extract_path_direct(&self, data: &[u8], gid: u32) -> Option<BezPath> {
        if let Ok(file) = FileRef::new(data) {
            let font_opt = match file {
                FileRef::Font(f) => Some(f),
                FileRef::Collection(c) => c.get(0).ok(),
            };
            if let Some(font) = font_opt {
                let mut pen = KurboPen::new();
                if let Some(glyph) = font.outline_glyphs().get(GlyphId::new(gid))
                    && glyph.draw(
                        DrawSettings::unhinted(Size::new(1000.0), LocationRef::default()),
                        &mut pen,
                    ).is_ok() {
                        let path = pen.finish();
                        if !path.is_empty() {
                            return Some(path);
                        }
                    }
            }
        }
        None
    }

    #[allow(clippy::too_many_arguments)]
    pub fn extract_path(
        &self,
        data: &[u8],
        mut gid: u32,
        char_code: u32,
        cid_to_gid_map: Option<&[u16]>,
        _is_vertical: bool,
        unicode_fallback: Option<char>,
        force_system_fallback: bool,
    ) -> Option<BezPath> {
        // Step 0: Apply CIDToGIDMap if present (for TrueType CIDFonts)
        if let Some(map) = cid_to_gid_map
            && (gid as usize) < map.len() {
                gid = map[gid as usize] as u32;
            }

        // If it's a bare CFF font, map CID -> GID via charset.
        if data.len() >= 2 && data[0] == 0x01 && data[1] == 0x00 {
            gid = cff_get_gid_for_cid(data, gid as u16).unwrap_or(gid as u16) as u32;
        }

        let sfnt_data = ensure_sfnt(data);
        let data_ref = sfnt_data.as_deref().unwrap_or(data);

        // Try skrifa first (using direct GID strategy)
        if let Ok(file) = FileRef::new(data_ref) {
            let font_opt = match file {
                FileRef::Font(f) => Some(f),
                FileRef::Collection(c) => c.get(0).ok(),
            };
            if let Some(font) = font_opt {
                let outlines = font.outline_glyphs();

                // STRATEGY 0: Try the character code via the font's own internal 'cmap' table.
                if !force_system_fallback
                    && let Some(cmap_gid) = tt_cmap_lookup(data, char_code)
                        && let Some(glyph) = outlines.get(GlyphId::new(cmap_gid)) {
                            let mut pen = KurboPen::new();
                            if glyph.draw(
                                DrawSettings::unhinted(Size::new(1000.0), LocationRef::default()),
                                &mut pen,
                            ).is_ok() {
                                let path = pen.finish();
                                if !path.is_empty() {
                                    return Some(path);
                                }
                            }
                        }

                // STRATEGY 1: For PDF TrueType fonts (especially subsetted), the code 
                // is intended to be the GID directly. 
                let direct_gid = if force_system_fallback { GlyphId::new(0) } else { GlyphId::new(gid) };
                if let Some(glyph) = outlines.get(direct_gid) {
                    let mut pen = KurboPen::new();
                    if glyph.draw(
                        DrawSettings::unhinted(Size::new(1000.0), LocationRef::default()),
                        &mut pen,
                    ).is_ok() {
                        let candidate_path = pen.finish();
                        if !candidate_path.is_empty() {
                            return Some(candidate_path);
                        }
                    }
                }

                // STRATEGY 2: Use the explicit Unicode fallback if available via font's own charmap.
                if let Some(uch) = unicode_fallback
                    && uch != '\0' && uch != '\u{FFFD}' {
                        let charmap = font.charmap();
                        if let Some(mapped_gid) = charmap.map(uch) {
                            let mut pen2 = KurboPen::new();
                            if let Some(glyph) = outlines.get(mapped_gid)
                                && glyph.draw(
                                    DrawSettings::unhinted(Size::new(1000.0), LocationRef::default()),
                                    &mut pen2,
                                ).is_ok() {
                                    let candidate_path = pen2.finish();
                                    if !candidate_path.is_empty() {
                                        return Some(candidate_path);
                                    }
                                }
                        }
                    }
            }
        }

        // STRATEGY 3: Use the legacy tt_cmap_lookup with unicode_fallback.
        if let Some(uch) = unicode_fallback {
             let gid_from_uni = tt_cmap_lookup(data_ref, uch as u32);
             if let Some(resolved_gid) = gid_from_uni
                 && resolved_gid > 0
                     && let Some(path) = self.extract_path_direct(data_ref, resolved_gid) {
                         return Some(path);
                     }
        }

        // STRATEGY 4: Use the legacy tt_cmap_lookup with raw char_code.
        let cmap_gid = tt_cmap_lookup(data_ref, char_code);
        if let Some(resolved_gid) = cmap_gid
            && resolved_gid > 0
                && let Some(path) = self.extract_path_direct(data_ref, resolved_gid) {
                    return Some(path);
                }

        // Fallback: ttf-parser
        let mut final_gid = gid;
        if data.len() >= 2
            && ((data[0] == 0x01 && data[1] == 0x00) || &data[0..4] == b"OTTO")
            && let Some(mapped_gid) = cff_get_gid_for_cid(data, gid as u16) {
                final_gid = mapped_gid as u32;
            }

        if let Ok(face) = ttf_parser::Face::parse(data, 0) {
            let mut pen = KurboPen::new();
            if let Some(_rect) = face.outline_glyph(ttf_parser::GlyphId(final_gid as u16), &mut pen)
            {
                return Some(pen.finish());
            }
        }

        // Stage 3: System Font Fallback (CRITICAL for Japanese subsetted fonts)
        // If the embedded font failed, we MUST use a system font with the CORRECT Unicode lookup.
        if let Some(uch) = unicode_fallback {
             // Skip rendering for null, replacement, space (which should be empty anyway),
             // and non-printable control characters to avoid visual "ghost" artifacts.
             if uch != '\0' && uch != ' ' && uch != '\u{FFFD}' && (uch as u32) >= 32 {
                 let candidates = [
                    // Western Fallbacks (Serif priority for Century compatibility)
                    "/System/Library/Fonts/Times.ttc",
                    "/System/Library/Fonts/NewYork.ttf",
                    "/System/Library/Fonts/Helvetica.ttc",
                    "/System/Library/Fonts/Supplemental/Arial.ttf",
                    "/System/Library/Fonts/Supplemental/Times New Roman.ttf",
                    // Japanese Fallbacks
                    "/System/Library/Fonts/ヒラギノ明朝 ProN.ttc",
                    "/System/Library/Fonts/Hiragino Sans GB.ttc",
                    "/System/Library/Fonts/Hiragino Mincho ProN W3.otf",
                    "/System/Library/Fonts/ヒラギノ明朝 ProN W3.otf",
                    "/Library/Fonts/Microsoft/MS Mincho.ttf",
                    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                ];
                for path in &candidates {
                    if let Ok(font_data) = std::fs::read(path)
                        && let Ok(file) = FileRef::new(&font_data) {
                            let font_opt = match file {
                                FileRef::Font(f) => Some(f),
                                FileRef::Collection(c) => c.get(0).ok(),
                            };
                            if let Some(f) = font_opt {
                                let charmap = f.charmap();
                                if let Some(mapped_gid) = charmap.map(uch) {
                                    let mut pen = KurboPen::new();
                                    if let Some(glyph) = f.outline_glyphs().get(mapped_gid)
                                        && glyph.draw(
                                            DrawSettings::unhinted(Size::new(1000.0), LocationRef::default()),
                                            &mut pen,
                                        ).is_ok() {
                                            return Some(pen.finish());
                                        }
                                }
                            }
                        }
                }
             }
        }

        // Stage 4: Visibility & Control Character Filtering
        // 1. Skip GID 0 (Notdef) unless specifically requested (which PDF shouldn't)
        if gid == 0 {
            return None;
        }

        // 2. Skip non-printable control characters (ASCII < 32)
        if let Some(uch) = unicode_fallback
            && (uch as u32) < 32 && uch != '\n' && uch != '\r' && uch != '\t' {
                return None;
            }

        None
    }

    /// Renders a sequence of glyphs into a single path.
    pub fn render_glyphs(
        &self,
        font_data: &[u8],
        glyphs: &[(u32, f32)], // (GlyphId, Width override if any)
        options: &TextLayoutOptions,
    ) -> BezPath {
        let mut combined_path = BezPath::new();
        let mut x_offset = 0.0;

        let scale = options.font_size / 1000.0;
        let h_scale = options.horizontal_scaling / 100.0;

        for (gid_u32, width) in glyphs.iter() {
            let gid = *gid_u32;
            let unicode_fallback = None;
            if let Some(path) = self.extract_path(font_data, gid, gid, None, false, unicode_fallback, false) {
                let transform = kurbo::Affine::translate((x_offset, 0.0))
                    * kurbo::Affine::scale_non_uniform(scale as f64 * h_scale as f64, scale as f64);

                let mut path = path;
                path.apply_affine(transform);
                combined_path.extend(path);
            }

            x_offset += *width as f64 * scale as f64 * h_scale as f64;
            x_offset += options.char_spacing as f64 * h_scale as f64;

            if gid == 32 {
                x_offset += options.word_spacing as f64 * h_scale as f64;
            }
        }

        combined_path
    }
}

impl ttf_parser::OutlineBuilder for KurboPen {
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
