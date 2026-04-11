//! PDF Object Resolver and Cache.

use crate::core::{Object, Reference, Resolver, PdfError, PdfResult, ParseErrorVariant};
use crate::lexer::parse_object;
use crate::xref::{XRefIndex, XRefEntry};
use std::convert::TryInto;

/// The standard resolver that reads from the document data using an xref index.
pub struct PdfResolver<'a> {
    /// The document byte buffer.
    pub data: &'a [u8],
    /// The cross-reference index.
    pub index: std::sync::Arc<dyn XRefIndex>,
    /// Security handler for decryption.
    pub security: Option<std::sync::Arc<crate::security::SecurityHandler>>,
    /// Rule 5: Internal cache for resolved objects.
    pub cache: std::sync::Mutex<std::collections::BTreeMap<Reference, Object>>,
}

/// A resolver that layers multiple resolvers, typically for editing (Memory + File).
/// (ISO 32000-2:2020 Clause 7.3.10)
pub struct LayeredResolver<'a> {
    /// Memory overlay containing modified or new objects.
    pub overlay: std::collections::BTreeMap<Reference, Object>,
    /// Base resolver (usually the original file data).
    pub base: Box<dyn Resolver + Send + Sync + 'a>,
}

impl<'a> LayeredResolver<'a> {
    /// Creates a new layered resolver with a base resolver.
    #[must_use] pub fn new(base: Box<dyn Resolver + Send + Sync + 'a>) -> Self {
        Self {
            overlay: std::collections::BTreeMap::new(),
            base,
        }
    }

    /// Inserts a modified or new object into the overlay.
    pub fn insert(&mut self, reference: Reference, object: Object) {
        self.overlay.insert(reference, object);
    }
}

impl Resolver for LayeredResolver<'_> {
    fn resolve(&self, reference: &Reference) -> PdfResult<Object> {
        // 1. Check overlay (Dirty objects) first
        if let Some(obj) = self.overlay.get(reference) {
            return Ok(obj.clone());
        }

        // 2. Fall back to base resolver
        self.base.resolve(reference)
    }
}

impl Resolver for PdfResolver<'_> {
    fn resolve(&self, reference: &Reference) -> PdfResult<Object> {
        // 1. Check cache first (Performance Optimization)
        if let Some(obj) = self.cache.lock().map_err(|_| PdfError::ResourceError("Resolver cache mutex poisoned".into()))?.get(reference) {
            return Ok(obj.clone());
        }

        let entry = self.index.get(reference.id)
            .ok_or(PdfError::ObjectNotFound(*reference))?;

        let obj = match entry {
            XRefEntry::InUse { offset, generation } => {
                self.resolve_in_use(reference, offset, generation)?
            }
            XRefEntry::Free { .. } => {
                return Err(PdfError::ObjectNotFound(*reference));
            }
            XRefEntry::Compressed { container_id, index } => {
                self.resolve_compressed(reference, container_id, index)?
            }
        };

        // Cache the result
        self.cache.lock().map_err(|_| PdfError::ResourceError("Resolver cache mutex poisoned".into()))?.insert(*reference, obj.clone());
        Ok(obj)
    }
}

impl PdfResolver<'_> {
    fn resolve_in_use(&self, r: &Reference, offset: u64, generation: u16) -> PdfResult<Object> {
        if generation != r.generation {
            return Err(PdfError::ParseError(ParseErrorVariant::GenerationMismatch { 
                id: r.id, 
                expected: r.generation, 
                found: generation 
            }));
        }
        
        let offset_usize: usize = offset.try_into()
            .map_err(|_| PdfError::ParseError(ParseErrorVariant::InvalidOffset { offset: offset as usize }))?;
        
        if offset_usize >= self.data.len() {
            return Err(PdfError::ParseError(ParseErrorVariant::InvalidOffset { offset: offset_usize }));
        }

        let obj_data = &self.data[offset_usize..];
        let (remaining, _) = crate::lexer::parse_id_gen_obj(obj_data)
            .map_err(|e| PdfError::ParseError(ParseErrorVariant::HeaderError { offset, details: format!("{e:?}") }))?;
        
        let (_, mut obj) = parse_object(remaining)
            .map_err(|e| PdfError::ParseError(ParseErrorVariant::general(offset, format!("Body parse error: {e:?}"))))?;
        
        if let Some(ref security) = self.security {
            obj = self.decrypt_resolved_object(r.id, r.generation, obj, security)?;
        }
        Ok(obj)
    }

    fn resolve_compressed(&self, r: &Reference, container_id: u32, _idx: u32) -> PdfResult<Object> {
        let container_ref = Reference { id: container_id, generation: 0 };
        let container_obj = self.resolve(&container_ref)
            .map_err(|e| PdfError::ResourceError(format!("Failed to resolve object stream {container_id}: {e}")))?;
        
        if let Object::Stream(dict, data) = container_obj {
            let decoded_data = crate::filter::decode_stream(&dict, &data)
                .map_err(|e| PdfError::ContentError(e.to_string().into()))?;
            let n = if let Some(Object::Integer(n)) = dict.get(b"N".as_slice()) { *n as usize } else { 0 };
            let first = if let Some(Object::Integer(f)) = dict.get(b"First".as_slice()) { *f as usize } else { 0 };
            
            assert!(n > 0);
            let mut current_input = &decoded_data[..std::cmp::min(first, decoded_data.len())];
            let mut found_offset = None;
            for _ in 0..std::cmp::min(n, 10000) {
                if current_input.is_empty() { break; }
                let (rem, oid) = nom::character::complete::digit1::<&[u8], nom::error::Error<&[u8]>>(current_input)
                    .map_err(|_| PdfError::ParseError(ParseErrorVariant::general(0, "Failed to parse obj id")))?;
                let (rem, _) = nom::character::complete::multispace1::<&[u8], nom::error::Error<&[u8]>>(rem).map_err(|_| PdfError::ParseError(ParseErrorVariant::general(0, "Space required")))?;
                let (rem, ooff) = nom::character::complete::digit1::<&[u8], nom::error::Error<&[u8]>>(rem).map_err(|_| PdfError::ParseError(ParseErrorVariant::general(0, "Failed to parse offset")))?;
                let (rem, _) = nom::character::complete::multispace0::<&[u8], nom::error::Error<&[u8]>>(rem).map_err(|_| PdfError::ParseError(ParseErrorVariant::general(0, "Space ok")))?;
                
                let oid_num: u32 = std::str::from_utf8(oid).map_err(|_| PdfError::ParseError(ParseErrorVariant::general(0, "UTF8 error")))?.parse().map_err(|_| PdfError::ParseError(ParseErrorVariant::general(0, "Parse error")))?;
                let ooff_num: usize = std::str::from_utf8(ooff).map_err(|_| PdfError::ParseError(ParseErrorVariant::general(0, "UTF8 error")))?.parse().map_err(|_| PdfError::ParseError(ParseErrorVariant::general(0, "Parse error")))?;
                if oid_num == r.id { found_offset = Some(ooff_num); break; }
                current_input = rem;
            }
            
            if let Some(rel_offset) = found_offset {
                let abs_offset = first + rel_offset;
                if abs_offset >= decoded_data.len() { return Err(PdfError::ParseError(ParseErrorVariant::InvalidOffset { offset: abs_offset })); }
                let (_, obj) = parse_object(&decoded_data[abs_offset..])
                    .map_err(|e| PdfError::ParseError(ParseErrorVariant::general(abs_offset as u64, format!("Compressed parse error: {e:?}"))))?;
                Ok(obj)
            } else { Err(PdfError::ObjectNotFound(*r)) }
        } else { Err(PdfError::InvalidType { expected: "Stream".to_string(), found: format!("{:?}", container_obj) }) }
    }

    fn decrypt_resolved_object(
        &self,
        id: u32,
        generation: u16,
        mut obj: Object,
        security: &crate::security::SecurityHandler,
    ) -> PdfResult<Object> {
        let mut stack = vec![&mut obj];
        let mut count = 0;
        
        while let Some(current) = stack.pop() {
            count += 1;
            if count > 1000 { return Err(PdfError::SecurityError("Too many nested objects during decryption".to_string())); }
            
            match current {
                Object::String(bytes) => {
                    let decrypted = security.decrypt_data(id, generation, bytes)
                        .map_err(|e| PdfError::SecurityError(e.to_string()))?;
                    *bytes = std::sync::Arc::new(decrypted);
                }
                Object::Stream(_dict, bytes) => {
                    let decrypted = security.decrypt_data(id, generation, bytes)
                        .map_err(|e| PdfError::SecurityError(e.to_string()))?;
                    *bytes = std::sync::Arc::new(decrypted);
                }
                Object::Array(arr) => {
                    for elm in std::sync::Arc::make_mut(arr).iter_mut() { stack.push(elm); }
                }
                Object::Dictionary(dict) => {
                    for (_, val) in std::sync::Arc::make_mut(dict).iter_mut() { stack.push(val); }
                }
                _ => {}
            }
        }
        
        debug_assert!(count > 0);
        Ok(obj)
    }
}

/// A mock resolver for testing purposes, storing objects in an in-memory map.
pub struct MockResolver {
    /// The in-memory storage of PDF objects, indexed by (id, generation).
    pub objects: std::collections::BTreeMap<(u32, u16), Object>,
}

impl Default for MockResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl MockResolver {
    /// Creates a new empty mock resolver.
    #[must_use] pub const fn new() -> Self {
        Self { objects: std::collections::BTreeMap::new() }
    }
    /// Adds an object to the mock resolver.
    pub fn add_object(&mut self, id: u32, generation: u16, obj: Object) {
        self.objects.insert((id, generation), obj);
    }
}

impl Resolver for MockResolver {
    fn resolve(&self, r: &Reference) -> PdfResult<Object> {
        self.objects.get(&(r.id, r.generation))
            .cloned()
            .ok_or(PdfError::ObjectNotFound(*r))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Object;

    #[test]
    fn test_layered_resolver_basic() {
        let mut mock = MockResolver::new();
        let ref1 = Reference { id: 1, generation: 0 };
        mock.add_object(1, 0, Object::Integer(100));

        let mut layered = LayeredResolver::new(Box::new(mock));
        
        // Resolve from base
        assert_eq!(layered.resolve(&ref1).unwrap(), Object::Integer(100));

        // Insert into overlay and resolve (Override)
        layered.insert(ref1, Object::Integer(200));
        assert_eq!(layered.resolve(&ref1).unwrap(), Object::Integer(200));
    }
}
