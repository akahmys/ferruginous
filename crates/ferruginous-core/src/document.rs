pub mod page;
pub mod conformance;

use self::page::Page;
use crate::{PdfResult, PdfArena, Handle, Object, PdfName};
use crate::error::PdfError;
use std::collections::BTreeMap;

/// Type alias for a dictionary handle to satisfy clippy complexity rules.
pub type DictHandle = Handle<BTreeMap<Handle<PdfName>, Object>>;

/// A refined PDF document.
pub struct Document {
    arena: PdfArena,
    root: Handle<Object>,
    info: Option<Handle<Object>>,
}

impl Document {
    /// Creates a new document wrapper.
    pub fn new(arena: PdfArena, root: Handle<Object>, info: Option<Handle<Object>>) -> Self {
        Self { arena, root, info }
    }

    /// Opens a PDF document from bytes with specific options.
    pub fn open(data: bytes::Bytes, options: &crate::ingest::IngestionOptions) -> PdfResult<Self> {
        let lopdf_doc = lopdf::Document::load_mem(&data)
            .map_err(|e| PdfError::Parse(e.to_string()))?;
        let (arena, root, info) = crate::ingest::LopdfIngestor::ingest(&lopdf_doc, options)?;
        Ok(Self::new(arena, root, info))
    }

    /// Attempts to open and repair a PDF document with specific options.
    pub fn open_repair(data: bytes::Bytes, options: &crate::ingest::IngestionOptions) -> PdfResult<Self> {
        // lopdf's load_mem is already quite robust, but we could add more repair logic here
        Self::open(data, options)
    }
    /// Returns a reference to the internal arena.
    pub fn arena(&self) -> &PdfArena {
        &self.arena
    }

    /// Returns the handle to the document root (Catalog).
    pub fn root_handle(&self) -> &Handle<Object> {
        &self.root
    }

    /// Returns the handle to the document info dictionary, if it exists.
    pub fn info_handle(&self) -> Option<Handle<Object>> {
        self.info
    }

    /// Resolves an indirect handle into an object.
    pub fn resolve(&self, handle: &Handle<Object>) -> PdfResult<Object> {
        self.arena.get_object(*handle)
            .ok_or_else(|| PdfError::Arena("Failed to resolve handle".into()))
    }

    /// Decodes a stream object.
    pub fn decode_stream(&self, obj: &Object) -> PdfResult<bytes::Bytes> {
        match obj {
            Object::Stream(dict_handle, data) => {
                let dict = self.arena.get_dict(*dict_handle)
                    .ok_or_else(|| PdfError::Filter("Missing stream dictionary".into()))?;
                self.arena.process_filters(data, &dict)
            }
            _ => Err(PdfError::Filter("Object is not a stream".into())),
        }
    }

    /// Returns the total number of pages in the document.
    pub fn page_count(&self) -> PdfResult<usize> {
        let pages_root = self.get_pages_root()?;
        let dict = self.arena.get_dict(pages_root)
            .ok_or_else(|| PdfError::Other("Invalid Pages root dictionary".into()))?;
        
        let count_key = self.arena.get_name_by_str("Count")
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

    fn get_pages_root(&self) -> PdfResult<Handle<BTreeMap<Handle<PdfName>, Object>>> {
        let catalog = self.arena.get_object(self.root)
            .ok_or_else(|| PdfError::Other("Missing document catalog".into()))?;
        
        let catalog_dict_handle = catalog.as_dict_handle()
            .ok_or_else(|| PdfError::Other("Catalog is not a dictionary".into()))?;
        
        let catalog_dict = self.arena.get_dict(catalog_dict_handle)
            .ok_or_else(|| PdfError::Other("Invalid catalog handle".into()))?;
        let pages_key = self.arena.name("Pages");

        catalog_dict.get(&pages_key)
            .and_then(|o| o.resolve(&self.arena).as_dict_handle())
            .ok_or_else(|| PdfError::Other("Missing or invalid Pages in catalog".into()))
    }

    fn find_page_recursive(
        &self,
        node_handle: Handle<BTreeMap<Handle<PdfName>, Object>>,
        mut target_index: usize,
        mut path: Vec<Handle<BTreeMap<Handle<PdfName>, Object>>>
    ) -> PdfResult<Page<'_>> {
        let dict = self.arena.get_dict(node_handle)
            .ok_or_else(|| PdfError::Other("Invalid node in page tree".into()))?;

        let type_key = self.arena.name("Type");
        let node_type = dict.get(&type_key).and_then(|o| o.resolve(&self.arena).as_name());

        if let Some(t) = node_type {
            let name = self.arena.get_name(t).ok_or_else(|| PdfError::Other("Invalid name handle".into()))?;
            if name.as_str() == "Page" {
                return Ok(Page::new(&self.arena, node_handle, path));
            }
        }

        // It's a Pages node (intermediate)
        let kids_key = self.arena.name("Kids");
        if let Some(kids_array_handle) = dict.get(&kids_key).and_then(|o| o.resolve(&self.arena).as_array()) {
            let kids = self.arena.get_array(kids_array_handle)
                .ok_or_else(|| PdfError::Other("Invalid kids array handle".into()))?;
            path.push(node_handle);

            for kid_obj in kids {
                let kid_handle = kid_obj.resolve(&self.arena).as_dict_handle()
                    .ok_or_else(|| PdfError::Other("Invalid kid object".into()))?;
                
                // Check if we can skip this node based on /Count
                let kid_dict = self.arena.get_dict(kid_handle)
                    .ok_or_else(|| PdfError::Other("Invalid kid dictionary".into()))?;
                let count_key = self.arena.name("Count");
                
                if let Some(count) = kid_dict.get(&count_key).and_then(|o: &Object| o.resolve(&self.arena).as_integer()) {
                    let count = usize::try_from(count).unwrap_or(0);
                    if target_index >= count {
                        target_index -= count;
                        continue;
                    }
                } else {
                    // Check if it's a Page node (count is usually missing in leaf Page nodes)
                    let type_key = self.arena.name("Type");
                    let node_type = kid_dict.get(&type_key).and_then(|o| o.resolve(&self.arena).as_name());
                    if let Some(t) = node_type
                        && let Some(name) = self.arena.get_name(t)
                            && name.as_str() == "Page"
                                && target_index > 0 {
                                    target_index -= 1;
                                    continue;
                                }
                }
                
                return self.find_page_recursive(kid_handle, target_index, path);
            }
        }

        Err(PdfError::Other("Page index out of bounds".into()))
    }

    /// Returns high-level compliance information about the document.
    pub fn compliance_info(&self) -> PdfResult<conformance::ComplianceInfo> {
        let mut info = conformance::ComplianceInfo::default();
        
        let catalog_handle = self.root;
        let catalog_dict = self.arena.get_object(catalog_handle)
            .and_then(|o| o.as_dict_handle())
            .and_then(|h| self.arena.get_dict(h))
            .ok_or_else(|| PdfError::Other("Invalid catalog".into()))?;

        // 1. Check for /StructTreeRoot
        let struct_tree_key = self.arena.name("StructTreeRoot");
        info.has_struct_tree = catalog_dict.contains_key(&struct_tree_key);

        // 2. Check for /MarkInfo -> /Marked true
        let mark_info_key = self.arena.name("MarkInfo");
        let marked_key = self.arena.name("Marked");
        if let Some(mark_info) = catalog_dict.get(&mark_info_key)
            && let Some(mark_dict) = mark_info.resolve(&self.arena).as_dict_handle().and_then(|h| self.arena.get_dict(h))
                && let Some(marked) = mark_dict.get(&marked_key).and_then(|o| o.resolve(&self.arena).as_bool()) {
                    info.is_marked = marked;
                }

        // 3. Extract Metadata Conformance (Simplified for now)
        // In a real implementation, we would parse the Metadata stream for PDF/UA-2 etc.
        // For M66, we stub this based on the Version and Presence of tags.
        let version_key = self.arena.name("Version");
        let pdf_20 = catalog_dict.get(&version_key)
            .and_then(|o| o.resolve(&self.arena).as_name())
            .and_then(|n| self.arena.get_name(n))
            .map(|n| n.as_str() == "2.0")
            .unwrap_or(false);

        if info.has_struct_tree && pdf_20 {
            info.metadata.pdf_ua_part = Some(2); // Assume UA-2 for 2.0 documents with structure
        }

        Ok(info)
    }

    /// Returns the handle to the Structure Tree Root dictionary, if it exists.
    pub fn get_structure_root(&self) -> PdfResult<Option<DictHandle>> {
        let catalog_handle = self.root;
        let catalog_dict = self.arena.get_object(catalog_handle)
            .and_then(|o| o.as_dict_handle())
            .and_then(|h| self.arena.get_dict(h))
            .ok_or_else(|| PdfError::Other("Invalid catalog".into()))?;

        let struct_tree_key = self.arena.name("StructTreeRoot");
        Ok(catalog_dict.get(&struct_tree_key)
            .and_then(|o| o.resolve(&self.arena).as_dict_handle()))
    }

    /// Returns the document metadata.
    pub fn metadata(&self) -> crate::metadata::MetadataInfo {
        crate::metadata::extract_metadata(self)
    }

    /// Returns a list of fonts used in the document.
    pub fn fonts(&self) -> Vec<crate::font::FontSummary> {
        crate::font::list_fonts(self)
    }
}
