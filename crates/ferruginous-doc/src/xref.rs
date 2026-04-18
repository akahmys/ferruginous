use std::collections::BTreeMap;
use ferruginous_core::{Object, PdfResult, PdfError, PdfName};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XRefEntry {
    /// In use: contains byte offset and generation number.
    InUse {
        offset: u64,
        generation: u16,
    },
    /// Free: contains next free object number and generation number.
    Free {
        next: u32,
        generation: u16,
    },
    /// Compressed: contains the object ID of the object stream and the index within it.
    Compressed {
        container_id: u32,
        index: u32,
    },
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct XRefIndex {
    pub entries: BTreeMap<u32, XRefEntry>,
}

impl XRefIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, id: u32, entry: XRefEntry) {
        self.entries.insert(id, entry);
    }

    pub fn get(&self, id: u32) -> Option<XRefEntry> {
        self.entries.get(&id).copied()
    }
}

/// Manages multiple cross-reference sections and resolves object locations.
///
/// Supports legacy xref tables and modern XRef streams, following the chain
/// of `/Prev` trailers for incremental updates.
#[derive(Debug, Default, Clone)]
pub struct XRefStore {
    pub entries: BTreeMap<u32, XRefEntry>,
    pub trailer: BTreeMap<PdfName, Object>,
}

impl XRefStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn max_id(&self) -> u32 {
        self.entries.keys().copied().max().unwrap_or(0)
    }
}

impl XRefStore {
    /// Merges an index into the store. Entries from newer sections (merged latest)
    /// take precedence if they overlap.
    pub fn merge(&mut self, index: XRefIndex) {
        for (id, entry) in index.entries {
            // Only insert if not already present (assuming merge is called from latest to oldest)
            self.entries.entry(id).or_insert(entry);
        }
    }

    pub fn get(&self, id: u32) -> Option<XRefEntry> {
        self.entries.get(&id).copied()
    }
}

pub fn parse_xref_table(input: &[u8]) -> PdfResult<(XRefIndex, &[u8])> {
    if !input.starts_with(b"xref") {
        return Err(PdfError::Syntactic { pos: 0, message: "Expected 'xref' keyword".into() });
    }
    
    let mut pos = 4;
    // Skip whitespace
    while pos < input.len() && is_pdf_whitespace(input[pos]) {
        pos += 1;
    }

    parse_xref_table_inner(input, pos)
}

/// Internal helper to parse subsections after the initial 'xref' keyword or if the keyword is missing.
pub fn parse_xref_table_inner(input: &[u8], mut pos: usize) -> PdfResult<(XRefIndex, &[u8])> {
    let mut index = XRefIndex::new();
    
    // Parse subsections
    loop {
        let chunk = &input[pos..];
        if chunk.starts_with(b"trailer") || chunk.is_empty() {
            break;
        }

        // Subsection header: [first_id] [count]
        let (first_id, count, header_len) = parse_subsection_header(chunk)?;
        pos += header_len;

        for i in 0..count {
            let entry_chunk = &input[pos..];
            if entry_chunk.len() < 20 {
                return Err(PdfError::Syntactic { pos, message: "XRef entry too short".into() });
            }
            let (entry, entry_len) = parse_xref_entry(entry_chunk)?;
            index.insert(first_id + i, entry);
            pos += entry_len;
        }
    }

    Ok((index, &input[pos..]))
}

/// ISO 32000-2:2020 Clause 7.5.8 - Cross-Reference Streams
pub fn parse_xref_stream(dict: &BTreeMap<PdfName, Object>, data: &[u8]) -> PdfResult<XRefIndex> {
    let mut index = XRefIndex::new();
    let w = extract_w_array(dict)?;
    let sections = extract_index_sections(dict)?;

    let entry_size = w[0] + w[1] + w[2];
    if entry_size == 0 {
        return Err(PdfError::Other("Invalid /W array: total entry size is 0".into()));
    }
    let mut data_pos = 0;

    for (first_id, count) in sections {
        for i in 0..count {
            if data_pos + entry_size > data.len() { break; }
            let entry_data = &data[data_pos..data_pos + entry_size];
            let entry = decode_xref_entry(entry_data, &w)?;
            index.insert(first_id + i, entry);
            data_pos += entry_size;
        }
    }
    Ok(index)
}

fn extract_w_array(dict: &BTreeMap<PdfName, Object>) -> PdfResult<Vec<usize>> {
    let w = if let Some(Object::Array(arr)) = dict.get(&"W".into()) {
        arr.iter()
            .map(|o| o.as_i64().ok_or_else(|| PdfError::Other("Invalid integer in /W".into())))
            .collect::<PdfResult<Vec<_>>>()?
            .into_iter()
            .map(|i| i as usize)
            .collect::<Vec<_>>()
    } else {
        return Err(PdfError::Other("Missing /W in XRef stream".into()));
    };
    if w.len() != 3 {
        return Err(PdfError::Other("Invalid /W length (must be 3)".into()));
    }
    Ok(w)
}

fn extract_index_sections(dict: &BTreeMap<PdfName, Object>) -> PdfResult<Vec<(u32, u32)>> {
    let mut sections = Vec::new();
    if let Some(Object::Array(arr)) = dict.get(&"Index".into()) {
        if arr.len() % 2 != 0 {
            return Err(PdfError::Syntactic {
                pos: 0,
                message: "/Index array must have an even number of elements".into(),
            });
        }
        for i in (0..arr.len()).step_by(2) {
            let first = arr.get(i).and_then(|o| o.as_i64()).ok_or_else(|| {
                PdfError::Other("Invalid first_id in /Index".into())
            })? as u32;
            let count = arr.get(i + 1).and_then(|o| o.as_i64()).ok_or_else(|| {
                PdfError::Other("Invalid count in /Index".into())
            })? as u32;
            sections.push((first, count));
        }
    } else {
        let size = match dict.get(&"Size".into()) {
            Some(Object::Integer(s)) => *s as u32,
            _ => 0,
        };
        sections.push((0, size));
    }
    Ok(sections)
}

fn decode_xref_entry(entry_data: &[u8], w: &[usize]) -> PdfResult<XRefEntry> {
    let field1 = read_int(&entry_data[..w[0]], 1);
    let field2 = read_int(&entry_data[w[0]..w[0]+w[1]], 0);
    let field3 = read_int(&entry_data[w[0]+w[1]..w[0]+w[1]+w[2]], 0);

    match field1 {
        0 => Ok(XRefEntry::Free { next: field2 as u32, generation: field3 as u16 }),
        1 => Ok(XRefEntry::InUse { offset: field2 as u64, generation: field3 as u16 }),
        2 => Ok(XRefEntry::Compressed { container_id: field2 as u32, index: field3 as u32 }),
        _ => Err(PdfError::Other(format!("Unknown XRef entry type: {}", field1))),
    }
}

fn read_int(data: &[u8], default: i64) -> i64 {
    if data.is_empty() {
        return default;
    }
    let mut val = 0;
    for &b in data {
        val = (val << 8) | (b as i64);
    }
    val
}

fn parse_subsection_header(chunk: &[u8]) -> PdfResult<(u32, u32, usize)> {
    // Find the end of the line first to avoid decoding binary data as UTF-8
    let mut line_end = 0;
    while line_end < chunk.len() && chunk[line_end] != b'\n' && chunk[line_end] != b'\r' {
        line_end += 1;
    }
    
    let s = std::str::from_utf8(&chunk[..line_end]).map_err(|_| PdfError::Other("Invalid UTF-8 in XRef header".into()))?;
    let mut parts = s.split_whitespace();
    let first_id = parts.next().ok_or_else(|| PdfError::Syntactic { pos: 0, message: "Missing first ID".into() })?.parse::<u32>().map_err(|_| PdfError::Other("Invalid ID".into()))?;
    let count = parts.next().ok_or_else(|| PdfError::Syntactic { pos: 0, message: "Missing count".into() })?.parse::<u32>().map_err(|_| PdfError::Other("Invalid count".into()))?;
    
    // Skip trailing whitespace/newlines to find the total header length
    let mut pos = line_end;
    while pos < chunk.len() && chunk[pos] != b'\n' && chunk[pos] != b'\r' {
        pos += 1;
    }
    while pos < chunk.len() && (chunk[pos] == b'\n' || chunk[pos] == b'\r') {
        pos += 1;
    }
    
    Ok((first_id, count, pos))
}

fn parse_xref_entry(chunk: &[u8]) -> PdfResult<(XRefEntry, usize)> {
    // 0000000000 65535 f 
    let s = std::str::from_utf8(&chunk[..20]).map_err(|_| PdfError::Other("Invalid UTF-8 in XRef entry".into()))?;
    let offset = s[..10].trim().parse::<u64>().map_err(|_| PdfError::Other("Invalid offset".into()))?;
    let generation = s[11..16].trim().parse::<u16>().map_err(|_| PdfError::Other("Invalid generation".into()))?;
    let type_char = s.chars().nth(17).ok_or_else(|| PdfError::Other("Invalid XRef entry type".into()))?;

    let entry = if type_char == 'n' {
        XRefEntry::InUse { offset, generation }
    } else {
        XRefEntry::Free { next: offset as u32, generation }
    };

    Ok((entry, 20))
}

pub(crate) fn is_pdf_whitespace(c: u8) -> bool {
    matches!(c, 0 | 9 | 10 | 12 | 13 | 32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_xref() {
        let input = b"xref\n0 1\n0000000000 65535 f \n3 1\n0000000010 00000 n \ntrailer";
        let (index, remaining) = parse_xref_table(input).unwrap();
        
        assert_eq!(index.get(0), Some(XRefEntry::Free { next: 0, generation: 65535 }));
        assert_eq!(index.get(3), Some(XRefEntry::InUse { offset: 10, generation: 0 }));
        assert_eq!(remaining, b"trailer");
    }
}
