//! Refinery 2.1 Sequential Object Arena (RR-15 Hardened).
//!
//! This model utilizes a reference-counted internal state (Arc<RefCell>) to allow
//! the Arena handle itself to be cheaply cloned while maintaining shared mutable state.

use crate::PdfResult;
use crate::handle::Handle;
use crate::object::{Object, ObjectEntry, PdfName};
use bytes::Bytes;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::Arc;

/// A sequential arena for PDF objects, optimized for cache locality and thread safety.
///
/// This implementation uses an `Arc<ArenaInner>` to enable zero-copy cloning of the arena
/// handle, allowing multiple components (Ingestor, Refinery, CLI) to share the same
/// document state without expensive duplication.
#[derive(Default, Clone)]
pub struct PdfArena {
    inner: Arc<ArenaInner>,
}

/// The internal heap-allocated state of the PdfArena.
///
/// All pools are wrapped in `RefCell` to allow interior mutability, enabling the
/// "Pass-based" refinement system where objects are updated in-place as they move
/// through the normalization pipeline.
#[derive(Default)]
struct ArenaInner {
    /// Contiguous pool of objects for maximum cache efficiency.
    objects: RefCell<Vec<ObjectEntry>>,
    /// Dedicated pools for complex types to allow typesafe handles.
    /// This separation prevents "Handle Confusion" (e.g., using an Array handle to access a Dictionary).
    dicts: RefCell<Vec<BTreeMap<Handle<PdfName>, Object>>>,
    arrays: RefCell<Vec<Vec<Object>>>,
    /// Interned names to ensure all `/Name` references in the document point to a single memory location.
    names: RefCell<Vec<PdfName>>,
    /// Index for fast name lookup during interning.
    name_map: RefCell<BTreeMap<PdfName, Handle<PdfName>>>,
    /// The document version (e.g., 1.7, 2.0).
    version: RefCell<f32>,
}

impl PdfArena {
    pub fn new() -> Self {
        Self::with_version(1.7)
    }

    pub fn with_version(version: f32) -> Self {
        let arena = Self::default();
        *arena.inner.version.borrow_mut() = version;
        arena
    }

    pub fn version(&self) -> f32 {
        *self.inner.version.borrow()
    }

    /// Interns a name and returns its handle.
    pub fn intern_name(&self, name: PdfName) -> Handle<PdfName> {
        if let Some(&handle) = self.inner.name_map.borrow().get(&name) {
            return handle;
        }
        let index = u32::try_from(self.inner.names.borrow().len()).unwrap_or(u32::MAX);
        if index == u32::MAX {
            return Handle::new(0);
        }
        let handle = Handle::new(index);
        self.inner.names.borrow_mut().push(name.clone());
        self.inner.name_map.borrow_mut().insert(name, handle);
        handle
    }

    /// Returns a handle for a name, interning it if necessary (Get-or-Create).
    pub fn name(&self, name: &str) -> Handle<PdfName> {
        if let Some(&handle) = self.inner.name_map.borrow().get(name) {
            return handle;
        }
        self.intern_name(PdfName::new(name))
    }

    /// Returns the string representation of a name handle.
    pub fn get_name_str(&self, handle: Handle<PdfName>) -> Option<String> {
        self.inner.names.borrow().get(handle.index() as usize).map(|n| n.as_str().to_string())
    }

    pub fn get_name_by_str(&self, name: &str) -> Option<Handle<PdfName>> {
        self.inner.name_map.borrow().get(name).copied()
    }

    pub fn get_name(&self, handle: Handle<PdfName>) -> Option<PdfName> {
        self.inner.names.borrow().get(handle.index() as usize).cloned()
    }

    /// Returns all valid dictionary handles in the arena.
    pub fn all_dict_handles(&self) -> Vec<Handle<BTreeMap<Handle<PdfName>, Object>>> {
        let count = u32::try_from(self.inner.dicts.borrow().len()).unwrap_or(0);
        (0..count).map(Handle::new).collect()
    }

    /// Allocates an object.
    pub fn alloc_object(&self, object: Object) -> Handle<Object> {
        let mut objects = self.inner.objects.borrow_mut();
        let index = u32::try_from(objects.len()).unwrap_or(u32::MAX);
        if index == u32::MAX {
            return Handle::new(0);
        }
        objects.push(ObjectEntry { object, generation: 0 });
        Handle::new(index)
    }

    /// Allocates a dictionary.
    pub fn alloc_dict(
        &self,
        dict: BTreeMap<Handle<PdfName>, Object>,
    ) -> Handle<BTreeMap<Handle<PdfName>, Object>> {
        let mut dicts = self.inner.dicts.borrow_mut();
        let index = u32::try_from(dicts.len()).unwrap_or(u32::MAX);
        if index == u32::MAX {
            return Handle::new(0);
        }
        dicts.push(dict);
        Handle::new(index)
    }

    /// Allocates an array.
    pub fn alloc_array(&self, array: Vec<Object>) -> Handle<Vec<Object>> {
        let mut arrays = self.inner.arrays.borrow_mut();
        let index = u32::try_from(arrays.len()).unwrap_or(u32::MAX);
        if index == u32::MAX {
            return Handle::new(0);
        }
        arrays.push(array);
        Handle::new(index)
    }

    /// Retrieves an object.
    pub fn get_object(&self, handle: Handle<Object>) -> Option<Object> {
        self.inner.objects.borrow().get(handle.index() as usize).map(|e| e.object.clone())
    }

    /// Updates an existing object.
    pub fn set_object(&self, handle: Handle<Object>, object: Object) {
        if let Some(entry) = self.inner.objects.borrow_mut().get_mut(handle.index() as usize) {
            entry.object = object;
        }
    }

    /// Retrieves a dictionary.
    pub fn get_dict(&self, handle: Handle<BTreeMap<Handle<PdfName>, Object>>) -> Option<BTreeMap<Handle<PdfName>, Object>> {
        self.inner.dicts.borrow().get(handle.index() as usize).cloned()
    }

    /// Updates an existing dictionary.
    pub fn set_dict(&self, handle: Handle<BTreeMap<Handle<PdfName>, Object>>, dict: BTreeMap<Handle<PdfName>, Object>) {
        if let Some(d) = self.inner.dicts.borrow_mut().get_mut(handle.index() as usize) {
            *d = dict;
        }
    }

    /// Retrieves an array.
    pub fn get_array(&self, handle: Handle<Vec<Object>>) -> Option<Vec<Object>> {
        self.inner.arrays.borrow().get(handle.index() as usize).cloned()
    }

    /// Searches for an existing indirect object that matches the provided object.
    pub fn find_indirect_handle(&self, object: &Object) -> Option<Handle<Object>> {
        let objects = self.inner.objects.borrow();
        for (i, entry) in objects.iter().enumerate() {
            if &entry.object == object {
                return Some(Handle::new(i as u32));
            }
        }
        None
    }

    pub fn object_count(&self) -> u32 {
        self.inner.objects.borrow().len() as u32
    }

    /// Applies filters to data using the stream dictionary context.
    pub fn process_filters(&self, data: &[u8], dict: &BTreeMap<Handle<PdfName>, Object>) -> PdfResult<Bytes> {
        crate::filters::process_arena_filters(data, dict, self)
    }
}

pub type RemappingTable = BTreeMap<(u32, u16), Handle<Object>>;
