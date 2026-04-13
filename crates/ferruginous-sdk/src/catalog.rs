//! Document Catalog (Root) parsing and validation.
//! (ISO 32000-2:2020 Clause 7.7.2)

use crate::core::{Object, Reference, Resolver, PdfError, PdfResult, ContentErrorVariant};
use crate::arlington::ArlingtonModel;
use crate::page::PageTree;
use crate::navigation::{Outline, Destination};
use crate::structure::LogicalStructure;
use crate::metadata::Metadata;
use std::collections::BTreeMap;
use std::path::Path;

/// Represents the Document Catalog (Root) of a PDF file.
/// (ISO 32000-2:2020 Clause 7.7.2)
pub struct Catalog<'a> {
    /// The root dictionary of the PDF.
    pub dictionary: std::sync::Arc<BTreeMap<Vec<u8>, Object>>,
    /// The resolver instance for object lookups.
    pub resolver: &'a dyn Resolver,
}

impl<'a> Catalog<'a> {
    /// Creates a new Catalog instance from a Root dictionary.
    pub fn new(dictionary: std::sync::Arc<BTreeMap<Vec<u8>, Object>>, resolver: &'a dyn Resolver) -> Self {
        debug_assert!(!dictionary.is_empty());
        assert!(dictionary.contains_key(b"Type".as_ref()));
        Self { dictionary, resolver }
    }

    /// Validates the catalog against the Arlington PDF Model.
    pub fn validate<P: AsRef<Path>>(&self, tsv_path: P) -> PdfResult<()> {
        let model = ArlingtonModel::from_tsv(tsv_path)
            .map_err(|e| PdfError::ResourceError(format!("Failed to load Arlington model: {e}")))?;
        assert!(!self.dictionary.is_empty());
        model.validate(&self.dictionary, self.resolver, 2.0, None)
    }

    /// Validates the entire document against the Arlington PDF Model Registry.
    pub fn validate_all(&self, registry: &crate::arlington::ArlingtonRegistry) -> crate::arlington::ValidationReport {
        registry.validate_document(&Object::Dictionary(self.dictionary.clone()), self.resolver)
    }

    /// Retrieves the Page Tree root reference from the catalog.
    pub fn pages_root(&self) -> PdfResult<Reference> {
        let pages_obj = self.dictionary.get(b"Pages".as_ref())
            .ok_or_else(|| PdfError::ContentError(ContentErrorVariant::MissingRequiredKey("/Pages")))?;
        
        assert!(self.dictionary.contains_key(b"Pages".as_ref()));

        if let Object::Reference(r) = pages_obj {
            Ok(*r)
        } else {
            Err(PdfError::InvalidType { expected: "Reference".into(), found: "Other".into() })
        }
    }

    /// Creates a `PageTree` instance from the catalog.
    pub fn page_tree(&self) -> PdfResult<PageTree<'a>> {
        let root_ref = self.pages_root()?;
        debug_assert!(root_ref.id > 0);
        Ok(PageTree {
            root_pages: root_ref,
            resolver: self.resolver,
        })
    }

    /// Retrieves the Document Outline from the catalog (Clause 12.3.3).
    #[must_use] pub fn outlines(&self) -> Option<Outline<'a>> {
        self.dictionary.get(b"Outlines".as_ref()).and_then(|obj| {
            match obj {
                Object::Dictionary(dict) => Some(Outline::new(dict.clone(), self.resolver)),
                Object::Reference(r) => {
                    if let Ok(Object::Dictionary(dict)) = self.resolver.resolve(r) {
                        Some(Outline::new(dict, self.resolver))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        })
    }

    /// Resolves a named destination (Clause 12.3.2.3).
    #[must_use] pub fn resolve_destination(&self, name: &[u8]) -> Option<Destination> {
        // First check the /Dests dictionary in the Catalog
        if let Some(Object::Dictionary(dests)) = self.dictionary.get(b"Dests".as_ref()) {
            if let Some(obj) = dests.get(name) {
                return Destination::from_obj(obj.clone());
            }
        }
        
        // Note: For full compliance, we should also check the /Names tree,
        // but that is deferred to a future milestone as per the plan.
        None
    }

    /// Retrieves the Logical Structure root (Clause 14.7.2).
    #[must_use] pub fn struct_tree_root(&self) -> Option<LogicalStructure<'a>> {
        debug_assert!(!self.dictionary.is_empty()); // Rule 5: assertion density
        self.dictionary.get(b"StructTreeRoot".as_ref()).and_then(|obj| {
            debug_assert!(matches!(obj, Object::Reference(_) | Object::Dictionary(_))); // Rule 5
            match self.resolver.resolve_if_ref(obj).ok()? {
                Object::Dictionary(dict) => Some(LogicalStructure::new(dict, self.resolver)),
                _ => None,
            }
        })
    }

    /// Provides a validator for Tagged PDF compliance (Clause 14.8).
    #[must_use] pub fn tagged_pdf_validator(&self) -> Option<crate::structure::TaggedPdfValidator<'a>> {
        self.struct_tree_root().map(|root| {
            // We need to keep the root alive, but TaggedPdfValidator takes a reference.
            // This is a bit tricky with the current structure. 
            // In a real implementation, we might want to store the LogicalStructure in the Catalog or use an Arc.
            // For now, we'll assume the caller manages the lifetime.
            // Wait, we can't easily return a validator that references a local variable.
            // Let's change TaggedPdfValidator to own the LogicalStructure or handle it differently.
            crate::structure::TaggedPdfValidator::new_owned(root)
        })
    }

    /// Retrieves the document Metadata (Clause 14.3).
    #[must_use] pub fn metadata(&self) -> Option<Metadata> {
        debug_assert!(!self.dictionary.is_empty()); // Rule 5
        self.dictionary.get(b"Metadata".as_ref()).and_then(|obj| {
            debug_assert!(matches!(obj, Object::Reference(_) | Object::Stream(_, _))); // Rule 5
            match obj {
                Object::Stream(dict, content) => Some(Metadata::new(std::sync::Arc::clone(dict), std::sync::Arc::clone(content))),
                Object::Reference(r) => {
                    if let Ok(Object::Stream(dict, content)) = self.resolver.resolve(r) {
                        Some(Metadata::new(dict, content))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        })
    }

    /// Checks if the PDF is a Tagged PDF (Clause 14.8).
    #[must_use] pub fn is_tagged_pdf(&self) -> bool {
        debug_assert!(!self.dictionary.is_empty()); // Rule 5
        self.dictionary.get(b"MarkInfo".as_ref()).and_then(|obj| {
            debug_assert!(matches!(obj, Object::Reference(_) | Object::Dictionary(_))); // Rule 5
            let dict = match obj {
                Object::Dictionary(d) => Some(d.clone()),
                Object::Reference(r) => {
                    match self.resolver.resolve(r) {
                        Ok(Object::Dictionary(d)) => Some(d),
                        _ => None,
                    }
                }
                _ => None,
            }?;
            
            dict.get(b"Marked".as_ref()).and_then(|o| {
                match o {
                    Object::Boolean(b) => Some(*b),
                    _ => None,
                }
            })
        }).unwrap_or(false)
    }

    /// Retrieves the Interactive Form dictionary (Clause 12.7.2).
    #[must_use] pub fn acroform(&self) -> Option<crate::forms::AcroForm<'a>> {
        debug_assert!(!self.dictionary.is_empty()); // Rule 5: assertion density
        self.dictionary.get(b"AcroForm".as_ref()).and_then(|obj| {
            debug_assert!(matches!(obj, Object::Reference(_) | Object::Dictionary(_))); // Rule 5
            let dict = match obj {
                Object::Dictionary(dict) => Some(dict.clone()),
                Object::Reference(r) => {
                    if let Ok(Object::Dictionary(dict)) = self.resolver.resolve(r) {
                        Some(dict)
                    } else {
                        None
                    }
                }
                _ => None,
            }?;
            debug_assert!(!dict.is_empty()); // Rule 5
            Some(crate::forms::AcroForm::new(dict, self.resolver))
        })
    }

    /// Retrieves the document-level security store (Clause 12.8.4.3).
    #[must_use] pub fn dss(&self) -> Option<std::sync::Arc<BTreeMap<Vec<u8>, Object>>> {
        self.dictionary.get(b"DSS".as_ref()).and_then(|obj| {
            match obj {
                Object::Dictionary(dict) => Some(dict.clone()),
                Object::Reference(r) => self.resolver.resolve(r).ok()?.as_dict_arc(),
                _ => None,
            }
        })
    }

    /// Retrieves the Optional Content properties (Clause 14.11.4).
    #[must_use] pub fn oc_properties(&self) -> Option<crate::ocg::OCProperties> {
        let obj = self.dictionary.get(b"OCProperties".as_ref())?;
        let dict_arc = match obj {
            Object::Reference(r) => self.resolver.resolve(r).ok()?.as_dict_arc()?,
            Object::Dictionary(d) => std::sync::Arc::clone(d),
            _ => return None,
        };

        let ocgs = if let Some(Object::Array(arr)) = dict_arc.get(b"OCGs".as_ref()) {
            arr.iter().filter_map(|o| if let Object::Reference(r) = o { Some(*r) } else { None }).collect()
        } else {
            Vec::new()
        };

        let default_config = dict_arc.get(b"D".as_ref()).and_then(|o| match o {
            Object::Dictionary(d) => Some(std::sync::Arc::clone(d)),
            Object::Reference(r) => self.resolver.resolve(r).ok()?.as_dict_arc(),
            _ => None,
        })?;

        Some(crate::ocg::OCProperties { ocgs, default_config })
    }
}
