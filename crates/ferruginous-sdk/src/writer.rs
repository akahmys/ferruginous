use std::io::{Write, Result};
use std::collections::BTreeMap;
use ferruginous_core::{Object, PdfName, Reference};

/// A physical PDF writer that serializes objects to a stream.
pub struct PdfWriter<W: Write> {
    inner: W,
    current_offset: usize,
    xref: BTreeMap<u32, usize>,
}

/// A dummy writer used for size estimation passes.
#[derive(Default)]
pub struct NullWriter {
    /// Total count of bytes processed.
    pub count: usize,
}

impl NullWriter {
    /// Creates a new NullWriter.
    pub fn new() -> Self {
        Self { count: 0 }
    }
}

impl Write for NullWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.count += buf.len();
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Parameters for the Linearization Dictionary (Object 1).
#[derive(Debug, Default, Clone)]
/// Parameters for the PDF linearization process.
pub struct LinearizationParams {
    /// Total file length in bytes.
    pub file_len: usize,
    /// Offset to the beginning of the hint stream.
    pub hint_stream_offset: usize,
    /// Length of the hint stream in bytes.
    pub hint_stream_len: usize,
    /// Object ID of the first page.
    pub first_page_obj: u32,
    /// Offset to the end of the first page section.
    pub end_of_first_page: usize,
    /// Total number of pages in the document.
    pub num_pages: usize,
    /// Offset to the main cross-reference table.
    pub main_xref_offset: usize,
}

/// A simple bit-packer for Hint Stream generation.
#[derive(Default)]
pub struct BitWriter {
    data: Vec<u8>,
    current_byte: u8,
    bits_left: u8,
}

impl BitWriter {
    /// Creates a new BitWriter for bit-packing.
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            current_byte: 0,
            bits_left: 8,
        }
    }

    /// Writes a value using the specified number of bits.
    pub fn write_bits(&mut self, value: u32, num_bits: u8) {
        let mut bits_to_write = num_bits;
        let val = if num_bits == 32 { value } else { value & ((1 << num_bits) - 1) };

        while bits_to_write > 0 {
            let chunk = std::cmp::min(bits_to_write, self.bits_left);
            let shift = bits_to_write - chunk;
            let mask = if chunk == 32 { 0xFFFFFFFF } else { ((1 << chunk) - 1) << shift };
            
            let bits = ((val & mask) >> shift) as u8;
            self.current_byte |= bits << (self.bits_left - chunk);
            
            self.bits_left -= chunk;
            bits_to_write -= chunk;
            
            if self.bits_left == 0 {
                self.data.push(self.current_byte);
                self.current_byte = 0;
                self.bits_left = 8;
            }
        }
    }

    /// Finalizes the bit-packing and returns the resulting byte vector.
    pub fn finish(mut self) -> Vec<u8> {
        if self.bits_left < 8 {
            self.data.push(self.current_byte);
        }
        self.data
    }
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

    /// Returns the current byte offset in the output stream.
    pub fn current_offset(&self) -> usize {
        self.current_offset
    }

    /// Writes the PDF header with the specified version and binary marker.
    pub fn write_header(&mut self, version: &str) -> Result<()> {
        self.write_all(format!("%PDF-{version}\r\n").as_bytes())?;
        // High-bit characters to indicate binary file
        self.write_all(b"%\xE2\xE3\xCF\xD3\r\n")?;
        Ok(())
    }

    /// Writes the Linearization Dictionary (Object 1).
    pub fn write_linearization_dict(&mut self, id: u32, params: &LinearizationParams) -> Result<()> {
        self.xref.insert(id, self.current_offset);
        self.write_all(format!("{id} 0 obj\r\n<<\r\n/Linearized 1.0\r\n").as_bytes())?;
        self.write_all(format!("/L {}\r\n", params.file_len).as_bytes())?;
        self.write_all(format!("/H [{} {}]\r\n", params.hint_stream_offset, params.hint_stream_len).as_bytes())?;
        self.write_all(format!("/O {}\r\n", params.first_page_obj).as_bytes())?;
        self.write_all(format!("/E {}\r\n", params.end_of_first_page).as_bytes())?;
        self.write_all(format!("/N {}\r\n", params.num_pages).as_bytes())?;
        self.write_all(format!("/T {}\r\n", params.main_xref_offset).as_bytes())?;
        self.write_all(b">>\r\nendobj\r\n")?;
        Ok(())
    }

    /// Writes an indirect object to the PDF stream and records its offset in the XRef table.
    pub fn write_indirect_object(&mut self, id: u32, generation: u16, obj: &Object) -> Result<()> {
        self.xref.insert(id, self.current_offset);
        self.write_all(format!("{id} {generation} obj\r\n").as_bytes())?;
        self.write_object(obj)?;
        self.write_all(b"\r\nendobj\r\n")?;
        Ok(())
    }

    /// Recursively writes a PDF object to the stream.
    pub fn write_object(&mut self, obj: &Object) -> Result<()> {
        match obj {
            Object::Boolean(b) => self.write_all(if *b { b"true" } else { b"false" }),
            Object::Integer(i) => self.write_all(i.to_string().as_bytes()),
            Object::Real(f) => self.write_all(format!("{f:.4}").as_bytes()),
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
                let mut d_with_length = d.as_ref().clone();
                d_with_length.insert(PdfName::new(b"Length"), Object::Integer(data.len() as i64));
                self.write_dict(&d_with_length)?;
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
                self.write_all(format!("#{b:02X}").as_bytes())?;
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

    /// Writes raw bytes to the output stream and updates the offset.
    pub fn write_all(&mut self, data: &[u8]) -> Result<()> {
        self.inner.write_all(data)?;
        self.current_offset += data.len();
        Ok(())
    }

    /// Finalizes the PDF by writing the XRef table and trailer.
    pub fn finish(&mut self, root: Reference, info: Option<Reference>) -> Result<()> {
        let start_xref = self.current_offset;
        let count = self.xref.keys().last().unwrap_or(&0) + 1;
        
        self.write_all(format!("xref\r\n0 {count}\r\n").as_bytes())?;
        self.write_all(b"0000000000 65535 f\r\n")?;
        
        for i in 1..count {
            if let Some(&offset) = self.xref.get(&i) {
                self.write_all(format!("{offset:010} 00000 n\r\n").as_bytes())?;
            } else {
                self.write_all(b"0000000000 00000 f\r\n")?;
            }
        }

        self.write_all(b"trailer\r\n<<\r\n")?;
        self.write_all(format!("/Size {count}\r\n").as_bytes())?;
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
