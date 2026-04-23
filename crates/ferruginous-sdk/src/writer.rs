//! PDF Physical Writer (Arena Bridge)
//! 
//! This module serializes the refined PdfArena back into a physical PDF byte stream.

use ferruginous_core::{Object, PdfName, Handle, PdfArena, PdfError, PdfResult};
use std::collections::BTreeMap;
use std::io::Write;

/// A physical PDF writer that serializes objects resolved from an arena.
pub struct PdfWriter<'a, W: Write> {
    inner: W,
    arena: &'a PdfArena,
    current_offset: usize,
    xref: BTreeMap<u32, usize>,
}

impl<'a, W: Write> PdfWriter<'a, W> {
    /// Creates a new PdfWriter with arena access.
    pub fn new(inner: W, arena: &'a PdfArena) -> Self {
        Self { 
            inner, 
            arena,
            current_offset: 0, 
            xref: BTreeMap::new() 
        }
    }

    /// Returns the current byte offset in the output stream.
    pub fn current_offset(&self) -> usize {
        self.current_offset
    }

    /// Writes the PDF header.
    pub fn write_header(&mut self, version: &str) -> PdfResult<()> {
        self.write_all(format!("%PDF-{version}\r\n").as_bytes())?;
        self.write_all(b"%\xE2\xE3\xCF\xD3\r\n")?;
        Ok(())
    }

    /// Recursively writes a PDF object to the stream.
    pub fn write_object(&mut self, obj: &Object) -> PdfResult<()> {
        match obj {
            Object::Boolean(b) => self.write_all(if *b { b"true" } else { b"false" }),
            Object::Integer(i) => self.write_all(i.to_string().as_bytes()),
            Object::Real(f) => self.write_all(format!("{f:.4}").as_bytes()),
            Object::String(s) => self.write_string_literal(s),
            Object::Name(n) => self.write_name(n),
            Object::Array(h) => {
                let a = self.arena.get_array(*h).ok_or_else(|| PdfError::Other("Array not found".into()))?;
                self.write_all(b"[")?;
                for (i, item) in a.iter().enumerate() {
                    if i > 0 { self.write_all(b" ")?; }
                    self.write_object(item)?;
                }
                self.write_all(b"]")
            }
            Object::Dictionary(h) => {
                let d = self.arena.get_dict(*h).ok_or_else(|| PdfError::Other("Dictionary not found".into()))?;
                self.write_dict(&d)
            }
            Object::Stream(dh, data) => {
                let d = self.arena.get_dict(*dh).ok_or_else(|| PdfError::Other("Dictionary not found".into()))?;
                self.write_all(b"<<")?;
                for (k, v) in d {
                    self.write_all(b"\r\n")?;
                    self.write_name(&k)?;
                    self.write_all(b" ")?;
                    self.write_object(&v)?;
                }
                // Ensure /Length is correct
                self.write_all(format!("\r\n/Length {}", data.len()).as_bytes())?;
                self.write_all(b"\r\n>>")?;
                self.write_all(b"\r\nstream\r\n")?;
                self.write_all(data)?;
                self.write_all(b"\r\nendstream")
            }
            Object::Null => self.write_all(b"null"),
            Object::Reference(h) => {
                // In the writer pass, we assign sequential IDs to handles.
                // For now, we use the handle index as the ID.
                self.write_all(format!("{} 0 R", h.index() + 1).as_bytes())
            }
        }
    }

    fn write_dict(&mut self, d: &BTreeMap<Handle<PdfName>, Object>) -> PdfResult<()> {
        self.write_all(b"<<")?;
        for (k, v) in d {
            self.write_all(b"\r\n")?;
            self.write_name(k)?;
            self.write_all(b" ")?;
            self.write_object(v)?;
        }
        self.write_all(b"\r\n>>")
    }

    fn write_name(&mut self, n: &Handle<PdfName>) -> PdfResult<()> {
        let name = self.arena.get_name(*n).ok_or_else(|| PdfError::Other("Name not found".into()))?;
        self.write_all(b"/")?;
        for &b in name.as_ref() {
            if b == b'#' || b <= 32 || b >= 127 {
                self.write_all(format!("#{b:02X}").as_bytes())?;
            } else {
                self.write_all(&[b])?;
            }
        }
        Ok(())
    }

    fn write_string_literal(&mut self, s: &[u8]) -> PdfResult<()> {
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

    /// Writes raw bytes to the output stream and tracks the current offset.
    pub fn write_all(&mut self, data: &[u8]) -> PdfResult<()> {
        self.inner.write_all(data)?;
        self.current_offset += data.len();
        Ok(())
    }

    /// Finalizes the PDF by writing the XRef table and trailer based on Arena state.
    pub fn finish(&mut self, root_handle: Handle<Object>) -> PdfResult<()> {
        // This is a simplified sequential writer. 
        // In M67, we should implement a proper multi-pass writing to assign IDs.
        // Finalize by writing objects
        
        // Write all objects in the arena
        let object_count = self.arena.object_count();
        for i in 0..object_count {
            let handle = Handle::<Object>::new(u32::try_from(i).expect("Object index too large"));
            if let Some(obj) = self.arena.get_object(handle) {
                let id = u32::try_from(i).expect("Object ID too large") + 1;
                self.xref.insert(id, self.current_offset);
                self.write_all(format!("{id} 0 obj\r\n").as_bytes())?;
                self.write_object(&obj)?;
                self.write_all(b"\r\nendobj\r\n")?;
            }
        }

        let final_start_xref = self.current_offset;
        let count = u32::try_from(object_count).expect("Object count too large") + 1;

        self.write_all(format!("xref\r\n0 {count}\r\n").as_bytes())?;
        self.write_all(b"0000000000 65535 f\r\n")?;
        for id in 1..count {
            let offset = self.xref.get(&id).unwrap_or(&0);
            self.write_all(format!("{offset:010} 00000 n\r\n").as_bytes())?;
        }

        self.write_all(b"trailer\r\n<<\r\n")?;
        self.write_all(format!("/Size {count}\r\n").as_bytes())?;
        self.write_all(format!("/Root {} 0 R\r\n", root_handle.index() + 1).as_bytes())?;
        self.write_all(b">>\r\nstartxref\r\n")?;
        self.write_all(final_start_xref.to_string().as_bytes())?;
        self.write_all(b"\r\n%%EOF\r\n")?;

        self.inner.flush()?;
        Ok(())
    }
}
