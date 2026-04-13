//! File trailer and incremental update detection.
//! (ISO 32000-2:2020 Clause 7.5.5)

use crate::lexer::parse_object;
use crate::core::{Object, Reference, PdfError, PdfResult, ParseErrorVariant};
use std::collections::BTreeMap;

/// Finds the last occurrence of a byte sequence in a slice.
fn rfind(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    debug_assert!(!needle.is_empty(), "rfind: needle empty");
    debug_assert!(haystack.len() >= needle.len(), "rfind: haystack too small");
    haystack.windows(needle.len()).rposition(|window| window == needle)
}

/// Information about the PDF file trailer and xref location.
/// (ISO 32000-2:2020 Clause 7.5.5)
#[derive(Debug, Clone, PartialEq)]
pub struct TrailerInfo {
    /// Byte offset of the last cross-reference section.
    pub last_xref_offset: u64,
    /// The trailer dictionary (Clause 7.5.5).
    pub trailer_dict: std::sync::Arc<BTreeMap<Vec<u8>, Object>>,
}

/// Locates the trailer and the last xref section in a PDF buffer.
/// (ISO 32000-2:2020 Clause 7.5.5)
pub fn find_trailer_info(data: &[u8]) -> PdfResult<TrailerInfo> {
    debug_assert!(!data.is_empty(), "find_trailer: data empty");
    debug_assert!(data.len() > 10, "find_trailer: data too short");
    // 1. Find the last %%EOF
    let eof_pos = rfind(data, b"%%EOF").ok_or_else(|| PdfError::ParseError(ParseErrorVariant::general(data.len() as u64, "Could not find %%EOF")))?;
    
    // 2. Find the last startxref before that
    let startxref_pos = rfind(&data[..eof_pos], b"startxref").ok_or_else(|| PdfError::ParseError(ParseErrorVariant::general(eof_pos as u64, "Could not find startxref")))?;
    
    // 3. Parse the offset after startxref
    let start_idx = startxref_pos.checked_add(9).ok_or_else(|| PdfError::ParseError(ParseErrorVariant::general(startxref_pos as u64, "Offset overflow in startxref pos")))?;
    if start_idx > eof_pos { return Err(PdfError::ParseError(ParseErrorVariant::general(start_idx as u64, "Invalid startxref position"))); }
    let offset_data = std::str::from_utf8(&data[start_idx..eof_pos])
        .map_err(|_| PdfError::ParseError(ParseErrorVariant::general(start_idx as u64, "Invalid UTF-8 in startxref offset")))?;
    let offset = offset_data.trim().parse::<u64>()
        .map_err(|_| PdfError::ParseError(ParseErrorVariant::general(start_idx as u64, format!("Could not parse startxref offset: {offset_data}"))))?;
    
    // 4. Find the trailer dictionary just before startxref
    let trailer_pos = rfind(&data[..startxref_pos], b"trailer");
    
    if let Some(pos) = trailer_pos {
        let dict_start = pos.checked_add(7).ok_or_else(|| PdfError::ParseError(ParseErrorVariant::general(pos as u64, "Offset overflow in trailer pos")))?;
        if dict_start <= startxref_pos {
            let dict_data = &data[dict_start..startxref_pos];
            if let Ok((_, obj)) = parse_object(dict_data) {
                if let Object::Stream(dict, _) | Object::Dictionary(dict) = obj {
                    return Ok(TrailerInfo { last_xref_offset: offset, trailer_dict: dict });
                }
            }
        }
    }
    
    // For XRef Streams, the trailer keyword is missing. 
    // loader.rs will extract the primary trailer from the XRef stream itself.
    Ok(TrailerInfo { last_xref_offset: offset, trailer_dict: std::sync::Arc::new(BTreeMap::new()) })
}

impl TrailerInfo {
    /// Returns the indirect reference to the Document Catalog (/Root).
    #[must_use] pub fn root(&self) -> Option<Reference> {
        self.trailer_dict.get(b"Root".as_slice()).and_then(|obj| {
            if let Object::Reference(r) = obj {
                Some(*r)
            } else {
                None
            }
        })
    }

    /// Returns the document ID (Clause 14.4).
    #[must_use] pub fn id(&self) -> Option<std::sync::Arc<Vec<u8>>> {
        if let Some(Object::Array(ids)) = self.trailer_dict.get(b"ID".as_slice()) {
            if let Some(Object::String(id)) = ids.first() {
                return Some(std::sync::Arc::clone(id));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_trailer_info() {
        let pdf = b"trailer\n<< /Root 1 0 R /Size 2 >>\nstartxref\n123\n%%EOF";
        let info = find_trailer_info(pdf).expect("Integration test failed: could not find trailer");
        assert_eq!(info.last_xref_offset, 123);
        assert_eq!(info.root(), Some(Reference { id: 1, generation: 0 }));
    }
}
