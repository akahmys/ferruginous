pub mod conformance;
pub mod page;
pub mod structure;

use self::page::Page;
use crate::error::PdfError;
use crate::font::{FallbackFontType, FontResource};
use crate::{FromPdfObject, Handle, Object, PdfArena, PdfName, PdfResult};
use parking_lot::RwLock;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Refined PDF Catalog (Root) Dictionary (ISO 32000-2:2020 Clause 7.7.2)
#[derive(Debug, Clone, FromPdfObject)]
#[pdf_dict(clause = "7.7.2")]
pub struct PdfCatalog {
    #[pdf_key("Pages")]
    pub pages: Handle<Object>,
    #[pdf_key("StructTreeRoot")]
    pub struct_tree_root: Option<Handle<Object>>,
    #[pdf_key("MarkInfo")]
    pub mark_info: Option<Object>,
    #[pdf_key("Metadata")]
    pub metadata: Option<Object>,
    #[pdf_key("Version")]
    pub version: Option<Handle<PdfName>>,
    #[pdf_key("AcroForm")]
    pub acro_form: Option<Object>,
    #[pdf_key("Names")]
    pub names: Option<Object>,
    #[pdf_key("Outlines")]
    pub outlines: Option<Object>,
    #[pdf_key("OpenAction")]
    pub open_action: Option<Object>,
    #[pdf_key("AA")]
    pub additional_actions: Option<Object>,
}

/// Type alias for a dictionary handle to satisfy clippy complexity rules.
pub type DictHandle = Handle<BTreeMap<Handle<PdfName>, Object>>;
type FontGroupMap = BTreeMap<(String, String), Vec<DictHandle>>;
type BestToUnicodeMap = BTreeMap<(String, String), Object>;

/// A refined PDF document.
pub struct Document {
    arena: PdfArena,
    root: Handle<Object>,
    info: Option<Handle<Object>>,
    pub ingestion_issues: Vec<String>,
    /// System font cache (shared across pages).
    pub system_fonts: Arc<BTreeMap<FallbackFontType, Arc<Vec<u8>>>>,
    /// Parsed FontResource cache to prevent redundant parsing across pages.
    pub font_cache: Arc<RwLock<BTreeMap<Handle<Object>, Arc<FontResource>>>>,
    pub force_fallback: bool,
}

impl Document {
    /// Creates a new document wrapper.
    pub fn new(arena: PdfArena, root: Handle<Object>, info: Option<Handle<Object>>) -> Self {
        Self {
            arena,
            root,
            info,
            ingestion_issues: Vec::new(),
            system_fonts: Arc::new(BTreeMap::new()),
            font_cache: Arc::new(RwLock::new(BTreeMap::new())),
            force_fallback: false,
        }
    }

    /// Creates a new document wrapper with issues.
    pub fn with_issues(
        arena: PdfArena,
        root: Handle<Object>,
        info: Option<Handle<Object>>,
        issues: Vec<String>,
    ) -> Self {
        Self {
            arena,
            root,
            info,
            ingestion_issues: issues,
            system_fonts: Arc::new(BTreeMap::new()),
            font_cache: Arc::new(RwLock::new(BTreeMap::new())),
            force_fallback: false,
        }
    }

    /// Opens a PDF document from bytes with specific options.
    pub fn open(data: bytes::Bytes, options: &crate::ingest::IngestionOptions) -> PdfResult<Self> {
        let mut lopdf_doc = lopdf::Document::load_mem(&data)
            .map_err(|e| PdfError::Parse { pos: 0, message: e.to_string().into() })?;

        // Attempt to decrypt with empty password if encrypted
        if lopdf_doc.is_encrypted() {
            match lopdf_doc.decrypt("") {
                Ok(_) => {}
                Err(_e) => {
                    // We will try manual Pass 0 decryption in Ingestor
                }
            }
        }

        let ingested = crate::ingest::Ingestor::ingest(&mut lopdf_doc, options)?;
        let mut doc =
            Self::with_issues(ingested.arena, ingested.root, ingested.info, ingested.issues);
        doc.force_fallback = options.force_fallback;
        
        // Populate font cache from ingestion
        {
            let mut cache = doc.font_cache.write();
            for (idx, res) in ingested.font_cache {
                cache.insert(Handle::new(idx), res);
            }
        }

        doc.load_system_fonts();
        doc.normalize_resources();
        doc.normalize_page_tree();
        Ok(doc)
    }

    /// Attempts to open and repair a PDF document with specific options.
    pub fn open_repair(
        data: bytes::Bytes,
        options: &crate::ingest::IngestionOptions,
    ) -> PdfResult<Self> {
        // lopdf's load_mem is already quite robust, but we could add more repair logic here
        Self::open(data, options)
    }

    /// Loads a PDF document from a file path using default options.
    pub fn load(path: &std::path::Path) -> PdfResult<Self> {
        let data = std::fs::read(path).map_err(|e| PdfError::Other(e.to_string().into()))?;
        Self::open(bytes::Bytes::from(data), &crate::ingest::IngestionOptions::default())
    }

    /// Loads system fonts from well-known paths.
    pub fn load_system_fonts(&mut self) {
        let mut fonts = BTreeMap::new();

        // macOS standard paths
        let mac_paths = [
            (
                crate::font::FallbackFontType::JapaneseSerif,
                "/System/Library/Fonts/ヒラギノ明朝 ProN.ttc",
            ),
            (
                crate::font::FallbackFontType::JapaneseSans,
                "/System/Library/Fonts/ヒラギノ角ゴ Interface.ttc",
            ),
            (crate::font::FallbackFontType::Serif, "/System/Library/Fonts/Times.ttc"),
            (crate::font::FallbackFontType::SansSerif, "/System/Library/Fonts/Helvetica.ttc"),
            (crate::font::FallbackFontType::Monospace, "/System/Library/Fonts/Courier.dfont"),
        ];

        for (ftype, path) in mac_paths {
            if let Ok(data) = std::fs::read(path) {
                fonts.insert(ftype, Arc::new(data));
            }
        }

        // Also check FERRUGINOUS_RESOURCES/fonts
        let resource_dir =
            std::env::var("FERRUGINOUS_RESOURCES").unwrap_or_else(|_| "resources".to_string());
        let base_path = std::path::Path::new(&resource_dir).join("fonts");
        let mappings = [
            (crate::font::FallbackFontType::Serif, "serif.ttf"),
            (crate::font::FallbackFontType::SansSerif, "sans.ttf"),
            (crate::font::FallbackFontType::Monospace, "mono.ttf"),
            (crate::font::FallbackFontType::JapaneseSerif, "mincho.ttf"),
            (crate::font::FallbackFontType::JapaneseSans, "gothic.ttf"),
        ];
        for (ftype, filename) in mappings {
            if let Ok(data) = std::fs::read(base_path.join(filename)) {
                fonts.insert(ftype, Arc::new(data));
            }
        }

        self.system_fonts = Arc::new(fonts);
    }
    /// Returns a reference to the internal arena.
    pub fn arena(&self) -> &PdfArena {
        &self.arena
    }

    /// Returns the handle to the document root (Catalog).
    pub fn root_handle(&self) -> &Handle<Object> {
        &self.root
    }

    /// Returns the catalog dictionary handle.
    pub fn catalog_handle(&self) -> Option<Handle<Object>> {
        Some(self.root)
    }

    /// Returns the handle to the document info dictionary, if it exists.
    pub fn info_handle(&self) -> Option<Handle<Object>> {
        self.info
    }

    /// Resolves an indirect handle into an object.
    pub fn resolve(&self, handle: &Handle<Object>) -> PdfResult<Object> {
        self.arena
            .get_object(*handle)
            .ok_or_else(|| PdfError::Arena("Failed to resolve handle".into()))
    }

    /// Retrieves a font resource, loading it if not already cached.
    pub fn get_font(&self, handle: Handle<Object>) -> PdfResult<Arc<FontResource>> {
        {
            let cache = self.font_cache.read();
            if let Some(res) = cache.get(&handle) {
                return Ok(Arc::clone(res));
            }
        }

        let obj = self.resolve(&handle)?;
        let dict_h = obj.as_dict_handle().ok_or_else(|| PdfError::Other("Not a dictionary".into()))?;
        let dict = self.arena.get_dict(dict_h).ok_or_else(|| PdfError::Other("Missing dictionary".into()))?;
        
        let font_res = FontResource::load(&dict, self)?;
        let arc_res = Arc::new(font_res);
        
        self.font_cache.write().insert(handle, Arc::clone(&arc_res));
        Ok(arc_res)
    }

    /// Decodes a stream object.
    pub fn decode_stream(&self, obj: &Object) -> PdfResult<bytes::Bytes> {
        match obj {
            Object::Stream(dict_handle, data) => {
                let dict = self.arena.get_dict(*dict_handle).ok_or_else(|| PdfError::Filter {
                    filter: "None".into(),
                    message: "Missing stream dictionary".into(),
                })?;
                let raw_bytes = self.arena.get_stream_bytes(data)?;
                self.arena.process_filters(&raw_bytes, &dict)
            }
            _ => Err(PdfError::Filter {
                filter: "None".into(),
                message: "Object is not a stream".into(),
            }),
        }
    }

    /// Resolves an indirect object handle to its current dictionary pool handle.
    pub fn resolve_to_dict(&self, handle: Handle<Object>) -> PdfResult<DictHandle> {
        self.arena
            .get_object(handle)
            .and_then(|obj| obj.as_dict_handle())
            .ok_or_else(|| PdfError::Other(format!("Object {:?} is not a dictionary", handle).into()))
    }

    /// Returns the total number of pages in the document.
    pub fn page_count(&self) -> PdfResult<usize> {
        let pages_root_h = self.get_pages_root()?;
        let pages_root_dh = self.resolve_to_dict(pages_root_h)?;
        let dict = self
            .arena
            .get_dict(pages_root_dh)
            .ok_or_else(|| PdfError::Other("Invalid Pages root dictionary".into()))?;

        let count_key = self
            .arena
            .get_name_by_str("Count")
            .ok_or_else(|| PdfError::Other("Missing Count name".into()))?;

        dict.get(&count_key)
            .and_then(|o| o.resolve(&self.arena).as_integer())
            .map(|i| usize::try_from(i).unwrap_or(0))
            .ok_or_else(|| PdfError::Other("Invalid or missing /Count in page tree".into()))
    }

    /// Retrieves a specific page by its 0-based index.
    pub fn get_page(&self, index: usize) -> PdfResult<Page<'_>> {
        let root_handle = self.get_pages_root()?;
        self.find_page_recursive(root_handle, index, Vec::new())
    }

    /// Returns a list of all page object handles in the document.
    pub fn find_all_pages(&self) -> Vec<Handle<Object>> {
        let mut pages = Vec::new();
        if let Ok(root) = self.get_pages_root() {
            let _ = self.walk_pages_recursive(root, &mut pages, 0);
        }
        pages
    }

    fn walk_pages_recursive(&self, node_h: Handle<Object>, out: &mut Vec<Handle<Object>>, depth: usize) -> PdfResult<()> {
        if depth > 32 { return Err(PdfError::Other("Page tree depth limit exceeded".into())); }
        
        let dict_h = self.resolve_to_dict(node_h)?;
        let dict = self.arena.get_dict(dict_h).ok_or_else(|| PdfError::Other("Invalid node in page tree".into()))?;

        let type_key = self.arena.name("Type");
        let node_type = dict.get(&type_key)
            .and_then(|o| o.resolve(&self.arena).as_name())
            .and_then(|h| self.arena.get_name(h));

        if let Some(name) = node_type && name.as_str() == "Page" {
            out.push(node_h);
            return Ok(());
        }

        let kids_key = self.arena.name("Kids");
        if let Some(kids_obj) = dict.get(&kids_key) {
            let ah = kids_obj.resolve(&self.arena).as_array().ok_or_else(|| PdfError::Other("Invalid Kids array".into()))?;
            if let Some(kids) = self.arena.get_array(ah) {
                for kid in kids {
                    if let Some(h) = kid.resolve(&self.arena).as_reference() {
                        let _ = self.walk_pages_recursive(h, out, depth + 1);
                    }
                }
            }
        }
        Ok(())
    }

    fn get_pages_root(&self) -> PdfResult<Handle<Object>> {
        let catalog_obj = self
            .arena
            .get_object(self.root)
            .ok_or_else(|| PdfError::Other("Missing document catalog".into()))?;
        let catalog = PdfCatalog::from_pdf_object(catalog_obj, &self.arena)?;
        Ok(catalog.pages)
    }

    fn find_page_recursive(
        &self,
        root_node: Handle<Object>,
        mut target_index: usize,
        _unused_path: Vec<Handle<Object>>,
    ) -> PdfResult<Page<'_>> {
        let mut current_node = root_node;
        let mut path = Vec::new();

        loop {
            // Hardening: Recursion depth limit for page tree
            if path.len() > 32 {
                return Err(PdfError::Other("Page tree depth limit exceeded".into()));
            }

            let dict_h = self.resolve_to_dict(current_node)?;
            let dict = self
                .arena
                .get_dict(dict_h)
                .ok_or_else(|| PdfError::Other("Invalid node in page tree".into()))?;

            let type_key = self.arena.name("Type");
            let node_type = dict
                .get(&type_key)
                .and_then(|o| o.resolve(&self.arena).as_name())
                .and_then(|h| self.arena.get_name(h));

            if let Some(name) = node_type
                && name.as_str() == "Page"
            {
                return Ok(Page::new(&self.arena, current_node, path));
            }

            // It's a Pages node (intermediate)
            let kids_key = self.arena.name("Kids");
            let kids_array_handle = dict
                .get(&kids_key)
                .and_then(|o| o.resolve(&self.arena).as_array())
                .ok_or_else(|| PdfError::Other("Missing Kids in Pages node".into()))?;

            let kids = self
                .arena
                .get_array(kids_array_handle)
                .ok_or_else(|| PdfError::Other("Invalid kids array handle".into()))?;

            path.push(current_node);
            let mut found = false;

            for kid_obj in kids {
                let kid_handle = kid_obj
                    .as_reference()
                    .ok_or_else(|| PdfError::Other("Invalid kid object (not a reference)".into()))?;

                let kid_dict_h = self.resolve_to_dict(kid_handle)?;
                let kid_dict = self
                    .arena
                    .get_dict(kid_dict_h)
                    .ok_or_else(|| PdfError::Other("Invalid kid dictionary".into()))?;

                let count = self.get_node_count(&kid_dict);
                if target_index < count {
                    current_node = kid_handle;
                    found = true;
                    break;
                } else {
                    target_index -= count;
                }
            }

            if !found {
                return Err(PdfError::Other("Page index out of bounds".into()));
            }
        }
    }

    fn get_node_count(&self, dict: &BTreeMap<Handle<PdfName>, Object>) -> usize {
        let count_key = self.arena.name("Count");
        if let Some(count) = dict.get(&count_key).and_then(|o| o.resolve(&self.arena).as_integer())
        {
            return usize::try_from(count).unwrap_or(0);
        }
        // Leaf Page nodes usually lack /Count, they count as 1
        let type_key = self.arena.name("Type");
        if let Some(t) = dict.get(&type_key).and_then(|o| o.resolve(&self.arena).as_name())
            && let Some(name) = self.arena.get_name(t)
            && name.as_str() == "Page"
        {
            return 1;
        }
        0
    }

    /// Returns high-level compliance information about the document.
    pub fn compliance_info(&self) -> PdfResult<conformance::ComplianceInfo> {
        let mut info = conformance::ComplianceInfo::default();

        let catalog_obj = self
            .arena
            .get_object(self.root)
            .ok_or_else(|| PdfError::Other("Missing document catalog".into()))?;
        let catalog = PdfCatalog::from_pdf_object(catalog_obj, &self.arena)?;

        // 1. Check for /StructTreeRoot
        info.has_struct_tree = catalog.struct_tree_root.is_some();

        // 2. Check for /MarkInfo -> /Marked true
        if let Some(mark_info_obj) = catalog.mark_info {
            let marked_key = self.arena.name("Marked");
            if let Some(mark_dict) = mark_info_obj
                .resolve(&self.arena)
                .as_dict_handle()
                .and_then(|h| self.arena.get_dict(h))
                && let Some(marked) =
                    mark_dict.get(&marked_key).and_then(|o| o.resolve(&self.arena).as_bool())
            {
                info.is_marked = marked;
            }
        }

        // 3. Extract Metadata Conformance
        let pdf_20 = catalog
            .version
            .and_then(|n| self.arena.get_name(n))
            .map(|n| n.as_str() == "2.0")
            .unwrap_or(false);

        if info.has_struct_tree && pdf_20 {
            info.metadata.pdf_ua_part = Some(2);
        }

        Ok(info)
    }

    /// Returns the handle to the Structure Tree Root dictionary, if it exists.
    pub fn get_structure_root(&self) -> PdfResult<Option<Handle<Object>>> {
        let catalog_obj = self
            .arena
            .get_object(self.root)
            .ok_or_else(|| PdfError::Other("Missing document catalog".into()))?;
        let catalog = PdfCatalog::from_pdf_object(catalog_obj, &self.arena)?;
        Ok(catalog.struct_tree_root)
    }

    /// Returns the document metadata.
    pub fn metadata(&self) -> crate::metadata::MetadataInfo {
        crate::metadata::extract_metadata(self)
    }

    /// Returns a list of fonts used in the document.
    pub fn fonts(&self) -> Vec<crate::font::FontSummary> {
        crate::font::list_fonts(self)
    }

    /// Normalizes document resources at load-time (Phase 3).
    /// Group fonts by BaseFont and CIDSystemInfo to share ToUnicode mappings.
    pub fn normalize_resources(&mut self) {
        // Clear font cache to force re-parsing with potential new system fonts
        self.font_cache.write().clear();

        let (font_groups, best_to_unicode) = self.discover_font_groups();
        self.propagate_tounicode_mappings(font_groups, best_to_unicode);
        self.resolve_missing_font_data();
    }

    /// Normalizes the page tree by flattening it into a single-level structure (Phase 4).
    /// Inherited attributes are pushed down to leaf Page nodes.
    pub fn normalize_page_tree(&mut self) {
        let count = match self.page_count() {
            Ok(c) => c,
            Err(_) => return,
        };

        let mut page_handles = Vec::new();
        let arena = &self.arena;

        // 1. Collect and flatten individual pages
        for i in 0..count {
            if let Ok(page) = self.get_page(i) {
                // We want to keep the original handle if possible to preserve references (e.g. from /Annots /P)
                let ph = page.obj_handle();
                if let Some(Object::Dictionary(dh)) = arena.get_object(ph) {
                    let mut dict = arena.get_dict(dh).unwrap_or_default();

                    // Flatten inherited attributes (ISO 32000-2:2020 Clause 7.7.3.3)
                    let attrs = ["Resources", "MediaBox", "CropBox", "Rotate"];
                    for attr in attrs {
                        let attr_name = arena.name(attr);
                        if !dict.contains_key(&attr_name)
                            && let Some(val) = page.resolve_attribute(attr) {
                                dict.insert(attr_name, val);
                            }
                    }

                    // Set Parent to the new root (we'll set the root handle later)
                    arena.set_dict(dh, dict);
                    page_handles.push(Object::Reference(ph));
                }
            }
        }

        if page_handles.is_empty() {
            return;
        }

        // 2. Create new flat Pages root
        let pages_root_key = arena.name("Pages");
        let type_key = arena.name("Type");
        let kids_key = arena.name("Kids");
        let count_key = arena.name("Count");

        let mut pages_root_dict = BTreeMap::new();
        pages_root_dict.insert(type_key, Object::Name(pages_root_key));
        pages_root_dict.insert(count_key, Object::Integer(count as i64));
        pages_root_dict.insert(kids_key, Object::Array(arena.alloc_array(page_handles.clone())));

        let pages_root_dh = arena.alloc_dict(pages_root_dict);
        let pages_root_h = arena.alloc_object(Object::Dictionary(pages_root_dh));

        // 3. Update Parent for all pages
        for page_ref in &page_handles {
            if let Object::Reference(ph) = page_ref
                && let Some(Object::Dictionary(dh)) = arena.get_object(*ph) {
                    let mut dict = arena.get_dict(dh).unwrap_or_default();
                    dict.insert(arena.name("Parent"), Object::Reference(pages_root_h));
                    arena.set_dict(dh, dict);
                }
        }

        // 4. Update Catalog
        if let Some(cat_obj_h) = self.catalog_handle()
            && let Ok(cat_dh) = self.resolve_to_dict(cat_obj_h) {
                let mut cat_dict = arena.get_dict(cat_dh).unwrap_or_default();
                cat_dict.insert(pages_root_key, Object::Reference(pages_root_h));
                arena.set_dict(cat_dh, cat_dict);
            }
    }

    fn resolve_missing_font_data(&mut self) {
        let system_fonts = self.system_fonts.clone();
        let cache = self.font_cache.clone();

        let mut cache_write = cache.write();
        for res in cache_write.values_mut() {
            let res_mut = Arc::make_mut(res);
            if res_mut.data.is_none()
                && let Some(ftype) = res_mut.fallback_type
                    && let Some(sys_data) = system_fonts.get(&ftype) {
                        res_mut.data = Some(Arc::clone(sys_data));
                        let _ = res_mut.perform_reconstruction();
                    }
        }
    }

    fn discover_font_groups(&self) -> (FontGroupMap, BestToUnicodeMap) {
        let arena = &self.arena;
        let mut font_groups = BTreeMap::new();
        let mut best_to_unicode = BTreeMap::new();
        let mut best_to_unicode_count = BTreeMap::new();

        let type_key = arena.name("Type");
        let font_val = arena.name("Font");
        let base_font_key = arena.name("BaseFont");
        let to_unicode_key = arena.name("ToUnicode");
        let _descendant_fonts_key = arena.name("DescendantFonts");

        for h in arena.all_dict_handles() {
            let Some(dict) = arena.get_dict(h) else { continue };
            if let Some(t_h) = dict.get(&type_key).and_then(|o| o.resolve(arena).as_name())
                && t_h == font_val
            {
                let base_font = dict
                    .get(&base_font_key)
                    .and_then(|o| o.resolve(arena).as_name())
                    .and_then(|h| arena.get_name_str(h))
                    .unwrap_or_else(|| "Untitled".to_string());
                let is_cid = dict.contains_key(&arena.name("DescendantFonts"));
                if is_cid {
                    let csi_str = self.extract_csi_string(&dict);
                    let key = (base_font, csi_str);
                    font_groups.entry(key.clone()).or_insert_with(Vec::new).push(h);

                    if let Some(tu) = dict.get(&to_unicode_key)
                        && let Ok(data) = self.decode_stream(&tu.resolve(arena))
                        && let Ok(m) = crate::font::cmap::CMap::parse(&data)
                    {
                        let count = m.mappings.len();
                        if count > *best_to_unicode_count.get(&key).unwrap_or(&0) {
                            best_to_unicode_count.insert(key.clone(), count);
                            best_to_unicode.insert(key, tu.clone());
                        }
                    }
                }
            }
        }
        (font_groups, best_to_unicode)
    }

    fn extract_csi_string(&self, dict: &BTreeMap<Handle<PdfName>, Object>) -> String {
        let arena = &self.arena;
        if let Some(df_obj) = dict.get(&arena.name("DescendantFonts"))
            && let Some(ah) = df_obj.resolve(arena).as_array()
            && let Some(arr) = arena.get_array(ah)
            && let Some(df_h) = arr.first().and_then(|o| o.resolve(arena).as_dict_handle())
            && let Some(df_dict) = arena.get_dict(df_h)
            && let Some(csi_obj) = df_dict.get(&arena.name("CIDSystemInfo"))
            && let Some(csi_h) = csi_obj.resolve(arena).as_dict_handle()
            && let Some(csi_dict) = arena.get_dict(csi_h)
        {
            let r = csi_dict
                .get(&arena.name("Registry"))
                .map(|o| o.resolve(arena))
                .as_ref()
                .and_then(|o| o.as_string())
                .map(|s| String::from_utf8_lossy(s).to_string())
                .unwrap_or_default();
            let o = csi_dict
                .get(&arena.name("Ordering"))
                .map(|o| o.resolve(arena))
                .as_ref()
                .and_then(|o| o.as_string())
                .map(|s| String::from_utf8_lossy(s).to_string())
                .unwrap_or_default();
            return format!("{}-{}", r, o);
        }
        String::new()
    }

    fn propagate_tounicode_mappings(
        &mut self,
        font_groups: FontGroupMap,
        best_to_unicode: BestToUnicodeMap,
    ) {
        let arena = &self.arena;
        let to_unicode_key = arena.name("ToUnicode");
        for (key, fonts) in font_groups {
            if let Some(best_tu) = best_to_unicode.get(&key) {
                for font_h in fonts {
                    if let Some(mut dict) = arena.get_dict(font_h)
                        && !dict.contains_key(&to_unicode_key)
                    {
                        dict.insert(to_unicode_key, best_tu.clone());
                        arena.set_dict(font_h, dict);
                    }
                }
            }
        }
    }

    /// Returns the sublimated data for a stream object.
    pub fn get_sublimated_data(
        &self,
        handle: Handle<Object>,
    ) -> Option<std::sync::Arc<crate::object::SublimatedData>> {
        self.arena.get_sublimated_data(handle)
    }
}
