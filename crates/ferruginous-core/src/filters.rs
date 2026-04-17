use std::io::Read;
use flate2::read::ZlibDecoder;
use crate::error::{PdfResult, PdfError};

/// ISO 32000-2:2020 Clause 7.4.4 - FlateDecode Filter
///
/// Decodes data encoded using the zlib/deflate compression method.
pub fn decode_flate(data: &[u8]) -> PdfResult<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(data);
    let mut decoded = Vec::new();
    decoder.read_to_end(&mut decoded).map_err(|e| PdfError::Other(format!("FlateDecode error: {}", e)))?;
    Ok(decoded)
}

/// Dispatches decoding to the appropriate filter based on the filter name.
pub fn decode_stream(filter: &str, data: &[u8]) -> PdfResult<Vec<u8>> {
    match filter {
        "FlateDecode" | "Fl" => decode_flate(data),
        _ => Err(PdfError::Other(format!("Unsupported filter: {}", filter))),
    }
}
