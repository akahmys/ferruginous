use super::{object::Dictionary, Object, Xref};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct Document {
    pub version: String,
    pub xref: Xref,
    pub objects: BTreeMap<u32, Object>,
    pub trailer: Dictionary,
    pub max_id: u32,
    pub repair_log: Vec<String>,
}

impl Document {
    pub fn new() -> Self {
        Document {
            version: "1.7".to_string(),
            xref: Xref::new(),
            objects: BTreeMap::new(),
            trailer: Dictionary::new(),
            max_id: 0,
            repair_log: Vec::new(),
        }
    }

    /// Resolves an object by ID.
    pub fn get_object(&self, id: u32) -> Option<&Object> {
        self.objects.get(&id)
    }

    /// Recursively resolve indirect references if the object is a Reference.
    pub fn resolve<'a>(&'a self, object: &'a Object) -> &'a Object {
        match object {
            Object::Reference(ref_id) => {
                if let Some(resolved) = self.get_object(ref_id.id) {
                    self.resolve(resolved)
                } else {
                    object // Return reference if resolution fails (should not happen in valid PDF)
                }
            }
            _ => object,
        }
    }

    /// Get a value from a dictionary, resolving references.
    pub fn get_from_dict<'a>(&'a self, dict: &'a Dictionary, key: &[u8]) -> Option<&'a Object> {
        dict.get(key).map(|obj| self.resolve(obj))
    }

    /// Recursively normalize strings in the document (Shift-JIS to UTF-8).
    pub fn apply_normalization(&mut self) {
        self.repair_log
            .push("Starting character normalization (Shift-JIS -> UTF-8)...".to_string());
        // Normalize trailer
        Self::normalize_dict(&mut self.trailer);

        // Normalize all objects
        let keys: Vec<u32> = self.objects.keys().cloned().collect();
        for id in keys {
            if let Some(obj) = self.objects.get_mut(&id) {
                Self::normalize_object(obj);
            }
        }
    }

    fn normalize_object(obj: &mut Object) {
        match obj {
            Object::String(bytes, _format) => {
                // Heuristic: If it contains high bits and isn't UTF-8/UTF-16, try SJIS
                // For this bridge, we assume legacy Japanese files use SJIS for metadata
                if bytes.iter().any(|&b| b > 127) {
                    let s = crate::normalize_sjis(bytes);
                    *bytes = bytes::Bytes::from(s);
                }
            }
            Object::Array(arr) => {
                for item in arr.iter_mut() {
                    Self::normalize_object(item);
                }
            }
            Object::Dictionary(dict) => {
                Self::normalize_dict(dict);
            }
            Object::Stream(stream) => {
                Self::normalize_dict(&mut stream.dict);
            }
            _ => {}
        }
    }

    fn normalize_dict(dict: &mut Dictionary) {
        for (_key, val) in dict.iter_mut() {
            Self::normalize_object(val);
        }
    }
}
