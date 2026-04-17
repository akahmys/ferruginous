//! Cross-reference (`XRef`) table and stream parsing.
//!
//! (ISO 32000-2:2020 Clause 7.5.4)

use std::collections::BTreeMap;
use std::convert::TryInto;

use nom::{
    bytes::complete::{tag, take_while_m_n},
    character::complete::{digit1, space1, line_ending},
    multi::many0,
    sequence::tuple,
    IResult,
    branch::alt,
};
use crate::lexer::pdf_multispace0;
use crate::core::{Object, PdfError, PdfResult, ParseErrorVariant, StructureErrorVariant};

/// Represents the location of an indirect object in the PDF file.
/// (ISO 32000-2:2020 Clause 7.5.4)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XRefEntry {
    /// In use: contains byte offset and generation number (Type 1).
    InUse {
        /// Byte offset from the beginning of the file.
        offset: u64,
        /// Generation number (0-65535).
        generation: u16,
    },
    /// Free: contains next free object number and generation number (Type 0).
    Free {
        /// Next free object number in the free list.
        next: u32,
        /// Generation number to use if this object number is reused.
        generation: u16,
    },
    /// Compressed: contains the object ID of the object stream and the index within it (Type 2).
    /// (ISO 32000-2:2020 Clause 7.5.8)
    Compressed {
        /// Object ID of the object stream containing this object.
        container_id: u32,
        /// Index of the object within the object stream.
        index: u32,
    },
}

impl XRefEntry {
    /// Writes the entry in the traditional 20-byte format (ISO 32000-2:2020 Clause 7.5.4).
    /// Format: "nnnnnnnnnn ggggg n/f\r\n" or "nnnnnnnnnn ggggg n/f \n"
    pub fn write_20byte<W: std::io::Write>(&self, w: &mut W) -> PdfResult<()> {
        match self {
            Self::InUse { offset, generation } => {
                writeln!(w, "{offset:010} {generation:05} n ").map_err(PdfError::from)?;
            }
            Self::Free { next, generation } => {
                writeln!(w, "{next:010} {generation:05} f ").map_err(PdfError::from)?;
            }
            Self::Compressed { .. } => {
                return Err(PdfError::InvalidType { 
                    expected: "Traditional XRef Entry".into(), 
                    found: "Compressed".into() 
                });
            }
        }
        Ok(())
    }
}

/// A trait for indexing and retrieving object locations.
pub trait XRefIndex: Send + Sync {
    /// Retrieves the entry for a given object ID.
    fn get(&self, id: u32) -> Option<XRefEntry>;
    /// Returns the highest object ID present in the index.
    fn max_id(&self) -> u32;
}

/// A basic implementation of `XRefIndex` using a `BTreeMap`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MemoryXRefIndex {
    /// Mapping from object ID to its `XRef` entry.
    pub entries: BTreeMap<u32, XRefEntry>,
}

impl XRefIndex for MemoryXRefIndex {
    fn get(&self, id: u32) -> Option<XRefEntry> {
        self.entries.get(&id).copied()
    }

    fn max_id(&self) -> u32 {
        self.entries.keys().next_back().copied().unwrap_or(0)
    }
}

impl MemoryXRefIndex {
    /// Inserts or updates an entry for the given object ID.
    pub fn insert(&mut self, id: u32, entry: XRefEntry) {
        self.entries.insert(id, entry);
    }

    /// Groups contiguous object IDs into subsections for xref table writing.
    #[must_use] pub fn subsections(&self) -> Vec<(u32, Vec<XRefEntry>)> {
        let mut subsections = Vec::new();
        let mut current_id = None;
        let mut current_vec = Vec::new();

        for (&id, entry) in &self.entries {
            match current_id {
                Some(last_id) if id == last_id + 1 => {
                    current_vec.push(*entry);
                    current_id = Some(id);
                }
                _ => {
                    if let Some(start_id) = current_id {
                        let prev_start = start_id.saturating_sub(current_vec.len() as u32 - 1);
                        subsections.push((prev_start, std::mem::take(&mut current_vec)));
                    }
                    current_vec.push(*entry);
                    current_id = Some(id);
                }
            }
        }

        if let Some(last_id) = current_id {
            let start_id = last_id.saturating_sub(current_vec.len() as u32 - 1);
            subsections.push((start_id, current_vec));
        }

        subsections
    }
}

// --- Traditional XRef Table Parser (Clause 7.5.4) ---

fn parse_xref_entry(input: &[u8]) -> IResult<&[u8], (u64, u16, u8)> {
    debug_assert!(!input.is_empty(), "parse_entry: input empty");
    debug_assert!(input.len() >= 20, "parse_entry: input too short");
    let (input, (offset_bytes, _, generation_bytes, _, type_char)) = tuple((
        take_while_m_n(10, 10, |b: u8| b.is_ascii_digit()),
        tag(" "),
        take_while_m_n(5, 5, |b: u8| b.is_ascii_digit()),
        tag(" "),
        alt((tag("n"), tag("f"))),
    ))(input)?;

    let (input, _) = alt((tag("\r\n"), tag(" \n"), tag(" \r"), tag("\n "), tag("  ")))(input)?;

    let offset_str = std::str::from_utf8(offset_bytes)
        .map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit)))?;
    let offset = offset_str.parse::<u64>()
        .map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit)))?;
    
    let generation_str = std::str::from_utf8(generation_bytes)
        .map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit)))?;
    let generation = generation_str.parse::<u16>()
        .map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit)))?;

    Ok((input, (offset, generation, type_char[0])))
}

fn parse_xref_subsection(input: &[u8]) -> IResult<&[u8], (u32, Vec<XRefEntry>)> {
    let (input, (first_id_bytes, _, count_bytes, _)) = tuple((
        digit1,
        space1,
        digit1,
        line_ending,
    ))(input)?;

    let first_id_str = std::str::from_utf8(first_id_bytes).map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit)))?;
    let first_id = first_id_str.parse::<u32>().map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit)))?;
    
    let count_str = std::str::from_utf8(count_bytes).map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit)))?;
    let count = count_str.parse::<usize>().map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit)))?;

    let mut entries = Vec::with_capacity(count);
    let mut current_input = input;
    for _ in 0..count {
        let (next_input, (offset, generation, type_byte)) = parse_xref_entry(current_input)?;
        let entry = if type_byte == b'n' {
            XRefEntry::InUse { offset, generation }
        } else {
            let next_id = offset.try_into().map_err(|_| nom::Err::Error(nom::error::Error::new(current_input, nom::error::ErrorKind::Digit)))?;
            XRefEntry::Free { next: next_id, generation }
        };
        entries.push(entry);
        current_input = next_input;
    }

    Ok((current_input, (first_id, entries)))
}

/// Parses a traditional `xref` section.
pub fn parse_xref_table(input: &[u8]) -> IResult<&[u8], MemoryXRefIndex> {
    debug_assert!(!input.is_empty(), "parse_table: input empty");
    debug_assert!(input.starts_with(b"xref"), "parse_table: must start with xref");
    let (input, _) = tag("xref")(input)?;
    let (input, ()) = pdf_multispace0(input)?;
    
    let (input, subsections) = many0(parse_xref_subsection)(input)?;
    
    let mut index = MemoryXRefIndex::default();
    for (start_id, entries) in subsections {
        for (i, entry) in entries.into_iter().enumerate() {
            let id = start_id.checked_add(i.try_into().unwrap_or(u32::MAX)).ok_or(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit)))?;
            index.insert(id, entry);
        }
    }
    
    Ok((input, index))
}

/// Represents an `XRef` section including the trailer dictionary (Clause 7.5.5).
pub struct XRefSection {
    /// The `XRef` mapping table.
    pub index: MemoryXRefIndex,
    /// The trailer dictionary accompanying this section.
    pub trailer: std::sync::Arc<BTreeMap<Vec<u8>, Object>>,
}

/// Parses an entire xref section including the trailer dictionary - Clause 7.5.5
pub fn parse_xref_section(input: &[u8]) -> IResult<&[u8], XRefSection> {
    debug_assert!(!input.is_empty(), "parse_xref_section: input empty");
    debug_assert!(input.len() > 4, "parse_xref_section: input too short");
    let (input, index) = parse_xref_table(input)?;
    let (input, ()) = pdf_multispace0(input)?;
    let (input, _) = tag("trailer")(input)?;
    let (input, ()) = pdf_multispace0(input)?;
    let (input, trailer_dict) = crate::lexer::parse_dictionary(input)?;
    
    // Convert Object to BTreeMap (guaranteed by parse_dictionary)
    if let crate::core::Object::Dictionary(dict) = trailer_dict {
        Ok((input, XRefSection { index, trailer: dict }))
    } else {
        Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag)))
    }
}



// --- XRef Stream Decoder (Clause 7.5.8) ---

/// Decodes an `XRef` Stream entry based on the /W (Widths) array.
#[must_use] pub fn decode_xref_stream_entry(
    data: &[u8],
    widths: &[usize; 3],
) -> (XRefEntry, usize) {
    debug_assert!(!data.is_empty(), "decode_entry: data empty");
    debug_assert!(widths.iter().sum::<usize>() > 0, "decode_entry: widths are zero");
    let mut offset = 0;
    
    let read_field = |data: &[u8], offset: &mut usize, width: usize| -> u64 {
        if width == 0 { return 0; }
        let end = offset.saturating_add(width);
        if end > data.len() {
            return 0; // Or return a Result if we want to be more strict
        }
        let val = read_uint(&data[*offset..end]);
        *offset = end;
        val
    };

    let t = if widths[0] > 0 {
        read_field(data, &mut offset, widths[0])
    } else {
        1 // Default type is 1 (normal) if first field is 0 width
    };

    let f2 = read_field(data, &mut offset, widths[1]);
    let f3 = read_field(data, &mut offset, widths[2]);

    let entry = match t {
        0 => XRefEntry::Free { 
            next: f2.try_into().unwrap_or(0), 
            generation: f3.try_into().unwrap_or(0) 
        },
        1 => XRefEntry::InUse { 
            offset: f2, 
            generation: f3.try_into().unwrap_or(0) 
        },
        2 => XRefEntry::Compressed { 
            container_id: f2.try_into().unwrap_or(0), 
            index: f3.try_into().unwrap_or(0) 
        },
        _ => XRefEntry::Free { next: 0, generation: 0 }, // Unknown type
    };

    (entry, offset)
}

fn xref_widths(dict: &BTreeMap<Vec<u8>, Object>) -> PdfResult<[usize; 3]> {
    debug_assert!(!dict.is_empty(), "get_widths: dict empty");
    let w_obj = dict.get(b"W".as_slice()).ok_or_else(|| PdfError::StructureError(StructureErrorVariant::MissingRequiredKey { key: "/W".into(), context: "XRef Stream".into() }))?;
    let w_array = if let Object::Array(a) = w_obj { a } else { return Err(PdfError::InvalidType { expected: "Array".into(), found: "Other".into() }); };
    if w_array.len() != 3 { return Err(PdfError::ParseError(ParseErrorVariant::General { offset: 0, details: "Invalid /W length".to_string() })); }
    
    let mut widths = [0usize; 3];
    for i in 0..3 {
        widths[i] = if let Object::Integer(n) = w_array[i] {
            n.try_into().map_err(|_| PdfError::ParseError(ParseErrorVariant::General { offset: 0, details: "Negative or overflow in /W".to_string() }))?
        } else {
            0
        };
    }
    Ok(widths)
}

/// Parses the binary content of an `XRef` Stream.
/// (ISO 32000-2:2020 Clause 7.5.8.3)
pub fn parse_xref_stream_content(
    data: &[u8],
    dict: &BTreeMap<Vec<u8>, Object>,
) -> PdfResult<MemoryXRefIndex> {
    debug_assert!(!data.is_empty(), "parse_content: data empty");
    debug_assert!(!dict.is_empty(), "parse_content: dict empty");
    let widths = xref_widths(dict)?;
    let entry_size: usize = widths.iter().sum();
    if entry_size == 0 { return Err(PdfError::ParseError(ParseErrorVariant::general(0, "Zero entry size"))); }

    let index_vec = xref_stream_indices(dict)?;
    let mut index = MemoryXRefIndex::default();
    let mut data_offset = 0;

    for i in (0..index_vec.len()).step_by(2) {
        let first_id = index_vec[i];
        let count = index_vec[i+1];

        for j in 0..count {
            if data_offset + entry_size > data.len() { break; }
            let (entry, _) = decode_xref_stream_entry(&data[data_offset..], &widths);
            index.insert(first_id + j, entry);
            data_offset += entry_size;
        }
    }
    Ok(index)
}

fn xref_stream_indices(dict: &BTreeMap<Vec<u8>, Object>) -> PdfResult<Vec<u32>> {
    debug_assert!(!dict.is_empty(), "get_indices: dict empty");
    if let Some(Object::Array(a)) = dict.get(b"Index".as_slice()) {
        debug_assert!(!a.is_empty(), "get_indices: Index array empty");
        let mut res = Vec::with_capacity(a.len());
        for obj in a.iter() {
            if let Object::Integer(n) = obj {
                res.push((*n).try_into().map_err(|_| PdfError::ParseError(ParseErrorVariant::general(0, "Negative index")))?);
            }
        }
        if res.len() % 2 != 0 { return Err(PdfError::ParseError(ParseErrorVariant::general(0, "Invalid /Index length"))); }
        Ok(res)
    } else {
        let size_obj = dict.get(b"Size".as_slice()).ok_or_else(|| PdfError::StructureError(StructureErrorVariant::MissingRequiredKey { key: "/Size".into(), context: "XRef Stream".into() }))?;
        let size = if let Object::Integer(s) = size_obj {
            (*s).try_into().map_err(|_| PdfError::ParseError(ParseErrorVariant::general(0, "Invalid /Size")))?
        } else { 0 };
        Ok(vec![0, size])
    }
}

fn read_uint(data: &[u8]) -> u64 {
    debug_assert!(!data.is_empty(), "read_uint: data empty");
    debug_assert!(data.len() <= 8, "read_uint: data too large for u64");
    let mut res = 0u64;
    for &b in data {
        res = (res << 8) | u64::from(b);
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_xref_table() {
        let input = b"xref\n0 1\n0000000000 65535 f \n3 2\n0000000010 00000 n \n0000000020 00001 n \n";
        let (_, index) = parse_xref_table(input).unwrap();
        
        assert_eq!(index.get(0), Some(XRefEntry::Free { next: 0, generation: 65535 }));
        assert_eq!(index.get(3), Some(XRefEntry::InUse { offset: 10, generation: 0 }));
        assert_eq!(index.get(4), Some(XRefEntry::InUse { offset: 20, generation: 1 }));
    }

    #[test]
    fn test_decode_xref_stream_entry() {
        let widths = [1, 4, 2];
        // Type 1, Offset 1024 (0x400), Gen 0
        let data = vec![0x01, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00];
        let (entry, size) = decode_xref_stream_entry(&data, &widths);
        assert_eq!(size, 7);
        assert_eq!(entry, XRefEntry::InUse { offset: 1024, generation: 0 });

        // Type 2, Container 10, Index 5
        let data2 = vec![0x02, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x05];
        let (entry2, _) = decode_xref_stream_entry(&data2, &widths);
        assert_eq!(entry2, XRefEntry::Compressed { container_id: 10, index: 5 });
    }

    #[test]
    fn test_memory_xref_index_max_id() {
        let mut index = MemoryXRefIndex::default();
        assert_eq!(index.max_id(), 0);
        
        index.insert(10, XRefEntry::InUse { offset: 100, generation: 0 });
        index.insert(5, XRefEntry::InUse { offset: 50, generation: 0 });
        assert_eq!(index.max_id(), 10);
    }
}
