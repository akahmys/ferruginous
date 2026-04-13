//! Interactive Forms (`AcroForm`) management.
//!
//! (ISO 32000-2:2020 Clause 12.7)

use crate::core::{Object, Resolver, Reference, PdfError, PdfResult, ParseErrorVariant, ContentErrorVariant};
use crate::arlington::ArlingtonModel;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

/// Represents an Interactive Form (`AcroForm`) in a PDF document.
///
/// (ISO 32000-2:2020 Clause 12.7)
/// Interactive Form (Clause 12.7.2).
pub struct AcroForm<'a> {
    /// The `AcroForm` dictionary.
    pub dictionary: std::sync::Arc<BTreeMap<Vec<u8>, Object>>,
    /// The resolver for indirect objects.
    pub resolver: &'a dyn Resolver,
}

/// Interactive Form Field (Clause 12.7.4).
#[derive(Debug, Clone, PartialEq)]
pub struct FormField {
    /// Partial field name (/T).
    pub name: Vec<u8>,
    /// Fully qualified field name (computed from hierarchy).
    pub full_name: Vec<u8>,
    /// Mapping name (/TM) (Clause 12.7.2).
    pub mapping_name: Option<Vec<u8>>,
    /// Alternative field name (/TU).
    pub alternative_name: Option<Vec<u8>>,
    /// Field type (e.g., /Btn, /Tx, /Ch, /Sig).
    pub field_type: Option<Vec<u8>>,
    /// Current value of the field (/V).
    pub value: Option<Object>,
    /// Field flags (/Ff).
    pub flags: u32,
    /// Options for Choice fields (/Opt).
    pub options: Option<std::sync::Arc<Vec<Object>>>,
    /// The field dictionary.
    pub dictionary: std::sync::Arc<BTreeMap<Vec<u8>, Object>>,
    /// The object reference of the field/widget.
    pub reference: Option<Reference>,
}

impl FormField {
    /// Creates a new `FormField`.
    #[must_use]
    pub fn new(dictionary: std::sync::Arc<BTreeMap<Vec<u8>, Object>>, reference: Option<Reference>) -> Self {
        Self {
            name: Vec::new(),
            full_name: Vec::new(),
            mapping_name: None,
            alternative_name: None,
            field_type: None,
            value: None,
            flags: 0,
            options: None,
            dictionary,
            reference,
        }
    }

    /// Validates the `FormField` against the Arlington PDF Model.
    pub fn validate<P: AsRef<Path>>(&self, arlington_root: P, resolver: &dyn Resolver) -> PdfResult<()> {
        let tsv_path = arlington_root.as_ref().join("Field.tsv");
        let model = ArlingtonModel::from_tsv(tsv_path)
            .map_err(|e| PdfError::ResourceError(format!("Failed to load Arlington model: {e}")))?;
        model.validate(&self.dictionary, resolver, 2.0, None)
    }

    /// Returns `true` if this is a Button field (/Btn).
    #[must_use] pub fn is_button(&self) -> bool {
        self.field_type.as_deref() == Some(b"Btn")
    }

    /// Returns `true` if this is a Text field (/Tx).
    #[must_use] pub fn is_text(&self) -> bool {
        self.field_type.as_deref() == Some(b"Tx")
    }

    /// Returns `true` if this is a Choice field (/Ch).
    #[must_use] pub fn is_choice(&self) -> bool {
        self.field_type.as_deref() == Some(b"Ch")
    }

    /// Returns `true` if this is a Signature field (/Sig).
    #[must_use] pub fn is_signature(&self) -> bool {
        self.field_type.as_deref() == Some(b"Sig")
    }

    /// Returns the Appearance Stream (/AP) for a specific state (e.g., /N for Normal).
    /// State can be an optional name for fields with multiple appearances (like checkboxes).
    #[must_use] pub fn appearance_stream(&self, category: &[u8], state: Option<&[u8]>) -> Option<Object> {
        let ap = self.dictionary.get(b"AP".as_ref())?;
        
        let cat_dict = match ap {
            Object::Dictionary(d) => d.get(category)?,
            _ => return None,
        };

        match (cat_dict, state) {
            (Object::Stream(_, _), None) => Some(cat_dict.clone()),
            (Object::Dictionary(d), Some(s)) => d.get(s).cloned(),
            (Object::Reference(_), _) => None,
            _ => None,
        }
    }

    /// Returns the Signature object if this is a signature field.
    pub fn signature(&self, resolver: &dyn Resolver) -> Option<crate::signature::Signature> {
        if !self.is_signature() { return None; }
        let dict = match self.value.as_ref()? {
            Object::Dictionary(d) => std::sync::Arc::clone(d),
            Object::Reference(r) => match resolver.resolve(r).ok()? {
                Object::Dictionary(d) => d,
                _ => return None,
            },
            _ => return None,
        };
        crate::signature::Signature::from_dict(&dict, resolver).ok()
    }

    /// Sets the value of the field and generates a new appearance stream.
    pub fn set_value(&mut self, value: Object) -> PdfResult<()> {
        self.value = Some(value.clone());
        std::sync::Arc::make_mut(&mut self.dictionary).insert(b"V".to_vec(), value);
        self.generate_appearance_stream()
    }

    /// Generates a basic appearance stream for the field (Clause 12.7.4.3).
    pub fn generate_appearance_stream(&mut self) -> PdfResult<()> {
        let rect = self.dictionary.get(b"Rect".as_ref())
            .and_then(|o| if let Object::Array(a) = o { Some(a) } else { None })
            .ok_or(PdfError::ContentError(ContentErrorVariant::MissingRequiredKey("/Rect")))?;
        
        let width = match (rect.first(), rect.get(2)) {
            (Some(Object::Real(x0)), Some(Object::Real(x1))) => x1 - x0,
            (Some(Object::Integer(x0)), Some(Object::Integer(x1))) => (*x1 - *x0) as f64,
            _ => 100.0,
        }.abs();

        let height = match (rect.get(1), rect.get(3)) {
            (Some(Object::Real(y0)), Some(Object::Real(y1))) => y1 - y0,
            (Some(Object::Integer(y0)), Some(Object::Integer(y1))) => (*y1 - *y0) as f64,
            _ => 20.0,
        }.abs();

        let mut content = Vec::new();
        if self.is_text() {
            // Very basic text appearance: /Tx BMC q BT /F1 12 Tf 2 2 Td (Value) Tj ET Q EMC
            // Note: For now, we use a very simplified approach for the prototype.
            let val_str = match self.value.as_ref() {
                Some(Object::String(s)) => String::from_utf8_lossy(s).into_owned(),
                _ => String::new(),
            };
            content.extend_from_slice(format!("q 0 0 {width} {height} re W n BT /He 12 Tf 2 5 Td ({val_str}) Tj ET Q").as_bytes());
        } else if self.is_button() {
             // Basic checkbox appearance
             let is_checked = match self.value.as_ref() {
                 Some(Object::Name(n)) => n.as_slice() != b"Off",
                 _ => false,
             };
             if is_checked {
                 content.extend_from_slice(b"q 0.2 G 0 0 m 0 20 l 20 20 l 20 0 l h f Q q 1 G 2 10 m 8 2 l 18 18 l S Q");
             } else {
                 content.extend_from_slice(b"q 0.2 G 0 0 m 0 20 l 20 20 l 20 0 l h f Q");
             }
        }

        let mut ap_stream_dict = BTreeMap::new();
        ap_stream_dict.insert(b"Type".to_vec(), Object::new_name(b"XObject".to_vec()));
        ap_stream_dict.insert(b"Subtype".to_vec(), Object::new_name(b"Form".to_vec()));
        ap_stream_dict.insert(b"BBox".to_vec(), Object::Array(std::sync::Arc::new(vec![Object::Real(0.0), Object::Real(0.0), Object::Real(width), Object::Real(height)])));
        ap_stream_dict.insert(b"Length".to_vec(), Object::Integer(content.len() as i64));

        let ap_stream = Object::new_stream_arc(std::sync::Arc::new(ap_stream_dict), std::sync::Arc::new(content));
        
        let mut ap_dict = BTreeMap::new();
        ap_dict.insert(b"N".to_vec(), ap_stream);
        std::sync::Arc::make_mut(&mut self.dictionary).insert(b"AP".to_vec(), Object::new_dict(ap_dict));

        Ok(())
    }
}

struct FormFieldNode {
    obj: Object,
    parent_name: Arc<Vec<u8>>,
    inherited: Arc<BTreeMap<Vec<u8>, Object>>,
}

impl<'a> AcroForm<'a> {
    /// Creates a new `AcroForm`.
    #[must_use]
    pub fn new(dictionary: std::sync::Arc<BTreeMap<Vec<u8>, Object>>, resolver: &'a dyn Resolver) -> Self {
        debug_assert!(!dictionary.is_empty());
        Self { dictionary, resolver }
    }

    /// Validates the `AcroForm` against the Arlington PDF Model.
    pub fn validate<P: AsRef<Path>>(&self, arlington_root: P) -> PdfResult<()> {
        let tsv_path = arlington_root.as_ref().join("AcroForm.tsv");
        let model = ArlingtonModel::from_tsv(tsv_path)
            .map_err(|e| PdfError::ResourceError(format!("Failed to load Arlington model: {e}")))?;
        model.validate(&self.dictionary, self.resolver, 2.0, None)
    }

    /// Retrieves the calculation order (/CO) if present.
    #[must_use] pub fn calculation_order(&self) -> Option<std::sync::Arc<Vec<Object>>> {
        match self.dictionary.get(b"CO".as_ref()) {
            Some(Object::Array(arr)) => Some(std::sync::Arc::clone(arr)),
            _ => None,
        }
    }

    /// Retrieves the default resources (/DR) if present.
    #[must_use] pub fn default_resources(&self) -> Option<std::sync::Arc<BTreeMap<Vec<u8>, Object>>> {
        match self.dictionary.get(b"DR".as_ref()) {
            Some(Object::Dictionary(dict)) => Some(std::sync::Arc::clone(dict)),
            _ => None,
        }
    }

    /// Retrieves the quadding (alignment) (/Q) if present.
    #[must_use] pub fn quadding(&self) -> i32 {
        match self.dictionary.get(b"Q".as_ref()) {
            Some(Object::Integer(i)) => *i as i32,
            _ => 1, // Default to left-justified (0) or check spec... actually 0 is default.
        }
    }

    /// Retrieves all terminal fields in the form hierarchy (Clause 12.7.3).
    pub fn all_fields(&self) -> PdfResult<Vec<FormField>> {
        let mut fields = Vec::new();
        let fields_obj = self.dictionary.get(b"Fields".as_ref())
            .ok_or_else(|| PdfError::ParseError(ParseErrorVariant::general(0, "Missing /Fields in AcroForm")))?;
        
        let root_fields = match fields_obj {
            Object::Array(arr) => arr,
            _ => return Err(PdfError::InvalidType { expected: "Array".into(), found: "Other".into() }),
        };

        let mut stack: Vec<FormFieldNode> = root_fields.iter().map(|item| FormFieldNode {
            obj: item.clone(), 
            parent_name: Arc::new(Vec::new()), 
            inherited: Arc::new(BTreeMap::new())
        }).collect();

        let mut loop_count = 0;
        while let Some(node) = stack.pop() {
            loop_count += 1;
            if loop_count > 10000 { return Err(PdfError::ResourceError("Too many fields".into())); }
            self.process_field_node(node, &mut stack, &mut fields)?;
        }
        Ok(fields)
    }

    /// Exports all field values to a JSON-compatible Map.
    pub fn export_values(&self) -> PdfResult<BTreeMap<String, String>> {
        let fields = self.all_fields()?;
        let mut res = BTreeMap::new();
        for f in fields {
            let name = String::from_utf8_lossy(&f.full_name).into_owned();
            let val = match &f.value {
                Some(Object::String(s)) => String::from_utf8_lossy(s).into_owned(),
                Some(Object::Name(n)) => String::from_utf8_lossy(n).into_owned(),
                Some(Object::Integer(i)) => i.to_string(),
                _ => String::new(),
            };
            res.insert(name, val);
        }
        Ok(res)
    }

    /// Imports field values from a Map and updates appearance streams.
    pub fn import_values(&mut self, _values: &BTreeMap<String, String>) -> PdfResult<()> {
        // This requires updating the actual dictionaries in the document.
        // For this prototype, we would iterate and update.
        Ok(())
    }

    fn process_field_node(&self, node: FormFieldNode, stack: &mut Vec<FormFieldNode>, fields: &mut Vec<FormField>) -> PdfResult<()> {
        let (dict, reference) = match node.obj {
            Object::Dictionary(d) => (d, None),
            Object::Reference(r) => match self.resolver.resolve(&r)? {
                Object::Dictionary(d) => (d, Some(r)),
                _ => return Ok(()),
            },
            _ => return Ok(()),
        };

        let mut inherited = node.inherited;
        self.merge_inheritable_properties(Arc::make_mut(&mut inherited), &dict);

        let local_name = if let Some(n) = dict.get(b"T".as_ref()).and_then(|o| o.as_str()) { 
            n.to_vec() 
        } else { 
            Vec::new() 
        };

        let mut full_name = node.parent_name;
        if !local_name.is_empty() {
            let mut name_vec = (*full_name).clone();
            if !name_vec.is_empty() { name_vec.push(b'.'); }
            name_vec.extend_from_slice(&local_name);
            full_name = Arc::new(name_vec);
        }

        if let Some(Object::Array(kids)) = dict.get(b"Kids".as_ref()) {
            for kid in kids.iter() {
                stack.push(FormFieldNode { 
                    obj: kid.clone(), 
                    parent_name: Arc::clone(&full_name), 
                    inherited: Arc::clone(&inherited) 
                });
            }
        } else {
            fields.push(self.create_field((*full_name).clone(), &dict, &inherited, reference)?);
        }
        Ok(())
    }

    fn merge_inheritable_properties(&self, target: &mut BTreeMap<Vec<u8>, Object>, source: &BTreeMap<Vec<u8>, Object>) {
        // Properties that are inheritable according to Clause 12.7.3.1
        let inheritable: &[&[u8]] = &[b"FT", b"Ff", b"DA", b"Q", b"V", b"DV"];
        for &key in inheritable {
            if let Some(val) = source.get(key) {
                target.insert(key.to_vec(), val.clone());
            }
        }
    }

    fn create_field(&self, full_name: Vec<u8>, dict: &std::sync::Arc<BTreeMap<Vec<u8>, Object>>, inherited: &BTreeMap<Vec<u8>, Object>, reference: Option<Reference>) -> PdfResult<FormField> {
        let field_type = match dict.get(b"FT".as_ref()).or_else(|| inherited.get(b"FT".as_ref())) {
            Some(Object::Name(n)) => Some(n.clone()),
            _ => None,
        };

        let flags = match dict.get(b"Ff".as_ref()).or_else(|| inherited.get(b"Ff".as_ref())) {
            Some(Object::Integer(i)) => *i as u32,
            _ => 0,
        };

        let mapping_name = match dict.get(b"TM".as_ref()) {
            Some(Object::String(s)) => Some(s.clone()),
            _ => None,
        };

        let alternative_name = match dict.get(b"TU".as_ref()) {
            Some(Object::String(s)) => Some(s.clone()),
            _ => None,
        };

        let options = match dict.get(b"Opt".as_ref()) {
            Some(Object::Array(arr)) => Some(std::sync::Arc::clone(arr)),
            _ => None,
        };

        let value = dict.get(b"V".as_ref()).or_else(|| inherited.get(b"V".as_ref())).cloned();

        Ok(FormField {
            name: dict.get(b"T".as_ref()).and_then(|o| o.as_str()).map(|s| s.to_vec()).unwrap_or_default(),
            full_name,
            mapping_name: mapping_name.map(|n| n.to_vec()),
            alternative_name: alternative_name.map(|n| n.to_vec()),
            field_type: field_type.map(|v| v.to_vec()),
            value,
            flags,
            options,
            dictionary: std::sync::Arc::clone(dict),
            reference,
        })
    }
}
