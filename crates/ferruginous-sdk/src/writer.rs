use std::io::{Write, Result};
use std::collections::BTreeMap;
use ferruginous_core::{Object, PdfName, Reference};

/// A physical PDF writer that serializes objects to a stream.
pub struct PdfWriter<W: Write> {
    inner: W,
    current_offset: usize,
    xref: BTreeMap<u32, usize>,
}

impl<W: Write> PdfWriter<W> {
    /// Creates a new PdfWriter from a writable stream.
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            current_offset: 0,
            xref: BTreeMap::new(),
        }
    }

    /// Writes the PDF header with the specified version and binary marker.
    pub fn write_header(&mut self, version: &str) -> Result<()> {
        self.write_all(format!("%PDF-{}\r\n", version).as_bytes())?;
        // High-bit characters to indicate binary file
        self.write_all(b"%\xE2\xE3\xCF\xD3\r\n")?;
        Ok(())
    }

    /// Writes an indirect object to the PDF stream and records its offset in the XRef table.
    pub fn write_indirect_object(&mut self, id: u32, generation: u16, obj: &Object) -> Result<()> {
        self.xref.insert(id, self.current_offset);
        self.write_all(format!("{} {} obj\r\n", id, generation).as_bytes())?;
        self.write_object(obj)?;
        self.write_all(b"\r\nendobj\r\n")?;
        Ok(())
    }

    /// Recursively writes a PDF object to the stream.
    pub fn write_object(&mut self, obj: &Object) -> Result<()> {
        match obj {
            Object::Boolean(b) => self.write_all(if *b { b"true" } else { b"false" }),
            Object::Integer(i) => self.write_all(i.to_string().as_bytes()),
            Object::Real(f) => self.write_all(format!("{:.4}", f).as_bytes()),
            Object::String(s) => self.write_string_literal(s),
            Object::Name(n) => self.write_name(n),
            Object::Array(a) => {
                self.write_all(b"[")?;
                for (i, item) in a.iter().enumerate() {
                    if i > 0 { self.write_all(b" ")?; }
                    self.write_object(item)?;
                }
                self.write_all(b"]")
            }
            Object::Dictionary(d) => self.write_dict(d),
            Object::Stream(d, data) => {
                self.write_dict(d)?;
                self.write_all(b"\r\nstream\r\n")?;
                self.write_all(data)?;
                self.write_all(b"\r\nendstream")
            }
            Object::Null => self.write_all(b"null"),
            Object::Reference(r) => self.write_all(format!("{} {} R", r.id, r.generation).as_bytes()),
        }
    }

    fn write_dict(&mut self, d: &BTreeMap<PdfName, Object>) -> Result<()> {
        self.write_all(b"<<")?;
        for (k, v) in d {
            self.write_all(b"\r\n")?;
            self.write_name(k)?;
            self.write_all(b" ")?;
            self.write_object(v)?;
        }
        self.write_all(b"\r\n>>")
    }

    fn write_name(&mut self, n: &PdfName) -> Result<()> {
        self.write_all(b"/")?;
        // Simple encoding: escape # and non-printable
        for &b in n.as_ref() {
            if b == b'#' || b <= 32 || b >= 127 {
                self.write_all(format!("#{:02X}", b).as_bytes())?;
            } else {
                self.write_all(&[b])?;
            }
        }
        Ok(())
    }

    fn write_string_literal(&mut self, s: &[u8]) -> Result<()> {
        self.write_all(b"(")?;
        for &b in s {
            match b {
                b'(' => self.write_all(b"\\(")?,
                b')' => self.write_all(b"\\)")?,
                b'\\' => self.write_all(b"\\\\")?,
                _ => self.write_all(&[b])?,
            }
        }
        self.write_all(b")")
    }

    fn write_all(&mut self, data: &[u8]) -> Result<()> {
        self.inner.write_all(data)?;
        self.current_offset += data.len();
        Ok(())
    }

    /// Finalizes the PDF by writing the XRef table and trailer.
    pub fn finish(&mut self, root: Reference, info: Option<Reference>) -> Result<()> {
        let start_xref = self.current_offset;
        let count = self.xref.keys().last().unwrap_or(&0) + 1;
        
        self.write_all(format!("xref\r\n0 {}\r\n", count).as_bytes())?;
        self.write_all(b"0000000000 65535 f\r\n")?;
        
        for i in 1..count {
            if let Some(&offset) = self.xref.get(&i) {
                self.write_all(format!("{:010} 00000 n\r\n", offset).as_bytes())?;
            } else {
                self.write_all(b"0000000000 00000 f\r\n")?;
            }
        }

        self.write_all(b"trailer\r\n<<\r\n")?;
        self.write_all(format!("/Size {}\r\n", count).as_bytes())?;
        self.write_all(format!("/Root {} {} R\r\n", root.id, root.generation).as_bytes())?;
        if let Some(r) = info {
            self.write_all(format!("/Info {} {} R\r\n", r.id, r.generation).as_bytes())?;
        }
        self.write_all(b">>\r\nstartxref\r\n")?;
        self.write_all(start_xref.to_string().as_bytes())?;
        self.write_all(b"\r\n%%EOF\r\n")?;
        
        self.inner.flush()
    }
}
