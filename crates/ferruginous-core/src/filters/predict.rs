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
    let row_size = (columns * colors * bits_per_component).div_ceil(8);
    let bytes_per_pixel = (colors * bits_per_component).div_ceil(8);
    let encoded_row_size = row_size + 1;
    
    if !data.len().is_multiple_of(encoded_row_size) {
        return Err(PdfError::Other("Invalid data length for PNG predictor".into()));
    }
    
    let num_rows = data.len() / encoded_row_size;
    let mut decoded = Vec::with_capacity(num_rows * row_size);
    let mut prev_row: Vec<u8> = vec![0; row_size];
    
    for row_idx in 0..num_rows {
        let start = row_idx * encoded_row_size;
        let predictor_type = data[start];
        let row_data = &data[start + 1..start + encoded_row_size];
        
        let mut current_row = vec![0; row_size];
        apply_predictor_to_row(predictor_type, row_data, &prev_row, &mut current_row, bytes_per_pixel)?;
        
        decoded.extend_from_slice(&current_row);
        prev_row = current_row;
    }
    Ok(decoded)
}

fn apply_predictor_to_row(
    p_type: u8,
    row_data: &[u8],
    prev_row: &[u8],
    current_row: &mut [u8],
    bpp: usize,
) -> PdfResult<()> {
    let row_size = current_row.len();
    match p_type {
        0 => current_row.copy_from_slice(row_data),
        1 => for i in 0..row_size {
            let left = if i >= bpp { current_row[i - bpp] } else { 0 };
            current_row[i] = row_data[i].wrapping_add(left);
        },
        2 => for i in 0..row_size {
            current_row[i] = row_data[i].wrapping_add(prev_row[i]);
        },
        3 => for i in 0..row_size {
            let left = if i >= bpp { current_row[i - bpp] } else { 0 };
            let avg = ((left as u16 + prev_row[i] as u16) / 2) as u8;
            current_row[i] = row_data[i].wrapping_add(avg);
        },
        4 => for i in 0..row_size {
            let left = if i >= bpp { current_row[i - bpp] } else { 0 };
            let upper_left = if i >= bpp { prev_row[i - bpp] } else { 0 };
            let p = paeth_predictor(left, prev_row[i], upper_left);
            current_row[i] = row_data[i].wrapping_add(p);
        },
        _ => return Err(PdfError::Other(format!("Unknown PNG predictor: {}", p_type))),
    }
    Ok(())
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
