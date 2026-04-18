use std::collections::BTreeMap;
use std::sync::Arc;
use bytes::Bytes;
use crate::error::PdfResult;

/// ISO 32000-2:2020 Clause 7.3.5 - Name Objects
///
/// A name object is an atomic symbol uniquely defined by a sequence of characters.
/// Names are used as keys in dictionaries.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PdfName(pub Bytes);

impl PdfName {
    /// Creates a new name from a byte slice, decoding any #xx hex sequences.
    pub fn new(data: &[u8]) -> Self {
        let mut result = Vec::with_capacity(data.len());
        let mut i = 0;
        while i < data.len() {
            if data[i] == b'#' && i + 2 < data.len()
                && let (Some(d1), Some(d2)) = (hex_to_val(data[i+1]), hex_to_val(data[i+2])) {
                    result.push((d1 << 4) | d2);
                    i += 3;
                    continue;
                }
            result.push(data[i]);
            i += 1;
        }
        Self(Bytes::from(result))
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or("")
    }
}

fn hex_to_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

impl AsRef<[u8]> for PdfName {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<&str> for PdfName {
    fn from(s: &str) -> Self {
        Self::new(s.as_bytes())
    }
}

impl From<Bytes> for PdfName {
    fn from(b: Bytes) -> Self {
        Self(b)
    }
}

/// ISO 32000-2:2020 Clause 7.3.10 - Indirect Objects
///
/// Uniquely identifies an indirect object by its object ID and generation number.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Reference {
    pub id: u32,
    pub generation: u16,
}

impl Reference {
    pub const fn new(id: u32, generation: u16) -> Self {
        Self { id, generation }
    }
}

/// A trait for resolving indirect references into actual PDF objects.
pub trait Resolver: Send + Sync {
    /// Resolves a reference into an object.
    fn resolve(&self, reference: &Reference) -> PdfResult<Object>;

    /// Resolves an object if it is a reference, otherwise returns the object itself.
    fn resolve_if_ref(&self, object: &Object) -> PdfResult<Object> {
        match object {
            Object::Reference(r) => self.resolve(r),
            _ => Ok(object.clone()),
        }
    }
}

/// ISO 32000-2:2020 Clause 7.3 - Object Types
#[derive(Debug, Clone, PartialEq)]
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
    Name(PdfName),
    /// Array objects (Clause 7.3.6)
    Array(Arc<Vec<Object>>),
    /// Dictionary objects (Clause 7.3.7)
    Dictionary(Arc<BTreeMap<PdfName, Object>>),
    /// Stream objects (Clause 7.3.8)
    ///
    /// Contains the stream dictionary and the raw data.
    Stream(Arc<BTreeMap<PdfName, Object>>, Bytes),
    /// Null object (Clause 7.3.9)
    Null,
    /// Indirect Objects (References) (Clause 7.3.10)
    Reference(Reference),
}

impl Object {
    pub fn type_name(&self) -> &'static str {
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

    pub fn as_dict(&self) -> Option<&BTreeMap<PdfName, Object>> {
        match self {
            Self::Dictionary(d) | Self::Stream(d, _) => Some(d.as_ref()),
            _ => None,
        }
    }

    pub fn as_dict_arc(&self) -> Option<Arc<BTreeMap<PdfName, Object>>> {
        match self {
            Self::Dictionary(d) | Self::Stream(d, _) => Some(d.clone()),
            _ => None,
        }
    }

    pub fn as_stream(&self) -> Option<(&BTreeMap<PdfName, Object>, &Bytes)> {
        match self {
            Self::Stream(d, b) => Some((d, b)),
            _ => None,
        }
    }

    /// Unified accessor for numbers as f64.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Integer(i) => Some(*i as f64),
            Self::Real(f) => Some(*f),
            _ => None,
        }
    }

    /// Unified accessor for numbers as i64 (casts Real if needed).
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Integer(i) => Some(*i),
            Self::Real(f) => Some(*f as i64),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<Arc<Vec<Object>>> {
        if let Self::Array(a) = self { Some(a.clone()) } else { None }
    }

    pub fn as_bool(&self) -> Option<bool> {
        if let Self::Boolean(b) = self { Some(*b) } else { None }
    }

    pub fn as_name(&self) -> Option<&PdfName> {
        if let Self::Name(n) = self { Some(n) } else { None }
    }

    pub fn as_reference(&self) -> Option<Reference> {
        if let Self::Reference(r) = self { Some(*r) } else { None }
    }

    pub fn as_string(&self) -> Option<&Bytes> {
        if let Self::String(s) = self { Some(s) } else { None }
    }

    /// Decodes stream data using filters specified in the stream dictionary.
    pub fn decode_stream(&self) -> PdfResult<Bytes> {
        match self {
            Self::Stream(dict, data) => {
                crate::filters::decode_stream_from_dict(dict, data.to_vec())
            }
            _ => Err(crate::error::PdfError::Other("Not a stream object".into())),
        }
    }

    /// Recursively collects all indirect object IDs referenced by this object.
    pub fn gather_references(&self, refs: &mut std::collections::HashSet<u32>) {
        match self {
            Self::Reference(r) => {
                refs.insert(r.id);
            }
            Self::Array(a) => {
                for obj in a.iter() {
                    obj.gather_references(refs);
                }
            }
            Self::Dictionary(d) | Self::Stream(d, _) => {
                for obj in d.values() {
                    obj.gather_references(refs);
                }
            }
            _ => {}
        }
    }
}
