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
    recursion_depth: u32,
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
            recursion_depth: 0,
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
        self.recursion_depth += 1;
        let res = match obj {
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
        };
        self.recursion_depth -= 1;
        res
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
            if i > 0 {
                self.write_all(b" ")?;
            }
            self.write_object(item)?;
        }
        self.write_all(b"]")
    }

    fn write_dictionary_obj(
        &mut self,
        h: Handle<BTreeMap<Handle<PdfName>, Object>>,
    ) -> PdfResult<()> {
        let d =
            self.arena.get_dict(h).ok_or_else(|| PdfError::Other("Dictionary not found".into()))?;
        self.write_dict(&d)
    }

    fn write_reference_obj(&mut self, h: Handle<Object>) -> PdfResult<()> {
        let id = *self.id_map.get(&h).ok_or_else(|| PdfError::Other(format!("Object {:?} not in id_map during writing", h).into()))?;
        self.write_all(format!("{id} 0 R").as_bytes())
    }

    fn write_stream_obj(
        &mut self,
        dh: Handle<BTreeMap<Handle<PdfName>, Object>>,
        data: &std::sync::Arc<ferruginous_core::object::SublimatedData>,
    ) -> PdfResult<()> {
        if self.recursion_depth > 1 {
            // RR-15: Streams MUST be indirect objects. Inline streams are illegal.
            return Err(PdfError::Other(format!(
                "Attempted to write an inline stream at depth {} in object {} (illegal in PDF)",
                self.recursion_depth, self.current_obj_id
            ).into()));
        }

        let d = self
            .arena
            .get_dict(dh)
            .ok_or_else(|| PdfError::Other("Dictionary not found".into()))?;
        let filter_key = self.arena.name("Filter");
        let length_key = self.arena.name("Length");

        let is_sublimated = !matches!(**data, ferruginous_core::object::SublimatedData::Raw(_));
        let stream_bytes = self.arena.get_stream_bytes(data)?;
        
        let has_dct = d.get(&filter_key).map(|v| {
            let resolved = v.resolve(self.arena);
            if let Some(n) = resolved.as_name() {
                let s = self.arena.get_name_str(n).unwrap_or_default();
                s == "DCTDecode" || s == "DCT"
            } else if let Some(ah) = resolved.as_array() {
                self.arena.get_array(ah).unwrap_or_default().iter().any(|o| {
                    let s = o.resolve(self.arena).as_name().and_then(|n| self.arena.get_name_str(n)).unwrap_or_default();
                    s == "DCTDecode" || s == "DCT"
                })
            } else {
                false
            }
        }).unwrap_or(false);

        let (stream_data, already_filtered) = if has_dct {
            // Preservation: Never decompress JPEGs
            (stream_bytes, true)
        } else if is_sublimated {
            (stream_bytes, false)
        } else {
            // Check if it's already FlateDecode
            let is_flate = d.get(&filter_key).and_then(|o| o.resolve(self.arena).as_name()).and_then(|nh| self.arena.get_name_str(nh)).map(|s| s == "FlateDecode" || s == "Fl").unwrap_or(false);
            
            if is_flate && self.compression_level.is_some() {
                (stream_bytes, true)
            } else {
                self.prepare_stream_data(&stream_bytes, &d, Some(filter_key))
            }
        };
        
        let mut final_data = stream_data.to_vec();
        let applied_new_compression = self.try_compress_stream(&mut final_data, already_filtered);

        self.write_all(b"<<")?;
        for (k, v) in &d {
            if *k == length_key
                || (*k == filter_key && (applied_new_compression || !already_filtered))
            {
                continue;
            }
            self.write_all(b"\r\n")?;
            self.write_name(k)?;
            self.write_all(b" ")?;
            self.write_object(v)?;
        }

        if applied_new_compression {
            self.write_all(b"\r\n/Filter /FlateDecode")?;
        }

        let type_key = self.arena.get_name_by_str("Type");
        let is_metadata = type_key
            .and_then(|tk| d.get(&tk))
            .and_then(|o| o.as_name())
            .and_then(|nh| self.arena.get_name_str(nh))
            .as_deref()
            == Some("Metadata");
        if let Some(sh) = &self.security_handler
            && (!is_metadata || sh.should_decrypt_metadata())
        {
            final_data =
                sh.encrypt_stream(&final_data, self.current_obj_id, self.current_obj_gen)?;
        }

        self.write_all(format!("\r\n/Length {}", final_data.len()).as_bytes())?;
        self.write_all(b"\r\n>>\r\nstream\r\n")?;
        self.write_all(&final_data)?;
        self.write_all(b"\r\nendstream")
    }

    fn prepare_stream_data(
        &self,
        data: &bytes::Bytes,
        d: &BTreeMap<Handle<PdfName>, Object>,
        filter_key: Option<Handle<PdfName>>,
    ) -> (bytes::Bytes, bool) {
        if let Some(fk) = filter_key
            && d.contains_key(&fk)
        {
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
            if std::io::Write::write_all(&mut encoder, data).is_ok()
                && let Ok(compressed) = encoder.finish()
            {
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
            if b == b'#'
                || b <= 32
                || b >= 127
                || b == b'('
                || b == b')'
                || b == b'<'
                || b == b'>'
                || b == b'['
                || b == b']'
                || b == b'{'
                || b == b'}'
                || b == b'/'
                || b == b'%'
            {
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

    fn write_indirect_object(
        &mut self,
        id: u32,
        generation: u16,
        handle: Handle<Object>,
    ) -> PdfResult<()> {
        self.xref.insert(id, self.current_offset());
        self.current_obj_id = id;
        self.current_obj_gen = generation;
        self.write_all(format!("{id} {generation} obj\r\n").as_bytes())?;
        let obj = self
            .arena
            .get_object(handle)
            .ok_or_else(|| PdfError::Other(format!("Object {id} missing").into()))?;
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
        let id =
            *self.id_map.get(&sig_h).ok_or_else(|| PdfError::Other("Sig object missing".into()))?;
        let off =
            *self.xref.get(&id).ok_or_else(|| PdfError::Other("Sig offset missing".into()))?;

        let end = self.find_obj_end(off);
        let (c_start, c_end) = self.find_contents_offsets(off, end)?;
        let (br_start, br_end) = self.find_byte_range_offsets(off, end)?;

        let br_str = format!(
            "0 {:010} {:010} {:010}",
            c_start - 1,
            c_end + 1,
            self.buffer.len() - (c_end + 1)
        );
        let br_bytes = br_str.as_bytes();
        if br_bytes.len() > (br_end - br_start) {
            return Err(PdfError::Other("ByteRange overflow".into()));
        }

        for i in br_start..br_end {
            self.buffer[i] = b' ';
        }
        self.buffer[br_start..br_start + br_bytes.len()].copy_from_slice(br_bytes);
        Ok(())
    }

    fn find_obj_end(&self, start: usize) -> usize {
        let mut end = start;
        while end + 6 <= self.buffer.len() {
            if &self.buffer[end..end + 6] == b"endobj" {
                return end + 6;
            }
            end += 1;
        }
        end
    }

    fn find_contents_offsets(&self, start: usize, end: usize) -> PdfResult<(usize, usize)> {
        let key = b"/Contents <";
        let pos = self.buffer[start..end]
            .windows(key.len())
            .position(|w| w == key)
            .ok_or_else(|| PdfError::Other("Missing /Contents".into()))?;
        let c_start = start + pos + 11;
        let c_end_pos = self.buffer[c_start..end]
            .iter()
            .position(|&b| b == b'>')
            .ok_or_else(|| PdfError::Other("Missing end of /Contents".into()))?;
        Ok((c_start, c_start + c_end_pos))
    }

    fn find_byte_range_offsets(&self, start: usize, end: usize) -> PdfResult<(usize, usize)> {
        let key = b"/ByteRange [";
        let pos = self.buffer[start..end]
            .windows(key.len())
            .position(|w| w == key)
            .ok_or_else(|| PdfError::Other("Missing /ByteRange".into()))?;
        let br_start = start + pos + 12;
        let br_end_pos = self.buffer[br_start..end]
            .iter()
            .position(|&b| b == b']')
            .ok_or_else(|| PdfError::Other("Missing end of /ByteRange".into()))?;
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
    fn finish_linearized(
        &mut self,
        root: Handle<Object>,
        info: Option<Handle<Object>>,
    ) -> PdfResult<()> {
        self.buffer.clear();
        self.xref.clear();
        self.id_map.clear();
        
        // 1. Header
        self.write_all(b"%PDF-2.0\r\n%\xe2\xe3\xcf\xd3\r\n")?;

        let (s2, s6, others, pgs, counts) = self.collect_lin_objects(root, info)?;
        let (total_size, primary_count) = self.assign_lin_ids(root, &s2, &s6, &others, pgs[0]);

        // 2. Section 1: Linearization Dictionary and First Xref (Reserved)
        let (dict_pos, p_xref_pos) = self.reserve_lin_headers(primary_count);
        
        // 3. Section 2 & 6: Write objects
        let (hint_pos, s6_start) = self.write_lin_objects_to_stream(root, &s2, &s6, &others, pgs.len())?;

        // 4. Main Xref and Trailer
        let main_xref_off = self.write_lin_main_xref(total_size, primary_count, p_xref_pos)?;
        
        // Resolve Page 1 object (ID 4) offset for the hint table
        let p1_off = *self.xref.get(&4).ok_or_else(|| PdfError::Other("Page 1 missing".into()))?;

        let state = LinState {
            dict_pos,
            pxref_pos: p_xref_pos,
            hint_pos,
            page1_offset: p1_off,
            page1_end: s6_start,
            main_xref_offset: main_xref_off,
            pages: pgs,
            page_obj_counts: counts,
            total_size,
            primary_count,
        };
        self.finalize_lin_headers(state)?;
        Ok(())
    }

    fn write_lin_objects_to_stream(
        &mut self,
        root: Handle<Object>,
        s2: &[Handle<Object>],
        s6: &[Handle<Object>],
        others: &[Handle<Object>],
        page_count: usize,
    ) -> PdfResult<(usize, usize)> {
        // RR-15: The Primary Hint Stream MUST be the first object in Section 2.
        let (hint_pos, ..) = self.reserve_hint_stream(page_count);
        
        self.write_indirect_object(2, 0, root)?;
        for &h in s2 {
            if h != root {
                let id = self.id_map[&h];
                self.write_indirect_object(id, 0, h)?;
            }
        }

        let s6_start = self.current_offset();
        for &h in s6 {
            let id = self.id_map[&h];
            self.write_indirect_object(id, 0, h)?;
        }

        for &h in others {
            let id = self.id_map[&h];
            self.write_indirect_object(id, 0, h)?;
        }
        Ok((hint_pos, s6_start))
    }

    fn collect_lin_objects(
        &self,
        root: Handle<Object>,
        info: Option<Handle<Object>>,
    ) -> PdfResult<(Vec<Handle<Object>>, Vec<Handle<Object>>, Vec<Handle<Object>>, Vec<Handle<Object>>, Vec<u32>)> {
        let mut all = BTreeSet::new();
        self.trace_reachable(Object::Reference(root), &mut all);
        if let Some(ih) = info {
            self.trace_reachable(Object::Reference(ih), &mut all);
        }

        let mut original_pages = Vec::new();
        if let Some(dh) = self.arena.get_object(root).and_then(|o| o.as_dict_handle())
            && let Some(dict) = self.arena.get_dict(dh)
            && let Some(Object::Reference(ph)) = dict.get(&self.arena.name("Pages"))
        {
            self.collect_pages_recursive(*ph, &mut original_pages)?;
        }

        if original_pages.is_empty() {
            return Err(PdfError::Other(format!("No pages found (Catalog root: {:?})", root).into()));
        }

        let mut assigned = BTreeSet::new();
        let mut page_obj_counts = Vec::new();
        let mut section2 = Vec::new();
        let mut section6 = Vec::new();

        // Section 2: Catalog, Info, Page 1 and its exclusive resources
        assigned.insert(root);
        if let Some(ih) = info { assigned.insert(ih); }

        let mut p1_reachable = BTreeSet::new();
        self.trace_reachable_no_parent(original_pages[0], &mut p1_reachable);
        
        // Ancestors of Page 1 (Page Tree nodes)
        let mut curr = original_pages[0];
        while let Some(parent) = self.get_parent_handle(curr) {
            if assigned.insert(parent) {
                section2.push(parent);
                curr = parent;
            } else { break; }
        }

        let mut p1_objs = vec![original_pages[0]];
        assigned.insert(original_pages[0]);
        for h in p1_reachable {
            if assigned.insert(h) {
                p1_objs.push(h);
            }
        }
        section2.extend(p1_objs);
        if let Some(ih) = info { section2.push(ih); }

        // Page 1 count = section2 objects + Catalog(1) + HintStream(1)
        page_obj_counts.push(section2.len() as u32 + 1); // +1 for Hint Stream (Catalog is already in section2)
        // Wait, root is in assigned but not necessarily in section2 yet (except if added manually)
        // Let's be explicit:
        let mut section2_final = vec![root];
        if let Some(ih) = info { section2_final.push(ih); }
        for h in section2 {
            if h != root && info != Some(h) {
                section2_final.push(h);
            }
        }
        // Section 2 in file: [Hint Stream(ID 3), Catalog(ID 2), ...others]
        // Catalog is ID 2. Page 1 is ID 4.
        page_obj_counts[0] = section2_final.len() as u32 + 1; // +1 for ID 3

        // Section 6: Other pages
        for &ph in &original_pages[1..] {
            let mut p_reachable = BTreeSet::new();
            self.trace_reachable_no_parent(ph, &mut p_reachable);
            
            let mut p_objs = vec![ph];
            assigned.insert(ph);
            for h in p_reachable {
                if assigned.insert(h) {
                    p_objs.push(h);
                }
            }
            page_obj_counts.push(p_objs.len() as u32);
            section6.extend(p_objs);
        }

        // "Others": anything remaining (shared objects, etc.)
        let others: Vec<_> = all.into_iter().filter(|h| !assigned.contains(h)).collect();

        Ok((section2_final, section6, others, original_pages, page_obj_counts))
    }

    fn get_parent_handle(&self, h: Handle<Object>) -> Option<Handle<Object>> {
        let dh = self.arena.get_object(h)?.as_dict_handle()?;
        let d = self.arena.get_dict(dh)?;
        let parent = d.get(&self.arena.name("Parent"))?.as_reference()?;
        Some(parent)
    }

    fn trace_reachable_selective(
        &self,
        h: Handle<Object>,
        reachable: &mut BTreeSet<Handle<Object>>,
        exclude_keys: &[&str],
    ) {
        let _ = reachable.insert(h);
        let Some(root_obj) = self.arena.get_object(h) else {
            return;
        };
        let mut stack = vec![root_obj];
        while let Some(obj) = stack.pop() {
            match obj {
                Object::Reference(rh) => {
                    if reachable.insert(rh) && let Some(inner) = self.arena.get_object(rh) {
                        stack.push(inner);
                    }
                }
                Object::Array(ah) => {
                    if let Some(a) = self.arena.get_array(ah) {
                        for item in a {
                            stack.push(item);
                        }
                    }
                }
                Object::Dictionary(dh) | Object::Stream(dh, _) => {
                    if let Some(d) = self.arena.get_dict(dh) {
                        for (k, v) in d {
                            let k_str = self.arena.get_name_str(k).unwrap_or_default();
                            if exclude_keys.contains(&k_str.as_str()) {
                                continue;
                            }
                            stack.push(v);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    fn trace_reachable_no_parent(&self, h: Handle<Object>, reachable: &mut BTreeSet<Handle<Object>>) {
        self.trace_reachable_selective(h, reachable, &["Parent"]);
    }

    fn assign_lin_ids(
        &mut self,
        root: Handle<Object>,
        section2: &[Handle<Object>],
        section6: &[Handle<Object>],
        others: &[Handle<Object>],
        page1: Handle<Object>,
    ) -> (u32, u32) {
        self.id_map.insert(root, 2);
        self.id_map.insert(page1, 4);
        // Note: ID 3 is for Hint Stream, but it's not a handle from the arena.
        
        let mut next_id = 5;
        for &h in section2 {
            self.id_map.entry(h).or_insert_with(|| {
                let id = next_id;
                next_id += 1;
                id
            });
        }
        let primary_count = next_id;
        for &h in section6 {
            self.id_map.entry(h).or_insert_with(|| {
                let id = next_id;
                next_id += 1;
                id
            });
        }
        for &h in others {
            self.id_map.entry(h).or_insert_with(|| {
                let id = next_id;
                next_id += 1;
                id
            });
        }
        (next_id, primary_count)
    }

    fn reserve_lin_headers(&mut self, _primary_count: u32) -> (usize, usize) {
        let dict_pos = self.current_offset();
        self.xref.insert(1, dict_pos);
        // Fixed 4096 bytes for linearization dictionary (generous)
        self.buffer.extend_from_slice(&vec![b' '; 4096]);
        let p_xref_pos = self.current_offset();
        // RR-15 HARDENING: Increased to 65536 bytes to accommodate large primary sections (up to 3000 objects)
        self.buffer.extend_from_slice(&vec![b' '; 65536]);
        (dict_pos, p_xref_pos)
    }

    fn reserve_hint_stream(&mut self, _page_count: usize) -> (usize, usize, usize, usize) {
        let pos = self.current_offset();
        self.xref.insert(3, pos);
        self.current_obj_id = 3;
        self.current_obj_gen = 0;
        // Fixed 65536 bytes for hint stream
        let total_reserve = 65536;
        self.buffer.extend_from_slice(&vec![b' '; total_reserve]);
        (pos, pos + 25, 0, 0) // Lengths calculated later
    }


    fn write_lin_main_xref(&mut self, total_size: u32, _primary_count: u32, prev_xref: usize) -> PdfResult<usize> {
        let off = self.current_offset();
        self.write_all(format!("xref\r\n0 {total_size}\r\n0000000000 65535 f\r\n").as_bytes())?;
        for id in 1..total_size {
            let o = self.xref.get(&id).copied().unwrap_or(0);
            if o == 0 {
                self.write_all(b"0000000000 65535 f\r\n")?;
            } else {
                self.write_all(format!("{o:010} 00000 n\r\n").as_bytes())?;
            }
        }
        let id_hex = "f00baa42f00baa42f00baa42f00baa42";
        // RR-15: Include /Prev to point to the first xref for compatibility
        self.write_all(format!("trailer\r\n<< /Size {total_size} /Prev {prev_xref} /Root 2 0 R /ID [<{id_hex}> <{id_hex}>] >>\r\nstartxref\r\n{off}\r\n%%EOF\r\n").as_bytes())?;
        Ok(off)
    }

    fn finalize_lin_headers(&mut self, s: LinState) -> PdfResult<()> {
        let id_hex = "f00baa42f00baa42f00baa42f00baa42";
        
        // 1. Generate Hint Stream
        let p_len = 36 + (s.pages.len() * 6);
        let s_len = 20; 
        let data_len = p_len + s_len;
        let h_dict = format!("3 0 obj\r\n<< /Length {data_len} /S {p_len} >>\r\nstream\r\n");
        let h_data = self.generate_hint_tables(
            &s.pages,
            &[],
            s.page1_offset,
            s.main_xref_offset,
            &s.page_obj_counts,
        );
        let h_footer = "\r\nendstream\r\nendobj\r\n";
        let mut full_h = Vec::new();
        full_h.extend_from_slice(h_dict.as_bytes());
        full_h.extend_from_slice(&h_data);
        full_h.extend_from_slice(h_footer.as_bytes());
        
        // 2. Generate Linearization Dictionary
        let h_total_len = full_h.len();
        let d_str = format!(
            "1 0 obj\r\n<< /Linearized 1 /L {} /P 0 /O 4 /E {} /N {} /T {} /H [{} {}] >>\r\nendobj\r\n",
            self.buffer.len(),
            s.page1_end,
            s.pages.len(),
            s.main_xref_offset,
            s.hint_pos,
            h_total_len
        );
        self.overwrite_with_padding(s.dict_pos, d_str.into_bytes(), 4096)?;

        // Overwrite hint stream (moves full_h)
        self.overwrite_with_padding(s.hint_pos, full_h, 65536)?;

        // 3. Generate First Xref Table
        let mut px = format!("xref\r\n0 {}\r\n0000000000 65535 f\r\n", s.primary_count);
        for id in 1..s.primary_count {
            let _ = write!(px, "{:010} 00000 n\r\n", self.xref.get(&id).unwrap_or(&0));
        }
        let _ = write!(
            px,
            "trailer\r\n<< /Size {} /Root 2 0 R /ID [<{id_hex}> <{id_hex}>] >>\r\n",
            s.total_size
        );
        self.overwrite_with_padding(s.pxref_pos, px.into_bytes(), 65536)?;
        Ok(())
    }

    fn overwrite_with_padding(&mut self, pos: usize, mut data: Vec<u8>, reserve_size: usize) -> PdfResult<()> {
        if data.len() > reserve_size {
            return Err(PdfError::Other(format!(
                "Linearization header overflow: data len {} exceeds reserved {} at pos {}",
                data.len(), reserve_size, pos
            ).into()));
        } else {
            // Pad with spaces
            data.extend(vec![b' '; reserve_size - data.len()]);
        }
        self.buffer[pos..pos + reserve_size].copy_from_slice(&data);
        Ok(())
    }
}

struct LinState {
    dict_pos: usize,
    pxref_pos: usize,
    hint_pos: usize,
    page1_offset: usize,
    page1_end: usize,
    main_xref_offset: usize,
    pages: Vec<Handle<Object>>,
    page_obj_counts: Vec<u32>,
    total_size: u32,
    primary_count: u32,
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
        // Items 6-13 (20 bytes total)
        for _ in 0..10 {
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

        // Pad to byte boundary
        writer.finish()
    }

    fn trace_reachable(&self, initial_obj: Object, reachable: &mut BTreeSet<Handle<Object>>) {
        let mut stack = vec![initial_obj];
        while let Some(obj) = stack.pop() {
            match obj {
                Object::Reference(h) => {
                    if reachable.insert(h)
                        && let Some(inner) = self.arena.get_object(h)
                    {
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
        h: Handle<Object>,
        pages: &mut Vec<Handle<Object>>,
    ) -> PdfResult<()> {
        let Some(obj) = self.arena.get_object(h) else {
            return Ok(());
        };
        let Some(dh) = obj.as_dict_handle() else {
            return Ok(());
        };
        let dict = self.arena.get_dict(dh).unwrap_or_default();

        let type_name = dict.get(&self.arena.name("Type")).and_then(|v| v.as_name());
        let type_str = type_name.and_then(|tn| self.arena.get_name_str(tn));

        if type_str.as_deref() == Some("Page") {
            pages.push(h);
        } else {
            // Pages node
            if let Some(kids_h) = dict.get(&self.arena.name("Kids")).and_then(|v| v.as_array()) {
                if let Some(kids) = self.arena.get_array(kids_h) {
                    for kid in kids {
                        if let Some(kid_h) = kid.as_reference() {
                            self.collect_pages_recursive(kid_h, pages)?;
                        }
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
