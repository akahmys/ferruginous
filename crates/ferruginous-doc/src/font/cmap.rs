use ferruginous_core::PdfResult;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Represents a mapping result from a CMap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MappingResult {
    Cid(u32),
    Unicode(Vec<u8>),
}

/// Defines a valid range of character codes.
/// (ISO 32000-2:2020 Clause 9.7.5.3)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeSpaceRange {
    pub start: Vec<u8>,
    pub end: Vec<u8>,
}

impl CodeSpaceRange {
    pub fn matches(&self, code: &[u8]) -> bool {
        if code.len() != self.start.len() {
            return false;
        }
        for (i, &b) in code.iter().enumerate() {
            if b < self.start[i] || b > self.end[i] {
                return false;
            }
        }
        true
    }
}

/// A Character Map (CMap) defines the mapping from character codes to CIDs or Unicode.
#[derive(Debug, Clone, Default)]
pub struct CMap {
    pub name: String,
    pub is_vertical: bool,
    pub is_identity: bool,
    pub codespace_ranges: Vec<CodeSpaceRange>,
    pub code_to_cid: BTreeMap<Vec<u8>, u32>,
    pub code_to_unicode: BTreeMap<Vec<u8>, Vec<u8>>,
}

impl CMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Segments the input byte stream into the next character code based on codespace ranges.
    pub fn next_code(&self, data: &[u8]) -> Option<(Vec<u8>, usize)> {
        for range in &self.codespace_ranges {
            let len = range.start.len();
            if data.len() >= len {
                let code = &data[..len];
                if range.matches(code) {
                    return Some((code.to_vec(), len));
                }
            }
        }
        // Fallback to 1 byte if no range matches (common for some malformed files)
        if !data.is_empty() { Some((vec![data[0]], 1)) } else { None }
    }

    pub fn lookup(&self, code: &[u8]) -> Option<MappingResult> {
        // 0. Handle Identity Mapping (CID == Code)
        if self.is_identity {
            return Some(MappingResult::Cid(bytes_to_u64(code) as u32));
        }

        // 1. Try exact match
        if let Some(&cid) = self.code_to_cid.get(code) {
            return Some(MappingResult::Cid(cid));
        }
        if let Some(unicode) = self.code_to_unicode.get(code) {
            return Some(MappingResult::Unicode(unicode.to_vec()));
        }

        // 2. Try normalized match (strip leading zeros)
        let mut stripped = code;
        while stripped.len() > 1 && stripped[0] == 0 {
            stripped = &stripped[1..];
            if let Some(&cid) = self.code_to_cid.get(stripped) {
                return Some(MappingResult::Cid(cid));
            }
            if let Some(unicode) = self.code_to_unicode.get(stripped) {
                return Some(MappingResult::Unicode(unicode.to_vec()));
            }
        }

        // 3. Try padded match (add leading zeros)
        for target_len in [2, 4] {
            if code.len() < target_len {
                let mut padded = vec![0u8; target_len - code.len()];
                padded.extend_from_slice(code);
                if let Some(&cid) = self.code_to_cid.get(&padded) {
                    return Some(MappingResult::Cid(cid));
                }
                if let Some(unicode) = self.code_to_unicode.get(&padded) {
                    return Some(MappingResult::Unicode(unicode.clone()));
                }
            }
        }

        None
    }

    /// Parses a CMap from a byte stream using a token-based approach.
    pub fn parse(data: &[u8]) -> PdfResult<Self> {
        let mut cmap = Self::new();
        let mut lexer = CMapLexer::new(data);

        while let Some(token) = lexer.next()? {
            match token {
                CMapToken::Keyword(k) => match k.as_str() {
                    "begincodespacerange" => cmap.parse_codespace_range(&mut lexer)?,
                    "beginbfchar" => cmap.parse_bfchar(&mut lexer)?,
                    "beginbfrange" => cmap.parse_bfrange(&mut lexer)?,
                    "begincidchar" => cmap.parse_cidchar(&mut lexer)?,
                    "begincidrange" => cmap.parse_cidrange(&mut lexer)?,
                    _ => {}
                },
                CMapToken::Name(n) if n == "CMapName" => {
                    if let Some(CMapToken::Name(name)) = lexer.next()? {
                        cmap.name = name;
                    }
                }
                CMapToken::Name(n) if n == "WMode" => {
                    if let Some(CMapToken::Integer(i)) = lexer.next()? {
                        cmap.is_vertical = i == 1;
                    }
                }
                _ => {}
            }
        }

        Ok(cmap)
    }

    fn parse_codespace_range(&mut self, lexer: &mut CMapLexer) -> PdfResult<()> {
        while let Some(token) = lexer.next()? {
            match token {
                CMapToken::HexString(start) => {
                    if let Some(CMapToken::HexString(end)) = lexer.next()? {
                        self.codespace_ranges.push(CodeSpaceRange { start, end });
                    }
                }
                CMapToken::Keyword(k) if k == "endcodespacerange" => break,
                _ => {}
            }
        }
        Ok(())
    }

    fn parse_bfchar(&mut self, lexer: &mut CMapLexer) -> PdfResult<()> {
        while let Some(token) = lexer.next()? {
            match token {
                CMapToken::HexString(code) => {
                    if let Some(CMapToken::HexString(uni)) = lexer.next()? {
                        self.code_to_unicode.insert(code, uni);
                    }
                }
                CMapToken::Keyword(k) if k == "endbfchar" => break,
                _ => {}
            }
        }
        Ok(())
    }

    fn parse_bfrange(&mut self, lexer: &mut CMapLexer) -> PdfResult<()> {
        while let Some(token) = lexer.next()? {
            match token {
                CMapToken::HexString(start) => {
                    let end = match lexer.next()? {
                        Some(CMapToken::HexString(e)) => e,
                        _ => break,
                    };
                    match lexer.next()? {
                        Some(CMapToken::HexString(uni_start)) => {
                            self.expand_unicode_range(start, end, uni_start);
                        }
                        Some(CMapToken::Array(arr)) => {
                            self.expand_unicode_array(start, end, arr);
                        }
                        _ => break,
                    }
                }
                CMapToken::Keyword(k) if k == "endbfrange" => break,
                _ => {}
            }
        }
        Ok(())
    }

    fn parse_cidchar(&mut self, lexer: &mut CMapLexer) -> PdfResult<()> {
        while let Some(token) = lexer.next()? {
            match token {
                CMapToken::HexString(code) => {
                    if let Some(CMapToken::Integer(cid)) = lexer.next()? {
                        self.code_to_cid.insert(code, cid as u32);
                    }
                }
                CMapToken::Keyword(k) if k == "endcidchar" => break,
                _ => {}
            }
        }
        Ok(())
    }

    fn parse_cidrange(&mut self, lexer: &mut CMapLexer) -> PdfResult<()> {
        while let Some(token) = lexer.next()? {
            match token {
                CMapToken::HexString(start) => {
                    let end = match lexer.next()? {
                        Some(CMapToken::HexString(e)) => e,
                        _ => break,
                    };
                    if let Some(CMapToken::Integer(cid_start)) = lexer.next()? {
                        self.expand_cid_range(start, end, cid_start as u32);
                    }
                }
                CMapToken::Keyword(k) if k == "endcidrange" => break,
                _ => {}
            }
        }
        Ok(())
    }

    fn expand_unicode_range(&mut self, start: Vec<u8>, end: Vec<u8>, uni: Vec<u8>) {
        let s = bytes_to_u64(&start);
        let e = bytes_to_u64(&end);
        let mut u = bytes_to_u64(&uni);
        for i in s..=e {
            self.code_to_unicode.insert(u64_to_bytes(i, start.len()), u64_to_bytes(u, uni.len()));
            u += 1;
        }
    }

    fn expand_unicode_array(&mut self, start: Vec<u8>, end: Vec<u8>, arr: Vec<CMapToken>) {
        let s = bytes_to_u64(&start);
        let e = bytes_to_u64(&end);
        for (idx, i) in (s..=e).enumerate() {
            if let Some(CMapToken::HexString(uni)) = arr.get(idx) {
                self.code_to_unicode.insert(u64_to_bytes(i, start.len()), uni.clone());
            }
        }
    }

    fn expand_cid_range(&mut self, start: Vec<u8>, end: Vec<u8>, mut cid: u32) {
        let s = bytes_to_u64(&start);
        let e = bytes_to_u64(&end);
        for i in s..=e {
            self.code_to_cid.insert(u64_to_bytes(i, start.len()), cid);
            cid += 1;
        }
    }
}

fn bytes_to_u64(bytes: &[u8]) -> u64 {
    let mut val = 0u64;
    for &b in bytes {
        val = (val << 8) | (b as u64);
    }
    val
}

fn u64_to_bytes(val: u64, len: usize) -> Vec<u8> {
    let mut bytes = vec![0u8; len];
    let mut v = val;
    for i in (0..len).rev() {
        bytes[i] = (v & 0xff) as u8;
        v >>= 8;
    }
    bytes
}

// --- CMap Lexer Implementation ---

#[derive(Debug, Clone, PartialEq)]
pub enum CMapToken {
    HexString(Vec<u8>),
    Integer(i64),
    Name(String),
    Keyword(String),
    Array(Vec<CMapToken>),
}

struct CMapLexer<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> CMapLexer<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self { input, pos: 0 }
    }

    fn next(&mut self) -> PdfResult<Option<CMapToken>> {
        self.skip_whitespace();
        if self.pos >= self.input.len() {
            return Ok(None);
        }
        let c = self.input[self.pos];
        match c {
            b'<' => self.lex_hex_string(),
            b'/' => self.lex_name(),
            b'[' => self.lex_array(),
            b'0'..=b'9' | b'-' => self.lex_number(),
            _ if c.is_ascii_alphabetic() => self.lex_keyword(),
            _ => {
                self.pos += 1;
                self.next()
            } // Skip unknown
        }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len()
            && matches!(self.input[self.pos], 0 | 9 | 10 | 12 | 13 | 32)
        {
            self.pos += 1;
        }
    }

    fn lex_hex_string(&mut self) -> PdfResult<Option<CMapToken>> {
        self.pos += 1; // '<'
        let mut hex = Vec::new();
        while self.pos < self.input.len() && self.input[self.pos] != b'>' {
            let c = self.input[self.pos];
            if c.is_ascii_hexdigit() {
                hex.push(c);
            }
            self.pos += 1;
        }
        if self.pos < self.input.len() {
            self.pos += 1;
        } // '>'

        let mut bytes = Vec::new();
        let mut i = 0;
        while i < hex.len() {
            let b1 = hex[i];
            let b2 = if i + 1 < hex.len() { hex[i + 1] } else { b'0' };

            let h1 = char::from(b1).to_digit(16).unwrap_or(0) as u8;
            let h2 = char::from(b2).to_digit(16).unwrap_or(0) as u8;
            bytes.push((h1 << 4) | h2);
            i += 2;
        }
        Ok(Some(CMapToken::HexString(bytes)))
    }

    fn lex_name(&mut self) -> PdfResult<Option<CMapToken>> {
        self.pos += 1; // '/'
        let start = self.pos;
        while self.pos < self.input.len()
            && !matches!(
                self.input[self.pos],
                0 | 9 | 10 | 12 | 13 | 32 | b'/' | b'<' | b'>' | b'[' | b']'
            )
        {
            self.pos += 1;
        }
        let name = std::str::from_utf8(&self.input[start..self.pos]).unwrap_or("").to_string();
        Ok(Some(CMapToken::Name(name)))
    }

    fn lex_keyword(&mut self) -> PdfResult<Option<CMapToken>> {
        let start = self.pos;
        while self.pos < self.input.len() && self.input[self.pos].is_ascii_alphabetic() {
            self.pos += 1;
        }
        let k = std::str::from_utf8(&self.input[start..self.pos]).unwrap_or("").to_string();
        Ok(Some(CMapToken::Keyword(k)))
    }

    fn lex_number(&mut self) -> PdfResult<Option<CMapToken>> {
        let start = self.pos;
        while self.pos < self.input.len()
            && (self.input[self.pos].is_ascii_digit() || self.input[self.pos] == b'-')
        {
            self.pos += 1;
        }
        let s = std::str::from_utf8(&self.input[start..self.pos]).unwrap_or("");
        let val = s.parse::<i64>().unwrap_or(0);
        Ok(Some(CMapToken::Integer(val)))
    }

    fn lex_array(&mut self) -> PdfResult<Option<CMapToken>> {
        self.pos += 1; // '['
        let mut arr = Vec::new();
        while let Some(token) = self.next()? {
            arr.push(token);
            self.skip_whitespace();
            if self.pos < self.input.len() && self.input[self.pos] == b']' {
                self.pos += 1;
                break;
            }
        }
        Ok(Some(CMapToken::Array(arr)))
    }
}

/// Global registry for built-in and cached CMaps.
pub struct CMapRegistry {
    maps: BTreeMap<String, Arc<CMap>>,
}

impl Default for CMapRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CMapRegistry {
    pub fn new() -> Self {
        let mut registry = Self { maps: BTreeMap::new() };
        registry.register_builtins();
        registry
    }

    fn register_builtins(&mut self) {
        // Identity-H: Horizontal identity mapping (CID == Code)
        let mut identity_h = CMap::new();
        identity_h.name = "Identity-H".to_string();
        identity_h.is_identity = true;
        identity_h.codespace_ranges.push(CodeSpaceRange { start: vec![0, 0], end: vec![255, 255] });
        self.maps.insert("Identity-H".to_string(), Arc::new(identity_h));

        // Identity-V: Vertical identity mapping
        let mut identity_v = CMap::new();
        identity_v.name = "Identity-V".to_string();
        identity_v.is_vertical = true;
        identity_v.is_identity = true;
        identity_v.codespace_ranges.push(CodeSpaceRange { start: vec![0, 0], end: vec![255, 255] });
        self.maps.insert("Identity-V".to_string(), Arc::new(identity_v));

        // UniJIS-UTF16-H: Standard Japanese Unicode mapping (Horizontal)
        let mut unijis_h = CMap::new();
        unijis_h.name = "UniJIS-UTF16-H".to_string();
        unijis_h.is_identity = true; // Maps 2-byte code to Unicode (UCS-2)
        unijis_h.codespace_ranges.push(CodeSpaceRange { start: vec![0, 0], end: vec![255, 255] });
        self.maps.insert("UniJIS-UTF16-H".to_string(), Arc::new(unijis_h));

        // 90ms-RKSJ-H: Shift-JIS mapping (Horizontal)
        let mut rksj_h = CMap::new();
        rksj_h.name = "90ms-RKSJ-H".to_string();
        rksj_h
            .codespace_ranges
            .push(CodeSpaceRange { start: vec![0x81, 0x40], end: vec![0x9f, 0xfc] });
        rksj_h
            .codespace_ranges
            .push(CodeSpaceRange { start: vec![0xe0, 0x40], end: vec![0xfc, 0xfc] });
        rksj_h.codespace_ranges.push(CodeSpaceRange { start: vec![0x00], end: vec![0x7f] });
        rksj_h.codespace_ranges.push(CodeSpaceRange { start: vec![0xa1], end: vec![0xdf] });
        self.maps.insert("90ms-RKSJ-H".to_string(), Arc::new(rksj_h));
    }

    pub fn get(&self, name: &str) -> Option<Arc<CMap>> {
        self.maps.get(name).cloned()
    }
}

pub fn get_builtin_cmap(name: &str) -> Option<Arc<CMap>> {
    static REGISTRY: std::sync::OnceLock<CMapRegistry> = std::sync::OnceLock::new();
    REGISTRY.get_or_init(CMapRegistry::new).get(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bfchar() {
        let data = b"/CMapName /Test def\n1 beginbfchar\n<0001> <0020>\nendbfchar";
        let cmap = CMap::parse(data).unwrap();
        assert_eq!(cmap.name, "Test");
        assert_eq!(cmap.lookup(&[0, 1]), Some(MappingResult::Unicode(vec![0, 32])));
    }

    #[test]
    fn test_codespace_segmentation() {
        let mut cmap = CMap::new();
        cmap.codespace_ranges
            .push(CodeSpaceRange { start: vec![0x81, 0x40], end: vec![0x9f, 0xfc] });
        cmap.codespace_ranges.push(CodeSpaceRange { start: vec![0x00], end: vec![0x7f] });

        // Single byte match
        assert_eq!(cmap.next_code(&[0x41, 0x42]), Some((vec![0x41], 1)));
        // Double byte match
        assert_eq!(cmap.next_code(&[0x81, 0x40, 0x41]), Some((vec![0x81, 0x40], 2)));
    }
}
