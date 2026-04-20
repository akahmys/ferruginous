use crate::error::{PdfError, PdfResult};
use flate2::read::ZlibDecoder;
use std::io::Read;

const MAX_DECODE_SIZE: usize = 256 * 1024 * 1024; // 256MB (RR-15 Rule 12)

/// ISO 32000-2:2020 Clause 7.4.4 - FlateDecode Filter
pub fn decode_flate(data: &[u8]) -> PdfResult<Vec<u8>> {
    if data.is_empty() {
        return Ok(Vec::new());
    }

    // 1. Try Zlib decompression (RFC 1950) first
    let mut decoder = ZlibDecoder::new(data);
    let mut decoded = Vec::new();

    match decoder.by_ref().take(MAX_DECODE_SIZE as u64).read_to_end(&mut decoded) {
        Ok(_) => {
            if decoded.len() == MAX_DECODE_SIZE {
                return Err(PdfError::Other("Maximum decode size exceeded (FlateDecode)".into()));
            }
            Ok(decoded)
        }
        Err(e) => {
            // 2. Fallback to Raw Deflate (RFC 1951)
            // Some PDF producers omit the Zlib header despite specified in the standard.
            // We log this as it might indicate a non-compliant or corrupted stream.
            eprintln!("WARNING: FlateDecode Zlib error: {}. Attempting Raw Deflate fallback.", e);

            use flate2::read::DeflateDecoder;
            let mut raw_decoder = DeflateDecoder::new(data);
            let mut raw_decoded = Vec::new();

            match raw_decoder.by_ref().take(MAX_DECODE_SIZE as u64).read_to_end(&mut raw_decoded) {
                Ok(_) => {
                    if raw_decoded.len() == MAX_DECODE_SIZE {
                        return Err(PdfError::Other(
                            "Maximum decode size exceeded (Raw Deflate)".into(),
                        ));
                    }
                    Ok(raw_decoded)
                }
                Err(raw_e) => Err(PdfError::Other(format!(
                    "FlateDecode failed: Zlib error ({}), Raw Deflate error ({})",
                    e, raw_e
                ))),
            }
        }
    }
}
