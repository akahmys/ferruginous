//! PDF Annotation (Annots) management.
//! (ISO 32000-2:2020 Clause 12.5)

use crate::core::{Object, Reference, Resolver};
use crate::navigation::{Destination, Action};
use std::collections::BTreeMap;

/// Represents a PDF Annotation (Clause 12.5).
pub struct Annotation<'a> {
    /// The annotation dictionary.
    pub dictionary: std::sync::Arc<BTreeMap<Vec<u8>, Object>>,
    /// The reference to this annotation object.
    pub reference: Reference,
    /// The resolver for indirect objects.
    pub resolver: &'a dyn Resolver,
}

impl<'a> Annotation<'a> {
    /// Creates a new Annotation.
    #[must_use]
    pub fn new(dictionary: std::sync::Arc<BTreeMap<Vec<u8>, Object>>, reference: Reference, resolver: &'a dyn Resolver) -> Self {
        Self { dictionary, reference, resolver }
    }

    /// Returns the subtype of the annotation (e.g., /Link, /Widget).
    #[must_use] pub fn subtype(&self) -> Option<Vec<u8>> {
        if let Some(Object::Name(n)) = self.dictionary.get(b"Subtype".as_slice()) {
            Some(n.to_vec())
        } else {
            None
        }
    }

    /// Returns the rectangle defining the annotation boundaries.
    #[must_use] pub fn rect(&self) -> Option<Vec<f64>> {
        if let Some(Object::Array(a)) = self.dictionary.get(b"Rect".as_slice()) {
            let mut res = Vec::with_capacity(4);
            for obj in a.iter() {
                if let Object::Integer(i) = obj { res.push(*i as f64); }
                else if let Object::Real(f) = obj { res.push(*f); }
            }
            Some(res)
        } else {
            None
        }
    }

    /// Returns the destination associated with this annotation.
    #[must_use] pub fn destination(&self) -> Option<Destination> {
        self.dictionary.get(b"Dest".as_ref()).and_then(|obj| {
            Destination::from_obj(obj.clone())
        })
    }

    /// Returns the action associated with this annotation.
    #[must_use] pub fn action(&self) -> Option<Action> {
        self.dictionary.get(b"A".as_ref()).and_then(|obj| {
            match obj {
                Object::Dictionary(d) => {
                    let subtype = d.get(b"S".as_ref()).and_then(|o| o.as_str()).map(|s| s.to_vec())?;
                    Some(Action::new(subtype, std::sync::Arc::clone(d)))
                }
                Object::Reference(r) => {
                    if let Ok(Object::Dictionary(d)) = self.resolver.resolve(&r) {
                        let subtype = d.get(b"S".as_ref()).and_then(|o| o.as_str()).map(|s| s.to_vec())?;
                        Some(Action::new(subtype, d))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        })
    }
}

/// Link Annotation (Clause 12.5.6.5).
pub struct LinkAnnotation<'a> {
    /// The underlying annotation.
    pub annotation: Annotation<'a>,
}

impl<'a> LinkAnnotation<'a> {
    /// Creates a new `LinkAnnotation`.
    #[must_use]
    pub const fn new(annotation: Annotation<'a>) -> Self {
        Self { annotation }
    }

    /// Returns the destination associated with this link.
    #[must_use] pub fn destination(&self) -> Option<Destination> {
        self.annotation.destination()
    }

    /// Returns the action associated with this link.
    #[must_use] pub fn action(&self) -> Option<Action> {
        self.annotation.action()
    }
}
