use std::collections::BTreeMap;
use bytes::Bytes;
use parking_lot::RwLock;
use ferruginous_core::{Object, Parser, PdfResult, PdfError, Reference, Resolver};
use ferruginous_core::lexer::Token;
use crate::xref::{XRefStore, XRefEntry, parse_xref_table, parse_xref_stream, is_pdf_whitespace};
use crate::security::{SecurityHandler, StandardSecurityHandler, NullSecurityHandler};
use crate::signature::Signature;
use crate::validation::{SignatureVerifier, ValidationStatus};
use std::sync::Arc;

pub struct Document {
    data: Bytes,
    store: XRefStore,
    root: Reference,
    cache: RwLock<BTreeMap<Reference, Object>>,
    security: Arc<dyn SecurityHandler>,
}

#[derive(Debug, Clone)]
pub struct SignatureVerificationResult {
    pub signature_id: u32,
    pub status: ValidationStatus,
    pub name: Option<String>,
    pub date: Option<String>,
    pub mdp_status: MdpStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MdpStatus {
    NoModifications,
    AllowedModifications,
    DisallowedModifications(String),
    NotSignatoryRevision,
}

impl Document {
    pub fn open(data: Bytes) -> PdfResult<Self> {
        Self::open_with_password(data, b"")
    }

    pub fn open_with_password(data: Bytes, password: &[u8]) -> PdfResult<Self> {
        let startxref_pos = find_startxref(&data)?;
        let mut store = XRefStore::new();
        // ... (rest of the logic omitted for brevity in chunking, assuming standard open)
        
        // Chain traversal for incremental updates
        let mut next_xref = Some(startxref_pos);
        while let Some(mut pos) = next_xref {
            // Skip leading whitespace before XRef
            while pos < data.len() && is_pdf_whitespace(data[pos]) {
                pos += 1;
            }
            
            let chunk = &data[pos..];
            let (index, next_prev) = if chunk.starts_with(b"xref") {
                // Legacy table
                let (idx, remaining_buf) = parse_xref_table(chunk)?;
                let remaining_offset = data.len() - remaining_buf.len();
                
                // Parse trailer
                let mut parser = Parser::new(data.slice(remaining_offset..));
                match parser.next()? {
                    Some(Token::Keyword(ref b)) if b.as_ref() == b"trailer" => {
                        let trailer = parser.parse_object()?.as_dict()
                            .ok_or_else(|| PdfError::Other("Invalid trailer".into()))?.clone();
                        
                        // Merge trailer keys (latest takes precedence)
                        for (k, v) in &trailer {
                            store.trailer.entry(k.clone()).or_insert(v.clone());
                        }
                        
                        let prev = trailer.get(&"Prev".into()).and_then(|o| o.as_i64()).map(|i| i as usize);
                        (idx, prev)
                    }
                    _ => (idx, None)
                }
            } else {
                // Potential XRef stream (Standard 1.5+)
                let mut parser = Parser::new(data.slice(pos..));
                
                // Try parsing as an indirect object header first
                let obj_res = match parser.parse_indirect_object_header() {
                    Ok(_) => parser.parse_object(),
                    Err(_) => {
                        // Fallback: try parsing as a literal object if header is missing
                        let mut p2 = Parser::new(data.slice(pos..));
                        p2.parse_object()
                    }
                };

                match obj_res {
                    Ok(obj) => {
                        if let Some(dict) = obj.as_dict() {
                            if dict.get(&"Type".into()).and_then(|o| o.as_name()).map(|n| n.as_ref()) == Some(b"XRef") {
                                if let Object::Stream(ref s_dict, _) = obj {
                                    let decoded_data = obj.decode_stream()?;
                                    let idx = parse_xref_stream(s_dict, &decoded_data)?;
                                    merge_trailer(&mut store, s_dict);
                                    let prev = s_dict.get(&"Prev".into()).and_then(|o| o.as_i64()).map(|i| i as usize);
                                    (idx, prev)
                                } else {
                                    return Err(PdfError::Other("Expected stream for XRef".into()));
                                }
                            } else {
                                // Not an XRef stream, maybe malformed legacy?
                                parse_malformed_legacy(chunk, &data, &mut store)?
                            }
                        } else {
                            // Not a dictionary, check for malformed legacy
                            parse_malformed_legacy(chunk, &data, &mut store)?
                        }
                    }
                    Err(_) => {
                        // Object parsing failed, check for malformed legacy
                        parse_malformed_legacy(chunk, &data, &mut store)?
                    }
                }
            };
            
            store.merge(index);
            next_xref = next_prev;
        }

        // Finalize root from the last effective trailer (the one encountered first in our chain walk)
        let root = store.trailer.get(&"Root".into())
            .and_then(|o| if let Object::Reference(r) = o { Some(r.clone()) } else { None })
            .ok_or_else(|| PdfError::Other("Missing /Root in trailer".into()))?;

        // Initialize Security Handler
        let security: Arc<dyn SecurityHandler> = if let Some(Object::Dictionary(e_dict)) = store.trailer.get(&"Encrypt".into()) {
            Arc::new(StandardSecurityHandler::new(&e_dict, password)?)
        } else {
            Arc::new(NullSecurityHandler)
        };

        Ok(Self {
            data,
            store,
            root,
            cache: RwLock::new(BTreeMap::new()),
            security,
        })
    }

    pub fn root(&self) -> Reference {
        self.root
    }

    pub fn get_page_root(&self) -> PdfResult<Reference> {
        let catalog = self.resolve(&self.root)?.as_dict()
            .ok_or_else(|| PdfError::Other("Invalid catalog".into()))?.clone();
        catalog.get(&"Pages".into())
            .and_then(|o| o.as_reference())
            .ok_or_else(|| PdfError::Other("Missing /Pages in catalog".into()))
    }

    pub fn get_page_count(&self) -> PdfResult<usize> {
        let pages_root = self.get_page_root()?;
        crate::page::PageTree::new(pages_root, self).count()
    }

    pub fn get_page(&self, index: usize) -> PdfResult<crate::page::Page<'_>> {
        let pages_root = self.get_page_root()?;
        crate::page::PageTree::new(pages_root, self).page(index)
    }

    fn is_encryption_dict(&self, reference: &Reference) -> bool {
        if let Some(Object::Reference(r)) = self.store.trailer.get(&"Encrypt".into()) {
            return r == reference;
        }
        false
    }

    /// Discovers all digital signatures in the document.
    pub fn signatures(&self) -> PdfResult<Vec<Signature>> {
        let mut signatures = Vec::new();
        
        // Scan all pages for /Sig widget annotations
        let catalog = self.resolve(&self.root)?.as_dict()
            .ok_or_else(|| PdfError::Other("Invalid catalog".into()))?.clone();
        
        let pages_ref = catalog.get(&"Pages".into())
            .and_then(|o| o.as_reference())
            .ok_or_else(|| PdfError::Other("Missing /Pages in catalog".into()))?;
            
        let mut stack = vec![pages_ref];
        while let Some(current_ref) = stack.pop() {
            let node = self.resolve(&current_ref)?.as_dict()
                .ok_or_else(|| PdfError::Other("Invalid page tree node".into()))?.clone();
                
            if node.get(&"Type".into()).and_then(|o| o.as_name()).map(|n| n.as_ref()) == Some(b"Pages") {
                if let Some(kids) = node.get(&"Kids".into()).and_then(|o| o.as_array()) {
                    for kid in kids.iter() {
                        if let Some(r) = kid.as_reference() {
                            stack.push(r);
                        }
                    }
                }
            } else {
                // It's a Page
                if let Some(annots) = node.get(&"Annots".into()).and_then(|o| o.as_array()) {
                    for annot_ref in annots.iter() {
                        if let Some(r) = annot_ref.as_reference() {
                            let annot = self.resolve(&r)?.as_dict()
                                .ok_or_else(|| PdfError::Other("Invalid annotation".into()))?.clone();
                                
                            if let Some(v) = annot.get(&"Subtype".into())
                                .and_then(|o| o.as_name())
                                .filter(|n| n.as_ref() == b"Widget")
                                .and_then(|_| annot.get(&"FT".into()))
                                .and_then(|o| o.as_name())
                                .filter(|n| n.as_ref() == b"Sig")
                                .and_then(|_| annot.get(&"V".into()))
                                .and_then(|o| o.as_reference()) 
                            {
                                let sig_dict = self.resolve(&v)?.as_dict()
                                    .ok_or_else(|| PdfError::Other("Invalid signature dictionary".into()))?.clone();
                                signatures.push(Signature::from_object(v.id, &sig_dict)?);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(signatures)
    }

    /// Verifies all digital signatures in the document.
    pub fn verify_signatures(&self) -> PdfResult<Vec<SignatureVerificationResult>> {
        let signatures = self.signatures()?;
        let mut results = Vec::new();
        let verifier = SignatureVerifier::with_root(self, self.root.id);

        for sig in signatures {
            let status = verifier.verify(&sig, &self.data)?;
            
            // Check for modifications after this signature
            let mdp_status = self.check_mdp_compliance(&sig)?;

            results.push(SignatureVerificationResult {
                signature_id: sig.obj_id,
                status,
                name: sig.name.clone(),
                date: sig.date.clone(),
                mdp_status,
            });
        }

        Ok(results)
    }

    fn check_mdp_compliance(&self, sig: &Signature) -> PdfResult<MdpStatus> {
        if sig.byte_range.len() < 4 {
            return Err(PdfError::Other("Invalid ByteRange".into()));
        }

        let last_offset = sig.byte_range[2];
        let last_len = sig.byte_range[3];
        let covered_end = last_offset + last_len;

        if covered_end < self.data.len() {
            // Document has been modified after this signature
            if let Some(doc_mdp) = &sig.doc_mdp {
                match doc_mdp.p {
                    1 => return Ok(MdpStatus::DisallowedModifications("DocMDP Level 1: No changes permitted".into())),
                    2 => {
                        // Level 2: Permits form filling.
                        // For now, we flag as "Allowed" if we don't have detailed diffing logic
                        return Ok(MdpStatus::AllowedModifications);
                    }
                    3 => return Ok(MdpStatus::AllowedModifications),
                    _ => return Ok(MdpStatus::DisallowedModifications(format!("Unknown DocMDP Level: {}", doc_mdp.p))),
                }
            }
            
            // If no MDP is specified, any modification might be suspicious but not strictly prohibited by MDP
            return Ok(MdpStatus::DisallowedModifications("Incremental update detected without MDP permission".into()));
        }

        Ok(MdpStatus::NoModifications)
    }
    pub fn compliance_info(&self) -> PdfResult<crate::conformance::ComplianceInfo> {
        crate::conformance::ComplianceInfo::extract(self, &self.root)
    }

    /// Exposes the internal object store.
    pub fn store(&self) -> &XRefStore {
        &self.store
    }

    /// Exposes the unified document trailer (after incremental updates).
    pub fn trailer(&self) -> &BTreeMap<ferruginous_core::PdfName, Object> {
        &self.store.trailer
    }

    /// Recursively discovers all unique object IDs reachable from the given root reference.
    /// This is essential for Phase 12 Linearization discovery.
    pub fn explore_dependencies(&self, root: &Reference) -> PdfResult<std::collections::HashSet<u32>> {
        let mut visited = std::collections::HashSet::new();
        let mut stack = vec![*root];
        
        while let Some(r) = stack.pop() {
            if visited.contains(&r.id) {
                continue;
            }
            visited.insert(r.id);
            
            // Resolve object to find its internal nested references
            // We use a lenient match here to skip potentially broken references during the crawl
            if let Ok(obj) = self.resolve(&r) {
                let mut refs = std::collections::HashSet::new();
                obj.gather_references(&mut refs);
                
                for id in refs {
                    if let Some(entry) = self.store.get(id) {
                        let generation_num = match entry {
                            crate::xref::XRefEntry::InUse { generation, .. } => generation,
                            crate::xref::XRefEntry::Compressed { .. } => 0,
                            _ => 0,
                        };
                        stack.push(Reference::new(id, generation_num));
                    }
                }
            }
        }
        Ok(visited)
    }

    fn resolve_compressed_object(&self, container_id: u32, index: u32) -> PdfResult<Object> {
        // Resolve the container stream
        let container_ref = Reference::new(container_id, 0);
        let container_obj = self.resolve(&container_ref)?;
        
        let (dict, _) = container_obj.as_stream().ok_or_else(|| PdfError::Other("Object stream container is not a stream".into()))?;
        if dict.get(&"Type".into()).and_then(|o| o.as_name()).map(|n| n.as_ref()) != Some(b"ObjStm") {
             return Err(PdfError::Other("Object stream container lacks /Type /ObjStm".into()));
        }

        let n = dict.get(&"N".into()).and_then(|o| o.as_i64()).ok_or_else(|| PdfError::Other("Missing /N in ObjStm".into()))? as usize;
        let first = dict.get(&"First".into()).and_then(|o| o.as_i64()).ok_or_else(|| PdfError::Other("Missing /First in ObjStm".into()))? as usize;

        let decoded_data = container_obj.decode_stream()?;
        
        // Parse the index portion of the ObjStm
        // Header is N pairs of [obj_id offset]
        let mut parser = Parser::new(decoded_data.slice(..first));
        let mut target_offset = None;
        
        for i in 0..n {
            let _obj_id = parser.parse_object()?.as_i64().ok_or_else(|| PdfError::Other("Invalid obj_id in ObjStm header".into()))? as u32;
            let offset = parser.parse_object()?.as_i64().ok_or_else(|| PdfError::Other("Invalid offset in ObjStm header".into()))? as usize;
            
            if i == index as usize {
                target_offset = Some(offset);
            }
        }

        let offset = target_offset.ok_or_else(|| PdfError::Other(format!("Compressed index {} out of range in ObjStm {}", index, container_id)))?;
        
        // Parse the object at the calculated offset
        let mut obj_parser = Parser::new(decoded_data.slice(first + offset..))
            .with_resolver(self);
        obj_parser.parse_object()
    }
}

impl Resolver for Document {
    fn resolve(&self, reference: &Reference) -> PdfResult<Object> {
        // Check cache first
        if let Some(obj) = self.cache.read().get(reference) {
            return Ok(obj.clone());
        }

        let entry = self.store.get(reference.id).ok_or(PdfError::ObjectNotFound(*reference))?;
        let mut obj = match entry {
            XRefEntry::InUse { offset, .. } => {
                let mut parser = Parser::new(self.data.slice(offset as usize..))
                    .with_resolver(self);
                parser.parse_indirect_object_header()?;
                parser.parse_object()?
            }
            XRefEntry::Compressed { container_id, index } => {
                self.resolve_compressed_object(container_id, index)?
            }
            _ => return Err(PdfError::Other("Attempted to resolve free or invalid object".into())),
        };

        // Decryption
        if !self.is_encryption_dict(reference) {
            let skip_decryption = if let Object::Stream(dict, _) = &obj {
                dict.get(&"Type".into()).and_then(|o| o.as_name()).map(|n| n.as_ref()) == Some(b"Metadata") && !self.security.encrypt_metadata()
            } else {
                false
            };

            if !skip_decryption {
                match &mut obj {
                    Object::String(b) => {
                        let decrypted = self.security.decrypt_bytes(b, reference.id, reference.generation)?;
                        *b = Bytes::from(decrypted);
                    }
                    Object::Stream(_d, b) => {
                        let decrypted = self.security.decrypt_bytes(b, reference.id, reference.generation)?;
                        *b = Bytes::from(decrypted);
                    }
                    _ => {}
                }
            }
        }

        self.cache.write().insert(*reference, obj.clone());
        Ok(obj)
    }
}

fn parse_malformed_legacy(
    chunk: &[u8],
    data: &Bytes,
    store: &mut XRefStore,
) -> PdfResult<(crate::xref::XRefIndex, Option<usize>)> {
    if chunk.is_empty() || !(chunk[0] as char).is_ascii_digit() {
        return Err(PdfError::Other("Not a malformed legacy XRef section".into()));
    }
    
    use crate::xref::parse_xref_table_inner;
    let (idx, remaining_buf) = parse_xref_table_inner(chunk, 0)?;
    let remaining_offset = data.len() - remaining_buf.len();
    
    let mut parser = Parser::new(data.slice(remaining_offset..));
    match parser.next()? {
        Some(Token::Keyword(ref b)) if b.as_ref() == b"trailer" => {
            let trailer = parser.parse_object()?.as_dict()
                .ok_or_else(|| PdfError::Other("Invalid trailer".into()))?.clone();
            merge_trailer(store, &trailer);
            let prev = trailer.get(&"Prev".into()).and_then(|o| o.as_i64()).map(|i| i as usize);
            Ok((idx, prev))
        }
        _ => Ok((idx, None))
    }
}

fn merge_trailer(store: &mut XRefStore, trailer: &BTreeMap<ferruginous_core::PdfName, Object>) {
    for (k, v) in trailer {
        store.trailer.entry(k.clone()).or_insert(v.clone());
    }
}

fn find_startxref(data: &[u8]) -> PdfResult<usize> {
    let end_search = data.len().saturating_sub(1024);
    let end = &data[end_search..];
    let pos = end.windows(9).rposition(|w| w == b"startxref").ok_or_else(|| PdfError::Syntactic { pos: 0, message: "Missing startxref".into() })?;
    let start = end_search + pos + 9;
    let s = std::str::from_utf8(&data[start..]).map_err(|_| PdfError::Other("Invalid UTF-8 in startxref".into()))?;
    let offset = s.split_whitespace().next().ok_or_else(|| PdfError::Syntactic { pos: start, message: "Missing offset".into() })?.parse::<usize>().map_err(|_| PdfError::Other("Invalid offset".into()))?;
    Ok(offset)
}
