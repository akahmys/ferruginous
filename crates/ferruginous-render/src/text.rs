use kurbo::{BezPath, Point};
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::raw::FileRef;
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

    // SFNT tags: 0x00010000 (TrueType), 'OTTO' (CFF OpenType), 'true' (Apple TT), 'typ1' (PostScript)
    let tag = &data[0..4];
    if tag == b"OTTO" || tag == [0, 1, 0, 0] || tag == b"true" || tag == b"typ1" {
        return None; // Already SFNT
    }

    // Check for CFF Magic (0x01 0x00 ...)
    if data[0] == 0x01 && data[1] == 0x00 {
        // Build a minimal OTF (SFNT) container with CFF , head, maxp, hhea, hmtx, and cmap tables
        let mut sfnt = Vec::with_capacity(data.len() + 512);

        let num_tables = 6u16;
        let cff_len = data.len() as u32;
        let head_len = 54u32;
        let maxp_len = 6u32;
        let hhea_len = 36u32;
        let hmtx_len = 4u32; // 1 metric (advance=1000, lsb=0)
        let cmap_len = 262u32 + 12u32; // Format 0

        let dir_len = 12 + (num_tables as u32 * 16);
        let mut offset = dir_len;

        // 1. Offset Table
        sfnt.extend_from_slice(b"OTTO");
        sfnt.extend_from_slice(&num_tables.to_be_bytes());
        sfnt.extend_from_slice(&64u16.to_be_bytes()); // searchRange
        sfnt.extend_from_slice(&2u16.to_be_bytes()); // entrySelector
        sfnt.extend_from_slice(&32u16.to_be_bytes()); // rangeShift

        // 2. Table Records (Sorted Alphabetically: CFF, cmap, head, hhea, hmtx, maxp)

        // CFF
        sfnt.extend_from_slice(b"CFF ");
        sfnt.extend_from_slice(&0u32.to_be_bytes());
        sfnt.extend_from_slice(&offset.to_be_bytes());
        sfnt.extend_from_slice(&cff_len.to_be_bytes());
        let cff_offset = offset;
        offset += (cff_len + 3) & !3;

        // cmap
        sfnt.extend_from_slice(b"cmap");
        sfnt.extend_from_slice(&0u32.to_be_bytes());
        sfnt.extend_from_slice(&offset.to_be_bytes());
        sfnt.extend_from_slice(&cmap_len.to_be_bytes());
        let cmap_offset = offset;
        offset += (cmap_len + 3) & !3;

        // head
        sfnt.extend_from_slice(b"head");
        sfnt.extend_from_slice(&0u32.to_be_bytes());
        sfnt.extend_from_slice(&offset.to_be_bytes());
        sfnt.extend_from_slice(&head_len.to_be_bytes());
        let head_offset = offset;
        offset += (head_len + 3) & !3;

        // hhea
        sfnt.extend_from_slice(b"hhea");
        sfnt.extend_from_slice(&0u32.to_be_bytes());
        sfnt.extend_from_slice(&offset.to_be_bytes());
        sfnt.extend_from_slice(&hhea_len.to_be_bytes());
        let hhea_offset = offset;
        offset += (hhea_len + 3) & !3;

        // hmtx
        sfnt.extend_from_slice(b"hmtx");
        sfnt.extend_from_slice(&0u32.to_be_bytes());
        sfnt.extend_from_slice(&offset.to_be_bytes());
        sfnt.extend_from_slice(&hmtx_len.to_be_bytes());
        let hmtx_offset = offset;
        offset += (hmtx_len + 3) & !3;

        // maxp
        sfnt.extend_from_slice(b"maxp");
        sfnt.extend_from_slice(&0u32.to_be_bytes());
        sfnt.extend_from_slice(&offset.to_be_bytes());
        sfnt.extend_from_slice(&maxp_len.to_be_bytes());
        let maxp_offset = offset;
        offset += (maxp_len + 3) & !3;

        // 3. Table Data
        sfnt.resize(offset as usize, 0);

        // Fill CFF
        sfnt[cff_offset as usize..cff_offset as usize + data.len()].copy_from_slice(data);

        // Fill cmap (Format 0 - Identity)
        let cmap_start = cmap_offset as usize;
        sfnt[cmap_start..cmap_start + 2].copy_from_slice(&0u16.to_be_bytes()); // version
        sfnt[cmap_start + 2..cmap_start + 4].copy_from_slice(&1u16.to_be_bytes()); // numTables
        sfnt[cmap_start + 4..cmap_start + 6].copy_from_slice(&3u16.to_be_bytes()); // platform
        sfnt[cmap_start + 6..cmap_start + 8].copy_from_slice(&1u16.to_be_bytes()); // encoding
        sfnt[cmap_start + 8..cmap_start + 12].copy_from_slice(&(12u32).to_be_bytes()); // offset
        let sub_start = cmap_start + 12;
        sfnt[sub_start..sub_start + 2].copy_from_slice(&0u16.to_be_bytes()); // format
        sfnt[sub_start + 2..sub_start + 4].copy_from_slice(&262u16.to_be_bytes()); // length
        for b in 0..256 {
            sfnt[sub_start + 6 + b] = b as u8;
        }

        // Fill head
        let head_start = head_offset as usize;
        sfnt[head_start..head_start + 4].copy_from_slice(&0x00010000u32.to_be_bytes());
        sfnt[head_start + 12..head_start + 16].copy_from_slice(&0x5F0F3CF5u32.to_be_bytes());
        sfnt[head_start + 18..head_start + 20].copy_from_slice(&1000u16.to_be_bytes()); // unitsPerEm

        // Fill hhea (minimal)
        let hhea_start = hhea_offset as usize;
        sfnt[hhea_start..hhea_start + 4].copy_from_slice(&0x00010000u32.to_be_bytes()); // version
        sfnt[hhea_start + 4..hhea_start + 6].copy_from_slice(&1000i16.to_be_bytes()); // ascender
        sfnt[hhea_start + 6..hhea_start + 8].copy_from_slice(&(-200i16).to_be_bytes()); // descender
        sfnt[hhea_start + 34..hhea_start + 36].copy_from_slice(&1u16.to_be_bytes()); // numHMetrics

        // Fill hmtx
        let hmtx_start = hmtx_offset as usize;
        sfnt[hmtx_start..hmtx_start + 2].copy_from_slice(&1000u16.to_be_bytes()); // advanceWidth
        sfnt[hmtx_start + 2..hmtx_start + 4].copy_from_slice(&0i16.to_be_bytes()); // lsb

        // Fill maxp
        let maxp_start = maxp_offset as usize;
        sfnt[maxp_start..maxp_start + 4].copy_from_slice(&0x00005000u32.to_be_bytes());
        sfnt[maxp_start + 4..maxp_start + 6].copy_from_slice(&65535u16.to_be_bytes());

        return Some(sfnt);
    }

    None
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

fn cff_parse_dict_for_op(dict: &[u8], target_op: u8) -> Option<usize> {
    let mut pos = 0;
    let mut last_operand = None;
    while pos < dict.len() {
        let b = dict[pos];
        if b <= 21 {
            // Operator
            pos += 1;
            if b == 12 {
                pos += 1;
            } // two-byte operator
            if b == target_op {
                return last_operand.map(|v| v as usize);
            }
        } else {
            // Operand
            if b == 28 {
                if pos + 2 >= dict.len() {
                    return None;
                }
                last_operand =
                    Some((((dict[pos + 1] as i16) << 8) | (dict[pos + 2] as i16)) as i32);
                pos += 3;
            } else if b == 29 {
                if pos + 4 >= dict.len() {
                    return None;
                }
                last_operand = Some(
                    ((dict[pos + 1] as i32) << 24)
                        | ((dict[pos + 2] as i32) << 16)
                        | ((dict[pos + 3] as i32) << 8)
                        | (dict[pos + 4] as i32),
                );
                pos += 5;
            } else if (32..=246).contains(&b) {
                last_operand = Some(b as i32 - 139);
                pos += 1;
            } else if (247..=250).contains(&b) {
                if pos + 1 >= dict.len() {
                    return None;
                }
                last_operand = Some((b as i32 - 247) * 256 + dict[pos + 1] as i32 + 108);
                pos += 2;
            } else if (251..=254).contains(&b) {
                if pos + 1 >= dict.len() {
                    return None;
                }
                last_operand = Some(-(b as i32 - 251) * 256 - dict[pos + 1] as i32 - 108);
                pos += 2;
            } else if b == 30 {
                pos += 1;
                while pos < dict.len() {
                    let nibbles = dict[pos];
                    pos += 1;
                    if (nibbles >> 4) == 0xF || (nibbles & 0xF) == 0xF {
                        break;
                    }
                }
                last_operand = Some(0); // Approximate
            } else {
                pos += 1;
            }
        }
    }
    None
}

pub fn cff_get_gid_for_cid(data: &[u8], target_cid: u16) -> Option<u16> {
    let mut cff_data = data;

    // Check if it's an SFNT container (starts with 'OTTO')
    if data.len() >= 12 && &data[0..4] == b"OTTO" {
        let num_tables = ((data[4] as u16) << 8) | (data[5] as u16);
        let mut pos = 12;
        for _ in 0..num_tables {
            if pos + 16 > data.len() {
                break;
            }
            let tag = &data[pos..pos + 4];
            if tag == b"CFF " {
                let offset = ((data[pos + 8] as u32) << 24)
                    | ((data[pos + 9] as u32) << 16)
                    | ((data[pos + 10] as u32) << 8)
                    | (data[pos + 11] as u32);
                let length = ((data[pos + 12] as u32) << 24)
                    | ((data[pos + 13] as u32) << 16)
                    | ((data[pos + 14] as u32) << 8)
                    | (data[pos + 15] as u32);
                if offset as usize + length as usize <= data.len() {
                    cff_data = &data[offset as usize..(offset + length) as usize];
                    break;
                }
            }
            pos += 16;
        }
    }

    if cff_data.len() < 4 || cff_data[0] != 1 {
        return None;
    }
    let hdr_size = cff_data[2] as usize;
    let mut pos = hdr_size;

    pos = match cff_skip_index(cff_data, pos) {
        Some(p) => p,
        None => {
            eprintln!("DEBUG: cff_skip_index failed at pos={}", pos);
            return None;
        }
    };

    let top_dict_idx_pos = pos;
    let top_dict_data = match cff_get_index_item(cff_data, top_dict_idx_pos, 0) {
        Some(d) => d,
        None => {
            eprintln!("DEBUG: top_dict_data extraction failed at pos={}", top_dict_idx_pos);
            return None;
        }
    };

    // operator 15 is charset
    let charset_offset = match cff_parse_dict_for_op(top_dict_data, 15) {
        Some(o) => o,
        None => {
            eprintln!(
                "DEBUG: charset_offset entirely missing in Top DICT! Maybe standard charset?"
            );
            return None;
        }
    };

    if charset_offset >= cff_data.len() {
        eprintln!("DEBUG: charset_offset out of bounds: {}", charset_offset);
        return None;
    }

    let format = cff_data[charset_offset];
    let mut charset_pos = charset_offset + 1;

    eprintln!("DEBUG: CFF charset format={} offset={}", format, charset_offset);

    // GID 0 is .notdef (CID 0). Charset starts at GID 1.
    if target_cid == 0 {
        return Some(0);
    }

    if format == 0 {
        let mut gid = 1;
        while charset_pos + 1 < cff_data.len() {
            let cid = ((cff_data[charset_pos] as u16) << 8) | (cff_data[charset_pos + 1] as u16);
            if cid == target_cid {
                return Some(gid);
            }
            charset_pos += 2;
            gid += 1;
        }
    } else if format == 1 || format == 2 {
        let mut gid = 1;
        while charset_pos + 2 < cff_data.len() {
            let first_cid =
                ((cff_data[charset_pos] as u16) << 8) | (cff_data[charset_pos + 1] as u16);
            let n_left = if format == 1 {
                cff_data[charset_pos + 2] as u16
            } else {
                ((cff_data[charset_pos + 2] as u16) << 8) | (cff_data[charset_pos + 3] as u16)
            };
            if target_cid >= first_cid && target_cid <= first_cid + n_left {
                eprintln!(
                    "DEBUG: Found CID match! target={}, first={}, n_left={}, baseline_gid={}",
                    target_cid, first_cid, n_left, gid
                );
                return Some(gid + (target_cid - first_cid));
            }
            charset_pos += if format == 1 { 3 } else { 4 };
            gid += n_left + 1;
        }
    }
    eprintln!("DEBUG: CFF charset fell through for cid={}", target_cid);
    None
}

impl SkrifaBridge {
    pub fn new() -> Self {
        Self {}
    }

    pub fn extract_path(
        &self,
        data: &[u8],
        mut gid: u32,
        _is_vertical: bool,
        unicode_fallback: Option<char>,
    ) -> Option<BezPath> {
        // If it's a bare CFF font, it might be a CID mapped subset.
        // Map CID -> GID before doing anything else!
        if data.len() >= 2 && data[0] == 0x01 && data[1] == 0x00 {
            gid = cff_get_gid_for_cid(data, gid as u16).unwrap_or(gid as u16) as u32;
        }

        let sfnt_data = ensure_sfnt(data);
        let data_ref = sfnt_data.as_deref().unwrap_or(data);

        // Try skrifa first
        if let Ok(file) = FileRef::new(data_ref) {
            let font_opt = match file {
                FileRef::Font(f) => Some(f),
                FileRef::Collection(c) => c.get(0).ok(),
            };
            if let Some(font) = font_opt {
                let mut pen = KurboPen::new();
                let _settings = DrawSettings::unhinted(Size::new(1000.0), LocationRef::default());
                let outlines = font.outline_glyphs();

                // Attempt 1: Direct GID lookup (Interpreter provides resolved GIDs)
                let mut path_found = false;
                let mut path = BezPath::new();

                if let Some(glyph) = outlines.get(GlyphId::new(gid))
                    && glyph
                        .draw(
                            DrawSettings::unhinted(Size::new(1000.0), LocationRef::default()),
                            &mut pen,
                        )
                        .is_ok()
                {
                    path = pen.finish();
                    path_found = true;
                }

                // Attempt 2: Charmap via Unicode (Secondary safety fallback)
                if !path_found
                    && let Some(uch) = unicode_fallback
                    && uch != '\0'
                {
                    let charmap = font.charmap();
                    if let Some(mapped_gid) = charmap.map(uch) {
                        let mut pen2 = KurboPen::new();
                        if let Some(glyph) = outlines.get(mapped_gid)
                            && glyph
                                .draw(
                                    DrawSettings::unhinted(
                                        Size::new(1000.0),
                                        LocationRef::default(),
                                    ),
                                    &mut pen2,
                                )
                                .is_ok()
                        {
                            path = pen2.finish();
                            path_found = true;
                        }
                    }
                }

                if path_found {
                    return Some(path);
                }
            }
        }

        // Fallback: ttf-parser (natively supports raw CFF and sometimes maps CIDs implicitly)
        // For CFF-based CIDFonts, we might need a CID -> GID mapping if skip_sfnt didn't handle it.
        let mut final_gid = gid;
        if data.len() >= 2
            && ((data[0] == 0x01 && data[1] == 0x00) || &data[0..4] == b"OTTO")
            && let Some(mapped_gid) = cff_get_gid_for_cid(data, gid as u16)
        {
            final_gid = mapped_gid as u32;
        }

        if let Ok(face) = ttf_parser::Face::parse(data, 0) {
            let mut pen = KurboPen::new();
            if let Some(_rect) = face.outline_glyph(ttf_parser::GlyphId(final_gid as u16), &mut pen)
            {
                return Some(pen.finish());
            }
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

        for (gid, width) in glyphs {
            if let Some(path) = self.extract_path(font_data, *gid, false, None) {
                let transform = kurbo::Affine::translate((x_offset, 0.0))
                    * kurbo::Affine::scale_non_uniform(scale as f64 * h_scale as f64, scale as f64);

                let mut path = path;
                path.apply_affine(transform);
                combined_path.extend(path);
            }

            x_offset += *width as f64 * scale as f64 * h_scale as f64;
            x_offset += options.char_spacing as f64 * h_scale as f64;

            if *gid == 32 {
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
