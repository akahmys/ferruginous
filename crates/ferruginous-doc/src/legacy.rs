#[cfg(feature = "legacy-bridge")]
use ferruginous_core::{Object, PdfResult, Reference, Resolver, PdfName};
#[cfg(feature = "legacy-bridge")]
use ferruginous_bridge_legacy::lopdf;
#[cfg(feature = "legacy-bridge")]
use std::sync::Arc;
#[cfg(feature = "legacy-bridge")]
use std::collections::BTreeMap;

#[cfg(feature = "legacy-bridge")]
pub struct LopdfResolver {
    doc: lopdf::Document,
}

#[cfg(feature = "legacy-bridge")]
impl LopdfResolver {
    pub fn new(data: &[u8]) -> PdfResult<Self> {
        let doc = lopdf::Document::load_mem(data)
            .map_err(|e| ferruginous_core::error::PdfError::Other(format!("lopdf error: {}", e)))?;
        Ok(Self { doc })
    }
}

#[cfg(feature = "legacy-bridge")]
impl Resolver for LopdfResolver {
    fn resolve(&self, reference: &Reference) -> PdfResult<Object> {
        let lopdf_ref = lopdf::ObjectId::new(reference.id, reference.generation);
        let obj = self.doc.get_object(lopdf_ref)
            .map_err(|_| ferruginous_core::error::PdfError::ObjectNotFound(*reference))?;
        
        Ok(convert_object(obj))
    }
}

#[cfg(feature = "legacy-bridge")]
fn convert_object(obj: &lopdf::Object) -> Object {
    match obj {
        lopdf::Object::Boolean(b) => Object::Boolean(*b),
        lopdf::Object::Integer(i) => Object::Integer(*i),
        lopdf::Object::Real(f) => Object::Real(*f),
        lopdf::Object::String(s, _) => Object::String(bytes::Bytes::copy_from_slice(s)),
        lopdf::Object::Name(n) => Object::Name(PdfName::new(n)),
        lopdf::Object::Array(arr) => {
            let converted = arr.iter().map(convert_object).collect();
            Object::Array(Arc::new(converted))
        }
        lopdf::Object::Dictionary(dict) => {
            let mut converted = BTreeMap::new();
            for (k, v) in dict {
                converted.insert(PdfName::new(k), convert_object(v));
            }
            Object::Dictionary(Arc::new(converted))
        }
        lopdf::Object::Stream(s) => {
            let mut converted_dict = BTreeMap::new();
            for (k, v) in &s.dict {
                converted_dict.insert(PdfName::new(k), convert_object(v));
            }
            Object::Stream(Arc::new(converted_dict), bytes::Bytes::copy_from_slice(&s.content))
        }
        lopdf::Object::Null => Object::Null,
        lopdf::Object::Reference(r) => Object::Reference(Reference::new(r.0, r.1)),
    }
}
