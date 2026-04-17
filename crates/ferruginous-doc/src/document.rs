use std::collections::BTreeMap;
use bytes::Bytes;
use parking_lot::RwLock;
use ferruginous_core::{Object, Parser, PdfResult, PdfError, Reference, Resolver};
use ferruginous_core::lexer::Token;
use crate::xref::{XRefStore, XRefEntry, parse_xref_table, parse_xref_stream};

pub struct Document {
    data: Bytes,
    store: XRefStore,
    root: Reference,
    cache: RwLock<BTreeMap<Reference, Object>>,
}

impl Document {
    pub fn open(data: Bytes) -> PdfResult<Self> {
        let startxref_pos = find_startxref(&data)?;
        let mut store = XRefStore::new();
        
        // Chain traversal for incremental updates
        let mut next_xref = Some(startxref_pos);
        while let Some(pos) = next_xref {
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
                // Potential XRef stream
                let mut parser = Parser::new(data.slice(pos..));
                let obj = parser.parse_object()?;
                let dict = obj.as_dict().ok_or_else(|| PdfError::Other("Expected XRef stream".into()))?;
                if dict.get(&"Type".into()).and_then(|o| o.as_name()).map(|n| n.as_ref()) == Some(b"XRef") {
                    if let Object::Stream(s_dict, s_data) = obj {
                        let idx = parse_xref_stream(&s_dict, &s_data)?;
                        
                        // Merge XRef stream dict (latest takes precedence)
                        for (k, v) in s_dict.as_ref() {
                            store.trailer.entry(k.clone()).or_insert(v.clone());
                        }
                        
                        let prev = s_dict.get(&"Prev".into()).and_then(|o| o.as_i64()).map(|i| i as usize);
                        (idx, prev)
                    } else {
                        return Err(PdfError::Other("Expected stream for XRef".into()));
                    }
                } else {
                    return Err(PdfError::Other("Not an XRef section".into()));
                }
            };
            
            store.merge(index);
            next_xref = next_prev;
        }

        // Finalize root from the last effective trailer (the one encountered first in our chain walk)
        let root = store.trailer.get(&"Root".into())
            .and_then(|o| if let Object::Reference(r) = o { Some(*r) } else { None })
            .ok_or_else(|| PdfError::Other("Missing /Root in trailer".into()))?;

        Ok(Self {
            data,
            store,
            root,
            cache: RwLock::new(BTreeMap::new()),
        })
    }

    pub fn root(&self) -> Reference {
        self.root
    }
}

impl Resolver for Document {
    fn resolve(&self, reference: &Reference) -> PdfResult<Object> {
        // Check cache first
        if let Some(obj) = self.cache.read().get(reference) {
            return Ok(obj.clone());
        }

        let entry = self.store.get(reference.id).ok_or(PdfError::ObjectNotFound(*reference))?;
        let obj = match entry {
            XRefEntry::InUse { offset, .. } => {
                let mut parser = Parser::new(self.data.slice(offset as usize..))
                    .with_resolver(self);
                parser.parse_indirect_object_header()?;
                parser.parse_object()?
            }
            XRefEntry::Compressed { .. } => {
                return Err(PdfError::Other("Object streams not yet implemented".into()));
            }
            _ => return Err(PdfError::Other("Attempted to resolve free or invalid object".into())),
        };

        // Cache the result
        self.cache.write().insert(*reference, obj.clone());
        Ok(obj)
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
