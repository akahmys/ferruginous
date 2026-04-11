//! Multimedia and 3D support (ISO 32000-2:2020 Clause 13).
//! Focuses on parsing RichMedia and 3D annotations for validation.

use crate::core::{Object, Reference, Resolver};
use std::collections::BTreeMap;

/// Represents a RichMedia annotation (Clause 13.2).
pub struct RichMedia<'a> {
    /// The multimedia dictionary.
    pub dictionary: std::sync::Arc<BTreeMap<Vec<u8>, Object>>,
    /// The reference to the multimedia object.
    pub reference: Reference,
    /// The resolver for indirect objects.
    pub resolver: &'a dyn Resolver,
}

impl<'a> RichMedia<'a> {
    /// Creates a new RichMedia annotation instance.
    pub fn new(dictionary: std::sync::Arc<BTreeMap<Vec<u8>, Object>>, reference: Reference, resolver: &'a dyn Resolver) -> Self {
        Self { dictionary, reference, resolver }
    }

    /// Returns the RichMediaSettings dictionary (Clause 13.2.1).
    pub fn settings(&self) -> Option<std::sync::Arc<BTreeMap<Vec<u8>, Object>>> {
        self.dictionary.get(b"RichMediaSettings".as_ref())
            .and_then(|obj| self.resolver.resolve_if_ref(obj).ok())
            .and_then(|obj| obj.as_dict_arc())
    }

    /// Returns the RichMediaContent dictionary (Clause 13.2.1).
    pub fn content(&self) -> Option<std::sync::Arc<BTreeMap<Vec<u8>, Object>>> {
        self.dictionary.get(b"RichMediaContent".as_ref())
            .and_then(|obj| self.resolver.resolve_if_ref(obj).ok())
            .and_then(|obj| obj.as_dict_arc())
    }
}

/// Represents a 3D annotation (Clause 13.6).
pub struct Annotation3D<'a> {
    /// The multimedia dictionary.
    pub dictionary: std::sync::Arc<BTreeMap<Vec<u8>, Object>>,
    /// The reference to the multimedia object.
    pub reference: Reference,
    /// The resolver for indirect objects.
    pub resolver: &'a dyn Resolver,
}

impl<'a> Annotation3D<'a> {
    /// Creates a new 3D annotation instance.
    pub fn new(dictionary: std::sync::Arc<BTreeMap<Vec<u8>, Object>>, reference: Reference, resolver: &'a dyn Resolver) -> Self {
        Self { dictionary, reference, resolver }
    }

    /// Returns the 3D stream (U3D/PRC) (Clause 13.6.3).
    pub fn stream_3d(&self) -> Option<(std::sync::Arc<BTreeMap<Vec<u8>, Object>>, std::sync::Arc<Vec<u8>>)> {
        self.dictionary.get(b"3DD".as_ref())
            .and_then(|obj| self.resolver.resolve_if_ref(obj).ok())
            .and_then(|obj| if let Object::Stream(dict, data) = obj { Some((std::sync::Arc::clone(&dict), std::sync::Arc::clone(&data))) } else { None })
    }

    /// Returns the 3D view dictionary (Clause 13.6.4).
    pub fn initial_view(&self) -> Option<std::sync::Arc<BTreeMap<Vec<u8>, Object>>> {
        self.dictionary.get(b"3DV".as_ref())
            .and_then(|obj| self.resolver.resolve_if_ref(obj).ok())
            .and_then(|obj| obj.as_dict_arc())
    }
}
