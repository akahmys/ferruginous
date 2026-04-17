use std::collections::BTreeMap;
use ferruginous_core::{PdfResult, PdfError};

/// Represents a mapping result from a CMap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MappingResult {
    Cid(u32),
    Unicode(Vec<u8>),
}

/// A Character Map (CMap) defines the mapping from character codes to CIDs or Unicode.
/// (ISO 32000-2:2020 Clause 9.7.5)
#[derive(Debug, Clone, Default)]
pub struct CMap {
    pub name: String,
    pub is_vertical: bool,
    pub code_to_cid: BTreeMap<Vec<u8>, u32>,
    pub code_to_unicode: BTreeMap<Vec<u8>, Vec<u8>>,
    // Ranges are handled by expanding them into the maps for simplicity in this initial version,
    // though for large CMaps we should use a more efficient range-based structure.
}

impl CMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Looks up a character code in the CMap.
    pub fn lookup(&self, code: &[u8]) -> Option<MappingResult> {
        if let Some(&cid) = self.code_to_cid.get(code) {
            return Some(MappingResult::Cid(cid));
        }
        if let Some(unicode) = self.code_to_unicode.get(code) {
            return Some(MappingResult::Unicode(unicode.clone()));
        }
        None
    }

    /// Parses a CMap from a byte stream.
    /// This is a simplified "clean slate" parser focusing on the most common blocks.
    pub fn parse(data: &[u8]) -> PdfResult<Self> {
        let mut cmap = Self::new();
        let content = std::str::from_utf8(data).map_err(|_| PdfError::Other("Invalid UTF-8 in CMap".into()))?;
        
        let lines: Vec<&str> = content.lines().map(|s| s.trim()).collect();
        let mut i = 0;
        
        while i < lines.len() {
            let line = lines[i];
            
            if line.contains("/CMapName") {
                 // Format: /CMapName /Name def
                 if let Some(name_part) = line.split('/').nth(2) {
                     cmap.name = name_part.split_whitespace().next().unwrap_or("").to_string();
                 }
            } else if line.contains("/WMode") {
                 // Format: /WMode 1 def
                 if line.contains(" 1 ") || line.ends_with(" 1 def") {
                     cmap.is_vertical = true;
                 }
            } else if line.contains("beginbfchar") {
                let count = line.split_whitespace().next().and_then(|s| s.parse::<usize>().ok()).unwrap_or(0);
                i += 1;
                for _ in 0..count {
                    if i >= lines.len() { break; }
                    Self::parse_bfchar(lines[i], &mut cmap.code_to_unicode);
                    i += 1;
                }
                continue;
            } else if line.contains("beginbfrange") {
                let count = line.split_whitespace().next().and_then(|s| s.parse::<usize>().ok()).unwrap_or(0);
                i += 1;
                for _ in 0..count {
                    if i >= lines.len() { break; }
                    Self::parse_bfrange(lines[i], &mut cmap.code_to_unicode);
                    i += 1;
                }
                continue;
            } else if line.contains("begincidchar") {
                let count = line.split_whitespace().next().and_then(|s| s.parse::<usize>().ok()).unwrap_or(0);
                i += 1;
                for _ in 0..count {
                    if i >= lines.len() { break; }
                    Self::parse_cidchar(lines[i], &mut cmap.code_to_cid);
                    i += 1;
                }
                continue;
            } else if line.contains("begincidrange") {
                let count = line.split_whitespace().next().and_then(|s| s.parse::<usize>().ok()).unwrap_or(0);
                i += 1;
                for _ in 0..count {
                    if i >= lines.len() { break; }
                    Self::parse_cidrange(lines[i], &mut cmap.code_to_cid);
                    i += 1;
                }
                continue;
            }
            
            i += 1;
        }
        
        Ok(cmap)
    }

    fn parse_bfchar(line: &str, map: &mut BTreeMap<Vec<u8>, Vec<u8>>) {
        // Format: <code1> <unicode1>
        let parts: Vec<&str> = line.split(['<', '>']).filter(|s| !s.trim().is_empty()).collect();
        if parts.len() >= 2
            && let (Some(code), Some(unicode)) = (hex_to_bytes(parts[0]), hex_to_bytes(parts[1])) {
                map.insert(code, unicode);
            }
    }

    fn parse_bfrange(line: &str, map: &mut BTreeMap<Vec<u8>, Vec<u8>>) {
        // Format: <start> <end> <unicode_start>
        // OR: <start> <end> [ <unicode1> <unicode2> ... ]
        let parts: Vec<&str> = line.split(['<', '>', '[', ']', ' ']).filter(|s| !s.trim().is_empty()).collect();
        if parts.len() < 3 { return; }
        
        let start_bytes = hex_to_bytes(parts[0]);
        let end_bytes = hex_to_bytes(parts[1]);
        
        if let (Some(start), Some(end)) = (start_bytes, end_bytes) {
            let start_val = bytes_to_val(&start);
            let end_val = bytes_to_val(&end);
            
            // Check if third part is an array start or a single hex
            if line.contains('[') {
                // Simplified array parsing: we only handle if everything is on one line for now
                // Real implementation would need a more robust stateful parser.
            } else {
                if let Some(uni_start) = hex_to_bytes(parts[2]) {
                    let mut uni_val = bytes_to_val(&uni_start);
                    for v in start_val..=end_val {
                        let code = val_to_bytes(v, start.len());
                        let unicode = val_to_bytes(uni_val, uni_start.len());
                        map.insert(code, unicode);
                        uni_val += 1;
                    }
                }
            }
        }
    }

    fn parse_cidchar(line: &str, map: &mut BTreeMap<Vec<u8>, u32>) {
        // Format: <code1> cid1
        let parts: Vec<&str> = line.split(['<', '>', ' ']).filter(|s| !s.trim().is_empty()).collect();
        if parts.len() >= 2
            && let Some(code) = hex_to_bytes(parts[0])
                && let Ok(cid) = parts[1].parse::<u32>() {
                    map.insert(code, cid);
                }
    }

    fn parse_cidrange(line: &str, map: &mut BTreeMap<Vec<u8>, u32>) {
        // Format: <start> <end> cid_start
        let parts: Vec<&str> = line.split(['<', '>', ' ']).filter(|s| !s.trim().is_empty()).collect();
        if parts.len() < 3 { return; }
        
        if let (Some(start), Some(end)) = (hex_to_bytes(parts[0]), hex_to_bytes(parts[1])) {
            let start_val = bytes_to_val(&start);
            let end_val = bytes_to_val(&end);
            if let Ok(mut cid) = parts[2].parse::<u32>() {
                for v in start_val..=end_val {
                    let code = val_to_bytes(v, start.len());
                    map.insert(code, cid);
                    cid += 1;
                }
            }
        }
    }
}

fn hex_to_bytes(hex: &str) -> Option<Vec<u8>> {
    let hex = hex.trim();
    if !hex.len().is_multiple_of(2) { return None; }
    let mut bytes = Vec::new();
    for i in (0..hex.len()).step_by(2) {
        let b = u8::from_str_radix(&hex[i..i+2], 16).ok()?;
        bytes.push(b);
    }
    Some(bytes)
}

fn bytes_to_val(bytes: &[u8]) -> u64 {
    let mut val = 0;
    for &b in bytes {
        val = (val << 8) | (b as u64);
    }
    val
}

fn val_to_bytes(val: u64, len: usize) -> Vec<u8> {
    let mut bytes = vec![0; len];
    let mut v = val;
    for i in (0..len).rev() {
        bytes[i] = (v & 0xFF) as u8;
        v >>= 8;
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bfchar() {
        let data = b"1 beginbfchar\n<0001> <0020>\nendbfchar";
        let cmap = CMap::parse(data).unwrap();
        assert_eq!(cmap.lookup(&[0, 1]), Some(MappingResult::Unicode(vec![0, 32])));
    }

    #[test]
    fn test_parse_bfrange() {
        let data = b"1 beginbfrange\n<0001> <0002> <0020>\nendbfrange";
        let cmap = CMap::parse(data).unwrap();
        assert_eq!(cmap.lookup(&[0, 1]), Some(MappingResult::Unicode(vec![0, 32])));
        assert_eq!(cmap.lookup(&[0, 2]), Some(MappingResult::Unicode(vec![0, 33])));
    }
}
