use crate::error::{PdfResult, PdfError};

pub mod flate;
pub mod dct;

pub use flate::decode_flate;
pub use dct::decode_dct;

/// Dispatches decoding to the appropriate filter based on the filter name.
/// (ISO 32000-2:2020 Clause 7.4)
pub fn decode_stream(filter: &str, data: &[u8]) -> PdfResult<Vec<u8>> {
    match filter {
        "FlateDecode" | "Fl" => decode_flate(data),
        "DCTDecode" | "DCT" => decode_dct(data),
        "CCITTFaxDecode" | "CCF" => Err(PdfError::Other("CCITTFaxDecode not yet implemented".into())),
        _ => Err(PdfError::Other(format!("Unsupported filter: {}", filter))),
    }
}

/// Decodes data using filters specified in a stream dictionary.
pub fn decode_stream_from_dict(
    dict: &std::collections::BTreeMap<crate::PdfName, crate::Object>,
    mut data: Vec<u8>
) -> PdfResult<Vec<u8>> {
    if let Some(filter) = dict.get(&crate::PdfName::new(b"Filter")) {
        match filter {
            crate::Object::Name(n) => {
                data = decode_stream(n.as_str(), &data)?;
            }
            crate::Object::Array(arr) => {
                for obj in arr.iter() {
                    if let crate::Object::Name(n) = obj {
                        data = decode_stream(n.as_str(), &data)?;
                    }
                }
            }
            _ => return Err(PdfError::Other("Invalid Filter type".into())),
        }
    }
    Ok(data)
}
