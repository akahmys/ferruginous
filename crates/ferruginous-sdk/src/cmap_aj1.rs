/// Mapping from Adobe-Japan1 CID to Unicode.
///
/// This is a partial implementation focusing on common Japanese characters.
pub fn cid_to_unicode_aj1(cid: u32) -> Option<char> {
    // 1. ASCII-like range (CIDs 1-230)
    if (1..=94).contains(&cid) {
        // Many of these map directly to ASCII offset by 31?
        // Actually AJ1 CID 1 is space (32), CID 2 is ! (33)
        return std::char::from_u32(cid + 31);
    }
    
    // 2. Hiragana (AJ1 CIDs 9354-9436 roughly, but varies by collection)
    // A more reliable way is to map standard ranges.
    
    // Standard JIS X 0208 mapping for Adobe-Japan1:
    // This is a simplified version.
    match cid {
        // Full-width numbers (0-9)
        232..=241 => std::char::from_u32(0xFF10 + (cid - 232)),
        // Full-width Uppercase (A-Z)
        243..=268 => std::char::from_u32(0xFF21 + (cid - 243)),
        // Full-width Lowercase (a-z)
        269..=294 => std::char::from_u32(0xFF41 + (cid - 269)),
        
        // common punctuation
        1 => Some(' '),
        6 => Some('・'),
        7 => Some('。'),
        8 => Some('、'),
        
        // Hiragana: CID 9354 ('ぁ') to 9443 ('ん')
        // (Note: CIDs depend on the specific AJ1 version, but 1-3 is most common)
        // Let's use a more robust range-based approach if possible.
        
        _ => None,
    }
}

/// Returns true if a CID in AJ1 is a space character.
pub fn is_aj1_space(cid: u32) -> bool {
    cid == 1 || cid == 231 // CID 1 = Space, CID 231 = Ideographic Space (U+3000)
}
