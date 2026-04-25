//! PDF Stream Decoding Filters (ISO 32000-2:2020 Clause 7.4)

use crate::PdfResult;
use crate::arena::PdfArena;
use crate::error::PdfError;
use crate::object::Object;
use bytes::Bytes;

pub mod flate;
pub mod predictor;

/// A trait for decoding PDF stream filters.
pub trait DecodingFilter {
    /// Decodes the input bytes according to the filter logic.
    fn decode(&self, input: &[u8], params: Option<&Object>, arena: &PdfArena) -> PdfResult<Bytes>;
}

/// Dispatches decoding requests to the appropriate filter implementation.
pub fn decode_stream(
    filter_name: &str,
    input: &[u8],
    params: Option<&Object>,
    arena: &PdfArena,
) -> PdfResult<Bytes> {
    match filter_name {
        "FlateDecode" | "Fl" => {
            let decoder = flate::FlateFilter;
            decoder.decode(input, params, arena)
        }
        _ => Err(PdfError::Filter(format!("Unsupported filter: {}", filter_name))),
    }
}

/// Orchestrates multi-filter decoding for a stream dictionary.
pub fn process_arena_filters(
    data: &[u8],
    dict: &std::collections::BTreeMap<crate::handle::Handle<crate::object::PdfName>, Object>,
    arena: &PdfArena,
) -> PdfResult<Bytes> {
    let filter_key = arena.intern_name(crate::object::PdfName::new("Filter"));
    let params_key = arena.intern_name(crate::object::PdfName::new("DecodeParms"));

    let mut current_data = Bytes::copy_from_slice(data);

    if let Some(filter_obj) = dict.get(&filter_key) {
        let filter_obj = filter_obj.resolve(arena);
        match filter_obj {
            Object::Name(h) => {
                let name = arena
                    .get_name(h)
                    .ok_or_else(|| PdfError::Other("Filter name not found".into()))?;
                let params = dict.get(&params_key).map(|o| o.resolve(arena));
                current_data = decode_stream(name.as_str(), &current_data, params.as_ref(), arena)?;
            }
            Object::Array(h) => {
                let filters = arena
                    .get_array(h)
                    .ok_or_else(|| PdfError::Other("Filter array not found".into()))?;
                let params_arr = dict.get(&params_key).and_then(|o| {
                    if let Object::Array(ah) = o.resolve(arena) {
                        arena.get_array(ah)
                    } else {
                        None
                    }
                });

                for (i, f_obj) in filters.iter().enumerate() {
                    if let Object::Name(fh) = f_obj.resolve(arena) {
                        let name = arena
                            .get_name(fh)
                            .ok_or_else(|| PdfError::Other("Filter name not found".into()))?;
                        let p = params_arr.as_ref().and_then(|a| a.get(i));
                        current_data = decode_stream(name.as_str(), &current_data, p, arena)?;
                    }
                }
            }
            _ => {}
        }
    }

    Ok(current_data)
}
