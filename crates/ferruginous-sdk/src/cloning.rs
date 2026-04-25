use ferruginous_core::{Handle, Object, PdfArena, PdfResult};
use std::collections::{BTreeMap, HashSet};

/// Utility for cloning PDF objects and migrating them between arenas or contexts.
pub struct ObjectCloner<'a> {
    source: &'a PdfArena,
    target: &'a PdfArena,
    /// Mapping from source Handle<Object> to target Handle<Object>.
    handle_map: BTreeMap<Handle<Object>, Handle<Object>>,
    /// Tracks recursion to avoid infinite loops in malformed PDFs.
    visited: HashSet<Handle<Object>>,
}

impl<'a> ObjectCloner<'a> {
    /// Creates a new object cloner for migrating objects between source and target arenas.
    pub fn new(source: &'a PdfArena, target: &'a PdfArena) -> Self {
        Self {
            source,
            target,
            handle_map: BTreeMap::new(),
            visited: HashSet::new(),
        }
    }

    /// Recursively clones a high-level Object into the target arena.
    ///
    /// If the object contains references, they are followed and cloned recursively.
    /// Internal object sharing is preserved via an internal remapping table.
    pub fn clone_object(&mut self, obj: &Object) -> PdfResult<Object> {
        match obj {
            Object::Boolean(b) => Ok(Object::Boolean(*b)),
            Object::Integer(i) => Ok(Object::Integer(*i)),
            Object::Real(f) => Ok(Object::Real(*f)),
            Object::String(s) => Ok(Object::String(s.clone())),
            Object::Hex(s) => Ok(Object::Hex(s.clone())),
            Object::Null => Ok(Object::Null),
            Object::Name(h) => {
                let name_str = self.source.get_name_str(*h).unwrap_or_default();
                Ok(Object::Name(self.target.name(&name_str)))
            }
            Object::Array(h) => {
                let source_arr = self.source.get_array(*h).unwrap_or_default();
                let mut target_arr = Vec::with_capacity(source_arr.len());
                for item in source_arr {
                    target_arr.push(self.clone_object(&item)?);
                }
                Ok(Object::Array(self.target.alloc_array(target_arr)))
            }
            Object::Dictionary(h) => {
                let source_dict = self.source.get_dict(*h).unwrap_or_default();
                let mut target_dict = BTreeMap::new();
                for (k, v) in source_dict {
                    let k_str = self.source.get_name_str(k).unwrap_or_default();
                    let target_k = self.target.name(&k_str);
                    target_dict.insert(target_k, self.clone_object(&v)?);
                }
                Ok(Object::Dictionary(self.target.alloc_dict(target_dict)))
            }
            Object::Stream(dh, data) => {
                let source_dict = self.source.get_dict(*dh).unwrap_or_default();
                let mut target_dict = BTreeMap::new();
                for (k, v) in source_dict {
                    let k_str = self.source.get_name_str(k).unwrap_or_default();
                    let target_k = self.target.name(&k_str);
                    target_dict.insert(target_k, self.clone_object(&v)?);
                }
                let target_dh = self.target.alloc_dict(target_dict);
                Ok(Object::Stream(target_dh, data.clone()))
            }
            Object::Reference(h) => {
                if let Some(&target_h) = self.handle_map.get(h) {
                    return Ok(Object::Reference(target_h));
                }

                if !self.visited.insert(*h) {
                    return Ok(Object::Null); // Loop detected
                }

                let source_obj = self
                    .source
                    .get_object(*h)
                    .ok_or_else(|| ferruginous_core::PdfError::Other("Dangling reference".into()))?;

                // Note: To avoid recursion depth issues, we allocate the placeholder first if needed,
                // but since PdfArena::alloc_object takes an Object, we can't easily allocate "empty".
                // Instead, we rely on the visited set and recursive calls.
                let target_val = self.clone_object(&source_obj)?;
                let target_h = self.target.alloc_object(target_val);
                self.handle_map.insert(*h, target_h);
                self.visited.remove(h);

                Ok(Object::Reference(target_h))
            }
        }
    }

    /// Clones a specific handle's object and returns the new handle.
    pub fn clone_handle(&mut self, handle: Handle<Object>) -> PdfResult<Handle<Object>> {
        if let Some(&target_h) = self.handle_map.get(&handle) {
            return Ok(target_h);
        }

        let obj = Object::Reference(handle);
        let cloned_ref = self.clone_object(&obj)?;
        if let Object::Reference(h) = cloned_ref {
            Ok(h)
        } else {
            Err(ferruginous_core::PdfError::Other("Failed to clone handle".into()))
        }
    }
}
