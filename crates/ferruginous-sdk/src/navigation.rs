//! PDF Navigation (Outlines and Destinations).
//!
//! (ISO 32000-2:2020 Clause 12.3)

use crate::core::{Object, Reference, Resolver, PdfError, PdfResult};
use std::collections::BTreeMap;

/// Represents a destination in a PDF document (Clause 12.3.2).
#[derive(Debug, Clone, PartialEq)]
pub enum Destination {
    /// Explicit destination: [page /XYZ left top zoom] etc. (Clause 12.3.2.2).
    Explicit {
        /// The target page reference.
        page: Reference,
        /// The view parameters (top, left, zoom, etc.).
        params: Vec<Object>,
    },
    /// Named destination: /Name or (Name) (Clause 12.3.2.3).
    Named(Vec<u8>),
}

impl Destination {
    /// Parses a Destination from a PDF object.
    #[must_use] pub fn from_obj(obj: Object) -> Option<Self> {
        match obj {
            Object::Name(n) => Some(Self::Named(n.to_vec())),
            Object::String(s) => Some(Self::Named(s.to_vec())),
            Object::Array(arr) => {
                if arr.is_empty() { return None; }
                if let Object::Reference(r) = &arr[0] {
                    Some(Self::Explicit {
                        page: *r,
                        params: arr[1..].to_vec(),
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

/// Represents a PDF Action (Clause 12.6).
#[derive(Debug, Clone)]
pub struct Action {
    /// The action type (e.g., /GoTo, /URI).
    pub subtype: Vec<u8>,
    /// The action dictionary containing parameters.
    pub dictionary: std::sync::Arc<BTreeMap<Vec<u8>, Object>>,
}

impl Action {
    /// Creates a new `Action` with a subtype and dictionary.
    #[must_use] pub fn new(subtype: Vec<u8>, dictionary: std::sync::Arc<BTreeMap<Vec<u8>, Object>>) -> Self {
        Self { subtype, dictionary }
    }

    /// Retrieves the destination if this is a `GoTo` action.
    #[must_use] pub fn destination(&self) -> Option<Destination> {
        if self.subtype == b"GoTo" {
            self.dictionary.get(b"D".as_ref()).and_then(|obj| {
                Destination::from_obj(obj.clone())
            })
        } else {
            None
        }
    }

    /// Retrieves the URI if this is a URI action.
    #[must_use] pub fn uri(&self) -> Option<Vec<u8>> {
        if self.subtype == b"URI" {
            self.dictionary.get(b"URI".as_ref()).and_then(|obj| {
                obj.as_str().map(|s| s.to_vec())
            })
        } else {
            None
        }
    }
}

/// Represents the Document Outline root (Clause 12.3.3).
pub struct Outline<'a> {
    /// The outline dictionary.
    pub dictionary: std::sync::Arc<BTreeMap<Vec<u8>, Object>>,
    /// The resolver for indirect objects.
    pub resolver: &'a dyn Resolver,
}

impl<'a> Outline<'a> {
    /// Creates a new Outline root instance.
    pub fn new(dictionary: std::sync::Arc<BTreeMap<Vec<u8>, Object>>, resolver: &'a dyn Resolver) -> Self {
        Self { dictionary, resolver }
    }

    /// Returns the reference to the first item in the outline tree.
    #[must_use] pub fn first(&self) -> Option<Reference> {
        self.dictionary.get(b"First".as_ref()).and_then(|obj| {
            if let Object::Reference(r) = obj { Some(*r) } else { None }
        })
    }
}

/// Outline (Bookmark) Item (Clause 12.3.3).
pub struct OutlineItem<'a> {
    /// The dictionary representing the outline item.
    pub dictionary: std::sync::Arc<BTreeMap<Vec<u8>, Object>>,
    /// The reference to this object.
    pub reference: Reference,
    /// The resolver for indirect objects.
    pub resolver: &'a dyn Resolver,
}

impl<'a> OutlineItem<'a> {
    /// Creates a new OutlineItem.
    #[must_use]
    pub fn new(dictionary: std::sync::Arc<BTreeMap<Vec<u8>, Object>>, reference: Reference, resolver: &'a dyn Resolver) -> Self {
        Self { dictionary, reference, resolver }
    }

    /// Returns the title of the outline item as a decoded UTF-8 String.
    #[must_use] pub fn title(&self) -> Option<String> {
        if let Some(Object::String(s)) = self.dictionary.get(b"Title".as_slice()) {
            Some(crate::core::string::decode_text_string(s))
        } else {
            None
        }
    }

    /// Returns the reference to the next outline item at the same level.
    #[must_use] pub fn next(&self) -> Option<Reference> {
        if let Some(Object::Reference(r)) = self.dictionary.get(b"Next".as_slice()) {
            Some(*r)
        } else {
            None
        }
    }

    /// Returns the reference to the first child outline item.
    #[must_use] pub fn first_child(&self) -> Option<Reference> {
        if let Some(Object::Reference(r)) = self.dictionary.get(b"First".as_slice()) {
            Some(*r)
        } else {
            None
        }
    }

    /// Returns the destination or action associated with this outline item.
    #[must_use] pub fn destination(&self) -> Option<Destination> {
        if let Some(obj) = self.dictionary.get(b"Dest".as_slice()) {
            match obj {
                Object::Name(n) => Some(Destination::Named(n.to_vec())),
                Object::String(s) => Some(Destination::Named(s.to_vec())),
                Object::Array(a) => {
                    if let Some(Object::Reference(r)) = a.first() {
                        Some(Destination::Explicit { 
                            page: *r, 
                            params: a[1..].to_vec() 
                        })
                    } else {
                        None
                    }
                }
                _ => None,
            }
        } else {
            None
        }
    }

    /// Returns an iterator that traverses this item's children non-recursively.
    #[must_use] pub fn children(&self) -> OutlineIterator<'a> {
        let mut stack = Vec::new();
        if let Some(r) = self.first_child() {
            stack.push(r);
        }
        OutlineIterator {
            stack,
            resolver: self.resolver,
        }
    }
}

/// A non-recursive iterator over the document outline tree (Clause 12.3.3.2).
pub struct OutlineIterator<'a> {
    /// Stack of outline items (references) to traverse.
    stack: Vec<Reference>,
    /// The resolver for indirect objects within the outline.
    resolver: &'a dyn Resolver,
}

impl<'a> Iterator for OutlineIterator<'a> {
    type Item = PdfResult<OutlineItem<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        // RR-10 v2 Rule 6: limit stack depth to prevent stack overflow
        if self.stack.len() > 32 { return None; }

        let current_ref = self.stack.pop()?;
        
        let obj = match self.resolver.resolve(&current_ref) {
            Ok(o) => o,
            Err(e) => return Some(Err(e)),
        };
        
        // Rule 11: Explicit type check
        let dict = if let Object::Dictionary(d) = obj { d } 
                   else { return Some(Err(PdfError::InvalidType { expected: "Dictionary".into(), found: "Other".into() })); };
        
        let item = OutlineItem::new(dict, current_ref, self.resolver);

        // Push next sibling then first child to ensure depth-first order
        if let Some(next_ref) = item.next() {
            self.stack.push(next_ref);
        }
        if let Some(child_ref) = item.first_child() {
            self.stack.push(child_ref);
        }

        Some(Ok(item))
    }
}
