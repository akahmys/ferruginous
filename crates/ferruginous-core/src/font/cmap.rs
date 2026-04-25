//! CMap (Character Map) Parser (ISO 32000-2 Clause 9.7)

use crate::PdfResult;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct CMap {
    pub name: String,
    pub wmode: i32,
    pub codespace_ranges: Vec<(Vec<u8>, Vec<u8>)>,
    pub mappings: BTreeMap<Vec<u8>, String>,
    pub mappings_cid: BTreeMap<Vec<u8>, u32>,
}

impl CMap {
    pub fn parse(data: &[u8]) -> PdfResult<Self> {
        let mut cmap = Self::default();
        let tokens = tokenize_cmap(data);

        let mut i = 0;
        let mut safety = 0;
        while i < tokens.len() {
            safety += 1;
            if safety > 1_000_000 {
                break;
            }
            match tokens[i] {
                b"/CMapName" => {
                    if i + 1 < tokens.len() {
                        cmap.name = String::from_utf8_lossy(tokens[i + 1])
                            .trim_start_matches('/')
                            .to_string();
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                b"/WMode" => {
                    if i + 1 < tokens.len() {
                        cmap.wmode =
                            std::str::from_utf8(tokens[i + 1]).unwrap_or("0").parse().unwrap_or(0);
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                b"begincodespacerange" => {
                    let count_token: &[u8] = if i > 0 { tokens[i - 1] } else { b"0" };
                    let count = std::str::from_utf8(count_token)
                        .map(|s| s.parse::<usize>().unwrap_or(0))
                        .unwrap_or(0);
                    i += 1;
                    for _ in 0..count {
                        if i + 1 < tokens.len() {
                            let start = parse_cmap_bytes(tokens[i]);
                            let end = parse_cmap_bytes(tokens[i + 1]);
                            cmap.codespace_ranges.push((start, end));
                            i += 2;
                        }
                    }
                }
                b"beginbfchar" => {
                    let count_token: &[u8] = if i > 0 { tokens[i - 1] } else { b"0" };
                    let count = std::str::from_utf8(count_token)
                        .map(|s| s.parse::<usize>().unwrap_or(0))
                        .unwrap_or(0);
                    i += 1;
                    for _ in 0..count {
                        if i + 1 < tokens.len() {
                            let src = parse_cmap_bytes(tokens[i]);
                            let dst_token = &tokens[i + 1];
                            let dst = if dst_token.starts_with(b"/") {
                                glyph_name_to_unicode(dst_token)
                            } else {
                                parse_cmap_string(dst_token)
                            };
                            cmap.mappings.insert(src, dst);
                            i += 2;
                        }
                    }
                    i += 1; // Advance past the last token in the block
                }
                b"beginbfrange" => {
                    let count_token: &[u8] = if i > 0 { tokens[i - 1] } else { b"0" };
                    let count = std::str::from_utf8(count_token)
                        .map(|s| s.parse::<usize>().unwrap_or(0))
                        .unwrap_or(0);
                    i += 1;
                    for _ in 0..count {
                        if i + 2 < tokens.len() {
                            let start = parse_cmap_bytes(tokens[i]);
                            let end = parse_cmap_bytes(tokens[i + 1]);
                            let dst_base = &tokens[i + 2];
                            i += 3;

                            if dst_base.starts_with(b"<") || dst_base.starts_with(b"(") {
                                let start_code_val = vec_to_u32(&start);
                                let end_code_val = vec_to_u32(&end);
                                let mut start_uni_val = vec_to_u32(&parse_cmap_bytes(dst_base));

                                for code in start_code_val..=end_code_val {
                                    let code_vec = u32_to_vec(code, start.len());
                                    if let Some(c) = std::char::from_u32(start_uni_val) {
                                        cmap.mappings.insert(code_vec, c.to_string());
                                    } else {
                                        let uni_str =
                                            String::from_utf16_lossy(&[start_uni_val as u16]);
                                        cmap.mappings.insert(code_vec, uni_str);
                                    }
                                    start_uni_val += 1;
                                }
                            } else if dst_base == b"[" {
                                let start_code_val = vec_to_u32(&start);
                                let mut offset = 0;
                                while i < tokens.len() && tokens[i] != b"]" {
                                    let uni_str = parse_cmap_string(tokens[i]);
                                    let code_vec = u32_to_vec(start_code_val + offset, start.len());
                                    cmap.mappings.insert(code_vec, uni_str);
                                    i += 1;
                                    offset += 1;
                                }
                                if i < tokens.len() && tokens[i] == b"]" {
                                    i += 1;
                                }
                            }
                        }
                    }
                    i += 1; // Advance past the last token in the block
                }
                b"begincidchar" => {
                    let count_token: &[u8] = if i > 0 { tokens[i - 1] } else { b"0" };
                    let count = std::str::from_utf8(count_token)
                        .map(|s| s.parse::<usize>().unwrap_or(0))
                        .unwrap_or(0);
                    i += 1;
                    for _ in 0..count {
                        if i + 1 < tokens.len() {
                            let src = parse_cmap_bytes(tokens[i]);
                            let cid = std::str::from_utf8(tokens[i + 1])
                                .unwrap_or("0")
                                .parse::<u32>()
                                .unwrap_or(0);
                            cmap.mappings_cid.insert(src, cid);
                            i += 2;
                        }
                    }
                }
                b"begincidrange" => {
                    let count_token: &[u8] = if i > 0 { tokens[i - 1] } else { b"0" };
                    let count = std::str::from_utf8(count_token)
                        .map(|s| s.parse::<usize>().unwrap_or(0))
                        .unwrap_or(0);
                    i += 1;
                    for _ in 0..count {
                        if i + 2 < tokens.len() {
                            let start = parse_cmap_bytes(tokens[i]);
                            let end = parse_cmap_bytes(tokens[i + 1]);
                            let cid_base = std::str::from_utf8(tokens[i + 2])
                                .unwrap_or("0")
                                .parse::<u32>()
                                .unwrap_or(0);
                            i += 3;

                            let start_val = vec_to_u32(&start);
                            let end_val = vec_to_u32(&end);
                            for val in start_val..=end_val {
                                let code_vec = u32_to_vec(val, start.len());
                                cmap.mappings_cid.insert(code_vec, cid_base + (val - start_val));
                            }
                        }
                    }
                }
                _ => i += 1,
            }
        }

        Ok(cmap)
    }

    pub fn map(&self, code: &[u8]) -> Option<String> {
        self.mappings.get(code).cloned()
    }

    pub fn to_cid(&self, code: &[u8]) -> u32 {
        if let Some(&cid) = self.mappings_cid.get(code) {
            return cid;
        }
        if self.name.starts_with("Identity") {
            if code.len() == 2 {
                return ((code[0] as u32) << 8) | (code[1] as u32);
            } else if code.len() == 1 {
                return code[0] as u32;
            }
        }
        if let Some(s) = self.mappings.get(code) {
            if let Some(cid_str) = s.strip_prefix("CID:") {
                return cid_str.parse().unwrap_or(0);
            }
            if s.len() == 1 {
                return s.chars().next().map(|c| c as u32).unwrap_or(0);
            }
        }
        if code.len() == 2 {
            return ((code[0] as u32) << 8) | (code[1] as u32);
        }
        code.first().copied().unwrap_or(0) as u32
    }

    pub fn decode_next(&self, data: &[u8]) -> (usize, Option<String>) {
        if data.is_empty() {
            return (0, None);
        }
        for (start, end) in &self.codespace_ranges {
            let len = start.len();
            if data.len() >= len {
                let segment = &data[0..len];
                if segment >= start.as_slice() && segment <= end.as_slice() {
                    return (len, self.map(segment));
                }
            }
        }
        (1, self.map(&data[0..1]))
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn is_multibyte(&self) -> bool {
        self.name.contains("Identity")
            || self.name.contains("JIS")
            || self.name.contains("RKSJ")
            || self.name.contains("EUC")
            || self.name.contains("GB-")
            || self.name.contains("KSC-")
            || self.name.contains("UTF-16")
            || self.codespace_ranges.iter().any(|(s, _)| s.len() >= 2)
    }

    pub fn decode_next_strict(&self, data: &[u8]) -> Option<(usize, Option<String>)> {
        self.decode_next_with_min_len(data, None)
    }

    pub fn decode_next_with_min_len(
        &self,
        data: &[u8],
        min_len: Option<usize>,
    ) -> Option<(usize, Option<String>)> {
        if data.is_empty() {
            return None;
        }

        // 1. Try codespace ranges
        if !self.codespace_ranges.is_empty() {
            for (start, end) in &self.codespace_ranges {
                let len = start.len();
                if data.len() >= len {
                    let segment = &data[0..len];
                    if segment >= start.as_slice() && segment <= end.as_slice() {
                        let m = self.map(segment);
                        let final_len = min_len.unwrap_or(len).max(len);
                        return Some((final_len, m));
                    }
                }
            }
        }

        // 2. Try direct mappings (with fallback for 1-byte/2-byte mismatches)
        for key in self.mappings.keys() {
            let k_len = key.len();
            let d_len = min_len.unwrap_or(k_len).max(k_len);

            if data.len() >= d_len {
                // Case A: Exact match
                if &data[0..k_len] == key.as_slice() {
                    return Some((d_len, self.mappings.get(key).cloned()));
                }
                // Case B: 2-byte stream matching 1-byte key (Legacy Distiller artifact)
                if k_len == 1 && d_len == 2 && data[0] == 0 && data[1] == key[0] {
                    return Some((2, self.mappings.get(key).cloned()));
                }
            }
        }

        None
    }

    pub fn identity_h() -> Self {
        Self { name: "Identity-H".into(), ..Default::default() }
    }

    pub fn identity_v() -> Self {
        Self { name: "Identity-V".into(), wmode: 1, ..Default::default() }
    }

    pub fn rksj_h() -> Self {
        Self { name: "90ms-RKSJ-H".into(), ..Default::default() }
    }

    pub fn unijis_h() -> Self {
        Self { name: "UniJIS-UTF16-H".into(), ..Default::default() }
    }

    pub fn load_named(name: &str) -> Option<Self> {
        match name {
            "Identity-H" => Some(Self::identity_h()),
            "Identity-V" => Some(Self::identity_v()),
            "90ms-RKSJ-H" => Some(Self::rksj_h()),
            "UniJIS-UTF16-H" => Some(Self::unijis_h()),
            _ => None,
        }
    }
}

fn tokenize_cmap(data: &[u8]) -> Vec<&[u8]> {
    let mut tokens = Vec::new();
    let mut i = 0;
    let mut safety = 0;
    while i < data.len() {
        safety += 1;
        if safety > 10_000_000 {
            break;
        }

        let b = data[i];
        if b == b' ' || b == b'\n' || b == b'\r' || b == b'\t' || b == b'\0' || b == b'\x0C' {
            i += 1;
            continue;
        }
        if b == b'%' {
            while i < data.len() && data[i] != b'\n' && data[i] != b'\r' {
                i += 1;
            }
            continue;
        }

        let start = i;
        match b {
            b'(' => {
                i += 1;
                let mut depth = 1;
                while i < data.len() && depth > 0 {
                    if data[i] == b'(' && !is_escaped(data, i) {
                        depth += 1;
                    } else if data[i] == b')' && !is_escaped(data, i) {
                        depth -= 1;
                    }
                    i += 1;
                }
                tokens.push(&data[start..i]);
            }
            b'<' => {
                i += 1;
                while i < data.len() && data[i] != b'>' {
                    i += 1;
                }
                if i < data.len() {
                    i += 1;
                }
                tokens.push(&data[start..i]);
            }
            _ if b"()<>[]".contains(&b) => {
                tokens.push(&data[i..i + 1]);
                i += 1;
            }
            _ => {
                while i < data.len() && !b"()<>[] \n\r\t".contains(&data[i]) {
                    i += 1;
                }
                tokens.push(&data[start..i]);
            }
        }
    }
    tokens
}

fn is_escaped(data: &[u8], pos: usize) -> bool {
    let mut backslashes = 0;
    let mut j = pos as i32 - 1;
    while j >= 0 && data[j as usize] == b'\\' {
        backslashes += 1;
        j -= 1;
    }
    backslashes % 2 != 0
}

fn parse_cmap_bytes(v: &[u8]) -> Vec<u8> {
    if v.starts_with(b"<") {
        parse_hex(v)
    } else if v.starts_with(b"(") {
        parse_literal_bytes(v)
    } else {
        v.to_vec()
    }
}

fn parse_hex(v: &[u8]) -> Vec<u8> {
    // Filter to only hex digits, ignoring whitespace and delimiters
    let s = v
        .iter()
        .filter(|&&b| (b >= b'0' && b <= b'9') || (b >= b'a' && b <= b'f') || (b >= b'A' && b <= b'F'))
        .collect::<Vec<_>>();
    let mut bytes = Vec::new();
    for i in (0..s.len()).step_by(2) {
        if i + 1 < s.len() {
            let hi = (*s[i] as char).to_digit(16).unwrap_or(0);
            let lo = (*s[i + 1] as char).to_digit(16).unwrap_or(0);
            bytes.push(((hi << 4) | lo) as u8);
        } else {
            // ISO 32000-2: Odd number of digits: append a '0'
            let hi = (*s[i] as char).to_digit(16).unwrap_or(0);
            bytes.push((hi << 4) as u8);
        }
    }
    bytes
}

fn parse_literal_bytes(v: &[u8]) -> Vec<u8> {
    if v.len() < 2 {
        return Vec::new();
    }
    let content = &v[1..v.len() - 1];
    let mut result = Vec::new();
    let mut j = 0;
    while j < content.len() {
        let b = content[j];
        if b == b'\\' {
            j += 1;
            if j < content.len() {
                match content[j] {
                    b'n' => result.push(b'\n'),
                    b'r' => result.push(b'\r'),
                    b't' => result.push(b'\t'),
                    b'(' | b')' | b'\\' => result.push(content[j]),
                    b'0'..=b'7' => {
                        let mut val = content[j] - b'0';
                        if j + 1 < content.len() && content[j + 1] >= b'0' && content[j + 1] <= b'7'
                        {
                            j += 1;
                            val = (val << 3) | (content[j] - b'0');
                            if j + 1 < content.len()
                                && content[j + 1] >= b'0'
                                && content[j + 1] <= b'7'
                            {
                                j += 1;
                                val = (val << 3) | (content[j] - b'0');
                            }
                        }
                        result.push(val);
                    }
                    _ => result.push(content[j]),
                }
            }
        } else {
            result.push(b);
        }
        j += 1;
    }
    result
}

fn parse_cmap_string(v: &[u8]) -> String {
    if v.starts_with(b"<") {
        parse_unicode_hex(v)
    } else if v.starts_with(b"(") {
        let result = parse_literal_bytes(v);
        if result.len() >= 2 && result.len().is_multiple_of(2) {
            let u16s: Vec<u16> =
                result.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
            String::from_utf16_lossy(&u16s)
        } else {
            String::from_utf8_lossy(&result).to_string()
        }
    } else {
        String::from_utf8_lossy(v).to_string()
    }
}

fn parse_unicode_hex(v: &[u8]) -> String {
    let bytes = parse_hex(v);
    if bytes.len() >= 2 && bytes.len().is_multiple_of(2) {
        let u16s: Vec<u16> =
            bytes.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
        String::from_utf16_lossy(&u16s)
    } else {
        String::from_utf8_lossy(&bytes).to_string()
    }
}

fn vec_to_u32(v: &[u8]) -> u32 {
    let mut val = 0u32;
    for &b in v {
        val = (val << 8) | b as u32;
    }
    val
}

fn u32_to_vec(val: u32, len: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(len);
    for i in (0..len).rev() {
        v.push(((val >> (i * 8)) & 0xFF) as u8);
    }
    v
}

pub fn glyph_name_to_unicode(v: &[u8]) -> String {
    let name = if v.starts_with(b"/") { &v[1..] } else { v };
    let name_str = String::from_utf8_lossy(name);
    
    if let Some(unicode) = crate::font::agl::lookup(&name_str) {
        unicode
    } else {
        name_str.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agl_lookup() {
        assert_eq!(glyph_name_to_unicode(b"/bullet"), "\u{2022}");
        assert_eq!(glyph_name_to_unicode(b"bullet"), "\u{2022}");
        assert_eq!(glyph_name_to_unicode(b"/uni2022"), "\u{2022}");
        assert_eq!(glyph_name_to_unicode(b"/u2022"), "\u{2022}");
        assert_eq!(glyph_name_to_unicode(b"/A"), "A");
    }

    #[test]
    fn test_hex_parsing() {
        assert_eq!(parse_hex(b"<ABC>"), vec![0xAB, 0xC0]);
        assert_eq!(parse_hex(b"<ABCD>"), vec![0xAB, 0xCD]);
    }
}
