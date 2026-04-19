use std::io::Read;
use flate2::read::ZlibDecoder;
use crate::error::{PdfResult, PdfError};

const MAX_DECODE_SIZE: usize = 256 * 1024 * 1024; // 256MB (RR-15 Rule 12)

/// ISO 32000-2:2020 Clause 7.4.4 - FlateDecode Filter
pub fn decode_flate(data: &[u8]) -> PdfResult<Vec<u8>> {
    eprintln!("DEBUG: FlateDecode input starts with: {:02x?}", &data[..std::cmp::min(data.len(), 8)]);
    // 1. Try Zlib decompression (RFC 1950) first
    let mut decoder = ZlibDecoder::new(data);
    let mut decoded = Vec::new();
    
    match decoder.by_ref().take(MAX_DECODE_SIZE as u64).read_to_end(&mut decoded) {
        Ok(_) => {
            if decoded.len() == MAX_DECODE_SIZE {
                return Err(PdfError::Other("Maximum decode size exceeded".into()));
            }
            Ok(decoded)
        }
        Err(_) => {
            // 2. Fallback to Raw Deflate (RFC 1951)
            use flate2::read::DeflateDecoder;
            let mut raw_decoder = DeflateDecoder::new(data);
            let mut raw_decoded = Vec::new();
            
            raw_decoder.by_ref().take(MAX_DECODE_SIZE as u64).read_to_end(&mut raw_decoded)
                .map_err(|e| PdfError::Other(format!("FlateDecode final error: {}", e)))?;
                
            if raw_decoded.len() == MAX_DECODE_SIZE {
                return Err(PdfError::Other("Maximum decode size exceeded".into()));
            }
            Ok(raw_decoded)
        }
    }
}
