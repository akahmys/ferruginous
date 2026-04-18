//! ISO 32000-2:2020 Clause 7.4.4.4 - Predictor Functions.
//!
//! Predictors are used to improve the compression ratio of FlateDecode streams
//! by transforming the data into a more compressible form.

use crate::error::{PdfResult, PdfError};

/// Applies predictors to decode stream data.
pub fn decode_predictor(
    predictor: i32,
    colors: usize,
    bits_per_component: usize,
    columns: usize,
    data: &[u8],
) -> PdfResult<Vec<u8>> {
    match predictor {
        1 => Ok(data.to_vec()), // No predictor
        10..=15 => decode_png_predictor(predictor, colors, bits_per_component, columns, data),
        _ => Err(PdfError::Other(format!("Unsupported Predictor: {}", predictor))),
    }
}

fn decode_png_predictor(
    _predictor: i32,
    colors: usize,
    bits_per_component: usize,
    columns: usize,
    data: &[u8],
) -> PdfResult<Vec<u8>> {
    // PNG predictors use one byte before each row to specify the algorithm for that row.
    // (ISO 32000-2 Clause 7.4.4.4, Table 12)
    
    let bytes_per_pixel = (colors * bits_per_component + 7) / 8;
    let row_size = (columns * colors * bits_per_component + 7) / 8;
    let encoded_row_size = row_size + 1; // 1 byte for predictor type
    
    if data.len() % encoded_row_size != 0 {
        return Err(PdfError::Other(format!("Invalid data length for PNG predictor: expected multiple of {}, got {}", encoded_row_size, data.len())));
    }
    
    let num_rows = data.len() / encoded_row_size;
    let mut decoded = Vec::with_capacity(num_rows * row_size);
    let mut prev_row: Vec<u8> = vec![0; row_size];
    
    for row_idx in 0..num_rows {
        let start = row_idx * encoded_row_size;
        let predictor_type = data[start];
        let row_data = &data[start + 1..start + encoded_row_size];
        
        let mut current_row = vec![0; row_size];
        
        match predictor_type {
            0 => { // None
                current_row.copy_from_slice(row_data);
            }
            1 => { // Sub
                for i in 0..row_size {
                    let left = if i >= bytes_per_pixel { current_row[i - bytes_per_pixel] } else { 0 };
                    current_row[i] = row_data[i].wrapping_add(left);
                }
            }
            2 => { // Up
                for i in 0..row_size {
                    let up = prev_row[i];
                    current_row[i] = row_data[i].wrapping_add(up);
                }
            }
            3 => { // Average
                for i in 0..row_size {
                    let left = if i >= bytes_per_pixel { current_row[i - bytes_per_pixel] } else { 0 };
                    let up = prev_row[i];
                    let avg = ((left as u16 + up as u16) / 2) as u8;
                    current_row[i] = row_data[i].wrapping_add(avg);
                }
            }
            4 => { // Paeth
                for i in 0..row_size {
                    let left = if i >= bytes_per_pixel { current_row[i - bytes_per_pixel] } else { 0 };
                    let up = prev_row[i];
                    let upper_left = if i >= bytes_per_pixel { prev_row[i - bytes_per_pixel] } else { 0 };
                    let p = paeth_predictor(left, up, upper_left);
                    current_row[i] = row_data[i].wrapping_add(p);
                }
            }
            _ => return Err(PdfError::Other(format!("Unknown PNG predictor type: {}", predictor_type))),
        }
        
        decoded.extend_from_slice(&current_row);
        prev_row = current_row;
    }
    
    Ok(decoded)
}

fn paeth_predictor(a: u8, b: u8, c: u8) -> u8 {
    let a = a as i16;
    let b = b as i16;
    let c = c as i16;
    let p = a + b - c;
    let pa = (p - a).abs();
    let pb = (p - b).abs();
    let pc = (p - c).abs();
    
    if pa <= pb && pa <= pc {
        a as u8
    } else if pb <= pc {
        b as u8
    } else {
        c as u8
    }
}
