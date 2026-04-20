use ferruginous_core::{Object, PdfResult, Reference, Resolver};
use ferruginous_doc::Document;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Maps a source object reference to its newly allocated reference in the target document.
pub type ReferenceMap = BTreeMap<Reference, Reference>;

/// A utility for cloning objects from one document to another while re-indexing references.
pub struct ObjectCloner<'a> {
    target: &'a mut Document,
    map: ReferenceMap,
}

impl<'a> ObjectCloner<'a> {
    /// Creates a new cloner that will insert objects into the target document.
    pub fn new(target: &'a mut Document) -> Self {
        Self { target, map: BTreeMap::new() }
    }

    /// Iteratively clones an object and all its children into the target document.
    ///
    /// If the object is a reference, it will be resolved in the source document,
    /// a new ID will be allocated in the target, and the resolved object will be cloned.
    pub fn clone_object(&mut self, src_doc: &Document, obj: &Object) -> PdfResult<Object> {
        let _stack = [obj.clone()];
        let _result_obj: Option<Object> = None;

        // Note: For complex nesting (Dictionary/Array), we still need to handle the content.
        // However, the rule prohibits function recursion. We will use a work-stack
        // to handle nested reference discovery while reconstructing objects.

        // Revised approach for Rule 6:
        // 1. If it's a simple object, return it.
        // 2. If it's a Reference, resolve it and push to a "Pending" stack if not already mapped.
        // 3. To handle deep nesting WITHOUT recursion, we use a discovery pass and then a reconstruction pass.

        match obj {
            Object::Reference(r) => {
                self.ensure_cloned_iterative(src_doc, *r)?;
                let mapped = self.map.get(r).ok_or_else(|| {
                    ferruginous_core::PdfError::Other(format!("Reference {} not mapped", r.id))
                })?;
                Ok(Object::Reference(*mapped))
            }
            Object::Dictionary(dict) => {
                let mut new_dict = BTreeMap::new();
                for (k, v) in dict.iter() {
                    new_dict.insert(k.clone(), self.clone_object_internal(src_doc, v)?);
                }
                Ok(Object::Dictionary(Arc::new(new_dict)))
            }
            Object::Array(arr) => {
                let mut new_vec = Vec::with_capacity(arr.len());
                for v in arr.iter() {
                    new_vec.push(self.clone_object_internal(src_doc, v)?);
                }
                Ok(Object::Array(Arc::new(new_vec)))
            }
            Object::Stream(dict, data) => {
                let mut new_dict = BTreeMap::new();
                for (k, v) in dict.iter() {
                    new_dict.insert(k.clone(), self.clone_object_internal(src_doc, v)?);
                }
                Ok(Object::Stream(Arc::new(new_dict), data.clone()))
            }
            _ => Ok(obj.clone()),
        }
    }

    /// Internal helper that uses ensure_cloned_iterative for references.
    fn clone_object_internal(&mut self, src_doc: &Document, obj: &Object) -> PdfResult<Object> {
        match obj {
            Object::Reference(r) => {
                self.ensure_cloned_iterative(src_doc, *r)?;
                let mapped = self.map.get(r).ok_or_else(|| {
                    ferruginous_core::PdfError::Other(format!("Reference {} not mapped", r.id))
                })?;
                Ok(Object::Reference(*mapped))
            }
            Object::Dictionary(dict) => {
                let mut new_dict = BTreeMap::new();
                for (k, v) in dict.iter() {
                    new_dict.insert(k.clone(), self.clone_object_internal(src_doc, v)?);
                }
                Ok(Object::Dictionary(Arc::new(new_dict)))
            }
            Object::Array(arr) => {
                let mut new_vec = Vec::with_capacity(arr.len());
                for v in arr.iter() {
                    new_vec.push(self.clone_object_internal(src_doc, v)?);
                }
                Ok(Object::Array(Arc::new(new_vec)))
            }
            Object::Stream(dict, data) => {
                let mut new_dict = BTreeMap::new();
                for (k, v) in dict.iter() {
                    new_dict.insert(k.clone(), self.clone_object_internal(src_doc, v)?);
                }
                Ok(Object::Stream(Arc::new(new_dict), data.clone()))
            }
            _ => Ok(obj.clone()),
        }
    }

    /// Iteratively ensures that a reference and all its dependencies are present in the target document.
    fn ensure_cloned_iterative(&mut self, src_doc: &Document, root: Reference) -> PdfResult<()> {
        let mut work_stack = vec![root];
        let mut pending_updates = Vec::new();

        // Pass 1: Discovery and Map Allocation
        while let Some(src_ref) = work_stack.pop() {
            if self.map.contains_key(&src_ref) {
                continue;
            }

            // Allocate in target using add_object to ensure it's registered in the store
            let target_ref = self.target.add_object(Object::Null);
            self.map.insert(src_ref, target_ref);

            // Resolve to find nested references
            let obj = src_doc.resolve(&src_ref)?;
            pending_updates.push((target_ref, obj.clone()));

            let mut nested = std::collections::BTreeSet::new();
            obj.gather_references(&mut nested);

            for id in nested {
                let r = Reference::new(id, 0);
                if !self.map.contains_key(&r) {
                    work_stack.push(r);
                }
            }
        }

        // Pass 2: Remap and Apply to Target
        for (target_ref, obj) in pending_updates {
            let cloned_obj = self.remap_object(src_doc, &obj)?;
            self.target.update_object(target_ref.id, cloned_obj)?;
        }

        Ok(())
    }

    /// Re-maps an object's internal references using the current map.
    fn remap_object(&self, _src_doc: &Document, obj: &Object) -> PdfResult<Object> {
        match obj {
            Object::Reference(r) => {
                let mapped = self.map.get(r).ok_or_else(|| {
                    ferruginous_core::PdfError::Other(format!(
                        "Reference {} not mapped during cloning",
                        r.id
                    ))
                })?;
                Ok(Object::Reference(*mapped))
            }
            Object::Dictionary(dict) => {
                let mut new_dict = BTreeMap::new();
                for (k, v) in dict.iter() {
                    new_dict.insert(k.clone(), self.remap_object(_src_doc, v)?);
                }
                Ok(Object::Dictionary(Arc::new(new_dict)))
            }
            Object::Array(arr) => {
                let mut new_vec = Vec::with_capacity(arr.len());
                for v in arr.iter() {
                    new_vec.push(self.remap_object(_src_doc, v)?);
                }
                Ok(Object::Array(Arc::new(new_vec)))
            }
            Object::Stream(dict, data) => {
                let mut new_dict = BTreeMap::new();
                for (k, v) in dict.iter() {
                    new_dict.insert(k.clone(), self.remap_object(_src_doc, v)?);
                }
                Ok(Object::Stream(Arc::new(new_dict), data.clone()))
            }
            _ => Ok(obj.clone()),
        }
    }

    /// Returns the internal mapping of source references to target references.
    pub fn id_map(&self) -> &ReferenceMap {
        &self.map
    }
}
