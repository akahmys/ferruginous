use std::io::Cursor;
use crate::error::{PdfResult, PdfError};

/// ISO 32000-2:2020 Clause 7.4.8 - DCTDecode Filter
///
/// Decodes data encoded using the JPEG baseline compression method.
pub fn decode_dct(data: &[u8]) -> PdfResult<Vec<u8>> {
    let cursor = Cursor::new(data);
    let mut decoder = jpeg_decoder::Decoder::new(cursor);
    
    let decoded = decoder.decode()
        .map_err(|e| PdfError::Other(format!("DCTDecode error: {}", e)))?;
        
    // Note: The jpeg-decoder returns raw pixels (usually RGB or CMYK).
    // In a PDF context, this is exactly what we want for the rendering stage.
    Ok(decoded)
}
