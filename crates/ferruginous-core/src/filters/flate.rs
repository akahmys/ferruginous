//! FlateDecode Filter (ISO 32000-2:2020 Clause 7.4.4)

use crate::filters::{DecodingFilter, predictor};
use crate::PdfResult;
use crate::error::PdfError;
use crate::object::Object;
use crate::arena::PdfArena;
use bytes::Bytes;
use std::io::Read;
use flate2::read::ZlibDecoder;

pub struct FlateFilter;

impl DecodingFilter for FlateFilter {
    fn decode(&self, input: &[u8], params: Option<&Object>, arena: &PdfArena) -> PdfResult<Bytes> {
        let mut decoder = ZlibDecoder::new(input);
        let mut decoded = Vec::new();
        
        decoder.read_to_end(&mut decoded)
            .map_err(|e| PdfError::Filter(format!("Flate decompression failed: {}", e)))?;
            
        // Apply predictors if present in DecodeParms
        if let Some(p) = params {
            decoded = predictor::apply_predictor(&decoded, p, arena)?;
        }
        
        Ok(Bytes::from(decoded))
    }
}
