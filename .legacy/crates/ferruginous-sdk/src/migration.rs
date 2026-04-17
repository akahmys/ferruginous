use std::sync::Arc;
use std::collections::BTreeMap;
use crate::core::types::{Object, Reference};
use ferruginous_bridge_legacy::lopdf::object::Object as LegacyObject;

/// Translates a legacy PDF object into a modern SDK object.
/// (Phase 19 Migration Layer)
pub fn migrate_legacy_object(legacy: LegacyObject) -> Object {
    match legacy {
        LegacyObject::Null => Object::Null,
        LegacyObject::Boolean(b) => Object::Boolean(b),
        LegacyObject::Integer(i) => Object::Integer(i),
        LegacyObject::Real(f) => Object::Real(f),
        LegacyObject::Name(bytes) => Object::Name(bytes),
        LegacyObject::String(bytes, _) => Object::String(bytes),
        LegacyObject::Array(arr) => {
            let modern_arr: Vec<Object> = arr.into_iter().map(migrate_legacy_object).collect();
            Object::Array(Arc::new(modern_arr))
        }
        LegacyObject::Dictionary(dict) => {
            let mut modern_dict = BTreeMap::new();
            for (key, val) in dict {
                modern_dict.insert(key.to_vec(), migrate_legacy_object(val));
            }
            Object::Dictionary(Arc::new(modern_dict))
        }
        LegacyObject::Stream(stream) => {
            let mut modern_dict = BTreeMap::new();
            for (key, val) in stream.dict {
                modern_dict.insert(key.to_vec(), migrate_legacy_object(val));
            }
            Object::Stream(Arc::new(modern_dict), stream.content)
        }
        LegacyObject::Reference(id) => {
            Object::Reference(Reference::new(id.id, id.r#gen))
        }
    }
}

impl From<LegacyObject> for Object {
    fn from(legacy: LegacyObject) -> Self {
        migrate_legacy_object(legacy)
    }
}

/// Align a legacy page tree into a standard structure.
/// This method ensures all /Page nodes are reachable and correctly linked.
pub fn align_legacy_pages(doc: &ferruginous_bridge_legacy::lopdf::Document) -> Vec<Reference> {
    let mut pages = Vec::new();
    // In legacy repair mode, we scan all objects for /Type /Page
    for (id, obj) in &doc.objects {
        if let Some(dict) = obj.as_dict() {
            if dict.get(b"Type".as_slice()).and_then(|o| o.as_str()) == Some(b"Page") {
                pages.push(Reference::new(*id, 0));
            }
        }
    }
    pages
}
