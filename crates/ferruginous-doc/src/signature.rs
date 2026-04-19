use ferruginous_core::{Object, PdfResult, PdfError};
use bytes::Bytes;
use std::collections::BTreeMap;
use ferruginous_core::PdfName;

/// Represents a PDF digital signature (/Sig dictionary).
/// ISO 32000-2:2020 Clause 12.8
#[derive(Debug, Clone)]
pub struct DocMdp {
    pub p: i32,
}

#[derive(Debug, Clone)]
pub struct FieldMdp {
    pub action: String,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Signature {
    pub obj_id: u32,
    pub sub_filter: String,
    pub byte_range: Vec<usize>,
    pub contents: Vec<u8>,
    pub name: Option<String>,
    pub date: Option<String>,
    pub reason: Option<String>,
    pub doc_mdp: Option<DocMdp>,
    pub field_mdp: Vec<FieldMdp>,
}

impl Signature {
    pub fn from_object(obj_id: u32, dict: &BTreeMap<PdfName, Object>) -> PdfResult<Self> {
        let sub_filter = dict.get(&"SubFilter".into())
            .and_then(|o| o.as_name())
            .map(|n| n.as_str().to_string())
            .ok_or_else(|| PdfError::Other("Missing /SubFilter in signature".into()))?;

        let byte_range = dict.get(&"ByteRange".into())
            .and_then(|o| o.as_array())
            .ok_or_else(|| PdfError::Other("Missing /ByteRange in signature".into()))?
            .iter()
            .map(|o| o.as_i64().map(|i| i as usize))
            .collect::<Option<Vec<usize>>>()
            .ok_or_else(|| PdfError::Other("Invalid /ByteRange values".into()))?;

        let contents = dict.get(&"Contents".into())
            .and_then(|o| match o {
                Object::String(b) => Some(b.as_ref().to_vec()),
                _ => None
            })
            .ok_or_else(|| PdfError::Other("Missing /Contents in signature".into()))?;

        let name = dict.get(&"Name".into())
            .and_then(|o| o.as_string())
            .map(decode_pdf_string);

        let date = dict.get(&"M".into())
            .and_then(|o| o.as_string())
            .map(decode_pdf_string);

        let reason = dict.get(&"Reason".into())
            .and_then(|o| o.as_string())
            .map(decode_pdf_string);

        // MDP Extraction
        let mut doc_mdp = None;
        let mut field_mdp = Vec::new();

        if let Some(refs) = dict.get(&"Reference".into()).and_then(|o| o.as_array()) {
            for ref_obj in refs.iter() {
                if let Some(ref_dict) = ref_obj.as_dict() {
                    let method = ref_dict.get(&"TransformMethod".into())
                        .and_then(|o| o.as_name())
                        .map(|n| n.as_str())
                        .unwrap_or("");
                    
                    let params = ref_dict.get(&"TransformParams".into()).and_then(|o| o.as_dict());

                    match method {
                        "DocMDP" => {
                            if let Some(p) = params.and_then(|d| d.get(&"P".into())).and_then(|o| o.as_i64()) {
                                doc_mdp = Some(DocMdp { p: p as i32 });
                            }
                        }
                        "FieldMDP" => {
                            if let Some(p) = params {
                                let action = p.get(&"Action".into())
                                    .and_then(|o| o.as_name())
                                    .map(|n| n.as_str().to_string())
                                    .unwrap_or_else(|| "All".to_string());
                                let fields = p.get(&"Fields".into())
                                    .and_then(|o| o.as_array())
                                    .map(|a| a.iter().filter_map(|o| o.as_string().map(decode_pdf_string)).collect())
                                    .unwrap_or_default();
                                field_mdp.push(FieldMdp { action, fields });
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(Self {
            obj_id,
            sub_filter,
            byte_range,
            contents,
            name,
            date,
            reason,
            doc_mdp,
            field_mdp,
        })
    }

    /// Extracts the actual bytes covered by the signature according to the ByteRange.
    pub fn extract_signed_data(&self, full_data: &Bytes) -> PdfResult<Vec<u8>> {
        if !self.byte_range.len().is_multiple_of(2) {
            return Err(PdfError::Other("Malformed ByteRange: length is not even".into()));
        }

        let mut signed_data = Vec::new();
        for chunk in self.byte_range.chunks_exact(2) {
            let offset = chunk[0];
            let length = chunk[1];
            if offset + length > full_data.len() {
                return Err(PdfError::Other("ByteRange exceeds document length".into()));
            }
            signed_data.extend_from_slice(&full_data[offset..offset + length]);
        }
        Ok(signed_data)
    }
}

/// Decodes a PDF string, handling UTF-16BE with BOM correctly.
fn decode_pdf_string(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xFE, 0xFF]) {
        // UTF-16BE
        let words: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&words)
    } else {
        // Fallback to UTF-8 or PDFDocEncoding (simplified as UTF-8 lossy here)
        String::from_utf8_lossy(bytes).into_owned()
    }
}
