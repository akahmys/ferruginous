//! PDF Physical Writer (Arena Bridge)
//!
//! This module serializes the refined PdfArena back into a physical PDF byte stream.

use ferruginous_core::{Handle, Object, PdfArena, PdfError, PdfName, PdfResult};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io::Write;

/// A physical PDF writer that serializes objects resolved from an arena.
pub struct PdfWriter<'a, W: Write> {
    inner: W,
    arena: &'a PdfArena,
    buffer: Vec<u8>,
    xref: BTreeMap<u32, usize>,
    compression_level: Option<u32>,
    id_map: BTreeMap<Handle<Object>, u32>,
    linearize: bool,
    sig_handle: Option<Handle<Object>>,
    string_encoding: crate::StringEncoding,
    security_handler: Option<ferruginous_core::security::SecurityHandler>,
    current_obj_id: u32,
    current_obj_gen: u16,
}

impl<'a, W: Write> PdfWriter<'a, W> {
    /// Creates a new PdfWriter with arena access.
    pub fn new(inner: W, arena: &'a PdfArena) -> Self {
        Self {
            inner,
            arena,
            buffer: Vec::new(),
            xref: BTreeMap::new(),
            compression_level: None,
            id_map: BTreeMap::new(),
            linearize: false,
            sig_handle: None,
            string_encoding: crate::StringEncoding::default(),
            security_handler: None,
            current_obj_id: 0,
            current_obj_gen: 0,
        }
    }

    /// Sets the security handler for encryption.
    pub fn set_security_handler(&mut self, handler: ferruginous_core::security::SecurityHandler) {
        self.security_handler = Some(handler);
    }

    /// Sets the encoding for string literals (Standard or Unicode).
    pub fn set_string_encoding(&mut self, encoding: crate::StringEncoding) {
        self.string_encoding = encoding;
    }

    /// Adds a handle to be targeted for digital signature patching.
    pub fn add_signature_target(&mut self, handle: Handle<Object>) {
        self.sig_handle = Some(handle);
    }

    /// Writes the PDF header with the specified version string.
    pub fn write_header(&mut self, version: &str) -> PdfResult<()> {
        self.write_all(format!("%PDF-{version}\r\n").as_bytes())?;
        self.write_all(b"%\xE2\xE3\xCF\xD3\r\n")?;
        Ok(())
    }

    /// Enables or disables PDF linearization (Fast Web View).
    pub fn set_linearize(&mut self, linearize: bool) {
        self.linearize = linearize;
    }

    /// Sets the Zlib compression level for streams (0-9).
    pub fn set_compression(&mut self, level: u32) {
        self.compression_level = Some(level.min(9));
    }

    /// Triggers a vacuum operation to remove unreferenced objects (implicit in linearization).
    pub fn set_vacuum(&mut self, _vacuum: bool) {
        // Implementation detail: vacuum is inherent in our ID remapping logic
    }

    /// Returns the current byte offset in the output buffer.
    pub fn current_offset(&self) -> usize {
        self.buffer.len()
    }

    fn encrypt_data(&self, data: &[u8]) -> PdfResult<Vec<u8>> {
        if let Some(sh) = &self.security_handler {
            sh.encrypt_stream(data, self.current_obj_id, self.current_obj_gen)
        } else {
            Ok(data.to_vec())
        }
    }

    /// Writes raw bytes directly to the output buffer.
    pub fn write_all(&mut self, data: &[u8]) -> PdfResult<()> {
        self.buffer.extend_from_slice(data);
        Ok(())
    }

    /// Recursively serializes a high-level Object into the output buffer.
    pub fn write_object(&mut self, obj: &Object) -> PdfResult<()> {
        match obj {
            Object::Boolean(b) => self.write_all(if *b { b"true" } else { b"false" }),
            Object::Integer(i) => self.write_all(i.to_string().as_bytes()),
            Object::Real(f) => self.write_all(format!("{f:.4}").as_bytes()),
            Object::String(s) => self.write_string_obj(s),
            Object::Hex(s) => self.write_hex_obj(s),
            Object::Text(s) => self.write_text_obj(s),
            Object::Name(n) => self.write_name(n),
            Object::Array(h) => self.write_array_obj(*h),
            Object::Dictionary(h) => self.write_dictionary_obj(*h),
            Object::Stream(dh, data) => self.write_stream_obj(*dh, data),
            Object::Null => self.write_all(b"null"),
            Object::Reference(h) => self.write_reference_obj(*h),
        }
    }

    fn write_string_obj(&mut self, s: &[u8]) -> PdfResult<()> {
        let data = self.encrypt_data(s)?;
        self.write_string_literal(&data)
    }

    fn write_hex_obj(&mut self, s: &[u8]) -> PdfResult<()> {
        let data = self.encrypt_data(s)?;
        self.write_string_hex(&data)
    }

    fn write_text_obj(&mut self, s: &str) -> PdfResult<()> {
        let encoding_str = match self.string_encoding {
            crate::StringEncoding::Utf8 => "utf8",
            crate::StringEncoding::Utf16BE => "utf16be",
        };
        let encoded = ferruginous_core::refine::text::encode_string(s, encoding_str);
        let data = self.encrypt_data(&encoded)?;
        self.write_string_literal(&data)
    }

    fn write_array_obj(&mut self, h: Handle<Vec<Object>>) -> PdfResult<()> {
        let a = self.arena.get_array(h).ok_or_else(|| PdfError::Other("Array not found".into()))?;
        self.write_all(b"[")?;
        for (i, item) in a.iter().enumerate() {
            if i > 0 { self.write_all(b" ")?; }
            self.write_object(item)?;
        }
        self.write_all(b"]")
    }

    fn write_dictionary_obj(&mut self, h: Handle<BTreeMap<Handle<PdfName>, Object>>) -> PdfResult<()> {
        let d = self.arena.get_dict(h).ok_or_else(|| PdfError::Other("Dictionary not found".into()))?;
        self.write_dict(&d)
    }

    fn write_reference_obj(&mut self, h: Handle<Object>) -> PdfResult<()> {
        let id = self.id_map.get(&h).copied().unwrap_or_else(|| h.index() + 1);
        self.write_all(format!("{id} 0 R").as_bytes())
    }

    fn write_stream_obj(&mut self, dh: Handle<BTreeMap<Handle<PdfName>, Object>>, data: &std::sync::Arc<ferruginous_core::object::SublimatedData>) -> PdfResult<()> {
        let d = self.arena.get_dict(dh).ok_or_else(|| PdfError::Other("Dictionary not found".into()))?;
        let filter_key = self.arena.get_name_by_str("Filter");
        let length_key = self.arena.get_name_by_str("Length");

        let stream_bytes = self.arena.get_stream_bytes(data)?;
        let (stream_data, already_filtered) = self.prepare_stream_data(&stream_bytes, &d, filter_key);
        let mut final_data = stream_data.to_vec();
        let applied_new_compression = self.try_compress_stream(&mut final_data, already_filtered);

        self.write_all(b"<<")?;
        for (k, v) in &d {
            if Some(k) == length_key.as_ref() || (Some(k) == filter_key.as_ref() && (applied_new_compression || !already_filtered)) {
                continue;
            }
            self.write_all(b"\r\n")?;
            self.write_name(k)?;
            self.write_all(b" ")?;
            self.write_object(v)?;
        }

        if applied_new_compression { self.write_all(b"\r\n/Filter /FlateDecode")?; }
        
        let type_key = self.arena.get_name_by_str("Type");
        let is_metadata = type_key.and_then(|tk| d.get(&tk)).and_then(|o| o.as_name()).and_then(|nh| self.arena.get_name_str(nh)).as_deref() == Some("Metadata");
        if let Some(sh) = &self.security_handler && (!is_metadata || sh.should_decrypt_metadata()) {
            final_data = sh.encrypt_stream(&final_data, self.current_obj_id, self.current_obj_gen)?;
        }

        self.write_all(format!("\r\n/Length {}", final_data.len()).as_bytes())?;
        self.write_all(b"\r\n>>\r\nstream\r\n")?;
        self.write_all(&final_data)?;
        self.write_all(b"\r\nendstream")
    }

    fn prepare_stream_data(&self, data: &bytes::Bytes, d: &BTreeMap<Handle<PdfName>, Object>, filter_key: Option<Handle<PdfName>>) -> (bytes::Bytes, bool) {
        if let Some(fk) = filter_key && d.contains_key(&fk) {
            if let Ok(decompressed) = self.arena.process_filters(data, d) {
                return (decompressed, false);
            }
            return (data.clone(), true);
        }
        (data.clone(), false)
    }

    fn try_compress_stream(&self, data: &mut Vec<u8>, already_filtered: bool) -> bool {
        if !already_filtered && let Some(level) = self.compression_level {
            use flate2::{Compression, write::ZlibEncoder};
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::new(level));
            if std::io::Write::write_all(&mut encoder, data).is_ok() && let Ok(compressed) = encoder.finish() {
                *data = compressed;
                return true;
            }
        }
        false
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
        let name =
            self.arena.get_name(*n).ok_or_else(|| PdfError::Other("Name not found".into()))?;
        self.write_all(b"/")?;
        for &b in name.as_ref() {
            if b == b'#' || b <= 32 || b >= 127 || b == b'(' || b == b')' || b == b'<' || b == b'>' || b == b'[' || b == b']' || b == b'{' || b == b'}' || b == b'/' || b == b'%' {
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

    fn write_string_hex(&mut self, s: &[u8]) -> PdfResult<()> {
        self.write_all(b"<")?;
        for &b in s {
            self.write_all(format!("{b:02X}").as_bytes())?;
        }
        self.write_all(b">")
    }

    fn write_indirect_object(&mut self, id: u32, generation: u16, handle: Handle<Object>) -> PdfResult<()> {
        self.xref.insert(id, self.current_offset());
        self.current_obj_id = id;
        self.current_obj_gen = generation;
        self.write_all(format!("{id} {generation} obj\r\n").as_bytes())?;
        let obj = self.arena.get_object(handle).ok_or_else(|| PdfError::Other(format!("Object {id} missing").into()))?;
        self.write_object(&obj)?;
        self.write_all(b"\r\nendobj\r\n")?;
        Ok(())
    }

    /// Finalizes the PDF by writing trailers and cross-reference tables.
    pub fn finish(
        &mut self,
        root_handle: Handle<Object>,
        info_handle: Option<Handle<Object>>,
    ) -> PdfResult<()> {
        if self.linearize {
            self.finish_linearized(root_handle, info_handle)?;
        } else {
            self.finish_standard(root_handle, info_handle)?;
        }
        self.patch_signatures()?;
        self.inner.write_all(&self.buffer).map_err(PdfError::Io)?;
        self.inner.flush().map_err(PdfError::Io)?;
        Ok(())
    }

    /// Appends an incremental update to the output.
    pub fn write_incremental_update(
        &mut self,
        _root_handle: Handle<Object>,
        prev_xref: usize,
        total_objects: u32,
        changed_handles: &[(u32, Handle<Object>)],
    ) -> PdfResult<()> {
        let update_start = self.current_offset();
        for (id, handle) in changed_handles {
            self.write_indirect_object(*id, 0, *handle)?;
        }

        let xref_offset = self.current_offset();
        self.write_all(b"xref\r\n")?;
        for (id, _) in changed_handles {
            self.write_all(format!("{id} 1\r\n").as_bytes())?;
            let off = self.xref.get(id).copied().unwrap_or(0);
            self.write_all(format!("{off:010} 00000 n\r\n").as_bytes())?;
        }

        let id_hex = "f00baa42f00baa42f00baa42f00baa42";
        self.write_all(format!("trailer\r\n<< /Size {total_objects} /Prev {prev_xref} /Root 2 0 R /ID [<{id_hex}> <{id_hex}>] >>\r\nstartxref\r\n{xref_offset}\r\n%%EOF\r\n").as_bytes())?;
        
        self.inner.write_all(&self.buffer[update_start..]).map_err(PdfError::Io)?;
        self.inner.flush().map_err(PdfError::Io)?;
        Ok(())
    }

    fn patch_signatures(&mut self) -> PdfResult<()> {
        let Some(sig_h) = self.sig_handle else { return Ok(()) };
        let id = *self.id_map.get(&sig_h).ok_or_else(|| PdfError::Other("Sig object missing".into()))?;
        let off = *self.xref.get(&id).ok_or_else(|| PdfError::Other("Sig offset missing".into()))?;
        
        let end = self.find_obj_end(off);
        let (c_start, c_end) = self.find_contents_offsets(off, end)?;
        let (br_start, br_end) = self.find_byte_range_offsets(off, end)?;
        
        let br_str = format!("0 {:010} {:010} {:010}", c_start - 1, c_end + 1, self.buffer.len() - (c_end + 1));
        let br_bytes = br_str.as_bytes();
        if br_bytes.len() > (br_end - br_start) { return Err(PdfError::Other("ByteRange overflow".into())); }
        
        for i in br_start..br_end { self.buffer[i] = b' '; }
        self.buffer[br_start..br_start + br_bytes.len()].copy_from_slice(br_bytes);
        Ok(())
    }

    fn find_obj_end(&self, start: usize) -> usize {
        let mut end = start;
        while end + 6 <= self.buffer.len() {
            if &self.buffer[end..end + 6] == b"endobj" { return end + 6; }
            end += 1;
        }
        end
    }

    fn find_contents_offsets(&self, start: usize, end: usize) -> PdfResult<(usize, usize)> {
        let key = b"/Contents <";
        let pos = self.buffer[start..end].windows(key.len()).position(|w| w == key).ok_or_else(|| PdfError::Other("Missing /Contents".into()))?;
        let c_start = start + pos + 11;
        let c_end_pos = self.buffer[c_start..end].iter().position(|&b| b == b'>').ok_or_else(|| PdfError::Other("Missing end of /Contents".into()))?;
        Ok((c_start, c_start + c_end_pos))
    }

    fn find_byte_range_offsets(&self, start: usize, end: usize) -> PdfResult<(usize, usize)> {
        let key = b"/ByteRange [";
        let pos = self.buffer[start..end].windows(key.len()).position(|w| w == key).ok_or_else(|| PdfError::Other("Missing /ByteRange".into()))?;
        let br_start = start + pos + 12;
        let br_end_pos = self.buffer[br_start..end].iter().position(|&b| b == b']').ok_or_else(|| PdfError::Other("Missing end of /ByteRange".into()))?;
        Ok((br_start, br_start + br_end_pos))
    }

    fn finish_standard(
        &mut self,
        root_handle: Handle<Object>,
        info_handle: Option<Handle<Object>>,
    ) -> PdfResult<()> {
        let mut reachable = BTreeSet::new();
        self.trace_reachable(Object::Reference(root_handle), &mut reachable);
        if let Some(ih) = info_handle {
            self.trace_reachable(Object::Reference(ih), &mut reachable);
        }

        let mut sorted_handles: Vec<_> = reachable.into_iter().collect();
        sorted_handles.sort_by_key(|h| h.index());

        let mut next_id = 1;
        for &handle in &sorted_handles {
            self.id_map.insert(handle, next_id);
            next_id += 1;
        }

        let mut current_id = 1;
        for &handle in &sorted_handles {
            self.write_indirect_object(current_id, 0, handle)?;
            current_id += 1;
        }

        let total_size = next_id;
        let start_xref = self.current_offset();
        self.write_all(format!("xref\r\n0 {total_size}\r\n0000000000 65535 f\r\n").as_bytes())?;
        for id in 1..total_size {
            let offset = self.xref.get(&id).copied().unwrap_or(0);
            self.write_all(format!("{offset:010} 00000 n\r\n").as_bytes())?;
        }

        let id_hex = "f00baa42f00baa42f00baa42f00baa42";
        self.write_all(b"trailer\r\n<<\r\n")?;
        self.write_all(format!("/Size {total_size}\r\n").as_bytes())?;
        self.write_all(format!("/Root {} 0 R\r\n", self.id_map[&root_handle]).as_bytes())?;
        if let Some(ih) = info_handle
            && let Some(&id) = self.id_map.get(&ih)
        {
            self.write_all(format!("/Info {id} 0 R\r\n").as_bytes())?;
        }
        self.write_all(format!("/ID [<{id_hex}> <{id_hex}>]\r\n").as_bytes())?;
        self.write_all(b">>\r\nstartxref\r\n")?;
        self.write_all(start_xref.to_string().as_bytes())?;
        self.write_all(b"\r\n%%EOF\r\n")?;
        Ok(())
    }

    #[allow(clippy::cast_possible_truncation)]
    fn finish_linearized(&mut self, root: Handle<Object>, info: Option<Handle<Object>>) -> PdfResult<()> {
        let (pages, shared, others, counts) = self.collect_lin_objects(root, info)?;
        let total_size = self.assign_lin_ids(root, &pages, &shared, &others);
        let primary_count = 5 + pages.len() as u32 + shared.len() as u32;

        self.write_lin_header()?;
        let (dict_pos, p_xref_pos) = self.reserve_lin_headers(primary_count);
        self.write_indirect_object(2, 0, root)?;
        let (hint_pos, _h_stream_start, h_len, s_h_len) = self.reserve_hint_stream(pages.len());

        let p1_start = self.current_offset();
        if !pages.is_empty() { self.write_indirect_object(4, 0, pages[0])?; }
        self.write_lin_objects(&pages, &shared, &others)?;
        
        let p1_end = *self.xref.get(&(primary_count - 1)).unwrap_or(&self.buffer.len());
        let main_xref_off = self.write_lin_main_xref(total_size)?;
        let state = LinState {
            d_pos: dict_pos, px_pos: p_xref_pos, h_pos: hint_pos, h_len, sh_len: s_h_len,
            p1_s: p1_start, p1_e: p1_end, mx_off: main_xref_off,
            pages, shared, counts, total: total_size, prim: primary_count,
        };
        self.finalize_lin_headers(state)?;
        Ok(())
    }

    fn collect_lin_objects(&self, root: Handle<Object>, info: Option<Handle<Object>>) -> PdfResult<(Vec<Handle<Object>>, Vec<Handle<Object>>, Vec<Handle<Object>>, Vec<u32>)> {
        let mut all = BTreeSet::new();
        self.trace_reachable(Object::Reference(root), &mut all);
        if let Some(ih) = info { self.trace_reachable(Object::Reference(ih), &mut all); }

        let mut pages = Vec::new();
        if let Some(dh) = self.arena.get_object(root).and_then(|o| o.as_dict_handle()) && let Some(dict) = self.arena.get_dict(dh) && let Some(Object::Reference(ph)) = dict.get(&self.arena.name("Pages")) && let Some(pdh) = self.arena.get_object(*ph).and_then(|o| o.as_dict_handle()) {
            self.collect_pages_recursive(pdh, &mut pages)?;
        }

        let mut visited: BTreeMap<Handle<Object>, usize> = BTreeMap::new();
        let mut shared_set: BTreeSet<Handle<Object>> = BTreeSet::new();
        let mut counts = Vec::new();

        for (idx, &ph) in pages.iter().enumerate() {
            let mut stack = vec![ph];
            let mut seen = BTreeSet::new();
            while let Some(h) = stack.pop() {
                if !seen.insert(h) { continue; }
                if let Some(&first) = visited.get(&h) { if first != idx { shared_set.insert(h); } } else { visited.insert(h, idx); }
                if let Some(inner) = self.arena.get_object(h) {
                    match inner {
                        Object::Array(ah) => if let Some(a) = self.arena.get_array(ah) { for item in a { if let Object::Reference(rh) = item { stack.push(rh); } } }
                        Object::Dictionary(dh) | Object::Stream(dh, _) => if let Some(d) = self.arena.get_dict(dh) {
                            for (k, v) in d {
                                let name = self.arena.get_name_str(k).unwrap_or_default();
                                if name == "Parent" || name == "Prev" || name == "Next" { continue; }
                                if let Object::Reference(rh) = v { stack.push(rh); }
                            }
                        }
                        _ => {}
                    }
                }
            }
            #[allow(clippy::cast_possible_truncation)]
            counts.push(seen.len() as u32);
        }
        let shared: Vec<_> = shared_set.into_iter().filter(|&h| !pages.contains(&h) && h != root).collect();
        let mut assigned = BTreeSet::new();
        assigned.insert(root);
        for &h in &pages { assigned.insert(h); }
        for &h in &shared { assigned.insert(h); }
        let mut others: Vec<_> = all.into_iter().filter(|h| !assigned.contains(h)).collect();
        others.sort_by_key(|h| h.index());
        Ok((pages, shared, others, counts))
    }

    fn assign_lin_ids(&mut self, _root: Handle<Object>, pages: &[Handle<Object>], shared: &[Handle<Object>], others: &[Handle<Object>]) -> u32 {
        let mut next_id = 3;
        for &h in pages { self.id_map.insert(h, next_id); next_id += 1; }
        for &h in shared { self.id_map.insert(h, next_id); next_id += 1; }
        for &h in others { self.id_map.insert(h, next_id); next_id += 1; }
        next_id
    }

    fn write_lin_header(&mut self) -> PdfResult<()> {
        self.write_all(b"%PDF-2.0\r\n%\xe2\xe3\xcf\xD3\r\n")
    }

    fn reserve_lin_headers(&mut self, primary_count: u32) -> (usize, usize) {
        let dict_pos = self.current_offset();
        self.buffer.extend_from_slice(&vec![b' '; 256]);
        let p_xref_pos = self.current_offset();
        let p_xref_size = 256 + (primary_count as usize * 20);
        self.buffer.extend_from_slice(&vec![b' '; p_xref_size]);
        (dict_pos, p_xref_pos)
    }

    fn reserve_hint_stream(&mut self, page_count: usize) -> (usize, usize, usize, usize) {
        let pos = self.current_offset();
        self.xref.insert(3, pos);
        self.current_obj_id = 3;
        self.current_obj_gen = 0;
        let p_len = 36 + (page_count * 6);
        let s_len = 20;
        let data_len = p_len + s_len;
        let footer = "\r\nendstream\r\nendobj\r\n";
        self.buffer.extend_from_slice(&vec![b' '; 64 + data_len + footer.len()]);
        (pos, pos + 25, data_len, s_len) // Approximation for h_stream_start
    }

    fn write_lin_objects(&mut self, pages: &[Handle<Object>], shared: &[Handle<Object>], others: &[Handle<Object>]) -> PdfResult<()> {
        let mut ordered = Vec::new();
        let mut seen = BTreeSet::new();
        for &h in pages { if seen.insert(h) { ordered.push(h); } }
        for &h in shared { if seen.insert(h) { ordered.push(h); } }
        for &h in others { if seen.insert(h) { ordered.push(h); } }
        for h in ordered {
            let id = self.id_map[&h];
            self.write_indirect_object(id, 0, h)?;
        }
        Ok(())
    }

    fn write_lin_main_xref(&mut self, total_size: u32) -> PdfResult<usize> {
        let off = self.current_offset();
        self.write_all(format!("xref\r\n0 {total_size}\r\n0000000000 65535 f\r\n").as_bytes())?;
        for id in 1..total_size {
            let o = self.xref.get(&id).copied().unwrap_or(0);
            self.write_all(format!("{o:010} 00000 n\r\n").as_bytes())?;
        }
        let id_hex = "f00baa42f00baa42f00baa42f00baa42";
        self.write_all(format!("trailer\r\n<< /Size {total_size} /Root 2 0 R /ID [<{id_hex}> <{id_hex}>] >>\r\nstartxref\r\n{off}\r\n%%EOF\r\n").as_bytes())?;
        Ok(off)
    }

    fn finalize_lin_headers(&mut self, s: LinState) -> PdfResult<()> {
        let p_len = s.h_len - s.sh_len;
        let id_hex = "f00baa42f00baa42f00baa42f00baa42";
        let h_dict = format!("3 0 obj\r\n<< /Length {} /S {p_len} >>\r\nstream\r\n", s.h_len);
        let h_data = self.generate_hint_tables(&s.pages, &s.shared.iter().map(|h| self.id_map[h]).collect::<Vec<_>>(), s.p1_s, s.mx_off, &s.counts);
        let h_full = format!("{h_dict}{}\r\nendstream\r\nendobj\r\n", std::str::from_utf8(&h_data).unwrap_or(""));
        self.buffer[s.h_pos..s.h_pos + h_full.len()].copy_from_slice(h_full.as_bytes());

        let d_str = format!("1 0 obj\r\n<< /Linearized 1 /L {} /P 0 /O 4 /E {} /N {} /T {} /H [{} {} {} {}] >>\r\nendobj\r\n", self.buffer.len(), s.p1_e, s.pages.len(), s.mx_off, s.h_pos + h_dict.len(), p_len, s.h_pos + h_dict.len() + p_len, s.sh_len);
        self.buffer[s.d_pos..s.d_pos + d_str.len()].copy_from_slice(d_str.as_bytes());

        let mut px = format!("xref\r\n0 {}\r\n0000000000 65535 f\r\n", s.prim);
        for id in 1..s.prim { let _ = write!(px, "{:010} 00000 n\r\n", self.xref.get(&id).unwrap_or(&0)); }
        let _ = write!(px, "trailer\r\n<< /Size {} /Prev {} /Root 2 0 R /ID [<{id_hex}> <{id_hex}>] >>\r\n", s.total, s.mx_off);
        self.buffer[s.px_pos..s.px_pos + px.len()].copy_from_slice(px.as_bytes());
        Ok(())
    }
}

struct LinState {
    d_pos: usize, px_pos: usize, h_pos: usize, h_len: usize, sh_len: usize,
    p1_s: usize, p1_e: usize, mx_off: usize,
    pages: Vec<Handle<Object>>, shared: Vec<Handle<Object>>, counts: Vec<u32>,
    total: u32, prim: u32,
}

impl<W: std::io::Write> PdfWriter<'_, W> {
    #[allow(clippy::cast_possible_truncation)]
    fn generate_hint_tables(
        &self,
        page_handles: &[Handle<Object>],
        shared_ids: &[u32],
        p1_offset: usize,
        main_xref_offset: usize,
        obj_counts: &[u32],
    ) -> Vec<u8> {
        let mut writer = BitWriter::new();
        writer.write_u32(1);
        writer.write_u32(p1_offset as u32);
        writer.write_u16(16);
        writer.write_u32(0);
        writer.write_u16(32);
        for _ in 0..11 {
            writer.write_u16(0);
        }

        for i in 0..page_handles.len() {
            let offset = self.xref.get(&self.id_map[&page_handles[i]]).unwrap_or(&0);
            let length = if i + 1 < page_handles.len() {
                self.xref
                    .get(&self.id_map[&page_handles[i + 1]])
                    .unwrap_or(&main_xref_offset)
                    .saturating_sub(*offset)
            } else {
                main_xref_offset.saturating_sub(*offset)
            };
            let count_delta = obj_counts.get(i).copied().unwrap_or(1).saturating_sub(1);
            writer.write_bits(count_delta, 16);
            writer.write_bits(length as u32, 32);
        }

        let shared_count = shared_ids.len() as u32;
        writer.write_u32(shared_ids.first().copied().unwrap_or(0));
        writer.write_u32(0);
        writer.write_u16(shared_count as u16);
        writer.write_u16(shared_count as u16);
        writer.write_u16(16);
        writer.write_u16(32);
        writer.write_u16(0);
        writer.write_u16(0);
        
        for &id in shared_ids {
            let offset = self.xref.get(&id).unwrap_or(&0);
            writer.write_bits(*offset as u32, 32);
        }
        
        writer.finish()
    }

    fn trace_reachable(
        &self,
        initial_obj: Object,
        reachable: &mut BTreeSet<Handle<Object>>,
    ) {
        let mut stack = vec![initial_obj];
        while let Some(obj) = stack.pop() {
            match obj {
                Object::Reference(h) => {
                    if reachable.insert(h)
                        && let Some(inner) = self.arena.get_object(h) {
                        stack.push(inner.clone());
                    }
                }
                Object::Array(h) => {
                    if let Some(a) = self.arena.get_array(h) {
                        for item in a {
                            stack.push(item.clone());
                        }
                    }
                }
                Object::Dictionary(h) | Object::Stream(h, _) => {
                    if let Some(d) = self.arena.get_dict(h) {
                        for v in d.values() {
                            stack.push(v.clone());
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn collect_pages_recursive(
        &self,
        root_pages_dh: Handle<BTreeMap<Handle<PdfName>, Object>>,
        pages: &mut Vec<Handle<Object>>,
    ) -> PdfResult<()> {
        let mut stack = vec![root_pages_dh];
        let type_key = self.arena.name("Type");
        let pages_n = self.arena.name("Pages");
        let page_n = self.arena.name("Page");
        let kids_k = self.arena.name("Kids");

        while let Some(dh) = stack.pop() {
            let Some(dict) = self.arena.get_dict(dh) else { continue };
            
            let Some(kids_obj) = dict.get(&kids_k) else { continue };
            let Some(kids_handle) = kids_obj.as_array() else { continue };
            let Some(kids) = self.arena.get_array(kids_handle) else { continue };

            for kid_obj in kids.iter().rev() {
                if let Object::Reference(h) = kid_obj
                    && let Some(Object::Dictionary(kdh)) = self.arena.get_object(*h) {
                        let kdict = self.arena.get_dict(kdh).ok_or_else(|| PdfError::Other("Kid dict missing".into()))?;
                        let ktype = kdict.get(&type_key).and_then(|o| o.as_name());
                        
                        if ktype == Some(pages_n) {
                            stack.push(kdh);
                        } else if ktype == Some(page_n) {
                            pages.push(*h);
                        }
                    }
            }
        }
        Ok(())
    }
}

// --- Utilities ---

struct BitWriter {
    data: Vec<u8>,
    current_byte: u8,
    bits_used: u8,
}

impl BitWriter {
    fn new() -> Self {
        Self { data: Vec::new(), current_byte: 0, bits_used: 0 }
    }
    fn write_bits(&mut self, value: u32, count: u8) {
        for i in (0..count).rev() {
            let bit = (value >> i) & 1;
            self.current_byte = (self.current_byte << 1) | (bit as u8);
            self.bits_used += 1;
            if self.bits_used == 8 {
                self.data.push(self.current_byte);
                self.current_byte = 0;
                self.bits_used = 0;
            }
        }
    }
    fn write_u32(&mut self, val: u32) {
        self.write_bits(val, 32);
    }
    fn write_u16(&mut self, val: u16) {
        self.write_bits(u32::from(val), 16);
    }
    fn finish(mut self) -> Vec<u8> {
        if self.bits_used > 0 {
            self.current_byte <<= 8 - self.bits_used;
            self.data.push(self.current_byte);
        }
        self.data
    }
}
