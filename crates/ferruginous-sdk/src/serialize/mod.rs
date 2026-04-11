use crate::core::Object;
use crate::core::error::{PdfError, PdfResult};
use std::io::{Write};

pub mod xref_stream;
pub mod object_stream;

/// Writes an indirect object with its ID and generation number.
/// (Clause 7.3.10 - Indirect Objects)
pub fn write_indirect_object<W: Write>(
    writer: &mut W,
    id: u32,
    generation: u16,
    obj: &Object,
) -> PdfResult<()> {
    writeln!(writer, "{id} {generation} obj").map_err(PdfError::from)?;
    write_object(writer, obj)?;
    writer.write_all(b"\nendobj\n").map_err(PdfError::from)?;
    Ok(())
}

/// Writes a full xref section including all its subsections (Clause 7.5.4).
pub fn write_xref_section<W: Write>(
    writer: &mut W,
    subsections: &[(u32, Vec<crate::xref::XRefEntry>)],
) -> PdfResult<()> {
    writer.write_all(b"xref\n").map_err(PdfError::from)?;
    for (start_id, entries) in subsections {
        writeln!(writer, "{} {}", start_id, entries.len()).map_err(PdfError::from)?;
        for entry in entries {
            entry.write_20byte(writer).map_err(PdfError::from)?;
        }
    }
    Ok(())
}

/// Writes the trailer dictionary and the file trailer (Clause 7.5.5).
pub fn write_trailer<W: Write>(
    writer: &mut W,
    dict: &std::collections::BTreeMap<Vec<u8>, Object>,
    last_xref_offset: u64,
) -> PdfResult<()> {
    writer.write_all(b"trailer\n").map_err(PdfError::from)?;
    write_object(writer, &Object::new_dict(dict.clone()))?;
    writer.write_all(b"\n").map_err(PdfError::from)?;
    
    write!(writer, "startxref\n{last_xref_offset}\n%%EOF\n").map_err(PdfError::from)?;
    Ok(())
}

/// Serializes a PDF object into its physical representation.
/// (Clause 7.3 - General Object Types)
pub fn write_object<W: Write>(writer: &mut W, obj: &Object) -> PdfResult<()> {
    match obj {
        Object::Boolean(b) => {
            writer.write_all(if *b { b"true" } else { b"false" }).map_err(PdfError::from)?;
        }
        Object::Integer(i) => {
            write!(writer, "{i}").map_err(PdfError::from)?;
        }
        Object::Real(f) => {
            write!(writer, "{f:.4}").map_err(PdfError::from)?;
        }
        Object::String(bytes) => {
            write_string(writer, bytes)?;
        }
        Object::Name(bytes) => {
            write_name(writer, bytes)?;
        }
        Object::Array(arr) => {
            write_array(writer, arr)?;
        }
        Object::Dictionary(dict) => {
            write_dictionary(writer, dict)?;
        }
        Object::Stream(dict, data) => {
            write_stream(writer, dict, data)?;
        }
        Object::Null => {
            writer.write_all(b"null").map_err(PdfError::from)?;
        }
        Object::Reference(r) => {
            write!(writer, "{} {} R", r.id, r.generation).map_err(PdfError::from)?;
        }
    }
    Ok(())
}

fn write_array<W: Write>(writer: &mut W, arr: &[Object]) -> PdfResult<()> {
    writer.write_all(b"[").map_err(PdfError::from)?;
    for (i, item) in arr.iter().enumerate() {
        if i > 0 { writer.write_all(b" ").map_err(PdfError::from)?; }
        write_object(writer, item)?;
    }
    writer.write_all(b"]").map_err(PdfError::from)?;
    Ok(())
}

fn write_dictionary<W: Write>(writer: &mut W, dict: &std::collections::BTreeMap<Vec<u8>, Object>) -> PdfResult<()> {
    writer.write_all(b"<< ").map_err(PdfError::from)?;
    for (key, val) in dict {
        writer.write_all(b"/").map_err(PdfError::from)?;
        write_name_content(writer, key)?;
        writer.write_all(b" ").map_err(PdfError::from)?;
        write_object(writer, val)?;
        writer.write_all(b" ").map_err(PdfError::from)?;
    }
    writer.write_all(b">>").map_err(PdfError::from)?;
    Ok(())
}

fn write_stream<W: Write>(writer: &mut W, dict: &std::collections::BTreeMap<Vec<u8>, Object>, data: &[u8]) -> PdfResult<()> {
    let mut stream_dict = dict.clone();
    stream_dict.insert(b"Length".to_vec(), Object::Integer(data.len() as i64));
    write_dictionary(writer, &stream_dict)?;
    writer.write_all(b"\nstream\r\n").map_err(PdfError::from)?;
    writer.write_all(data).map_err(PdfError::from)?;
    writer.write_all(b"\r\nendstream").map_err(PdfError::from)?;
    Ok(())
}

/// Calculates the minimum number of bytes required to store the given unsigned integer.
pub fn required_bytes(val: u64) -> usize {
    if val == 0 { return 1; }
    if val < 0x100 { 1 }
    else if val < 0x10000 { 2 }
    else if val < 0x1000000 { 3 }
    else if val < 0x100000000 { 4 }
    else if val < 0x10000000000 { 5 }
    else if val < 0x1000000000000 { 6 }
    else if val < 0x100000000000000 { 7 }
    else { 8 }
}

/// Writes an unsigned integer to the vector with the specified width (Big-endian).
pub fn write_uint(vec: &mut Vec<u8>, val: u64, width: usize) {
    for i in (0..width).rev() {
        vec.push(((val >> (i * 8)) & 0xFF) as u8);
    }
}

fn write_string<W: Write>(writer: &mut W, bytes: &[u8]) -> PdfResult<()> {
    writer.write_all(b"(").map_err(PdfError::from)?;
    for &b in bytes {
        match b {
            b'(' => writer.write_all(b"\\(").map_err(PdfError::from)?,
            b')' => writer.write_all(b"\\)").map_err(PdfError::from)?,
            b'\\' => writer.write_all(b"\\\\").map_err(PdfError::from)?,
            b'\n' => writer.write_all(b"\\n").map_err(PdfError::from)?,
            b'\r' => writer.write_all(b"\\r").map_err(PdfError::from)?,
            b'\t' => writer.write_all(b"\\t").map_err(PdfError::from)?,
            b if b.is_ascii_graphic() || b == b' ' => writer.write_all(&[b]).map_err(PdfError::from)?,
            _ => write!(writer, "\\{b:03o}").map_err(PdfError::from)?,
        }
    }
    writer.write_all(b")").map_err(PdfError::from)?;
    Ok(())
}

fn write_name<W: Write>(writer: &mut W, bytes: &[u8]) -> PdfResult<()> {
    writer.write_all(b"/").map_err(PdfError::from)?;
    write_name_content(writer, bytes)
}

fn write_name_content<W: Write>(writer: &mut W, bytes: &[u8]) -> PdfResult<()> {
    for &b in bytes {
        if b.is_ascii_alphanumeric() || b == b'_' || b == b'.' || b == b'-' {
            writer.write_all(&[b]).map_err(PdfError::from)?;
        } else {
            write!(writer, "#{b:02X}").map_err(PdfError::from)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    use std::collections::BTreeMap;

    #[test]
    fn test_write_basic_objects() {
        let mut buf = Vec::new();
        
        write_object(&mut buf, &Object::Integer(42)).unwrap();
        assert_eq!(buf, b"42");
        buf.clear();

        write_object(&mut buf, &Object::new_name(b"Test".to_vec())).unwrap();
        assert_eq!(buf, b"/Test");
        buf.clear();

        let mut dict = BTreeMap::new();
        dict.insert(b"Type".to_vec(), Object::new_name(b"Catalog".to_vec()));
        write_object(&mut buf, &Object::new_dict(dict)).unwrap();
        // Strict Write: No trailing space before >>
        assert_eq!(buf, b"<< /Type /Catalog >>");
    }

    #[test]
    fn test_strict_writing_compliance() {
        let mut buf = Vec::new();
        let mut dict = BTreeMap::new();
        dict.insert(b"F".to_vec(), Object::new_name(b"Test".to_vec()));
        let data = b"binarydata";
        
        write_stream(&mut buf, &dict, data).unwrap();
        
        // Strict Write: Stream starts with \nstream\r\n and ends with \r\nendstream
        // Note: write_stream adds Length automatically.
        let expected_start = b"<< /F /Test /Length 10 >>\nstream\r\n";
        let expected_end = b"\r\nendstream";
        
        assert!(buf.starts_with(expected_start), "Stream start mismatch. Found: {:?}", String::from_utf8_lossy(&buf));
        assert!(buf.ends_with(expected_end), "Stream end mismatch. Found: {:?}", String::from_utf8_lossy(&buf));
    }
}
