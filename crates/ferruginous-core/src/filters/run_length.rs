use crate::error::{PdfResult, PdfError};

/// ISO 32000-2:2020 Clause 7.4.5 - RunLengthDecode Filter
///
/// The RunLengthDecode filter decodes data that has been encoded in a 
/// simple byte-oriented run-length compression algorithm.
pub fn decode_run_length(data: &[u8]) -> PdfResult<Vec<u8>> {
    let mut out = Vec::new();
    let mut i = 0;
    
    while i < data.len() {
        let n = data[i];
        i += 1;
        
        if n == 128 {
            // EOD
            break;
        } else if n <= 127 {
            // Literal run: copy n + 1 bytes
            let count = (n as usize) + 1;
            if i + count > data.len() {
                return Err(PdfError::Other("Unexpected end of data in RunLengthDecode (literal)".into()));
            }
            out.extend_from_slice(&data[i..i + count]);
            i += count;
        } else {
            // Fill run: repeat next byte 257 - n times
            let count = 257 - (n as usize);
            if i >= data.len() {
                return Err(PdfError::Other("Unexpected end of data in RunLengthDecode (fill)".into()));
            }
            let byte = data[i];
            i += 1;
            out.resize(out.len() + count, byte);
        }
    }
    
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_length_decode() {
        // Example from some PDF references:
        // [0x02, 'a', 'b', 'c', 0xFF, 'd', 0x80]
        // 0x02 -> copy 3: "abc"
        // 0xFF -> repeat 'd' 2 times: "dd"
        // 0x80 -> EOD
        let input = [0x02, b'a', b'b', b'c', 0xFF, b'd', 0x80];
        let expected = b"abcdd";
        let result = decode_run_length(&input).unwrap();
        assert_eq!(result, expected);
    }
}
