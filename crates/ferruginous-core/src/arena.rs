use crate::PdfResult;
use crate::handle::Handle;
use crate::object::{Object, ObjectEntry, PdfName, SublimatedData};
use bytes::Bytes;
use parking_lot::RwLock;
use std::collections::BTreeMap;
use std::sync::Arc;
// Use absolute path for workspace dependency
use ::image;

/// A sequential arena for PDF objects, optimized for cache locality and thread safety.
///
/// This implementation uses an `Arc<ArenaInner>` to enable zero-copy cloning of the arena
/// handle, allowing multiple components (Ingestor, Refinery, CLI) to share the same
/// document state without expensive duplication.
#[derive(Default, Clone)]
pub struct PdfArena {
    inner: Arc<ArenaInner>,
}

/// The internal heap-allocated state of the PdfArena.
///
/// All pools are wrapped in `RwLock` to allow thread-safe interior mutability, enabling the
/// "Pass-based" refinement system where objects are updated in-place as they move
/// through the normalization pipeline.
#[derive(Default)]
struct ArenaInner {
    /// Contiguous pool of objects for maximum cache efficiency.
    objects: RwLock<Vec<ObjectEntry>>,
    /// Dedicated pools for complex types to allow typesafe handles.
    /// This separation prevents "Handle Confusion" (e.g., using an Array handle to access a Dictionary).
    dicts: RwLock<Vec<BTreeMap<Handle<PdfName>, Object>>>,
    arrays: RwLock<Vec<Vec<Object>>>,
    /// Interned names to ensure all `/Name` references in the document point to a single memory location.
    names: RwLock<Vec<PdfName>>,
    /// Index for fast name lookup during interning.
    name_map: RwLock<BTreeMap<PdfName, Handle<PdfName>>>,
    /// The document version (e.g., 1.7, 2.0).
    version: RwLock<f32>,
}

impl PdfArena {
    pub fn new() -> Self {
        Self::with_version(1.7)
    }

    pub fn with_version(version: f32) -> Self {
        let arena = Self::default();
        *arena.inner.version.write() = version;
        arena
    }

    pub fn version(&self) -> f32 {
        *self.inner.version.read()
    }

    pub fn set_version(&self, version: f32) {
        *self.inner.version.write() = version;
    }

    /// Interns a name, returning a deduplicated handle.
    pub fn intern_name(&self, name: PdfName) -> Handle<PdfName> {
        if let Some(h) = self.inner.name_map.read().get(&name) {
            return *h;
        }

        let mut map = self.inner.name_map.write();
        let mut names = self.inner.names.write();

        // Re-check after acquiring write lock
        if let Some(h) = map.get(&name) {
            return *h;
        }

        let h = Handle::new(names.len() as u32);
        names.push(name.clone());
        map.insert(name, h);
        h
    }

    /// Returns a handle for a name, interning it if necessary (Get-or-Create).
    pub fn name(&self, name: &str) -> Handle<PdfName> {
        if let Some(h) = self.inner.name_map.read().get(name) {
            return *h;
        }
        self.intern_name(PdfName::new(name))
    }

    /// Returns the string representation of a name handle.
    pub fn get_name_str(&self, handle: Handle<PdfName>) -> Option<String> {
        self.inner.names.read().get(handle.index() as usize).map(|n| n.as_str().to_string())
    }

    pub fn get_name_by_str(&self, name: &str) -> Option<Handle<PdfName>> {
        self.inner.name_map.read().get(name).copied()
    }

    pub fn get_name(&self, handle: Handle<PdfName>) -> Option<PdfName> {
        self.inner.names.read().get(handle.index() as usize).cloned()
    }

    /// Returns all valid dictionary handles in the arena.
    pub fn all_dict_handles(&self) -> Vec<Handle<BTreeMap<Handle<PdfName>, Object>>> {
        let count = self.inner.dicts.read().len() as u32;
        (0..count).map(Handle::new).collect()
    }

    /// Registers a new object, returning a unique handle.
    pub fn alloc_object(&self, object: Object) -> Handle<Object> {
        let mut objects = self.inner.objects.write();
        let h = Handle::new(objects.len() as u32);
        objects.push(ObjectEntry { object, generation: 0 });
        h
    }

    /// Allocates a dictionary.
    pub fn alloc_dict(
        &self,
        dict: BTreeMap<Handle<PdfName>, Object>,
    ) -> Handle<BTreeMap<Handle<PdfName>, Object>> {
        let mut dicts = self.inner.dicts.write();
        let index = u32::try_from(dicts.len()).unwrap_or(u32::MAX);
        if index == u32::MAX {
            return Handle::new(0);
        }
        dicts.push(dict);
        Handle::new(index)
    }

    /// Allocates an array.
    pub fn alloc_array(&self, array: Vec<Object>) -> Handle<Vec<Object>> {
        let mut arrays = self.inner.arrays.write();
        let index = u32::try_from(arrays.len()).unwrap_or(u32::MAX);
        if index == u32::MAX {
            return Handle::new(0);
        }
        arrays.push(array);
        Handle::new(index)
    }

    pub fn get_object(&self, handle: Handle<Object>) -> Option<Object> {
        self.inner.objects.read().get(handle.index() as usize).map(|e| e.object.clone())
    }

    pub fn set_object(&self, handle: Handle<Object>, object: Object) {
        if let Some(e) = self.inner.objects.write().get_mut(handle.index() as usize) {
            e.object = object;
        }
    }

    pub fn get_object_entry(&self, handle: Handle<Object>) -> Option<ObjectEntry> {
        self.inner.objects.read().get(handle.index() as usize).cloned()
    }

    pub fn set_object_entry(&self, handle: Handle<Object>, entry: ObjectEntry) {
        if let Some(e) = self.inner.objects.write().get_mut(handle.index() as usize) {
            *e = entry;
        }
    }

    /// Retrieves a dictionary.
    pub fn get_dict(
        &self,
        handle: Handle<BTreeMap<Handle<PdfName>, Object>>,
    ) -> Option<BTreeMap<Handle<PdfName>, Object>> {
        self.inner.dicts.read().get(handle.index() as usize).cloned()
    }

    /// Updates an existing dictionary.
    pub fn set_dict(
        &self,
        handle: Handle<BTreeMap<Handle<PdfName>, Object>>,
        dict: BTreeMap<Handle<PdfName>, Object>,
    ) {
        if let Some(d) = self.inner.dicts.write().get_mut(handle.index() as usize) {
            *d = dict;
        }
    }

    /// Retrieves an array.
    pub fn get_array(&self, handle: Handle<Vec<Object>>) -> Option<Vec<Object>> {
        self.inner.arrays.read().get(handle.index() as usize).cloned()
    }

    /// Searches for an existing indirect object that matches the provided object.
    pub fn find_object(&self, object: &Object) -> Option<Handle<Object>> {
        let objects = self.inner.objects.read();
        for (i, entry) in objects.iter().enumerate() {
            if &entry.object == object {
                return Some(Handle::new(i as u32));
            }
        }
        None
    }

    pub fn object_count(&self) -> u32 {
        self.inner.objects.read().len() as u32
    }

    /// Applies filters to data using the stream dictionary context.
    pub fn process_filters(
        &self,
        data: &[u8],
        dict: &BTreeMap<Handle<PdfName>, Object>,
    ) -> PdfResult<Bytes> {
        crate::filters::process_arena_filters(data, dict, self)
    }

    /// Performs sublimation (normalization) on all objects in the arena.
    pub fn sublimate_all(&self) -> PdfResult<()> {
        for i in 0..self.object_count() {
            let handle = Handle::new(i);
            if let Some(Object::Stream(dh, data_arc)) = self.get_object(handle) {
                // If it's already structured, skip
                if matches!(&*data_arc, SublimatedData::Commands(_) | SublimatedData::Image { .. })
                {
                    continue;
                }

                // Determine if it's a content stream
                let is_content = if let Some(dict) = self.get_dict(dh) {
                    let subtype: Option<String> = dict
                        .get(&self.name("Subtype"))
                        .and_then(|o| o.resolve(self).as_name())
                        .and_then(|n| self.get_name(n))
                        .map(|n| n.as_str().to_string());
                    subtype.is_none() || subtype.as_deref() == Some("Form")
                } else {
                    false
                };

                // Get raw bytes (decompressing if it was Compressed)
                let raw_bytes = self.get_stream_bytes(&data_arc)?;
                self.sublimate_stream(handle, raw_bytes, is_content)?;
            }
        }
        Ok(())
    }

    /// Sublimates raw stream data into a compressed or structured format.
    pub fn sublimate_stream(
        &self,
        handle: Handle<Object>,
        data: Bytes,
        is_content_stream: bool,
    ) -> PdfResult<()> {
        let sublimated = if is_content_stream {
            // STUB: This will be replaced by the real IR parser in Pass 2
            SublimatedData::Raw(data)
        } else if let Some(Object::Stream(dh, _)) = self.get_object(handle)
            && let Some(dict) = self.get_dict(dh)
            && {
                
                dict
                    .get(&self.name("Subtype"))
                    .and_then(|o: &Object| o.resolve(self).as_name())
                    .and_then(|n: Handle<PdfName>| self.get_name(n))
                    .map(|n: PdfName| n.as_str() == "Image")
                    .unwrap_or(false)
            }
        {
            // Sublimation Phase 1: Pre-decode Image XObjects
            match self.sublimate_image(&data, &dict) {
                Ok(img_data) => img_data,
                Err(_) => {
                    // Fallback to compression if decoding fails
                    let compressed = zstd::encode_all(&*data, 3)
                        .map_err(|e| crate::PdfError::Other(e.to_string().into()))?;
                    SublimatedData::Compressed { original_len: data.len(), data: compressed }
                }
            }
        } else if data.len() > 1024 {
            // Heuristic: If it already looks like Zstd, don't double compress
            if data.starts_with(&[0x28, 0xB5, 0x2F, 0xFD]) {
                SublimatedData::Compressed {
                    original_len: 0, // Unknown if we don't parse it
                    data: data.to_vec(),
                }
            } else {
                // Compress large non-content streams (images, fonts) with Zstd
                let compressed = zstd::encode_all(&*data, 3)
                    .map_err(|e| crate::PdfError::Other(e.to_string().into()))?;
                SublimatedData::Compressed { original_len: data.len(), data: compressed }
            }
        } else {
            SublimatedData::Raw(data)
        };

        if let Some(Object::Stream(dh, _)) = self.get_object(handle) {
            self.set_object(handle, Object::Stream(dh, std::sync::Arc::new(sublimated)));
        }
        Ok(())
    }

    pub(crate) fn sublimate_image(
        &self,
        raw_data: &[u8],
        dict: &BTreeMap<Handle<PdfName>, Object>,
    ) -> PdfResult<crate::object::SublimatedData> {
        // Normalization-at-Load: Always ensure we are working with decompressed data.
        let decoded = self
            .process_filters(raw_data, dict)
            .unwrap_or_else(|_| Bytes::copy_from_slice(raw_data));
        let data = &decoded;

        let width =
            dict.get(&self.name("Width")).and_then(|o| o.resolve(self).as_integer()).unwrap_or(0)
                as u32;
        let height =
            dict.get(&self.name("Height")).and_then(|o| o.resolve(self).as_integer()).unwrap_or(0)
                as u32;
        let _bpc = dict
            .get(&self.name("BitsPerComponent"))
            .and_then(|o| o.resolve(self).as_integer())
            .unwrap_or(8) as u32;
        let color_space = dict
            .get(&self.name("ColorSpace"))
            .and_then(|o| o.resolve(self).as_name())
            .and_then(|n| self.get_name(n))
            .map(|n| n.as_str().to_string())
            .unwrap_or_else(|| "DeviceRGB".to_string());

        if width == 0 || height == 0 {
            return Err(crate::PdfError::Other("Invalid image dimensions".into()));
        }

        // Try decoding as JPEG first if it looks like one
        if data.starts_with(&[0xFF, 0xD8, 0xFF])
            && let Ok(img) = image::load_from_memory_with_format(data, image::ImageFormat::Jpeg) {
                let rgba = img.to_rgba8();
                return Ok(crate::object::SublimatedData::Image {
                    width: rgba.width(),
                    height: rgba.height(),
                    format: crate::graphics::PixelFormat::Rgba8,
                    data: rgba.into_raw(),
                });
            }

        // Raw pixel decoding (for FlateDecode or uncompressed)
        match color_space.as_str() {
            "DeviceGray" | "G" => {
                Ok(crate::object::SublimatedData::Image {
                    width,
                    height,
                    format: crate::graphics::PixelFormat::Gray8,
                    data: data.to_vec(),
                })
            }
            "DeviceRGB" | "RGB" => {
                let mut rgba_data = Vec::with_capacity((width * height * 4) as usize);
                for chunk in data.chunks_exact(3).take((width * height) as usize) {
                    rgba_data.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
                }
                Ok(crate::object::SublimatedData::Image {
                    width,
                    height,
                    format: crate::graphics::PixelFormat::Rgba8,
                    data: rgba_data,
                })
            }
            _ => {
                // Fallback: If we can't decode it as raw, try image crate general load
                if let Ok(img) = image::load_from_memory(data) {
                    let rgba = img.to_rgba8();
                    return Ok(crate::object::SublimatedData::Image {
                        width: rgba.width(),
                        height: rgba.height(),
                        format: crate::graphics::PixelFormat::Rgba8,
                        data: rgba.into_raw(),
                    });
                }
                Err(crate::PdfError::Other(
                    format!("Unsupported color space: {}", color_space).into(),
                ))
            }
        }
    }

    /// Accesses the raw bytes of a stream, transparently decompressing if necessary.
    pub fn get_stream_bytes(
        &self,
        data: &crate::object::SublimatedData,
    ) -> PdfResult<bytes::Bytes> {
        match data {
            crate::object::SublimatedData::Raw(b) => Ok(b.clone()),
            crate::object::SublimatedData::Compressed { data, .. } => {
                let decoded = zstd::decode_all(&**data)
                    .map_err(|e| crate::PdfError::Other(e.to_string().into()))?;
                Ok(bytes::Bytes::from(decoded))
            }
            crate::object::SublimatedData::Commands(cmds) => {
                let mut output = Vec::new();
                for cmd in cmds {
                    output.extend_from_slice(format!("{:?}\n", cmd).as_bytes());
                }
                Ok(bytes::Bytes::from(output))
            }
            crate::object::SublimatedData::Image { data, .. } => {
                Ok(bytes::Bytes::from(data.clone()))
            }
        }
    }

    pub fn get_sublimated_data(
        &self,
        handle: Handle<Object>,
    ) -> Option<std::sync::Arc<crate::object::SublimatedData>> {
        if let Some(Object::Stream(_, data)) = self.get_object(handle) {
            Some(data.clone())
        } else {
            None
        }
    }

    /// Finds an indirect object handle that points to the given dictionary handle.
    pub fn find_object_by_dict_handle(
        &self,
        dh: Handle<BTreeMap<Handle<PdfName>, Object>>,
    ) -> Option<Handle<Object>> {
        let objects = self.inner.objects.read();
        for (i, entry) in objects.iter().enumerate() {
            if let Object::Dictionary(h) = entry.object
                && h == dh {
                    return Some(Handle::new(i as u32));
                }
        }
        None
    }

    /// Returns high-level statistics about the arena's memory usage and object counts.
    pub fn get_stats(&self) -> ArenaStats {
        ArenaStats {
            object_count: self.inner.objects.read().len() as u32,
            dictionary_count: self.inner.dicts.read().len() as u32,
            array_count: self.inner.arrays.read().len() as u32,
            name_count: self.inner.names.read().len() as u32,
            version: self.version(),
        }
    }
}

/// High-level statistics about a PdfArena instance.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArenaStats {
    pub object_count: u32,
    pub dictionary_count: u32,
    pub array_count: u32,
    pub name_count: u32,
    pub version: f32,
}

pub type RemappingTable = BTreeMap<(u32, u16), Handle<Object>>;
