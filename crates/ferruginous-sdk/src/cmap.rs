//! `CMap` (Character Map) parser and lookup logic.
//!
//! (ISO 32000-2:2020 Clause 9.7.5)

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::fs;
use std::env;
use std::sync::{OnceLock, Mutex};
use crate::lexer::{is_pdf_whitespace, pdf_multispace0, parse_object};
use nom::{
    bytes::complete::tag,
    bytes::complete::take_while1,
};
use crate::core::Object;
use crate::core::error::{PdfError, PdfResult, ParseErrorVariant};

// Embedded core Adobe CMap resources
const CMAP_UNIJIS_UTF8_H: &[u8] = include_bytes!("../assets/cmaps/UniJIS-UTF8-H");
const CMAP_UNIJIS_UTF16_H: &[u8] = include_bytes!("../assets/cmaps/UniJIS-UTF16-H");
const CMAP_90MS_RKSJ_H: &[u8] = include_bytes!("../assets/cmaps/90ms-RKSJ-H");

static EXTERNAL_CMAP_DIRS: OnceLock<Vec<PathBuf>> = OnceLock::new();
static CMAP_CACHE: OnceLock<Mutex<BTreeMap<String, CMap>>> = OnceLock::new();

fn get_cmap_search_dirs() -> &'static [PathBuf] {
    EXTERNAL_CMAP_DIRS.get_or_init(|| {
        let mut dirs = Vec::new();

        // 1. Environment variable
        if let Ok(val) = env::var("FERRUGINOUS_CMAP_DIR") {
            dirs.push(PathBuf::from(val));
        }

        // 2. Common system paths
        #[cfg(target_os = "macos")]
        {
            dirs.push(PathBuf::from("/opt/homebrew/share/poppler/cMap"));
            dirs.push(PathBuf::from("/usr/local/share/ghostscript/Resource/CMap"));
            dirs.push(PathBuf::from("/opt/homebrew/share/ghostscript/Resource/CMap"));
        }
        #[cfg(not(target_os = "macos"))]
        {
            dirs.push(PathBuf::from("/usr/share/poppler/cMap"));
            dirs.push(PathBuf::from("/usr/share/ghostscript/Resource/CMap"));
        }

        dirs
    })
}

fn get_cmap_cache() -> &'static Mutex<BTreeMap<String, CMap>> {
    CMAP_CACHE.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// Represents a mapping result from a `CMap` lookup.
/// (ISO 32000-2:2020 Clause 9.7.5.4)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MappingResult {
    /// Mapped to a Character Identifier (CID).
    Cid(u32),
    /// Mapped to a Unicode byte sequence (UTF-16BE by default).
    Unicode(std::sync::Arc<Vec<u8>>),
}

/// Represents a range mapping in a `CMap` (`BaseFont`).
/// (ISO 32000-2:2020 Clause 9.7.5.5)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BfRange {
    /// Start of the character code range.
    pub start: Vec<u8>,
    /// End of the character code range.
    pub end: Vec<u8>,
    /// The destination mapping for the start of the range.
    pub dst: std::sync::Arc<Vec<u8>>,
}

/// Represents a range mapping in a `CMap` (CID).
/// (ISO 32000-2:2020 Clause 9.7.5.4)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CidRange {
    /// Start of the character code range.
    pub start: Vec<u8>,
    /// End of the character code range.
    pub end: Vec<u8>,
    /// The starting CID for the range.
    pub dst_start: u32,
}

/// Represents a mapping from character codes to CIDs or Unicode (Clause 9.7.5).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CMap {
    /// The name of the `CMap` (e.g., /Identity-H).
    pub name: String,
    /// Direct mapping from character code to Unicode.
    pub bf_chars: BTreeMap<Vec<u8>, std::sync::Arc<Vec<u8>>>,
    /// Range mapping from character code to Unicode.
    pub bf_ranges: Vec<BfRange>,
    /// Direct mapping from character code to CID.
    pub cid_chars: BTreeMap<Vec<u8>, u32>,
    /// Range mapping from character code to CID.
    pub cid_ranges: Vec<CidRange>,
    /// The codespace ranges defining valid input lengths.
    pub codespace_ranges: Vec<(Vec<u8>, Vec<u8>)>,
    /// Whether this CMap is vertical (WMode 1).
    pub is_vertical: bool,
    /// Parent CMap defined via `usecmap` operator (Clause 9.7.5.2).
    pub parent: Option<Box<CMap>>,
}

impl CMap {
    /// Creates a new, empty `CMap`.
    #[must_use] pub fn new() -> Self {
        Self::default()
    }

    /// Creates a predefined `CMap` by name (such as Identity-H).
    ///
    /// Attempts to load from internal assets, then external system paths
    /// (ISO 32000-2:2020 Clause 9.7.5.8).
    pub fn new_predefined(name: &str) -> Option<Self> {
        // Use cache to avoid redundant parsing
        if let Ok(cache) = get_cmap_cache().lock() {
            if let Some(cmap) = cache.get(name) {
                return Some(cmap.clone());
            }
        }

        let cmap = Self::new_predefined_inner(name)?;

        if let Ok(mut cache) = get_cmap_cache().lock() {
            cache.insert(name.to_string(), cmap.clone());
        }
        Some(cmap)
    }

    fn new_predefined_inner(name: &str) -> Option<Self> {
        // 1. Check embedded full CMaps
        let embedded_data = match name {
            "UniJIS-UTF8-H" => Some(CMAP_UNIJIS_UTF8_H),
            "UniJIS-UTF16-H" => Some(CMAP_UNIJIS_UTF16_H),
            "90ms-RKSJ-H" => Some(CMAP_90MS_RKSJ_H),
            _ => None,
        };

        if let Some(data) = embedded_data {
            if let Ok(mut parsed) = Self::parse(data) {
                parsed.name = name.to_string();
                return Some(parsed);
            }
        }

        // 2. Try external resolution (Plan B)
        if let Some(data) = Self::load_external(name) {
            if let Ok(mut parsed) = Self::parse(&data) {
                parsed.name = name.to_string();
                return Some(parsed);
            }
        }

        // 3. Fallbacks for missing or purely dynamic CMaps
        match name {
            "Identity-H" | "Identity-V" => {
                let mut cmap = Self::new();
                cmap.name = name.to_string();
                cmap.codespace_ranges.push((vec![0, 0], vec![255, 255]));
                // Identity mapping: CID = CharCode
                cmap.cid_ranges.push(CidRange {
                    start: vec![0, 0],
                    end: vec![255, 255],
                    dst_start: 0,
                });
                if name.ends_with("-V") {
                    cmap.is_vertical = true;
                }
                Some(cmap)
            }
            // Add stubs for other known names if they weren't matched above
            n if n.starts_with("UniJIS") || n.starts_with("UniGB") || n.starts_with("UniCNS") || n.starts_with("UniKS") || n.ends_with("-H") || n.ends_with("-V") => {
                // Determine if it should be vertical based on name if we reached here
                let mut cmap = Self::new();
                cmap.name = n.to_string();
                cmap.codespace_ranges.push((vec![0, 0], vec![255, 255]));
                cmap.cid_ranges.push(CidRange {
                    start: vec![0, 0],
                    end: vec![255, 255],
                    dst_start: 0,
                });
                if n.ends_with("-V") || n.contains("Vertical") {
                    cmap.is_vertical = true;
                }
                Some(cmap)
            }
            _ => None,
        }
    }

    fn load_external(name: &str) -> Option<Vec<u8>> {
        for dir in get_cmap_search_dirs() {
            // Try name directly, and common collection subdirs
            let paths = [
                dir.join(name),
                dir.join("Adobe-Japan1").join(name),
                dir.join("Adobe-GB1").join(name),
                dir.join("Adobe-CNS1").join(name),
                dir.join("Adobe-Korea1").join(name),
            ];

            for path in &paths {
                if path.exists() {
                    if let Ok(data) = fs::read(path) {
                        return Some(data);
                    }
                }
            }
        }
        None
    }

    /// Parses a `CMap` stream into a structured mapping table.
    /// (ISO 32000-2:2020 Clause 9.7.5.2)
    pub fn parse(input: &[u8]) -> PdfResult<Self> {
        debug_assert!(!input.is_empty(), "CMap::parse: input empty");
        let mut cmap = Self::new();
        let mut current_input = input;

        let mut loop_count = 0;
        const MAX_OPS: usize = 100_000;

        while !current_input.is_empty() {
            loop_count += 1;
            debug_assert!(loop_count < MAX_OPS, "CMap::parse: excessive operations");
            if loop_count > MAX_OPS { return Err(PdfError::ParseError("CMap parse limit exceeded".into())); }

            if let Ok((next, ())) = pdf_multispace0(current_input) {
                current_input = next;
                if current_input.is_empty() { break; }
            }

            // Look for operators
            if current_input.starts_with(b"beginbfchar") {
                let (next, _) = tag("beginbfchar")(current_input).map_err(|e: nom::Err<nom::error::Error<&[u8]>>| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
                let (next, entries) = Self::parse_mapping_block(next, "endbfchar")?;
                for (src, dst) in entries {
                    cmap.bf_chars.insert(src, dst);
                }
                current_input = next;
            } else if current_input.starts_with(b"beginbfrange") {
                let (next, _) = tag("beginbfrange")(current_input).map_err(|e: nom::Err<nom::error::Error<&[u8]>>| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
                let (next, ranges) = Self::parse_range_block(next, "endbfrange")?;
                for (start, end, dst) in ranges {
                    if let Object::String(d) = dst {
                        cmap.bf_ranges.push(BfRange { start, end, dst: d });
                    } else if let Object::Array(arr) = dst {
                        let start_val = if start.len() == 1 {
                            start[0] as u32
                        } else {
                            if start.len() == 2 {
                                u16::from_be_bytes([start[0], start[1]]) as u32
                            } else {
                                0
                            }
                        };
                        let end_val = if end.len() == 1 {
                            end[0] as u32
                        } else {
                            if end.len() == 2 {
                                u16::from_be_bytes([end[0], end[1]]) as u32
                            } else {
                                0
                            }
                        };
                        let mut i = 0;
                        while start_val + i <= end_val && (i as usize) < arr.len() {
                            if let Object::String(d) = &arr[i as usize] {
                                let mut src = start.clone();
                                let mut carry = i;
                                for j in (0..src.len()).rev() {
                                    let sum = src[j] as u32 + carry;
                                    src[j] = (sum % 256) as u8;
                                    carry = sum / 256;
                                }
                                cmap.bf_chars.insert(src, d.clone());
                            }
                            i += 1;
                        }
                    }
                }
                current_input = next;
            } else if current_input.starts_with(b"begincidchar") {
                let (next, _) = tag("begincidchar")(current_input).map_err(|e: nom::Err<nom::error::Error<&[u8]>>| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
                let (next, entries) = Self::parse_cid_block(next, "endcidchar")?;
                for (src, dst) in entries {
                    cmap.cid_chars.insert(src, dst);
                }
                current_input = next;
            } else if current_input.starts_with(b"begincidrange") {
                let (next, _) = tag("begincidrange")(current_input).map_err(|e: nom::Err<nom::error::Error<&[u8]>>| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
                let (next, ranges) = Self::parse_cid_range_block(next, "endcidrange")?;
                for (start, end, dst_start) in ranges {
                    cmap.cid_ranges.push(CidRange { start, end, dst_start });
                }
                current_input = next;
            } else if current_input.starts_with(b"begincodespacerange") {
                let (next, _) = tag("begincodespacerange")(current_input).map_err(|e: nom::Err<nom::error::Error<&[u8]>>| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
                let (next, ranges) = Self::parse_codespace_block(next)?;
                cmap.codespace_ranges.extend(ranges);
                current_input = next;
            } else if let Ok((next, obj)) = parse_object(current_input) {
                let (after_spaces, ()) = pdf_multispace0(next).unwrap_or((next, ()));
                if after_spaces.starts_with(b"usecmap") {
                    if let Object::Name(n) = obj {
                        let name_str = String::from_utf8_lossy(&n);
                        if let Some(parent_cmap) = Self::new_predefined(&name_str) {
                            if parent_cmap.is_vertical {
                                cmap.is_vertical = true;
                            }
                            cmap.parent = Some(Box::new(parent_cmap));
                        }
                    }
                    current_input = &after_spaces[7..]; // skip "usecmap"
                } else if let Object::Name(n) = &obj {
                    if n.as_slice() == b"WMode" {
                        if let Ok((val_next, Object::Integer(v))) = parse_object(after_spaces) {
                            if v == 1 {
                                cmap.is_vertical = true;
                            }
                            current_input = val_next;
                            continue;
                        }
                    }
                    current_input = next;
                } else if let Object::Integer(v) = obj {
                    // Check if followed by WMode
                    if after_spaces.starts_with(b"WMode") {
                        if v == 1 {
                             cmap.is_vertical = true;
                        }
                        current_input = &after_spaces[5..];
                    } else {
                        current_input = next;
                    }
                } else {
                    current_input = next;
                }
            } else {
                current_input = &current_input[1..];
            }
        }
        Ok(cmap)
    }

    #[allow(dead_code)]
    fn skip_ps_token(input: &[u8]) -> &[u8] {
        if let Ok((next, _)) = parse_object(input) {
            next
        } else if let Ok((next, _)) = take_while1::<_, &[u8], nom::error::Error<&[u8]>>(|b| !is_pdf_whitespace(b))(input) {
            next
        } else {
            if input.is_empty() { return input; }
            &input[1..]
        }
    }

    fn parse_mapping_block<'a>(input: &'a [u8], end_tag: &str) -> PdfResult<(&'a [u8], Vec<(Vec<u8>, std::sync::Arc<Vec<u8>>)>)> {
        let mut current = input;
        let mut results = Vec::new();
        loop {
            let (next, ()) = pdf_multispace0(current).map_err(|e: nom::Err<nom::error::Error<&[u8]>>| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
            if next.starts_with(end_tag.as_bytes()) {
                let (next, _) = tag(end_tag)(next).map_err(|e: nom::Err<nom::error::Error<&[u8]>>| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
                return Ok((next, results));
            }
            let (next, src) = parse_object(next).map_err(|e| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
            let (next, ()) = pdf_multispace0(next).map_err(|e: nom::Err<nom::error::Error<&[u8]>>| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
            let (next, dst) = parse_object(next).map_err(|e| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
            
            if let (Object::String(s), Object::String(d)) = (src, dst) {
                results.push((s.to_vec(), d));
            }
            current = next;
        }
    }

    fn parse_range_block<'a>(input: &'a [u8], end_tag: &str) -> PdfResult<(&'a [u8], Vec<(Vec<u8>, Vec<u8>, Object)>)> {
        let mut current = input;
        let mut results = Vec::new();
        loop {
            let (next, ()) = pdf_multispace0(current).map_err(|e: nom::Err<nom::error::Error<&[u8]>>| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
            if next.starts_with(end_tag.as_bytes()) {
                let (next, _) = tag(end_tag)(next).map_err(|e: nom::Err<nom::error::Error<&[u8]>>| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
                return Ok((next, results));
            }
            let (next, start) = parse_object(next).map_err(|e| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
            let (next, ()) = pdf_multispace0(next).map_err(|e: nom::Err<nom::error::Error<&[u8]>>| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
            let (next, end) = parse_object(next).map_err(|e| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
            let (next, ()) = pdf_multispace0(next).map_err(|e: nom::Err<nom::error::Error<&[u8]>>| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
            let (next, dst) = parse_object(next).map_err(|e| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
            
            if let (Object::String(s), Object::String(e)) = (&start, &end) {
                results.push((s.to_vec(), e.to_vec(), dst.clone()));
            }
            current = next;
        }
    }

    fn parse_cid_block<'a>(input: &'a [u8], end_tag: &str) -> PdfResult<(&'a [u8], Vec<(Vec<u8>, u32)>)> {
        let mut current = input;
        let mut results = Vec::new();
        loop {
            let (next, ()) = pdf_multispace0(current).map_err(|e: nom::Err<nom::error::Error<&[u8]>>| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
            if next.starts_with(end_tag.as_bytes()) {
                let (next, _) = tag(end_tag)(next).map_err(|e: nom::Err<nom::error::Error<&[u8]>>| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
                return Ok((next, results));
            }
            let (next, src) = parse_object(next).map_err(|e| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
            let (next, ()) = pdf_multispace0(next).map_err(|e: nom::Err<nom::error::Error<&[u8]>>| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
            let (next, dst) = parse_object(next).map_err(|e| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
            
            if let (Object::String(s), Object::Integer(d)) = (src, dst) {
                results.push((s.to_vec(), d as u32));
            }
            current = next;
        }
    }

    fn parse_cid_range_block<'a>(input: &'a [u8], end_tag: &str) -> PdfResult<(&'a [u8], Vec<(Vec<u8>, Vec<u8>, u32)>)> {
        let mut current = input;
        let mut results = Vec::new();
        loop {
            let (next, ()) = pdf_multispace0(current).map_err(|e: nom::Err<nom::error::Error<&[u8]>>| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
            if next.starts_with(end_tag.as_bytes()) {
                let (next, _) = tag(end_tag)(next).map_err(|e: nom::Err<nom::error::Error<&[u8]>>| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
                return Ok((next, results));
            }
            let (next, start) = parse_object(next).map_err(|e| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
            let (next, ()) = pdf_multispace0(next).map_err(|e: nom::Err<nom::error::Error<&[u8]>>| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
            let (next, end) = parse_object(next).map_err(|e| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
            let (next, ()) = pdf_multispace0(next).map_err(|e: nom::Err<nom::error::Error<&[u8]>>| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
            let (next, dst) = parse_object(next).map_err(|e| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
            
            if let (Object::String(s), Object::String(e), Object::Integer(d)) = (start, end, dst) {
                results.push((s.to_vec(), e.to_vec(), d as u32));
            }
            current = next;
        }
    }

    fn parse_codespace_block(input: &[u8]) -> PdfResult<(&[u8], Vec<(Vec<u8>, Vec<u8>)>)> {
        let mut current = input;
        let mut results = Vec::new();
        loop {
            let (next, ()) = pdf_multispace0(current).map_err(|e: nom::Err<nom::error::Error<&[u8]>>| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
            if next.starts_with(b"endcodespacerange") {
                let (next, _) = tag("endcodespacerange")(next).map_err(|e: nom::Err<nom::error::Error<&[u8]>>| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
                return Ok((next, results));
            }
            let (next, start) = parse_object(next).map_err(|e| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
            let (next, ()) = pdf_multispace0(next).map_err(|e| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
            let (next, end) = parse_object(next).map_err(|e| PdfError::ParseError(ParseErrorVariant::CMapError(e.to_string())))?;
            
            if let (Object::String(s), Object::String(e)) = (start, end) {
                results.push((s.to_vec(), e.to_vec()));
            }
            current = next;
        }
    }

    /// Determines the length of the character code starting at the given position.
    #[must_use] pub fn code_length(&self, input: &[u8]) -> usize {
        if self.codespace_ranges.is_empty() {
             // FALLBACK: If we have no ranges, but this is a multi-byte CMap (based on name), default to 2
             if self.name.contains("Identity") || self.name.contains("UniJIS") || self.name.contains("UniGB") {
                 return 2.min(input.len());
             }
             return 1;
        }
        
        for (start, end) in &self.codespace_ranges {
            let len = start.len();
            if input.len() >= len {
                let candidate = &input[..len];
                if candidate >= start.as_slice() && candidate <= end.as_slice() {
                    return len;
                }
            }
        }
        
        // Final fallback: if no range matched, but we are in a known multi-byte CMap, try to recover
        if self.name.contains("Identity") || self.name.contains("UniJIS") {
            return 2.min(input.len());
        }
        
        if let Some(ref parent) = self.parent {
            return parent.code_length(input);
        }

        1
    }

    /// Looks up a character code in the `CMap`.
    #[must_use] pub fn lookup(&self, code: &[u8]) -> Option<MappingResult> {
        debug_assert!(!code.is_empty(), "CMap::lookup: code empty");
        
        if let Some(&cid) = self.cid_chars.get(code) {
            return Some(MappingResult::Cid(cid));
        }

        for range in &self.cid_ranges {
            if code >= range.start.as_slice() && code <= range.end.as_slice() {
                let offset = Self::compute_offset(code, &range.start);
                return Some(MappingResult::Cid(range.dst_start + offset));
            }
        }

        if let Some(unicode) = self.bf_chars.get(code) {
            return Some(MappingResult::Unicode(std::sync::Arc::clone(unicode)));
        }

        for range in &self.bf_ranges {
            if code >= range.start.as_slice() && code <= range.end.as_slice() {
                let offset = Self::compute_offset(code, &range.start);
                let mut res = (*range.dst).clone();
                if !res.is_empty() {
                    let last = res.len() - 1;
                    res[last] = res[last].wrapping_add(offset as u8);
                }
                return Some(MappingResult::Unicode(std::sync::Arc::new(res)));
            }
        }

        if let Some(ref parent) = self.parent {
            return parent.lookup(code);
        }

        None
    }

    fn compute_offset(code: &[u8], start: &[u8]) -> u32 {
        let mut code_val = 0u32;
        let mut start_val = 0u32;
        let len = code.len().min(start.len());
        for i in 0..len {
            code_val = (code_val << 8) | u32::from(code[i]);
            start_val = (start_val << 8) | u32::from(start[i]);
        }
        code_val.saturating_sub(start_val)
    }
}
