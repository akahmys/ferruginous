//! `CMap` (Character Map) parser and lookup logic.
//! (ISO 32000-2:2020 Clause 9.7.5)

use std::collections::BTreeMap;
use crate::lexer::{is_pdf_whitespace, pdf_multispace0, parse_object};
use nom::{
    bytes::complete::tag,
    bytes::complete::take_while1,
};
use crate::core::Object;
use crate::core::error::{PdfError, PdfResult, ParseErrorVariant};

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
}

impl CMap {
    /// Creates a new, empty `CMap`.
    #[must_use] pub fn new() -> Self {
        Self::default()
    }

    /// Creates a predefined `CMap` by name (Identity-H, Identity-V).
    /// (ISO 32000-2:2020 Clause 9.7.5.8)
    pub fn new_predefined(name: &str) -> Option<Self> {
        match name {
            "Identity-H" => {
                let mut cmap = Self::new();
                cmap.name = name.to_string();
                cmap.codespace_ranges.push((vec![0, 0], vec![255, 255]));
                // Identity-H assumes 2-byte input maps to same CID
                cmap.cid_ranges.push(CidRange {
                    start: vec![0, 0],
                    end: vec![255, 255],
                    dst_start: 0,
                });
                Some(cmap)
            }
            "Identity-V" => {
                let mut cmap = Self::new_predefined("Identity-H")?;
                cmap.name = name.to_string();
                cmap.is_vertical = true;
                Some(cmap)
            }
            _ => None,
        }
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
                    cmap.bf_ranges.push(BfRange { start, end, dst });
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
            } else {
                current_input = Self::skip_ps_token(current_input);
            }
        }

        Ok(cmap)
    }

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

    fn parse_range_block<'a>(input: &'a [u8], end_tag: &str) -> PdfResult<(&'a [u8], Vec<(Vec<u8>, Vec<u8>, std::sync::Arc<Vec<u8>>)>)> {
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
            
            if let (Object::String(s), Object::String(e), Object::String(d)) = (start, end, dst) {
                results.push((s.to_vec(), e.to_vec(), d));
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
        if self.codespace_ranges.is_empty() { return 1; }
        
        for (start, end) in &self.codespace_ranges {
            let len = start.len();
            if input.len() >= len {
                let candidate = &input[..len];
                if candidate >= start.as_slice() && candidate <= end.as_slice() {
                    return len;
                }
            }
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

        None
    }

    fn compute_offset(code: &[u8], start: &[u8]) -> u32 {
        debug_assert!(code.len() == start.len(), "CMap::compute_offset: mismatched lengths");
        let mut offset = 0u32;
        let mut loop_count = 0;
        for i in 0..code.len() {
            loop_count += 1;
            debug_assert!(loop_count <= 4, "CMap::compute_offset: excessive bytes");
            offset = (offset << 8) | u32::from(code[i].saturating_sub(start[i]));
        }
        offset
    }
}
