//! PDF Text String Decoding (ISO 32000-2:2020 Clause 7.9.2.2)

/// Decodes a PDF text string from raw bytes into a Rust UTF-8 String.
///
/// Supports PDF 2.0 UTF-8 (with BOM), UTF-16BE (with BOM), and PDFDocEncoding.
pub fn decode_text_string(bytes: &[u8]) -> String {
    if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
        // UTF-8 with BOM (PDF 2.0 extension to Text Strings)
        String::from_utf8_lossy(&bytes[3..]).into_owned()
    } else if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        // UTF-16BE
        let mut u16_data = Vec::with_capacity((bytes.len() - 2) / 2);
        let mut i = 2;
        while i + 1 < bytes.len() {
            u16_data.push(u16::from_be_bytes([bytes[i], bytes[i+1]]));
            i += 2;
        }
        String::from_utf16_lossy(&u16_data)
    } else {
        // PDFDocEncoding (Fallback to Latin-1 approximation)
        // Note: A strict ISO 32000-2 implementation uses the exact 256-char PDFDocEncoding table,
        // which maps some unused Latin-1 characters to specific typographic glyphs.
        bytes.iter().map(|&b| b as char).collect()
    }
}
