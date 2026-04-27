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
            Object::String(s) => {
                let mut data = s.to_vec();
                if let Some(sh) = &self.security_handler {
                    data = sh.encrypt_stream(&data, self.current_obj_id, self.current_obj_gen)?;
                }
                self.write_string_literal(&data)
            }
            Object::Hex(s) => {
                let mut data = s.to_vec();
                if let Some(sh) = &self.security_handler {
                    data = sh.encrypt_stream(&data, self.current_obj_id, self.current_obj_gen)?;
                }
                self.write_string_hex(&data)
            }
            Object::Name(n) => self.write_name(n),
            Object::Array(h) => {
                let a = self
                    .arena
                    .get_array(*h)
                    .ok_or_else(|| PdfError::Other("Array not found".into()))?;
                self.write_all(b"[")?;
                for (i, item) in a.iter().enumerate() {
                    if i > 0 {
                        self.write_all(b" ")?;
                    }
                    self.write_object(item)?;
                }
                self.write_all(b"]")
            }
            Object::Dictionary(h) => {
                let d = self
                    .arena
                    .get_dict(*h)
                    .ok_or_else(|| PdfError::Other("Dictionary not found".into()))?;
                self.write_dict(&d)
            }
            Object::Stream(dh, data) => {
                let d = self
                    .arena
                    .get_dict(*dh)
                    .ok_or_else(|| PdfError::Other("Dictionary not found".into()))?;
                let filter_key = self.arena.get_name_by_str("Filter");
                let length_key = self.arena.get_name_by_str("Length");

                let mut stream_data = data.clone();
                let mut already_filtered = false;

                // Check if the stream is already compressed/filtered
                if let Some(fk) = filter_key && d.contains_key(&fk) {
                    if let Ok(decompressed) = self.arena.process_filters(data, &d) {
                        // Successfully decompressed, we can now treat it as raw data
                        stream_data = decompressed;
                    } else {
                        // Decompression failed or unsupported. 
                        already_filtered = true;
                    }
                }

                let applied_new_compression = if !already_filtered && let Some(level) = self.compression_level {
                    use flate2::Compression;
                    use flate2::write::ZlibEncoder;
                    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::new(level));
                    std::io::Write::write_all(&mut encoder, &stream_data).map_err(|e| PdfError::Other(e.to_string().into()))?;
                    stream_data = bytes::Bytes::from(encoder.finish().map_err(|e| PdfError::Other(e.to_string().into()))?);
                    true
                } else {
                    false
                };

                self.write_all(b"<<")?;
                for (k, v) in d {
                    // Skip Length (always re-calculated) and Filter (only if we are applying a new one)
                    if Some(k) == length_key {
                        continue;
                    }
                    if Some(k) == filter_key && (applied_new_compression || !already_filtered) {
                        continue;
                    }
                    self.write_all(b"\r\n")?;
                    self.write_name(&k)?;
                    self.write_all(b" ")?;
                    self.write_object(&v)?;
                }
                if applied_new_compression {
                    self.write_all(b"\r\n/Filter /FlateDecode")?;
                }
                
                let mut final_data = stream_data.to_vec();
                if let Some(sh) = &self.security_handler {
                    final_data = sh.encrypt_stream(&final_data, self.current_obj_id, self.current_obj_gen)?;
                }

                self.write_all(format!("\r\n/Length {}", final_data.len()).as_bytes())?;
                self.write_all(b"\r\n>>")?;
                self.write_all(b"\r\nstream\r\n")?;
                self.write_all(&final_data)?;
                self.write_all(b"\r\nendstream")
            }
            Object::Null => self.write_all(b"null"),
            Object::Reference(h) => {
                let id = self
                    .id_map
                    .get(h)
                    .copied()
                    .unwrap_or_else(|| h.index() + 1);
                self.write_all(format!("{id} 0 R").as_bytes())
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
        let Some(sig_handle) = self.sig_handle else { return Ok(()) };
        let id = *self
            .id_map
            .get(&sig_handle)
            .ok_or_else(|| PdfError::Other("Signature object not found".into()))?;
        let obj_offset = *self
            .xref
            .get(&id)
            .ok_or_else(|| PdfError::Other("Signature offset not found".into()))?;
        let mut obj_end = obj_offset;
        while obj_end + 6 <= self.buffer.len() {
            if &self.buffer[obj_end..obj_end + 6] == b"endobj" {
                obj_end += 6;
                break;
            }
            obj_end += 1;
        }
        let obj_range = obj_offset..obj_end;
        let contents_key = b"/Contents <";
        let c_pos = self.buffer[obj_range.clone()]
            .windows(contents_key.len())
            .position(|w| w == contents_key)
            .ok_or_else(|| PdfError::Other("Missing /Contents".into()))?;
        let contents_start = obj_offset + c_pos + 11;
        let c_end_pos = self.buffer[contents_start..obj_end]
            .iter()
            .position(|&b| b == b'>')
            .ok_or_else(|| PdfError::Other("Missing end of /Contents".into()))?;
        let contents_end = contents_start + c_end_pos;
        let br_key = b"/ByteRange [";
        let br_pos = self.buffer[obj_range]
            .windows(br_key.len())
            .position(|w| w == br_key)
            .ok_or_else(|| PdfError::Other("Missing /ByteRange".into()))?;
        let br_start = obj_offset + br_pos + 12;
        let br_end_pos = self.buffer[br_start..obj_end]
            .iter()
            .position(|&b| b == b']')
            .ok_or_else(|| PdfError::Other("Missing end of /ByteRange".into()))?;
        let br_end = br_start + br_end_pos;
        let gap_start = contents_start - 1;
        let gap_end = contents_end + 1;
        let br_str = format!("0 {:010} {:010} {:010}", gap_start, gap_end, self.buffer.len() - gap_end);
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
        root_handle: Handle<Object>,
        info_handle: Option<Handle<Object>>,
    ) -> PdfResult<()> {
        let mut all_reachable = BTreeSet::new();
        self.trace_reachable(Object::Reference(root_handle), &mut all_reachable);
        if let Some(ih) = info_handle {
            self.trace_reachable(Object::Reference(ih), &mut all_reachable);
        }

        let mut page_handles = Vec::new();
        if let Some(Object::Dictionary(dh)) = self.arena.get_object(root_handle)
            && let Some(dict) = self.arena.get_dict(dh)
            && let Some(pages_obj) = dict.get(&self.arena.name("Pages"))
            && let Object::Reference(pages_handle) = pages_obj
            && let Some(pages_dh) =
                self.arena.get_object(*pages_handle).and_then(|o| o.as_dict_handle())
        {
            self.collect_pages_recursive(pages_dh, &mut page_handles)?;
        }
        let page_count = page_handles.len() as u32;

        let mut ref_counts: BTreeMap<Handle<Object>, usize> = BTreeMap::new();
        for &page_h in &page_handles {
            let mut page_reachable = BTreeSet::new();
            self.trace_page_resources(page_h, &mut page_reachable);
            for &h in &page_reachable {
                *ref_counts.entry(h).or_default() += 1;
            }
        }
        let shared_objects: Vec<_> = ref_counts
            .iter()
            .filter(|&(h, count)| *count > 1 && !page_handles.contains(h) && *h != root_handle)
            .map(|(&h, _)| h)
            .collect();

        let mut next_id = 3;
        let mut page_ids = Vec::new();
        for &h in &page_handles {
            self.id_map.insert(h, next_id);
            page_ids.push(next_id);
            next_id += 1;
        }

        let mut shared_ids = Vec::new();
        for &h in &shared_objects {
            self.id_map.insert(h, next_id);
            shared_ids.push(next_id);
            next_id += 1;
        }

        let mut assigned = BTreeSet::new();
        assigned.insert(root_handle);
        
        let mut ordered_objects = Vec::new();
        for page_h in page_handles.iter().copied() {
            if assigned.insert(page_h) {
                ordered_objects.push(page_h);
            }
        }
        for h in shared_objects {
            if assigned.insert(h) {
                ordered_objects.push(h);
            }
        }
        
        let mut others: Vec<_> =
            all_reachable.into_iter().filter(|h| !assigned.contains(h)).collect();
        others.sort_by_key(|h| h.index());
        for h in others {
            self.id_map.insert(h, next_id);
            ordered_objects.push(h);
            next_id += 1;
        }
        let total_size = next_id;
        let primary_count = 5 + page_ids.len() as u32 + shared_ids.len() as u32;
        
        let mut obj_counts = Vec::new();
        for &page_h in &page_handles {
            let mut page_reachable = BTreeSet::new();
            self.trace_page_resources(page_h, &mut page_reachable);
            obj_counts.push((1 + page_reachable.len()) as u32);
        }

        self.write_all(b"%PDF-2.0\r\n%\xe2\xe3\xcf\xD3\r\n")?;
        let dict_pos = self.current_offset();
        self.write_all(&vec![b' '; 256])?;
        let primary_xref_pos = self.current_offset();
        let primary_xref_size = 256 + (primary_count as usize * 20);
        self.write_all(&vec![b' '; primary_xref_size])?;

        self.write_indirect_object(2, 0, root_handle)?;

        let hint_table_pos = self.current_offset();
        self.xref.insert(3, hint_table_pos);
        self.current_obj_id = 3;
        self.current_obj_gen = 0;
        let page_hint_len = 36 + (page_count as usize * 6);
        let shared_hint_len = 20;
        let hint_data_len = page_hint_len + shared_hint_len;
        let hint_footer = "\r\nendstream\r\nendobj\r\n";
        self.write_all(&vec![b' '; 64 + hint_data_len + hint_footer.len()])?;

        let page1_start = self.current_offset();
        if !page_handles.is_empty() {
            self.write_indirect_object(4, 0, page_handles[0])?;
        }

        for h in ordered_objects {
            let id = self.id_map[&h];
            self.write_indirect_object(id, 0, h)?;
        }
        let page1_end = *self.xref.get(&(primary_count - 1)).unwrap_or(&self.buffer.len());

        let main_xref_offset = self.current_offset();
        let xref_header = format!("xref\r\n0 {total_size}\r\n");
        let t_offset = main_xref_offset + xref_header.len();
        self.write_all(xref_header.as_bytes())?;
        self.write_all(b"0000000000 65535 f\r\n")?;
        for id in 1..total_size {
            let off = self.xref.get(&id).copied().unwrap_or(0);
            self.write_all(format!("{off:010} 00000 n\r\n").as_bytes())?;
        }

        let id_hex = "f00baa42f00baa42f00baa42f00baa42";
        self.write_all(format!("trailer\r\n<< /Size {total_size} /Root 2 0 R /ID [<{id_hex}> <{id_hex}>] >>\r\nstartxref\r\n{main_xref_offset}\r\n%%EOF\r\n").as_bytes())?;

        let hint_dict = format!("3 0 obj\r\n<< /Length {hint_data_len} /S {page_hint_len} >>\r\nstream\r\n");
        let h_stream_start = hint_table_pos + hint_dict.len();
        let hint_data =
            self.generate_hint_tables(&page_handles, &shared_ids, page1_start, main_xref_offset, &obj_counts);
        let hint_full =
            format!("{hint_dict}{}{hint_footer}", std::str::from_utf8(&hint_data).unwrap_or(""));
        self.buffer[hint_table_pos..hint_table_pos + hint_full.len()]
            .copy_from_slice(hint_full.as_bytes());

        let dict_str = format!(
            "1 0 obj\r\n<< /Linearized 1 /L {} /P 0 /O 4 /E {} /N {} /T {} /H [{} {} {} {}] >>\r\nendobj\r\n",
            self.buffer.len(),
            page1_end,
            page_count,
            t_offset,
            h_stream_start,
            page_hint_len,
            h_stream_start + page_hint_len,
            shared_hint_len
        );
        for i in dict_pos..dict_pos + 256 {
            self.buffer[i] = b' ';
        }
        self.buffer[dict_pos..dict_pos + dict_str.len()].copy_from_slice(dict_str.as_bytes());

        let mut p_xref = format!("xref\r\n0 {primary_count}\r\n0000000000 65535 f\r\n");
        for id in 1..primary_count {
            let _ = write!(
                p_xref,
                "{:010} 00000 n\r\n",
                self.xref.get(&id).unwrap_or(&0)
            );
        }
        let _ = write!(
            p_xref,
            "trailer\r\n<< /Size {total_size} /Prev {main_xref_offset} /Root 2 0 R /ID [<{id_hex}> <{id_hex}>] >>\r\n"
        );
        let p_xref_bytes = p_xref.as_bytes();
        for i in primary_xref_pos..primary_xref_pos + primary_xref_size {
            self.buffer[i] = b' ';
        }
        self.buffer[primary_xref_pos..primary_xref_pos + p_xref_bytes.len()]
            .copy_from_slice(p_xref_bytes);

        Ok(())
    }

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

    fn trace_page_resources(
        &self,
        initial_h: Handle<Object>,
        visited: &mut BTreeSet<Handle<Object>>,
    ) {
        let mut stack = vec![Object::Reference(initial_h)];
        while let Some(obj) = stack.pop() {
            match obj {
                Object::Reference(h) => {
                    if visited.insert(h)
                        && let Some(inner) = self.arena.get_object(h) {
                        stack.push(inner.clone());
                    }
                }
                Object::Array(ah) => {
                    if let Some(a) = self.arena.get_array(ah) {
                        for item in a {
                            stack.push(item.clone());
                        }
                    }
                }
                Object::Dictionary(dh) | Object::Stream(dh, _) => {
                    if let Some(dict) = self.arena.get_dict(dh) {
                        for (k, v) in dict {
                            let name = self.arena.get_name_str(k).unwrap_or_default();
                            // Skip tree traversal keys to avoid cycles or unnecessary depth
                            if name == "Parent" || name == "Prev" || name == "Next" {
                                continue;
                            }
                            stack.push(v.clone());
                        }
                    }
                }
                _ => {}
            }
        }
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
