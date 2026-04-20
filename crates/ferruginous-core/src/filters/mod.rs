use crate::error::{PdfError, PdfResult};

pub mod dct;
pub mod flate;
pub mod predict;
pub mod run_length;

pub use dct::decode_dct;
pub use flate::decode_flate;
pub use predict::decode_predictor;

/// Dispatches decoding to the appropriate filter based on the filter name.
/// (ISO 32000-2:2020 Clause 7.4)
pub fn decode_stream(filter: &str, data: &[u8]) -> PdfResult<Vec<u8>> {
    match filter {
        "FlateDecode" | "Fl" => decode_flate(data),
        "DCTDecode" | "DCT" => decode_dct(data),
        "RunLengthDecode" | "RL" => run_length::decode_run_length(data),
        "CCITTFaxDecode" | "CCF" => {
            Err(PdfError::Other("CCITTFaxDecode not yet implemented".into()))
        }
        _ => Err(PdfError::Other(format!("Unsupported filter: {}", filter))),
    }
}

/// Decodes data using filters specified in a stream dictionary.
pub fn decode_stream_from_dict(
    dict: &std::collections::BTreeMap<crate::PdfName, crate::Object>,
    mut data: Vec<u8>,
) -> PdfResult<bytes::Bytes> {
    let filters = match dict.get(&crate::PdfName::new(b"Filter")) {
        Some(crate::Object::Name(n)) => vec![n.as_str().to_string()],
        Some(crate::Object::Array(arr)) => {
            arr.iter().filter_map(|o| o.as_name().map(|n| n.as_str().to_string())).collect()
        }
        _ => vec![],
    };

    let parms = match dict
        .get(&crate::PdfName::new(b"DecodeParms"))
        .or_else(|| dict.get(&crate::PdfName::new(b"DP")))
    {
        Some(crate::Object::Dictionary(d)) => vec![Some(d.clone())],
        Some(crate::Object::Array(arr)) => arr.iter().map(|o| o.as_dict_arc()).collect(),
        None => vec![],
        _ => vec![],
    };

    for (i, filter) in filters.iter().enumerate() {
        data = decode_stream(filter, &data)?;

        // Predictor handling for this filter stage
        if let Some(parm_dict) = parms.get(i).and_then(|p| p.as_ref()) {
            let predictor =
                parm_dict.get(&"Predictor".into()).and_then(|o| o.as_i64()).unwrap_or(1) as i32;
            if predictor > 1 {
                let colors =
                    parm_dict.get(&"Colors".into()).and_then(|o| o.as_i64()).unwrap_or(1) as usize;
                let bits_per_component =
                    parm_dict.get(&"BitsPerComponent".into()).and_then(|o| o.as_i64()).unwrap_or(8)
                        as usize;
                let columns =
                    parm_dict.get(&"Columns".into()).and_then(|o| o.as_i64()).unwrap_or(1) as usize;

                data = crate::filters::predict::decode_predictor(
                    predictor,
                    colors,
                    bits_per_component,
                    columns,
                    &data,
                )?;
            }
        }
    }

    Ok(bytes::Bytes::from(data))
}
