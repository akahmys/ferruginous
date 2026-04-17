//! PDF Object Stream Encoding (ISO 32000-2:2020 Clause 7.5.7)

use crate::core::{Object, PdfResult, PdfError};
use crate::serialize::write_object;
use std::collections::BTreeMap;

/// Builder for creating an Object Stream (/Type /ObjStm).
pub struct ObjectStreamBuilder {
    /// Mapping from object ID to the object itself.
    objects: BTreeMap<u32, Object>,
}

impl ObjectStreamBuilder {
    /// Creates a new empty builder.
    pub fn new() -> Self {
        Self { objects: BTreeMap::new() }
    }

    /// Adds an object to the stream. 
    /// Note: Per Clause 7.5.7, only non-stream objects may be stored.
    pub fn add_object(&mut self, id: u32, obj: Object) -> PdfResult<()> {
        if matches!(obj, Object::Stream(_, _)) {
            return Err(PdfError::InvalidType { 
                expected: "Non-stream Object".into(), 
                found: "Stream".into() 
            });
        }
        self.objects.insert(id, obj);
        Ok(())
    }

    /// Compiles the objects into a single Object::Stream.
    pub fn build(self) -> PdfResult<Object> {
        let mut header = Vec::new();
        let mut body = Vec::new();
        
        // Pass 1: Serialize objects to body and record offsets
        let mut offsets = Vec::with_capacity(self.objects.len());
        for (&id, obj) in &self.objects {
            let offset = body.len();
            offsets.push((id, offset));
            write_object(&mut body, obj)?;
            body.push(b' '); // Separator
        }

        // Pass 2: Create header (id offset pairs)
        for (id, offset) in offsets {
             use std::io::Write;
             write!(&mut header, "{id} {offset} ")
                 .map_err(PdfError::from)?;
        }

        let first_offset = header.len();
        
        // Combine header and body
        let mut data = header;
        data.extend(body);

        let mut dict = BTreeMap::new();
        dict.insert(b"Type".to_vec(), Object::new_name(b"ObjStm".to_vec()));
        dict.insert(b"N".to_vec(), Object::Integer(self.objects.len() as i64));
        dict.insert(b"First".to_vec(), Object::Integer(first_offset as i64));

        Ok(Object::new_stream(dict, data))
    }
}

impl Default for ObjectStreamBuilder {
    fn default() -> Self {
        Self::new()
    }
}
