//! PDF Page Representation (ISO 32000-2:2020 Clause 7.7.3)

use crate::arena::PdfArena;
use crate::handle::Handle;
use crate::object::{Object, PdfName};
use std::collections::BTreeMap;

/// A high-level representation of a PDF page.
pub struct Page<'a> {
    arena: &'a PdfArena,
    dict_handle: Handle<BTreeMap<Handle<PdfName>, Object>>,
    parent_chain: Vec<Handle<BTreeMap<Handle<PdfName>, Object>>>,
}

impl<'a> Page<'a> {
    pub fn new(
        arena: &'a PdfArena,
        dict_handle: Handle<BTreeMap<Handle<PdfName>, Object>>,
        parent_chain: Vec<Handle<BTreeMap<Handle<PdfName>, Object>>>,
    ) -> Self {
        Self { arena, dict_handle, parent_chain }
    }

    /// Resolves a page attribute, following the inheritance chain (ISO 32000-2 Clause 7.7.3.3).
    pub fn resolve_attribute(&self, name: &str) -> Option<Object> {
        let name_handle = self.arena.get_name_by_str(name)?;

        // 1. Check local page dictionary
        if let Some(dict) = self.arena.get_dict(self.dict_handle)
            && let Some(val) = dict.get(&name_handle)
        {
            return Some(val.resolve(self.arena));
        }

        // 2. Check parent chain (Page Tree nodes)
        for &parent_handle in self.parent_chain.iter().rev() {
            if let Some(parent_dict) = self.arena.get_dict(parent_handle)
                && let Some(val) = parent_dict.get(&name_handle)
            {
                return Some(val.resolve(self.arena));
            }
        }

        None
    }

    /// Returns the handle to the page dictionary.
    pub fn dict_handle(&self) -> Handle<BTreeMap<Handle<PdfName>, Object>> {
        self.dict_handle
    }
}
