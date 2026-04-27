//! PDF Logical Structure Types (ISO 32000-2:2020 Clause 14.7)

use crate::{FromPdfObject, Handle, Object, PdfName};
use std::collections::BTreeMap;

/// PDF Structure Tree Root (Clause 14.7.2)
#[derive(Debug, Clone, FromPdfObject)]
#[pdf_dict(clause = "14.7.2")]
pub struct StructTreeRoot {
    #[pdf_key("K")]
    pub kids: Option<Object>,
    #[pdf_key("ParentTree")]
    pub parent_tree: Option<Handle<Object>>,
}

/// PDF Structure Element (Clause 14.7.3)
#[derive(Debug, Clone, FromPdfObject)]
#[pdf_dict(clause = "14.7.3")]
pub struct StructElement {
    #[pdf_key("S")]
    pub subtype: Handle<PdfName>,
    #[pdf_key("P")]
    pub parent: Handle<BTreeMap<Handle<PdfName>, Object>>,
    #[pdf_key("K")]
    pub kids: Option<Object>,
    #[pdf_key("Alt")]
    pub alt: Option<String>,
    #[pdf_key("ActualText")]
    pub actual_text: Option<String>,
}
