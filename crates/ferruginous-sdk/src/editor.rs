//! PDF Document Editor (ISO 32000-2:2020 Clause 7.5.6)

use crate::core::{Object, Reference, Resolver, PdfError, PdfResult, ValidationErrorVariant};
use crate::loader::PdfDocument;
use crate::resolver::LayeredResolver;
use crate::xref::{XRefIndex, XRefEntry, MemoryXRefIndex};
use crate::serialize::xref_stream::XRefStreamBuilder;
use crate::serialize::object_stream::ObjectStreamBuilder;
use md5::{Md5, Digest as Md5Digest};
use std::time::SystemTime;

/// The primary interface for modifying a PDF document.
/// (ISO 32000-2:2020 Clause 7.5.6 Incremental Updates)
pub struct PdfEditor<'a> {
    /// The owned `PdfDocument` serving as the base for edits.
    pub document: PdfDocument,
    /// The layered resolver managing memory-based overrides.
    pub resolver: LayeredResolver<'a>,
    /// The next available object ID for new indirect objects.
    next_object_id: u32,
    /// The Optional Content context managing layer visibility.
    pub oc_context: Option<crate::ocg::OCContext>,
}

impl PdfEditor<'_> {
    /// Creates a new editor for the given document.
    pub fn new(doc: PdfDocument) -> PdfResult<Self> {
        let max_id = doc.xref_index.max_id();
        let base_resolver = Box::new(crate::resolver::PdfResolver {
            data: Box::leak(Box::new(doc.data.clone())), // Ensure base data lives long enough
            index: std::sync::Arc::new(doc.xref_index.clone()),
            security: doc.security.clone(),
            cache: std::sync::Mutex::new(std::collections::BTreeMap::new()),
        });

        let editor = Self {
            document: doc,
            resolver: LayeredResolver::new(base_resolver),
            next_object_id: max_id.checked_add(1).unwrap_or(u32::MAX),
            oc_context: None,
        };
        
        // Verify catalog structure
        let catalog = editor.document.catalog().map_err(|_| PdfError::StructureError(crate::core::StructureErrorVariant::MissingRoot))?;
        if catalog.dictionary.is_empty() {
            return Err(PdfError::StructureError(crate::core::StructureErrorVariant::MissingRoot));
        }
        
        Ok(editor)
    }

    /// Resolves an object by reference, checking the memory overlay first.
    pub fn get_object(&self, reference: &Reference) -> PdfResult<Object> {
        self.resolver.resolve(reference)
    }

    /// Updates an existing object in the memory overlay.
    pub fn update_object(&mut self, reference: Reference, object: Object) {
        debug_assert!(reference.id > 0, "Object ID must be positive");
        self.resolver.insert(reference, object);
    }

    /// Creates a new object with a unique ID and adds it to the memory overlay.
    pub fn create_object(&mut self, object: Object) -> PdfResult<Reference> {
        let reference = Reference {
            id: self.next_object_id,
            generation: 0,
        };
        
        self.resolver.insert(reference, object);
        
        // Prepare next ID
        self.next_object_id = self.next_object_id.checked_add(1).ok_or_else(|| PdfError::ResourceError("Object ID overflow".into()))?;
        
        Ok(reference)
    }

    /// Returns the original document data (read-only base).
    #[must_use] pub fn original_data(&self) -> &[u8] {
        &self.document.data
    }

    /// Appends the incremental update to the writer.
    /// (ISO 32000-2:2020 Clause 7.5.6)
    pub fn save_incremental<W: std::io::Write + std::io::Seek>(
        &mut self,
        writer: &mut W,
        use_xref_stream: bool,
        use_object_streams: bool,
    ) -> PdfResult<()> {
        // 1. Write original data
        writer.write_all(&self.document.data).map_err(PdfError::from)?;
        
        let mut new_entries = std::collections::BTreeMap::new();
        let mut obj_stream_builder = ObjectStreamBuilder::new();
        let mut compressed_ids = Vec::new();

        // 2. Process objects: separate stream and non-stream if compression is enabled
        for (reference, object) in &self.resolver.overlay {
            if use_object_streams && !matches!(object, Object::Stream(_, _)) {
                // We must clone to move into the builder if we want to keep the overlay intact
                obj_stream_builder.add_object(reference.id, object.clone())?;
                compressed_ids.push(reference.id);
            } else {
                let offset = writer.stream_position().map_err(PdfError::from)?;
                crate::serialize::write_indirect_object(writer, reference.id, reference.generation, object)?;
                new_entries.insert(reference.id, XRefEntry::InUse {
                    offset,
                    generation: reference.generation,
                });
            }
        }

        // 3. Write Object Stream if needed
        let mut obj_stm_id = None;
        if use_object_streams && !compressed_ids.is_empty() {
            let id = self.next_object_id;
            obj_stm_id = Some(id);
            let offset = writer.stream_position().map_err(PdfError::from)?;
            let obj_stm = obj_stream_builder.build()?;
            crate::serialize::write_indirect_object(writer, id, 0, &obj_stm)?;
            
            new_entries.insert(id, XRefEntry::InUse { offset, generation: 0 });
            for (index, &id) in compressed_ids.iter().enumerate() {
                new_entries.insert(id, XRefEntry::Compressed {
                    container_id: Some(id).unwrap_or(0), // Final ID will be obj_stm_id
                    index: index as u32,
                });
            }
            // Fix container_id after we have it
            if let Some(stm_id) = obj_stm_id {
                for &id in &compressed_ids {
                    if let Some(XRefEntry::Compressed { container_id, .. }) = new_entries.get_mut(&id) {
                        *container_id = stm_id;
                    }
                }
            }
        }

        // 4. Handle XRef and Trailer
        let mut trailer_dict = self.document.last_trailer.trailer_dict.clone();
        if self.document.last_trailer.last_xref_offset > 0 {
            std::sync::Arc::make_mut(&mut trailer_dict).insert(b"Prev".to_vec(), Object::Integer(self.document.last_trailer.last_xref_offset as i64));
        }
        let size = obj_stm_id.map_or(self.next_object_id, |id| id + 1);
        std::sync::Arc::make_mut(&mut trailer_dict).insert(b"Size".to_vec(), Object::Integer(i64::from(size)));

        // 5. Deterministic /ID (Clause 14.4)
        let mut hasher = Md5::new();
        
        // Context information for ID generation
        if let Ok(now) = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
            hasher.update(&now.as_secs().to_be_bytes());
        }
        hasher.update(&self.document.data.len().to_be_bytes());
        
        // Permanent ID should ideally remain same if already exists
        let existing_id = self.document.last_trailer.trailer_dict.get(b"ID".as_ref()).and_then(|o| {
            if let Object::Array(a) = o { a.first().cloned() } else { None }
        });

        let permanent_id = if let Some(id) = existing_id {
            id
        } else {
            let mut p_hasher = Md5::new();
            p_hasher.update(&self.document.data[..self.document.data.len().min(4096)]); // Sample start
            Object::new_string(p_hasher.finalize().to_vec())
        };

        // Changing ID for this incremental session
        for (id, _) in &new_entries {
            hasher.update(id.to_be_bytes());
        }
        let changing_id = Object::new_string(hasher.finalize().to_vec());
        
        let id_array = Object::new_array(vec![permanent_id, changing_id]);
        std::sync::Arc::make_mut(&mut trailer_dict).insert(b"ID".to_vec(), id_array);

        if use_xref_stream {
            let xref_stream_id = size;
            let xref_pos = writer.stream_position().map_err(PdfError::from)?;
            new_entries.insert(xref_stream_id, XRefEntry::InUse { offset: xref_pos, generation: 0 });
            
            let builder = XRefStreamBuilder::new(new_entries);
            let mut xref_stream_obj = builder.build(xref_stream_id + 1)?;

            if let Object::Stream(ref dict, _) = xref_stream_obj {
                let mut dict = dict.clone();
                let dict_mut = std::sync::Arc::make_mut(&mut dict);
                for (k, v) in trailer_dict.iter() {
                    if !matches!(k.as_slice(), b"Type" | b"W" | b"Index" | b"Size") {
                        dict_mut.insert(k.clone(), v.clone());
                    }
                }
                xref_stream_obj = if let Object::Stream(_, data) = xref_stream_obj {
                    Object::Stream(dict, data)
                } else { unreachable!() };
            }
            crate::serialize::write_indirect_object(writer, xref_stream_id, 0, &xref_stream_obj)?;
            write!(writer, "startxref\n{xref_pos}\n%%EOF\n").map_err(PdfError::from)?;
        } else {
            let xref_pos = writer.stream_position().map_err(PdfError::from)?;
            let index = MemoryXRefIndex { entries: new_entries };
            crate::serialize::write_xref_section(writer, &index.subsections())?;
            crate::serialize::write_trailer(writer, &trailer_dict, xref_pos)?;
        }
        Ok(())
    }

    /// Adds a new annotation to a page.
    /// (ISO 32000-2:2020 Clause 12.5)
    pub fn add_annotation(
        &mut self,
        page_ref: Reference,
        annot_dict: std::collections::BTreeMap<Vec<u8>, Object>,
    ) -> PdfResult<Reference> {
        // 1. Basic validation of required keys
        if annot_dict.get(b"Type".as_ref()) != Some(&Object::new_name(b"Annot".to_vec())) {
            return Err(PdfError::Validation(ValidationErrorVariant::General("Missing or invalid /Type /Annot".into())));
        }
        if !annot_dict.contains_key(b"Subtype".as_ref()) {
            return Err(PdfError::Validation(ValidationErrorVariant::General("Missing required key /Subtype".into())));
        }
        if !annot_dict.contains_key(b"Rect".as_ref()) {
            return Err(PdfError::Validation(ValidationErrorVariant::General("Missing required key /Rect".into())));
        }

        // 2. Create the annotation as an indirect object
        let annot_ref = self.create_object(Object::new_dict(annot_dict))?;

        // 3. Update the page's /Annots array
        let page_obj = self.get_object(&page_ref)?;
        let mut page_dict = if let Object::Dictionary(d) = page_obj { (*d).clone() } else {
            return Err(PdfError::InvalidType { expected: "Dictionary".into(), found: "Other".into() });
        };

        if let Some(annots_obj) = page_dict.get(b"Annots".as_ref()).cloned() {
            match annots_obj {
                Object::Array(arr) => {
                    let mut arr = (*arr).clone();
                    arr.push(Object::Reference(annot_ref));
                    page_dict.insert(b"Annots".to_vec(), Object::new_array(arr));
                    self.update_object(page_ref, Object::new_dict(page_dict));
                }
                Object::Reference(r) => {
                    let annots_resolved = self.get_object(&r)?;
                    if let Object::Array(arr) = annots_resolved {
                        let mut arr = (*arr).clone();
                        arr.push(Object::Reference(annot_ref));
                        self.update_object(r, Object::new_array(arr));
                    } else {
                        return Err(PdfError::InvalidType { expected: "Array".into(), found: "Other".into() });
                    }
                }
                _ => return Err(PdfError::InvalidType { expected: "Array or Reference".into(), found: "Other".into() }),
            }
        } else {
            // Create new /Annots array
            page_dict.insert(b"Annots".to_vec(), Object::new_array(vec![Object::Reference(annot_ref)]));
            self.update_object(page_ref, Object::new_dict(page_dict));
        }

        Ok(annot_ref)
    }

    /// Removes an annotation from a page.
    pub fn remove_annotation(&mut self, page_ref: Reference, annot_ref: Reference) -> PdfResult<()> {
        let page_obj = self.get_object(&page_ref)?;
        let mut page_dict = if let Object::Dictionary(d) = page_obj { (*d).clone() } else {
            return Err(PdfError::InvalidType { expected: "Dictionary".into(), found: "Other".into() });
        };

        if let Some(annots_obj) = page_dict.get(b"Annots".as_ref()).cloned() {
            match annots_obj {
                Object::Array(arr) => {
                    let mut arr = (*arr).clone();
                    arr.retain(|x| x != &Object::Reference(annot_ref));
                    page_dict.insert(b"Annots".to_vec(), Object::new_array(arr));
                    self.update_object(page_ref, Object::new_dict(page_dict));
                }
                Object::Reference(r) => {
                    let annots_resolved = self.get_object(&r)?;
                    if let Object::Array(arr) = annots_resolved {
                        let mut arr = (*arr).clone();
                        arr.retain(|x| x != &Object::Reference(annot_ref));
                        self.update_object(r, Object::new_array(arr));
                    } else {
                        return Err(PdfError::InvalidType { expected: "Array".into(), found: "Other".into() });
                    }
                }
                _ => return Err(PdfError::InvalidType { expected: "Array or Reference".into(), found: "Other".into() }),
            }
        }
        Ok(())
    }

    /// Reorders the pages in the document based on a new index mapping.
    /// (ISO 32000-2:2020 Clause 7.7.3)
    pub fn reorder_pages(&mut self, new_indices: &[usize]) -> PdfResult<()> {
        let catalog_ref = self.document.last_trailer.trailer_dict.get(b"Root".as_ref())
            .and_then(|o| if let Object::Reference(r) = o { Some(*r) } else { None })
            .ok_or_else(|| PdfError::StructureError(crate::core::StructureErrorVariant::MissingRoot))?;
        
        // Use a temporary catalog instance to find the page tree root
        let catalog_obj = self.get_object(&catalog_ref)?;
        let dict_arc = catalog_obj.as_dict_arc().ok_or_else(|| PdfError::InvalidType { expected: "Dictionary".into(), found: "Other".into() })?;
        let catalog = crate::catalog::Catalog::new(dict_arc, &self.resolver);
        let pages_ref = catalog.pages_root()?;

        // Collect all existing page references
        let tree = catalog.page_tree()?;
        let count = tree.get_count();
        if new_indices.len() != count {
            return Err(PdfError::Validation(crate::core::ValidationErrorVariant::General("New indices length mismatch".into())));
        }

        let mut old_page_refs = Vec::with_capacity(count);
        for i in 0..count {
            let page = tree.get_page(i)?;
            old_page_refs.push(page.reference);
        }

        // Create new kids array
        let mut new_kids = Vec::with_capacity(count);
        for &idx in new_indices {
            if idx >= count {
                return Err(PdfError::Validation(crate::core::ValidationErrorVariant::General(format!("Invalid page index {idx}"))));
            }
            new_kids.push(Object::Reference(old_page_refs[idx]));
        }

        // Rebuild /Pages dictionary (Flattened root for edits)
        let mut new_pages_dict = std::collections::BTreeMap::new();
        new_pages_dict.insert(b"Type".to_vec(), Object::new_name(b"Pages".to_vec()));
        new_pages_dict.insert(b"Count".to_vec(), Object::Integer(count as i64));
        new_pages_dict.insert(b"Kids".to_vec(), Object::new_array(new_kids));
        
        self.update_object(pages_ref, Object::new_dict(new_pages_dict));
        
        // Ensure all pages point to this root as Parent
        for &page_ref in &old_page_refs {
            let page_obj = self.get_object(&page_ref)?;
            let mut page_dict = page_obj.as_dict()
                .ok_or_else(|| PdfError::InvalidType { expected: "Dictionary".into(), found: "Other".into() })?
                .clone();
            page_dict.insert(b"Parent".to_vec(), Object::Reference(pages_ref));
            self.update_object(page_ref, Object::new_dict(page_dict));
        }

        Ok(())
    }

    /// Deletes a page from the document.
    pub fn delete_page(&mut self, index: usize) -> PdfResult<()> {
        let tree = self.document.page_tree_at(&self.resolver)?;
        let count = tree.get_count();
        if index >= count || count == 1 {
            return Err(PdfError::Validation(crate::core::ValidationErrorVariant::General("Cannot delete page (index out of range or last page)".into())));
        }

        let mut new_order = (0..count).collect::<Vec<_>>();
        new_order.remove(index);
        
        // We can reuse reorder logic if we modify it to accept shorter arrays, 
        // but for safety, let's implement a clean reorder that handles count changes.
        self.apply_page_layout(&new_order)
    }

    /// Internal helper to sync page tree after structural changes.
    fn apply_page_layout(&mut self, new_indices: &[usize]) -> PdfResult<()> {
        let catalog_ref = self.document.last_trailer.trailer_dict.get(b"Root".as_ref())
            .and_then(|o| if let Object::Reference(r) = o { Some(*r) } else { None })
            .ok_or_else(|| PdfError::StructureError(crate::core::StructureErrorVariant::MissingRoot))?;
        
        let catalog_obj = self.get_object(&catalog_ref)?;
        let dict_arc = catalog_obj.as_dict_arc().ok_or_else(|| PdfError::InvalidType { expected: "Dictionary".into(), found: "Other".into() })?;
        let catalog = crate::catalog::Catalog::new(dict_arc, &self.resolver);
        let pages_ref = catalog.pages_root()?;
        let tree = catalog.page_tree()?;
        let old_count = tree.get_count();

        let mut old_page_refs = Vec::with_capacity(old_count);
        for i in 0..old_count {
            let page = tree.get_page(i)?;
            old_page_refs.push(page.reference);
        }

        let mut new_kids = Vec::with_capacity(new_indices.len());
        for &idx in new_indices {
            new_kids.push(Object::Reference(old_page_refs[idx]));
        }

        let mut new_pages_dict = std::collections::BTreeMap::new();
        new_pages_dict.insert(b"Type".to_vec(), Object::new_name(b"Pages".to_vec()));
        new_pages_dict.insert(b"Count".to_vec(), Object::Integer(new_indices.len() as i64));
        new_pages_dict.insert(b"Kids".to_vec(), Object::new_array(new_kids));
        
        self.update_object(pages_ref, Object::new_dict(new_pages_dict));
        
        for &idx in new_indices {
            let page_ref = old_page_refs[idx];
            let page_obj = self.get_object(&page_ref)?;
            let mut page_dict = page_obj.as_dict()
                .ok_or_else(|| PdfError::InvalidType { expected: "Dictionary".into(), found: "Other".into() })?
                .clone();
            page_dict.insert(b"Parent".to_vec(), Object::Reference(pages_ref));
            self.update_object(page_ref, Object::new_dict(page_dict));
        }

        Ok(())
    }

    /// Imports a range of pages from another PDF document.
    /// (ISO 32000-2:2020 Clause 7.10.2)
    pub fn import_pages(&mut self, source_doc: &PdfDocument, range: std::ops::Range<usize>) -> PdfResult<()> {
        let source_tree = source_doc.page_tree()?;
        let source_count = source_tree.get_count();
        
        let mut imported_refs = Vec::new();
        let mut id_map = std::collections::BTreeMap::new();

        for i in range {
            if i >= source_count { break; }
            let source_page = source_tree.get_page(i)?;
            let new_ref = self.clone_object_recursively(&source_page.reference, source_page.resolver, &mut id_map)?;
            imported_refs.push(new_ref);
        }

        // Add these new pages to our page tree
        let catalog_ref = self.document.last_trailer.trailer_dict.get(b"Root".as_ref())
            .and_then(|o| if let Object::Reference(r) = o { Some(*r) } else { None })
            .ok_or_else(|| PdfError::StructureError(crate::core::StructureErrorVariant::MissingRoot))?;
        
        let catalog_obj = self.get_object(&catalog_ref)?;
        let dict_arc = catalog_obj.as_dict_arc().ok_or_else(|| PdfError::InvalidType { expected: "Dictionary".into(), found: "Other".into() })?;
        let catalog = crate::catalog::Catalog::new(dict_arc, &self.resolver);
        let pages_ref = catalog.pages_root()?;
        let tree = catalog.page_tree()?;
        let old_count = tree.get_count();

        let mut current_kids = Vec::with_capacity(old_count + imported_refs.len());
        for i in 0..old_count {
            let page = tree.get_page(i)?;
            current_kids.push(Object::Reference(page.reference));
        }

        for new_page_ref in imported_refs {
            current_kids.push(Object::Reference(new_page_ref));
        }

        let mut new_pages_dict = std::collections::BTreeMap::new();
        new_pages_dict.insert(b"Type".to_vec(), Object::new_name(b"Pages".to_vec()));
        new_pages_dict.insert(b"Count".to_vec(), Object::Integer(current_kids.len() as i64));
        new_pages_dict.insert(b"Kids".to_vec(), Object::new_array(current_kids));
        
        self.update_object(pages_ref, Object::new_dict(new_pages_dict));
        
        // Ensure ALL pages (old and new) in the new flat list point to this root
        if let Object::Dictionary(dict) = self.get_object(&pages_ref)? {
            if let Some(Object::Array(kids)) = dict.get(b"Kids".as_ref()) {
                for kid in kids.iter() {
                    if let Object::Reference(page_ref) = kid {
                        let page_obj = self.get_object(page_ref)?;
                        let mut page_dict = page_obj.as_dict()
                            .ok_or_else(|| PdfError::InvalidType { expected: "Dictionary".into(), found: "Other".into() })?
                            .clone();
                        page_dict.insert(b"Parent".to_vec(), Object::Reference(pages_ref));
                        self.update_object(*page_ref, Object::new_dict(page_dict));
                    }
                }
            }
        }

        Ok(())
    }

    fn clone_object_recursively(
        &mut self,
        source_ref: &Reference,
        source_resolver: &dyn crate::core::Resolver,
        id_map: &mut std::collections::BTreeMap<Reference, Reference>,
    ) -> PdfResult<Reference> {
        if let Some(target_ref) = id_map.get(source_ref) {
            return Ok(*target_ref);
        }

        let obj = source_resolver.resolve(source_ref)?;
        
        // Create target reference and advance next_object_id
        let target_ref = Reference { id: self.next_object_id, generation: 0 };
        self.next_object_id = self.next_object_id.checked_add(1).ok_or_else(|| PdfError::ResourceError("Object ID overflow during clone".into()))?;
        id_map.insert(*source_ref, target_ref);

        let cloned_obj = self.clone_value_recursively(obj, source_resolver, id_map)?;
        self.update_object(target_ref, cloned_obj);

        Ok(target_ref)
    }

    fn clone_value_recursively(
        &mut self,
        obj: Object,
        source_resolver: &dyn crate::core::Resolver,
        id_map: &mut std::collections::BTreeMap<Reference, Reference>,
    ) -> PdfResult<Object> {
        match obj {
            Object::Reference(r) => {
                let new_ref = self.clone_object_recursively(&r, source_resolver, id_map)?;
                Ok(Object::Reference(new_ref))
            }
            Object::Array(arr) => {
                let mut new_arr = Vec::with_capacity(arr.len());
                for item in arr.iter() {
                    new_arr.push(self.clone_value_recursively(item.clone(), source_resolver, id_map)?);
                }
                Ok(Object::new_array(new_arr))
            }
            Object::Dictionary(dict) => {
                let mut new_dict = std::collections::BTreeMap::new();
                for (k, v) in dict.iter() {
                    new_dict.insert(k.clone(), self.clone_value_recursively(v.clone(), source_resolver, id_map)?);
                }
                Ok(Object::new_dict(new_dict))
            }
            Object::Stream(dict, data) => {
                let mut new_dict = std::collections::BTreeMap::new();
                for (k, v) in dict.iter() {
                    new_dict.insert(k.clone(), self.clone_value_recursively(v.clone(), source_resolver, id_map)?);
                }
                Ok(Object::new_stream(new_dict, (*data).clone()))
            }
            _ => Ok(obj),
        }
    }

    /// Renames the structure type (S) of a structure element (Clause 14.7.2).
    pub fn rename_structure_element_tag(&mut self, reference: Reference, new_tag: Vec<u8>) -> PdfResult<()> {
        let dict = self.resolver.resolve(&reference)?.as_dict_arc()
            .ok_or_else(|| PdfError::StructureError(crate::core::StructureErrorVariant::InvalidFormat))?;
        
        let mut new_dict = (*dict).clone();
        new_dict.insert(b"S".to_vec(), Object::new_name(new_tag));
        
        self.update_object(reference, Object::new_dict_arc(std::sync::Arc::new(new_dict)));
        Ok(())
    }

    /// Moves a structure element to a new parent at a specific index (Clause 14.7).
    pub fn move_structure_element(&mut self, child_ref: Reference, new_parent_ref: Reference, index: usize) -> PdfResult<()> {
        // 1. Remove from current parent
        let catalog_ref = self.document.last_trailer.trailer_dict.get(b"Root".as_ref())
            .and_then(|o| if let Object::Reference(r) = o { Some(*r) } else { None })
            .ok_or_else(|| PdfError::StructureError(crate::core::StructureErrorVariant::MissingRoot))?;
        
        let catalog_obj = self.get_object(&catalog_ref)?;
        let dict_arc = catalog_obj.as_dict_arc().ok_or_else(|| PdfError::InvalidType { expected: "Dictionary".into(), found: "Other".into() })?;
        let catalog = crate::catalog::Catalog::new(dict_arc, &self.resolver);
        let structs_obj = catalog.dictionary.get(b"StructTreeRoot".as_ref()).cloned()
            .ok_or_else(|| PdfError::StructureError(crate::core::StructureErrorVariant::InvalidFormat))?;
        let _struct_root_ref = if let Object::Reference(r) = structs_obj { r } else {
            return Err(PdfError::StructureError(crate::core::StructureErrorVariant::InvalidFormat));
        };

        // Simplified scan for current parent in memory (overlay and base)
        // In a real implementation, we'd use the /P (Parent) entry if available.
        let child_obj = self.get_object(&child_ref)?;
        let child_dict = child_obj.as_dict().ok_or_else(|| PdfError::InvalidType { expected: "Dictionary".into(), found: "Other".into() })?;
        let current_parent_ref = child_dict.get(b"P".as_ref()).and_then(|o| if let Object::Reference(r) = o { Some(*r) } else { None });

        if let Some(p_ref) = current_parent_ref {
            let mut p_dict = self.get_object(&p_ref)?.as_dict()
                .ok_or_else(|| PdfError::InvalidType { expected: "Dictionary".into(), found: "Other".into() })?
                .clone();
            if let Some(Object::Array(kids)) = p_dict.get(b"Kids".as_ref()) {
                let mut new_kids = (**kids).clone();
                new_kids.retain(|k| if let Object::Reference(r) = k { r != &child_ref } else { true });
                p_dict.insert(b"Kids".to_vec(), Object::new_array(new_kids));
                self.update_object(p_ref, Object::new_dict(p_dict));
            }
        }

        // 2. Add to new parent
        let mut new_p_dict = self.get_object(&new_parent_ref)?.as_dict()
            .ok_or_else(|| PdfError::InvalidType { expected: "Dictionary".into(), found: "Other".into() })?
            .clone();
        
        let mut kids: Vec<Object> = if let Some(Object::Array(k)) = new_p_dict.get(b"Kids".as_ref()) {
            (**k).clone()
        } else if let Some(Object::Reference(r)) = new_p_dict.get(b"Kids".as_ref()) {
            self.get_object(r)?.as_array().map(|a| a.to_vec()).unwrap_or_default()
        } else {
            Vec::new()
        };

        let insert_idx = index.min(kids.len());
        kids.insert(insert_idx, Object::Reference(child_ref));
        new_p_dict.insert(b"Kids".to_vec(), Object::new_array(kids));
        self.update_object(new_parent_ref, Object::new_dict(new_p_dict));

        // 3. Update P entry in child
        let mut child_dict_mut = child_dict.clone();
        child_dict_mut.insert(b"P".to_vec(), Object::Reference(new_parent_ref));
        self.update_object(child_ref, Object::new_dict(child_dict_mut));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Object;
    use crate::xref::{MemoryXRefIndex, XRefEntry};
    

    #[test]
    fn test_editor_creation_and_object_id() {
        let mut index = MemoryXRefIndex::default();
        index.insert(1, XRefEntry::InUse { offset: 0, generation: 0 }); // Catalog
        index.insert(10, XRefEntry::InUse { offset: 100, generation: 0 });
        
        // Mock data containing a simple catalog dictionary at offset 0
        let mut data = b"1 0 obj << /Type /Catalog >> endobj".to_vec();
        data.extend(vec![0; 150]);

        let mut trailer_dict = std::collections::BTreeMap::new();
        trailer_dict.insert(b"Root".to_vec(), Object::Reference(Reference { id: 1, generation: 0 }));
        
        // Mock PdfDocument
        let doc = PdfDocument {
            data,
            xref_index: index,
            last_trailer: crate::trailer::TrailerInfo { 
                last_xref_offset: 0, 
                trailer_dict: trailer_dict.into() 
            },
            security: None,
        };

        let editor = PdfEditor::new(doc).expect("Failed to create editor");
        assert_eq!(editor.next_object_id, 11);
    }

    #[test]
    fn test_editor_create_object() {
        let mut index = MemoryXRefIndex::default();
        index.insert(1, XRefEntry::InUse { offset: 0, generation: 0 });

        let mut data = b"1 0 obj << /Type /Catalog >> endobj".to_vec();
        data.extend(vec![0; 50]);

        let mut trailer_dict = std::collections::BTreeMap::new();
        trailer_dict.insert(b"Root".to_vec(), Object::Reference(Reference { id: 1, generation: 0 }));

        let doc = PdfDocument {
            data,
            xref_index: index,
            last_trailer: crate::trailer::TrailerInfo { 
                last_xref_offset: 0, 
                trailer_dict: trailer_dict.into() 
            },
            security: None,
        };

        let mut editor = PdfEditor::new(doc).expect("Failed to create editor");
        let r = editor.create_object(Object::Integer(42)).expect("test");
        
        assert_eq!(r.id, 2);
        assert_eq!(editor.get_object(&r).unwrap(), Object::Integer(42));
        assert_eq!(editor.next_object_id, 3);
    }

    #[test]
    fn test_reorder_pages_logic() {
        let mut index = MemoryXRefIndex::default();
        index.insert(1, XRefEntry::InUse { offset: 0, generation: 0 }); // Catalog
        index.insert(2, XRefEntry::InUse { offset: 100, generation: 0 }); // Pages Root
        index.insert(3, XRefEntry::InUse { offset: 200, generation: 0 }); // Page 1
        index.insert(4, XRefEntry::InUse { offset: 300, generation: 0 }); // Page 2

        let mut data = vec![0; 1000];
        // 1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj
        data[0..100].copy_from_slice(&[0; 100]); // clear
        let cat_bytes = b"1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj";
        data[0..cat_bytes.len()].copy_from_slice(cat_bytes);
        
        // 2 0 obj << /Type /Pages /Count 2 /Kids [3 0 R 4 0 R] >> endobj
        let pages_bytes = b"2 0 obj << /Type /Pages /Count 2 /Kids [3 0 R 4 0 R] >> endobj";
        data[100..100+pages_bytes.len()].copy_from_slice(pages_bytes);

        data[200..207].copy_from_slice(b"3 0 obj");
        data[300..307].copy_from_slice(b"4 0 obj");
        let mut trailer_dict = std::collections::BTreeMap::new();
        trailer_dict.insert(b"Root".to_vec(), Object::Reference(Reference { id: 1, generation: 0 }));

        let doc = PdfDocument {
            data,
            xref_index: index,
            last_trailer: crate::trailer::TrailerInfo { 
                last_xref_offset: 0, 
                trailer_dict: trailer_dict.into() 
            },
            security: None,
        };

        let mut editor = PdfEditor::new(doc).expect("Failed to create editor");
        
        // Mock Catalog
        let mut cat_dict = std::collections::BTreeMap::new();
        cat_dict.insert(b"Type".to_vec(), Object::new_name(b"Catalog".to_vec()));
        cat_dict.insert(b"Pages".to_vec(), Object::Reference(Reference { id: 2, generation: 0 }));
        editor.update_object(Reference { id: 1, generation: 0 }, Object::new_dict(cat_dict));

        // Mock Pages Root
        let mut pages_dict = std::collections::BTreeMap::new();
        pages_dict.insert(b"Type".to_vec(), Object::new_name(b"Pages".to_vec()));
        pages_dict.insert(b"Count".to_vec(), Object::Integer(2));
        pages_dict.insert(b"Kids".to_vec(), Object::new_array(vec![
            Object::Reference(Reference { id: 3, generation: 0 }),
            Object::Reference(Reference { id: 4, generation: 0 }),
        ]));
        editor.update_object(Reference { id: 2, generation: 0 }, Object::new_dict(pages_dict));

        // Mock Page 1 & 2
        for id in 3..=4 {
            let mut p = std::collections::BTreeMap::new();
            p.insert(b"Type".to_vec(), Object::new_name(b"Page".to_vec()));
            p.insert(b"Parent".to_vec(), Object::Reference(Reference { id: 2, generation: 0 }));
            editor.update_object(Reference { id: id as u32, generation: 0 }, Object::new_dict(p));
        }

        // Action: Reorder (Reverse)
        editor.reorder_pages(&[1, 0]).expect("Reorder failed");

        // Verify
        let new_pages = editor.get_object(&Reference { id: 2, generation: 0 }).unwrap();
        let kids = new_pages.as_dict().unwrap().get(b"Kids".as_ref()).unwrap().as_array().unwrap();
        assert_eq!(kids[0], Object::Reference(Reference { id: 4, generation: 0 }));
        assert_eq!(kids[1], Object::Reference(Reference { id: 3, generation: 0 }));
    }
}
