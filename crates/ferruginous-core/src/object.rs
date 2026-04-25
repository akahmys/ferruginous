//! Refinery 2.1 Refined PDF 2.0 Object Model.
//!
//! This model adheres strictly to ISO 32000-2:2020 and utilizes the Handle system
//! for all internal references, ensuring maximum memory efficiency.

use crate::arena::PdfArena;
use crate::handle::Handle;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// ISO 32000-2:2020 Clause 7.3.5 - Name Objects
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PdfName(pub Bytes);

impl PdfName {
    pub fn new(s: &str) -> Self {
        Self(Bytes::copy_from_slice(s.as_bytes()))
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or("")
    }

    pub fn to_string_lossy(&self) -> String {
        String::from_utf8_lossy(&self.0).to_string()
    }
}

impl AsRef<[u8]> for PdfName {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl std::borrow::Borrow<str> for PdfName {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Object {
    /// Boolean objects (Clause 7.3.2)
    Boolean(bool),
    /// Numeric objects (Integer) (Clause 7.3.3)
    Integer(i64),
    /// Numeric objects (Real) (Clause 7.3.3)
    Real(f64),
    /// String objects (Clause 7.3.4)
    String(Bytes),
    /// Name objects (Clause 7.3.5)
    Name(Handle<PdfName>),
    /// Array objects (Clause 7.3.6)
    /// References an entry in the Arena that holds the actual Vec<Object>.
    Array(Handle<Vec<Object>>),
    /// Dictionary objects (Clause 7.3.7)
    /// References an entry in the Arena that holds the BTreeMap.
    Dictionary(Handle<BTreeMap<Handle<PdfName>, Object>>),
    /// Stream objects (Clause 7.3.8)
    /// References a dictionary handle and holds the raw/encoded data.
    Stream(Handle<BTreeMap<Handle<PdfName>, Object>>, Bytes),
    /// Hexadecimal string objects (Clause 7.3.4.3)
    Hex(Bytes),
    /// Null object (Clause 7.3.9)
    Null,
    /// Reference to an indirect object (external to this object but in the same arena).
    /// This is equivalent to PDF's "R" operator.
    Reference(Handle<Object>),
}

impl Object {
    pub fn as_name(&self) -> Option<Handle<PdfName>> {
        if let Self::Name(h) = self { Some(*h) } else { None }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Real(f) => Some(*f),
            Self::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<Handle<Vec<Object>>> {
        if let Self::Array(h) = self { Some(*h) } else { None }
    }

    pub fn as_bool(&self) -> Option<bool> {
        if let Self::Boolean(b) = self { Some(*b) } else { None }
    }

    pub fn as_dict_handle(&self) -> Option<Handle<BTreeMap<Handle<PdfName>, Object>>> {
        match self {
            Self::Dictionary(h) => Some(*h),
            Self::Stream(h, _) => Some(*h),
            _ => None,
        }
    }

    pub fn as_integer(&self) -> Option<i64> {
        if let Self::Integer(i) = self { Some(*i) } else { None }
    }

    /// Resolves the object if it's a reference, otherwise returns self.
    pub fn resolve(&self, arena: &PdfArena) -> Object {
        let mut current = self.clone();
        let mut depth = 0;
        while let Self::Reference(h) = current {
            if let Some(obj) = arena.get_object(h) {
                current = obj;
            } else {
                break;
            }
            depth += 1;
            if depth > 10 {
                break;
            } // Safety break for circular references
        }
        current
    }

    pub fn as_string(&self) -> Option<String> {
        match self {
            Self::String(b) => Some(String::from_utf8_lossy(b).into_owned()),
            _ => None,
        }
    }

    pub fn as_name_str(&self, arena: &PdfArena) -> Option<String> {
        self.as_name().and_then(|h| arena.get_name_str(h))
    }

    pub fn as_reference(&self) -> Option<Handle<Object>> {
        if let Self::Reference(h) = self { Some(*h) } else { None }
    }
}

pub struct ObjectEntry {
    pub object: Object,
    pub generation: u16,
}

/// A legacy-compatible PDF indirect reference (ISO 32000-2 Clause 7.3.10).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct Reference {
    pub id: u32,
    pub generation: u16,
}

impl Reference {
    pub const fn new(id: u32, generation: u16) -> Self {
        Self { id, generation }
    }
}

impl From<Handle<Object>> for Reference {
    fn from(h: Handle<Object>) -> Self {
        Self::new(h.index(), 0)
    }
}

impl From<Reference> for Handle<Object> {
    fn from(r: Reference) -> Self {
        Self::new(r.id)
    }
}
