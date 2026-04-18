use std::io::Read;
use flate2::read::ZlibDecoder;
use crate::error::{PdfResult, PdfError};

const MAX_DECODE_SIZE: usize = 256 * 1024 * 1024; // 256MB (RR-15 Rule 12)

/// ISO 32000-2:2020 Clause 7.4.4 - FlateDecode Filter
pub fn decode_flate(data: &[u8]) -> PdfResult<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(data);
    let mut decoded = Vec::new();
    
    // Use a limited reader to prevent OOM attacks
    decoder.by_ref().take(MAX_DECODE_SIZE as u64).read_to_end(&mut decoded)
        .map_err(|e| PdfError::Other(format!("FlateDecode error: {}", e)))?;
        
    if decoded.len() == MAX_DECODE_SIZE {
        return Err(PdfError::Other("Maximum decode size exceeded".into()));
    }
    
    Ok(decoded)
}
