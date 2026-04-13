//! Core PDF object types and trait definitions.
//! (ISO 32000-2:2020 Clause 7.3)

use std::collections::BTreeMap;
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use crate::core::error::PdfResult;

/// ISO 32000-2:2020 Clause 7.3.10 - Indirect Objects
/// Uniquely identifies an indirect object by its object ID and generation number.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct Reference {
    /// The object number (non-negative integer).
    pub id: u32,
    /// The generation number (non-negative integer).
    pub generation: u16,
}

impl Reference {
    /// Creates a new reference with the given ID and generation.
    #[must_use] pub const fn new(id: u32, generation: u16) -> Self {
        Self { id, generation }
    }
}

/// A trait for resolving indirect references into actual PDF objects.
/// (Clause 7.3.10 - Indirect Objects)
pub trait Resolver: Send + Sync {
    /// Resolves a reference into an object.
    /// (Rule 11: Explicit Error)
    fn resolve(&self, reference: &Reference) -> PdfResult<Object>;

    /// Resolves an object if it is a reference, otherwise returns the object itself.
    fn resolve_if_ref(&self, object: &Object) -> PdfResult<Object> {
        match object {
            Object::Reference(r) => self.resolve(r),
            _ => Ok(object.clone()),
        }
    }
}

/// ISO 32000-2:2020 Clause 7.3.1 - General Object Types
/// Represents any PDF object type defined in the specification.
/// (Rule 15: Uses Arc for O(1) clones of complex data)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Object {
    /// Boolean objects (Clause 7.3.2) - representing true or false values.
    Boolean(bool),
    /// Numeric objects (Integer) (Clause 7.3.3) - representing precise 64-bit integers.
    Integer(i64),
    /// Numeric objects (Real) (Clause 7.3.3) - representing floating-point numbers.
    Real(f64),
    /// String objects (Clause 7.3.4) - representing sequences of bytes.
    String(Arc<Vec<u8>>),
    /// Name objects (Clause 7.3.5) - representing unique, atomized names (keys).
    Name(Arc<Vec<u8>>),
    /// Array objects (Clause 7.3.6) - representing an ordered collection of objects.
    Array(Arc<Vec<Object>>),
    /// Dictionary objects (Clause 7.3.7) - representing a map of name keys to objects.
    Dictionary(Arc<BTreeMap<Vec<u8>, Object>>),
    /// Stream objects (Clause 7.3.8) - representing a dictionary and a data payload.
    Stream(Arc<BTreeMap<Vec<u8>, Object>>, Arc<Vec<u8>>),
    /// Null object (Clause 7.3.9) - representing the absence of an object.
    Null,
    /// Indirect Objects (References) (Clause 7.3.10) - pointing to an indirect object by ID.
    Reference(Reference),
}

impl Object {
    /// Creates a new name object.
    #[must_use] pub fn new_name(data: Vec<u8>) -> Self {
        Self::Name(Arc::new(data))
    }

    /// Creates a new string object.
    #[must_use] pub fn new_string(data: Vec<u8>) -> Self {
        Self::String(Arc::new(data))
    }

    /// Creates a new array object.
    #[must_use] pub fn new_array(data: Vec<Object>) -> Self {
        Self::Array(Arc::new(data))
    }

    /// Creates a new dictionary object.
    #[must_use] pub fn new_dict(data: BTreeMap<Vec<u8>, Object>) -> Self {
        Self::Dictionary(Arc::new(data))
    }

    /// Creates a new stream object.
    #[must_use] pub fn new_stream(dict: BTreeMap<Vec<u8>, Object>, data: Vec<u8>) -> Self {
        Self::Stream(Arc::new(dict), Arc::new(data))
    }

    /// Creates a new dictionary object from an existing Arc.
    #[must_use] pub fn new_dict_arc(data: Arc<BTreeMap<Vec<u8>, Object>>) -> Self {
        Self::Dictionary(data)
    }

    /// Creates a new stream object from existing Arcs.
    #[must_use] pub fn new_stream_arc(dict: Arc<BTreeMap<Vec<u8>, Object>>, data: Arc<Vec<u8>>) -> Self {
        Self::Stream(dict, data)
    }

    /// Creates a new array object from an existing Arc.
    #[must_use] pub fn new_array_arc(arr: Arc<Vec<Object>>) -> Self {
        Self::Array(arr)
    }

    /// Returns the object as a dictionary if it is one.
    #[must_use] pub fn as_dict(&self) -> Option<&BTreeMap<Vec<u8>, Object>> {
        match self {
            Self::Dictionary(dict) | Self::Stream(dict, _) => Some(dict),
            _ => None,
        }
    }

    /// Returns the object's dictionary Arc if it is one.
    #[must_use] pub fn as_dict_arc(&self) -> Option<Arc<BTreeMap<Vec<u8>, Object>>> {
        match self {
            Self::Dictionary(dict) | Self::Stream(dict, _) => Some(Arc::clone(dict)),
            _ => None,
        }
    }

    /// Returns the object as an array if it is one.
    #[must_use] pub fn as_array(&self) -> Option<&[Object]> {
        if let Self::Array(arr) = self { Some(arr) } else { None }
    }

    /// Returns the object's array Arc if it is one.
    #[must_use] pub fn as_array_arc(&self) -> Option<Arc<Vec<Object>>> {
        if let Self::Array(arr) = self { Some(Arc::clone(arr)) } else { None }
    }

    /// Returns the object as a string if it is one.
    #[must_use] pub fn as_str(&self) -> Option<&[u8]> {
        match self {
            Self::String(s) | Self::Name(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the name of the object type.
    #[must_use] pub fn type_name(&self) -> &'static str {
        match self {
            Self::Boolean(_) => "Boolean",
            Self::Integer(_) => "Integer",
            Self::Real(_) => "Real",
            Self::String(_) => "String",
            Self::Name(_) => "Name",
            Self::Array(_) => "Array",
            Self::Dictionary(_) => "Dictionary",
            Self::Stream(_, _) => "Stream",
            Self::Null => "Null",
            Self::Reference(_) => "Reference",
        }
    }
}
