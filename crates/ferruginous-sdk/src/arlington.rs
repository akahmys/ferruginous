//! Arlington PDF Model integration for specification-based validation.
//! (<https://github.com/pdf-association/arlington-pdf-model>)

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use crate::core::{Object, PdfError, PdfResult, ValidationErrorVariant};

/// The set of supported property types in the Arlington PDF Model.
/// (<https://github.com/pdf-association/arlington-pdf-model>)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArlingtonType {
    /// Clause 7.3.3 - Numeric objects (Integer)
    Integer,
    /// Clause 7.3.2 - Boolean objects
    Boolean,
    /// Clause 7.3.5 - Name objects
    Name,
    /// Clause 7.3.4 - String objects
    String,
    /// Clause 7.3.6 - Array objects
    Array,
    /// Clause 7.3.7 - Dictionary objects
    Dictionary,
    /// Clause 7.3.8 - Stream objects
    Stream,
    /// Clause 7.3.9 - Null object
    Null,
    /// Clause 7.3.10 - Indirect Objects (References)
    Reference,
    /// Clause 8.4 - Rectangles
    Rectangle,
    /// Date strings (deprecated/legacy format)
    Date,
    /// Clause 7.3.3 - Numeric objects (Real)
    Real,
    /// Generic number (Integer or Real)
    Number,
    /// Text string (String)
    Text,
    /// Any object type
    Any,
    /// An unknown type string from the model
    Unknown(String),
}

impl ArlingtonType {
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "integer" => Self::Integer,
            "boolean" => Self::Boolean,
            "name" => Self::Name,
            "string" => Self::String,
            "array" => Self::Array,
            "dictionary" => Self::Dictionary,
            "stream" => Self::Stream,
            "null" => Self::Null,
            "reference" => Self::Reference,
            "rectangle" => Self::Rectangle,
            "date" => Self::Date,
            "real" => Self::Real,
            "number" => Self::Number,
            "text" => Self::Text,
            "any" => Self::Any,
            _ => Self::Unknown(s.to_string()),
        }
    }

    /// Checks if a PDF object matches this Arlington type.
    #[must_use] pub fn matches(&self, obj: &Object) -> bool {
        if matches!(self, Self::Any) {
            return true;
        }
        if let Object::Reference(_) = obj {
            return true; // Clause 7.3.10 - Most objects can be indirect
        }
        match (self, obj) {
            (Self::Integer, Object::Integer(_)) => true,
            (Self::Boolean, Object::Boolean(_)) => true,
            (Self::Name, Object::Name(_)) => true,
            (Self::String, Object::String(_)) => true,
            (Self::Text, Object::String(_)) => true,
            (Self::Array, Object::Array(_)) => true,
            (Self::Dictionary, Object::Dictionary(_)) => true,
            (Self::Stream, Object::Stream(_, _)) => true,
            (Self::Null, Object::Null) => true,
            (Self::Reference, Object::Reference(_)) => true,
            (Self::Rectangle, Object::Array(arr)) if arr.len() == 4 => true,
            (Self::Date, Object::String(_)) => true,
            (Self::Number | Self::Real, Object::Real(_) | Object::Integer(_)) => true,
            _ => false,
        }
    }
}

/// Definition of a key within an Arlington PDF Model dictionary.
#[derive(Debug, Clone)]
pub struct KeyDefinition {
    /// The name of the key (e.g., "Type", "Contents").
    pub key: Vec<u8>,
    /// The list of valid PDF object types for this key.
    pub types: Vec<ArlingtonType>,
    /// Whether the key must be present in the dictionary.
    pub required: bool,
}

/// Represents a complete Arlington PDF Model for a specific object type.
#[derive(Debug, Default)]
pub struct ArlingtonModel {
    /// Map of key names to their definitions.
    pub keys: BTreeMap<Vec<u8>, KeyDefinition>,
    /// List of parent model names this model inherits from.
    pub parents: Vec<String>,
}

impl ArlingtonModel {
    /// Loads an Arlington model from a TSV file.
    pub fn from_tsv<P: AsRef<Path>>(path: P) -> PdfResult<Self> {
        let file = File::open(path).map_err(PdfError::from)?;
        let reader = BufReader::new(file);
        let mut model = Self::default();

        let mut lines = reader.lines();
        let _header = lines.next();

        for line in lines {
            let line = line.map_err(PdfError::from)?;
            if line.trim().is_empty() { continue; }
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() < 5 { continue; }

            let key = parts[0];
            let types = parts[1].trim()
                .trim_matches(|c| c == '[' || c == ']')
                .split([',', ';'])
                .map(|s| ArlingtonType::from_str(s.trim()))
                .filter(|t| !matches!(t, ArlingtonType::Unknown(s) if s.is_empty()))
                .collect();
            
            let required = parts[4].to_lowercase() == "true";

            model.keys.insert(key.as_bytes().to_vec(), KeyDefinition {
                key: key.as_bytes().to_vec(),
                types,
                required,
            });
        }

        Ok(model)
    }

    /// Validates a dictionary against the model structure, including inherited keys.
    pub fn validate(&self, dict: &BTreeMap<Vec<u8>, Object>, registry: Option<&ArlingtonRegistry>) -> PdfResult<()> {
        let mut errors = Vec::new();
        let all_defined_keys = if let Some(reg) = registry {
            self.get_all_keys(reg)
        } else {
            self.keys.clone()
        };

        // 1. Check required keys
        for (key, def) in &all_defined_keys {
            if def.required && !dict.contains_key(key) {
                errors.push(format!("Missing required key: /{}", String::from_utf8_lossy(key)));
            }
        }

        // 2. Check types for existing keys
        for (key, val) in dict {
            if let Some(def) = all_defined_keys.get(key) {
                if !def.types.iter().any(|t| t.matches(val)) {
                    errors.push(format!("Key /{} has invalid type. Expected one of {:?}", String::from_utf8_lossy(key), def.types));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(PdfError::Validation(ValidationErrorVariant::Arlington(errors)))
        }
    }

    /// Recursively collects all keys including those from parent models.
    pub fn get_all_keys(&self, registry: &ArlingtonRegistry) -> BTreeMap<Vec<u8>, KeyDefinition> {
        let mut all_keys = BTreeMap::new();
        for parent_name in &self.parents {
            if let Some(parent_model) = registry.get(parent_name) {
                all_keys.extend(parent_model.get_all_keys(registry));
            }
        }
        for (k, v) in &self.keys {
            all_keys.insert(k.clone(), v.clone());
        }
        all_keys
    }
}

/// A registry containing multiple Arlington models, indexed by name.
#[derive(Debug, Default)]
pub struct ArlingtonRegistry {
    /// Mapping from object name (e.g., "`FileBody`", "Catalog") to its model.
    pub models: BTreeMap<String, ArlingtonModel>,
}

impl ArlingtonRegistry {
    /// Creates a new empty Arlington registry.
    #[must_use] pub fn new() -> Self {
        Self::default()
    }

    /// Loads all TSV files from the given directory into the registry.
    /// Rule 2: Explicit loop boundaries and error handling.
    pub fn load_all<P: AsRef<Path>>(&mut self, dir: P) -> PdfResult<()> {
        let entries = std::fs::read_dir(dir).map_err(PdfError::from)?;
        let mut count = 0;

        for entry in entries {
            let entry = entry.map_err(PdfError::from)?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("tsv") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let model = ArlingtonModel::from_tsv(&path)?;
                    self.models.insert(stem.to_string(), model);
                    count += 1;
                }
            }
            assert!(count < 1000, "Too many TSV files in arlington directory");
        }
        
        assert!(count > 0, "No TSV files found in arlington directory");
        Ok(())
    }

    /// Retrieves an Arlington model by its name.
    #[must_use] pub fn get(&self, name: &str) -> Option<&ArlingtonModel> {
        self.models.get(name)
    }

    /// Performs a recursive validation of the entire document structure.
    pub fn validate_document(&self, root: &Object, resolver: &dyn crate::core::Resolver) -> ValidationReport {
        let mut report = ValidationReport::default();
        let mut visited = std::collections::BTreeSet::new();
        
        self.validate_recursive(root, "Catalog", resolver, &mut visited, &mut report);
        
        report
    }

    fn validate_recursive(&self, obj: &Object, model_name: &str, resolver: &dyn crate::core::Resolver, visited: &mut std::collections::BTreeSet<crate::core::Reference>, report: &mut ValidationReport) {
        let obj = match obj {
            Object::Reference(r) => {
                if !visited.insert(*r) { return; }
                match resolver.resolve(r) {
                    Ok(o) => o,
                    Err(e) => {
                        report.errors.push(format!("Failed to resolve reference {:?}: {}", r, e));
                        return;
                    }
                }
            }
            _ => obj.clone(),
        };

        if let Object::Dictionary(dict) = &obj {
            // Determine the best model for this dictionary
            let actual_model = if let Some(Object::Name(n)) = dict.get(b"Type".as_ref()) {
                let type_name = String::from_utf8_lossy(n);
                self.get(&type_name).map(|_| type_name.into_owned()).unwrap_or_else(|| model_name.to_string())
            } else {
                model_name.to_string()
            };

            if let Some(model) = self.get(&actual_model) {
                if let Err(PdfError::Validation(ValidationErrorVariant::Arlington(errs))) = model.validate(dict, Some(self)) {
                    report.errors.extend(errs.into_iter().map(|e| format!("[{}] {}", actual_model, e)));
                }
            }

            // Recurse into children (simplified: just follow all dictionaries/arrays)
            // Rule 9: Limit recursion or use a stack to avoid overflow
            for (key, val) in dict.iter() {
                // Avoid infinite recursion into parent or same-level objects if not references
                // (Already handled by visited set for references)
                if matches!(val, Object::Dictionary(_) | Object::Array(_) | Object::Reference(_)) {
                    // Try to guess child model based on key (very simplified)
                    let child_model = match key.as_slice() {
                        b"Pages" => "PageTree",
                        b"Kids" => model_name, // Inherit if it's a tree
                        b"Resources" => "ResourceDictionary",
                        _ => "Any",
                    };
                    self.validate_recursive(val, child_model, resolver, visited, report);
                }
            }
        } else if let Object::Array(arr) = &obj {
            for item in arr.iter() {
                self.validate_recursive(item, model_name, resolver, visited, report);
            }
        }
    }
}

/// A comprehensive report of PDF validation results.
#[derive(Debug, Default, Clone)]
pub struct ValidationReport {
    /// List of validation errors found.
    pub errors: Vec<String>,
    /// List of warnings or non-critical issues.
    pub warnings: Vec<String>,
}

impl ValidationReport {
    /// Returns true if the document is considered valid.
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

#[cfg(test)]
mod tests {



    #[test]
    const fn test_arlington_validation() {
        // Logic will be demonstrated in integration tests with the TSV file.
    }
}
