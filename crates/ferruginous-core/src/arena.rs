//! Refinery 2.1 Sequential Object Arena (RR-15 Hardened).

use std::collections::BTreeMap;
use std::cell::RefCell;
use crate::handle::Handle;
use crate::object::{Object, PdfName, ObjectEntry};
use crate::PdfResult;
use bytes::Bytes;

/// A sequential arena for PDF objects, optimized for cache locality.
#[derive(Default)]
pub struct PdfArena {
    /// Contiguous pool of objects for maximum cache efficiency.
    objects: RefCell<Vec<ObjectEntry>>,
    /// Dedicated pools for complex types to allow typesafe handles.
    dicts: RefCell<Vec<BTreeMap<Handle<PdfName>, Object>>>,
    arrays: RefCell<Vec<Vec<Object>>>,
    /// Interned names
    names: RefCell<Vec<PdfName>>,
    /// Index for fast name lookup during interning.
    name_map: RefCell<BTreeMap<PdfName, Handle<PdfName>>>,
}

impl PdfArena {
    pub fn new() -> Self {
        Self::default()
    }

    /// Interns a name and returns its handle.
    pub fn intern_name(&self, name: PdfName) -> Handle<PdfName> {
        if let Some(&handle) = self.name_map.borrow().get(&name) {
            return handle;
        }
        let index = u32::try_from(self.names.borrow().len()).expect("Arena capacity exceeded");
        let handle = Handle::new(index);
        self.names.borrow_mut().push(name.clone());
        self.name_map.borrow_mut().insert(name, handle);
        handle
    }

    /// Returns a handle for a name, interning it if necessary (Get-or-Create).
    pub fn name(&self, name: &str) -> Handle<PdfName> {
        // Optimized check without cloning the string if already interned
        if let Some(&handle) = self.name_map.borrow().get(name) {
            return handle;
        }
        self.intern_name(PdfName::new(name))
    }

    /// Returns the string representation of a name handle.
    pub fn get_name_str(&self, handle: Handle<PdfName>) -> Option<String> {
        self.names.borrow().get(handle.index() as usize).map(|n| n.as_str().to_string())
    }

    pub fn get_name_by_str(&self, name: &str) -> Option<Handle<PdfName>> {
        self.name_map.borrow().get(name).copied()
    }

    pub fn get_name(&self, handle: Handle<PdfName>) -> Option<PdfName> {
        self.names.borrow().get(handle.index() as usize).cloned()
    }

    /// Returns all valid dictionary handles in the arena.
    pub fn all_dict_handles(&self) -> Vec<Handle<BTreeMap<Handle<PdfName>, Object>>> {
        let count = u32::try_from(self.dicts.borrow().len()).expect("Arena capacity exceeded");
        (0..count).map(Handle::new).collect()
    }

    /// Allocates an object.
    pub fn alloc_object(&self, object: Object) -> Handle<Object> {
        let mut objects = self.objects.borrow_mut();
        let index = u32::try_from(objects.len()).expect("Arena capacity exceeded");
        objects.push(ObjectEntry {
            object,
            generation: 0,
        });
        Handle::new(index)
    }

    /// Allocates a dictionary.
    pub fn alloc_dict(&self, dict: BTreeMap<Handle<PdfName>, Object>) -> Handle<BTreeMap<Handle<PdfName>, Object>> {
        let mut dicts = self.dicts.borrow_mut();
        let index = u32::try_from(dicts.len()).expect("Arena capacity exceeded");
        dicts.push(dict);
        Handle::new(index)
    }

    /// Allocates an array.
    pub fn alloc_array(&self, array: Vec<Object>) -> Handle<Vec<Object>> {
        let mut arrays = self.arrays.borrow_mut();
        let index = u32::try_from(arrays.len()).expect("Arena capacity exceeded");
        arrays.push(array);
        Handle::new(index)
    }

    // Accessors (Returning cloned values to avoid persistent borrows)
    pub fn get_object(&self, handle: Handle<Object>) -> Option<Object> {
        self.objects.borrow().get(handle.index() as usize).map(|e| e.object.clone())
    }

    pub fn get_dict(&self, handle: Handle<BTreeMap<Handle<PdfName>, Object>>) -> Option<BTreeMap<Handle<PdfName>, Object>> {
        self.dicts.borrow().get(handle.index() as usize).cloned()
    }

    pub fn get_array(&self, handle: Handle<Vec<Object>>) -> Option<Vec<Object>> {
        self.arrays.borrow().get(handle.index() as usize).cloned()
    }

    /// Special accessor for mutation (rarely needed).
    pub fn set_object(&self, handle: Handle<Object>, object: Object) {
        if let Some(entry) = self.objects.borrow_mut().get_mut(handle.index() as usize) {
            entry.object = object;
        }
    }

    pub fn set_dict(&self, handle: Handle<BTreeMap<Handle<PdfName>, Object>>, dict: BTreeMap<Handle<PdfName>, Object>) {
        if let Some(entry) = self.dicts.borrow_mut().get_mut(handle.index() as usize) {
            *entry = dict;
        }
    }

    // Processing
    pub fn process_filters(&self, data: &[u8], dict: &BTreeMap<Handle<PdfName>, Object>) -> PdfResult<Bytes> {
         // Re-implementing simplified filter processing or delegating to filters module
         crate::filters::process_arena_filters(data, dict, self)
    }

    /// Returns the number of objects in the primary pool.
    pub fn object_count(&self) -> usize {
        self.objects.borrow().len()
    }

    /// Returns the number of dictionaries in the pool.
    pub fn dict_count(&self) -> usize {
        self.dicts.borrow().len()
    }

    /// Returns the number of arrays in the pool.
    pub fn array_count(&self) -> usize {
        self.arrays.borrow().len()
    }
}

/// A map between physical (Object ID, Gen) and Arena Handle.
pub type RemappingTable = BTreeMap<(u32, u16), Handle<Object>>;
