//! PDF Cross-Reference Stream Encoding (ISO 32000-2:2020 Clause 7.5.8)

use crate::core::{Object, PdfResult};
use crate::xref::XRefEntry;
use std::collections::BTreeMap;

/// Helper for calculating and writing XRef streams.
pub struct XRefStreamBuilder {
    entries: BTreeMap<u32, XRefEntry>,
}

impl XRefStreamBuilder {
    /// Creates a new builder from the given entries.
    pub fn new(entries: BTreeMap<u32, XRefEntry>) -> Self {
        Self { entries }
    }

    /// Compiles the entries into an Object::Stream with the correct /W and /Index.
    pub fn build(self, size: u32) -> PdfResult<Object> {
        let (data, widths) = self.encode_binary()?;
        
        let mut dict = BTreeMap::new();
        dict.insert(b"Type".to_vec(), Object::new_name(b"XRef".to_vec()));
        dict.insert(b"Size".to_vec(), Object::Integer(i64::from(size)));
        dict.insert(b"W".to_vec(), Object::new_array(vec![
            Object::Integer(widths[0] as i64),
            Object::Integer(widths[1] as i64),
            Object::Integer(widths[2] as i64),
        ]));

        // Calculate Index array
        let mut index_array = Vec::new();
        let subsections = self.subsections();
        for (start, count) in subsections {
            index_array.push(Object::Integer(i64::from(start)));
            index_array.push(Object::Integer(i64::from(count)));
        }
        if !index_array.is_empty() {
             dict.insert(b"Index".to_vec(), Object::new_array(index_array));
        }

        Ok(Object::new_stream(dict, data))
    }

    fn encode_binary(&self) -> PdfResult<(Vec<u8>, [usize; 3])> {
        let mut max_f1 = 0u64;
        let mut max_f2 = 0u64;
        let mut max_f3 = 0u64;

        // Pass 1: Determine required widths
        for entry in self.entries.values() {
            let (f1, f2, f3) = match entry {
                XRefEntry::Free { next, generation } => (0, u64::from(*next), u64::from(*generation)),
                XRefEntry::InUse { offset, generation } => (1, *offset, u64::from(*generation)),
                XRefEntry::Compressed { container_id, index } => (2, u64::from(*container_id), u64::from(*index)),
            };
            max_f1 = max_f1.max(f1);
            max_f2 = max_f2.max(f2);
            max_f3 = max_f3.max(f3);
        }

        let w1 = required_bytes(max_f1);
        let w2 = required_bytes(max_f2);
        let w3 = required_bytes(max_f3);
        let widths = [w1, w2, w3];
        let entry_size = w1 + w2 + w3;

        // Pass 2: Write binary data
        let mut data = Vec::with_capacity(self.entries.len() * entry_size);
        for entry in self.entries.values() {
            let (f1, f2, f3) = match entry {
                XRefEntry::Free { next, generation } => (0, u64::from(*next), u64::from(*generation)),
                XRefEntry::InUse { offset, generation } => (1, *offset, u64::from(*generation)),
                XRefEntry::Compressed { container_id, index } => (2, u64::from(*container_id), u64::from(*index)),
            };
            write_uint(&mut data, f1, w1);
            write_uint(&mut data, f2, w2);
            write_uint(&mut data, f3, w3);
        }

        Ok((data, widths))
    }

    fn subsections(&self) -> Vec<(u32, u32)> {
        let mut result = Vec::new();
        let mut current_start = None;
        let mut current_count = 0;
        let mut last_id = 0;

        for &id in self.entries.keys() {
            match current_start {
                None => {
                    current_start = Some(id);
                    current_count = 1;
                }
                Some(_) if id == last_id + 1 => {
                    current_count += 1;
                }
                Some(start) => {
                    result.push((start, current_count));
                    current_start = Some(id);
                    current_count = 1;
                }
            }
            last_id = id;
        }

        if let Some(start) = current_start {
            result.push((start, current_count));
        }

        result
    }
}

use crate::serialize::{write_uint, required_bytes};
