//! Resource dictionary and `XObject` management.
//!
//! (ISO 32000-2:2020 Clause 7.7.3.3)

use crate::core::{Object, Resolver};
use std::collections::BTreeMap;

/// Represents a raw `XObject` stream and its metadata.
pub struct RawXObject {
    /// The `XObject`'s dictionary.
    pub dictionary: std::sync::Arc<BTreeMap<Vec<u8>, Object>>,
    /// The decoded or raw stream data.
    pub data: std::sync::Arc<Vec<u8>>,
}

/// Represents the Resources dictionary of a PDF page or node.
/// (ISO 32000-2:2020 Clause 7.7.3.3)
pub struct Resources<'a> {
    /// The resources dictionary containing sub-dictionaries for fonts, `XObjects`, etc.
    pub dictionary: std::sync::Arc<BTreeMap<Vec<u8>, Object>>,
    /// The resolver for indirect objects within the resources.
    pub resolver: &'a dyn Resolver,
}

impl<'a> Resources<'a> {
    /// Creates a new Resources instance.
    pub fn new(dictionary: std::sync::Arc<BTreeMap<Vec<u8>, Object>>, resolver: &'a dyn Resolver) -> Self {
        debug_assert!(!dictionary.is_empty());
        Self { dictionary, resolver }
    }

    /// Retrieves a sub-dictionary from the resources (e.g., Font, `XObject`, `ColorSpace`).
    #[must_use] pub fn get_sub_dict(&self, key: &[u8]) -> Option<std::sync::Arc<BTreeMap<Vec<u8>, Object>>> {
        let obj = self.dictionary.get(key)?;
        assert!(!key.is_empty());
        
        match obj {
            Object::Dictionary(dict) => Some(std::sync::Arc::clone(dict)),
            Object::Reference(r) => {
                // If it's a reference, resolve it
                if let Ok(Object::Dictionary(dict)) = self.resolver.resolve(r) {
                    Some(dict)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Returns the Font resources dictionary (Clause 9.2).
    #[must_use] pub fn get_fonts(&self) -> Option<std::sync::Arc<BTreeMap<Vec<u8>, Object>>> {
        self.get_sub_dict(b"Font")
    }

    /// Retrieves a specific font by its resource name.
    #[must_use] pub fn get_font(&self, name: &[u8]) -> Option<std::sync::Arc<BTreeMap<Vec<u8>, Object>>> {
        let fonts = self.get_fonts()?;
        let obj = fonts.get(name)?;
        
        match obj {
            Object::Dictionary(dict) => Some(std::sync::Arc::clone(dict)),
            Object::Reference(r) => {
                if let Ok(Object::Dictionary(dict)) = self.resolver.resolve(r) {
                    Some(dict)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Returns the `XObject` resources dictionary (Clause 8.8).
    #[must_use] pub fn get_xobjects(&self) -> Option<std::sync::Arc<BTreeMap<Vec<u8>, Object>>> {
        self.get_sub_dict(b"XObject")
    }

    /// Retrieves a specific `XObject` by its resource name.
    #[must_use] pub fn get_xobject(&self, name: &[u8]) -> Option<RawXObject> {
        let xobjects = self.get_xobjects()?;
        let obj = xobjects.get(name)?;
        
        let actual_obj = match obj {
            Object::Reference(r) => self.resolver.resolve(r).ok()?,
            _ => obj.clone(),
        };

        if let Object::Stream(dict, data) = actual_obj {
            Some(RawXObject { dictionary: dict, data })
        } else {
            None
        }
    }

    /// Returns the External Graphics State resources dictionary (Clause 8.4.5).
    #[must_use] pub fn get_ext_gstates(&self) -> Option<std::sync::Arc<BTreeMap<Vec<u8>, Object>>> {
        self.get_sub_dict(b"ExtGState")
    }

    /// Returns the Shading resources dictionary (Clause 8.7.4.3).
    #[must_use] pub fn get_shadings(&self) -> Option<std::sync::Arc<BTreeMap<Vec<u8>, Object>>> {
        self.get_sub_dict(b"Shading")
    }

    /// Returns the Properties resources dictionary (Clause 14.11.4).
    #[must_use] pub fn get_properties_dict(&self) -> Option<std::sync::Arc<BTreeMap<Vec<u8>, Object>>> {
        self.get_sub_dict(b"Properties")
    }

    /// Retrieves a specific property by its resource name.
    #[must_use] pub fn get_properties(&self, name: &[u8]) -> Option<Object> {
        let props = self.get_properties_dict()?;
        props.get(name).cloned()
    }

    /// Retrieves a specific Shading object by its resource name.
    #[must_use] pub fn get_shading(&self, name: &[u8]) -> Option<std::sync::Arc<BTreeMap<Vec<u8>, Object>>> {
        let shadings = self.get_shadings()?;
        let obj = shadings.get(name)?;
        
        match obj {
            Object::Dictionary(dict) => Some(std::sync::Arc::clone(dict)),
            Object::Reference(r) => {
                if let Ok(Object::Dictionary(dict)) = self.resolver.resolve(r) {
                    Some(dict)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Returns the Pattern resources dictionary (Clause 8.7).
    #[must_use] pub fn get_patterns(&self) -> Option<std::sync::Arc<BTreeMap<Vec<u8>, Object>>> {
        self.get_sub_dict(b"Pattern")
    }

    /// Retrieves a specific Pattern object by its resource name.
    #[must_use] pub fn get_pattern(&self, name: &[u8]) -> Option<Object> {
        let patterns = self.get_patterns()?;
        let obj = patterns.get(name)?;
        
        match obj {
            Object::Reference(r) => self.resolver.resolve(r).ok(),
            _ => Some(obj.clone()),
        }
    }

    /// Returns the ColorSpace resources dictionary (Clause 8.6.2).
    #[must_use] pub fn get_color_spaces(&self) -> Option<std::sync::Arc<BTreeMap<Vec<u8>, Object>>> {
        self.get_sub_dict(b"ColorSpace")
    }

    /// Retrieves a specific ColorSpace object by its resource name.
    #[must_use] pub fn get_color_space(&self, name: &[u8]) -> Option<Object> {
        let color_spaces = self.get_color_spaces()?;
        let obj = color_spaces.get(name)?;
        
        match obj {
            Object::Reference(r) => self.resolver.resolve(r).ok(),
            _ => Some(obj.clone()),
        }
    }
}
