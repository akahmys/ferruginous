//! High-level PDF document loader and entry point.
//!
//! (ISO 32000-2:2020 Clause 7.5.8)

use crate::xref::{MemoryXRefIndex, parse_xref_section, parse_xref_stream_content};
use crate::trailer::{find_trailer_info, TrailerInfo};
use crate::lexer::parse_object;
use crate::core::{Object, Resolver, PdfError, PdfResult, ParseErrorVariant, StructureErrorVariant, ContentErrorVariant};
use std::collections::BTreeSet;
use std::convert::TryInto;
use bytes::Bytes;

#[cfg(feature = "legacy")]
use ferruginous_bridge_legacy::{LopdfBridge, LegacyBridge};

/// Represents the physical and logical structure of a PDF document.
/// (ISO 32000-2:2020 Clause 7.5.8)
#[derive(Clone)]
pub struct PdfDocument {
    /// Raw byte buffer of the PDF file.
    pub data: Bytes,
    /// Flattened cross-reference index.
    pub xref_index: MemoryXRefIndex,
    /// Information from the last trailer dictionary.
    pub last_trailer: TrailerInfo,
    /// Security handler for encrypted documents.
    pub security: Option<std::sync::Arc<crate::security::SecurityHandler>>,
}

fn parse_single_xref_section(
    data: &[u8], 
    offset: usize
) -> PdfResult<(MemoryXRefIndex, Object)> {
    debug_assert!(!data.is_empty(), "parse_single: data empty");
    debug_assert!(offset < data.len(), "parse_single: offset out of bounds");
    if data.get(offset..offset + 4) == Some(b"xref") {
        let (_, section) = parse_xref_section(&data[offset..])
            .map_err(|e| PdfError::ParseError(ParseErrorVariant::general(offset as u64, format!("Xref parse error at {offset}: {e:?}"))))?;
        Ok((section.index, Object::new_dict_arc(section.trailer)))
    } else {
        let obj_bytes = &data[offset..];
        let (input, _header) = crate::lexer::parse_id_gen_obj(obj_bytes)
            .map_err(|e| PdfError::ParseError(ParseErrorVariant::HeaderError { offset: offset as u64, details: format!("{e:?}") }))?;
        let (_, obj) = parse_object(input)
            .map_err(|e| PdfError::ParseError(ParseErrorVariant::general(offset as u64, format!("Xref stream parse error: {e:?}"))))?;
        
        if let Object::Stream(dict, stream_data) = obj {
            if dict.get(b"Type".as_ref()) == Some(&Object::new_name(b"XRef".to_vec())) {
                let decoded_data = crate::filter::decode_stream(&dict, &stream_data)
                    .map_err(|e| PdfError::ContentError(ContentErrorVariant::General(e.to_string())))?;
                let index = parse_xref_stream_content(&decoded_data, &dict)
                    .map_err(|e| PdfError::ParseError(ParseErrorVariant::general(offset as u64, e.to_string())))?;
                return Ok((index, Object::new_stream_bytes(std::sync::Arc::clone(&dict), stream_data)));
            }
        }
        Err(PdfError::StructureError(StructureErrorVariant::NoXRefFound(offset)))
    }
}

/// Loads the physical structure of a PDF (`XRef` tables and trailers).
pub fn load_document_structure(data: &[u8]) -> PdfResult<PdfDocument> {
    debug_assert!(!data.is_empty(), "load_structure: data empty");
    
    let mut data_bytes = Bytes::copy_from_slice(data);

    #[cfg(feature = "legacy")]
    {
        if !data.starts_with(b"%PDF-2.0") {
            let bridge = LopdfBridge::new();
            if let Ok(normalized) = bridge.load_and_normalize(data_bytes.clone()) {
                data_bytes = normalized;
            }
        }
    }

    let start_pos = find_pdf_header(&data_bytes)?;
    let initial_info = find_trailer_info(&data_bytes)
        .map_err(|e| PdfError::ParseError(ParseErrorVariant::general(0, e.to_string())))?;

    let (merged_index, trailer_dict) = resolve_xref_chain(&data_bytes, start_pos, initial_info.last_xref_offset)?;
    
    // Create new TrailerInfo from the resolved trailer dictionary
    let final_info = TrailerInfo {
        last_xref_offset: initial_info.last_xref_offset,
        trailer_dict,
    };

    let security = init_security(data, &merged_index, &final_info);
    
    Ok(PdfDocument { 
        data: data_bytes,
        xref_index: merged_index, 
        last_trailer: final_info,
        security,
    })
}

fn find_pdf_header(data: &[u8]) -> PdfResult<usize> {
    let search_limit = std::cmp::min(data.len(), 1024);
    data[..search_limit].windows(5).position(|w| w == b"%PDF-")
        .ok_or_else(|| PdfError::ParseError(ParseErrorVariant::general(0, "Could not find PDF header")))
}

fn resolve_xref_chain(data: &[u8], start_pos: usize, initial_offset: u64) -> PdfResult<(MemoryXRefIndex, std::sync::Arc<std::collections::BTreeMap<Vec<u8>, Object>>)> {
    let mut merged_index = MemoryXRefIndex::default();
    let mut visited_offsets = BTreeSet::new();
    let mut current_xref_offset = initial_offset;
    let mut primary_trailer_dict = None;
    let mut loop_count = 0;
    const MAX_XREF_LAYERS: usize = 1000;

    loop {
        loop_count += 1;
        if loop_count > MAX_XREF_LAYERS { return Err(PdfError::StructureError(StructureErrorVariant::TooManyXRefLayers)); }
        
        let offset_usize = adjust_xref_offset(data, start_pos, current_xref_offset)?;
        if visited_offsets.contains(&(offset_usize as u64)) { break; }
        visited_offsets.insert(offset_usize as u64);

        let (index, trailer_obj) = parse_single_xref_section(data, offset_usize)?;
        
        // The first trailer we encounter is the primary one (it contains /Root)
        let current_trailer = match trailer_obj {
            Object::Dictionary(d) => d,
            Object::Stream(d, _) => d,
            _ => return Err(PdfError::StructureError(StructureErrorVariant::InvalidTrailerType)),
        };

        if primary_trailer_dict.is_none() {
            primary_trailer_dict = Some(std::sync::Arc::clone(&current_trailer));
        }

        for (id, entry) in index.entries {
            merged_index.entries.entry(id).or_insert(entry);
        }
        
        if let Some(Object::Integer(prev)) = current_trailer.get(b"Prev".as_ref()) {
            current_xref_offset = (*prev).try_into().map_err(|_| PdfError::StructureError(StructureErrorVariant::InvalidPrev))?;
        } else if let Some(Object::Reference(r)) = current_trailer.get(b"XRefStm".as_ref()) {
             // Handle hybrid-reference PDF where a trailer dictionary points to a hidden XRef stream
             // (Simplified handled as a loop hop)
             current_xref_offset = r.id as u64; // This is a simplification
        } else { break; }
    }
    
    let trailer_dict = primary_trailer_dict.ok_or(PdfError::StructureError(StructureErrorVariant::MissingRoot))?;
    Ok((merged_index, trailer_dict))
}

fn adjust_xref_offset(data: &[u8], start_pos: usize, offset: u64) -> PdfResult<usize> {
    let mut offset_usize: usize = offset.try_into()
        .map_err(|_| PdfError::ParseError(ParseErrorVariant::InvalidOffset { offset: offset as usize }))?;
    
    if offset_usize >= data.len() || (data.get(offset_usize..offset_usize+4) != Some(b"xref") && data.get(offset_usize).is_none_or(|b| !b.is_ascii_digit())) {
        let adjusted = offset_usize + start_pos;
        if adjusted < data.len() {
            offset_usize = adjusted;
        }
    }

    while offset_usize < data.len() && data[offset_usize].is_ascii_whitespace() { offset_usize += 1; }
    Ok(offset_usize)
}

fn init_security(
    data: &[u8],
    index: &crate::xref::MemoryXRefIndex,
    trailer: &crate::trailer::TrailerInfo
) -> Option<std::sync::Arc<crate::security::SecurityHandler>> {
    debug_assert!(!data.is_empty(), "init_security: data empty");
    debug_assert!(!index.entries.is_empty(), "init_security: empty index");
    use crate::resolver::PdfResolver;
    use std::sync::Arc;
    
    let encrypt_ref = match trailer.trailer_dict.get(b"Encrypt".as_ref()) {
        Some(Object::Reference(r)) => Some(*r),
        _ => None,
    };
    
    if let Some(r) = encrypt_ref {
        let resolver = PdfResolver {
            data,
            index: Arc::new(index.clone()),
            security: None,
            cache: std::sync::Mutex::new(std::collections::BTreeMap::new()),
        };
        if let Ok(Object::Dictionary(dict)) = resolver.resolve(&r) {
            if let Ok(handler) = crate::security::SecurityHandler::new(&dict, trailer.id().as_deref().map(|v| v.as_ref()), b"") {
                return Some(Arc::new(handler));
            }
        }
    }
    None
}

impl PdfDocument {
    /// Returns the root Catalog of the PDF document.
    pub fn catalog(&self) -> PdfResult<crate::catalog::Catalog<'_>> {
        use crate::resolver::PdfResolver;
        use std::sync::Arc;
        
        let root_ref = self.last_trailer.root()
            .ok_or(PdfError::StructureError(StructureErrorVariant::MissingRoot))?;
        
        let resolver = PdfResolver {
            data: &self.data,
            index: Arc::new(self.xref_index.clone()),
            security: self.security.clone(),
            cache: std::sync::Mutex::new(std::collections::BTreeMap::new()),
        };
        
        let obj = resolver.resolve(&root_ref)
            .map_err(|e| PdfError::ResourceError(format!("Failed to resolve /Root: {e}")))?;

        if let Object::Dictionary(dict) = obj {
            debug_assert!(!dict.is_empty());
            Ok(crate::catalog::Catalog::new(std::sync::Arc::clone(&dict), Box::leak(Box::new(resolver))))
        } else {
            Err(PdfError::InvalidType { expected: "Dictionary".into(), found: "Other".into() })
        }
    }

    /// Returns the `PageTree` for the document using the provided resolver.
    pub fn page_tree_at<'a, R: crate::core::Resolver + 'a>(&self, resolver: &'a R) -> PdfResult<crate::page::PageTree<'a>> {
        let root_ref = self.last_trailer.root()
            .ok_or(PdfError::StructureError(StructureErrorVariant::MissingRoot))?;
        
        let obj = resolver.resolve(&root_ref)?;
        if let Object::Dictionary(dict) = obj {
            let catalog = crate::catalog::Catalog::new(dict, resolver);
            catalog.page_tree()
        } else {
            Err(PdfError::InvalidType { expected: "Dictionary".into(), found: "Other".into() })
        }
    }

    /// Returns a new resolver for the document.
    pub fn resolver(&self) -> crate::resolver::PdfResolver<'_> {
        use crate::resolver::PdfResolver;
        use std::sync::Arc;
        PdfResolver {
            data: &self.data,
            index: Arc::new(self.xref_index.clone()),
            security: self.security.clone(),
            cache: std::sync::Mutex::new(std::collections::BTreeMap::new()),
        }
    }

    /// Returns the `PageTree` for the document using the internally managed catalog and resolver.
    pub fn page_tree(&self) -> PdfResult<crate::page::PageTree<'_>> {
        let catalog = self.catalog()?;
        catalog.page_tree()
            .map_err(|e| PdfError::ResourceError(e.to_string()))
    }
}
