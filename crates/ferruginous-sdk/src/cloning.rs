//! Object Migration Bridge (Legacy Bridge)
//! 
//! (Note: This module is currently undergoing modernization for the Arena model.)

use ferruginous_core::{Object, PdfResult};

/// Utility for cloning PDF objects and migrating them between arenas or contexts.
#[derive(Default)]
pub struct ObjectCloner;

impl ObjectCloner {
    /// Creates a new object cloner instance.
    pub fn new() -> Self {
        Self
    }

    /// Clones the given object. (Note: Currently a stub that clones in-place.)
    pub fn clone_object(&self, obj: &Object) -> PdfResult<Object> {
        Ok(obj.clone()) // STUB: Just clones in the same arena for now
    }
}
