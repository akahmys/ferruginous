//! Logical Structure (Tagged PDF) management.
//! (ISO 32000-2:2020 Clause 14.7)

use crate::core::{Object, Reference, Resolver, PdfResult};
use std::collections::BTreeMap;

/// Represents the Logical Structure Root of a Tagged PDF.
/// (ISO 32000-2:2020 Clause 14.7.2)
pub struct LogicalStructure<'a> {
    /// The structure tree root dictionary.
    pub dictionary: std::sync::Arc<BTreeMap<Vec<u8>, Object>>,
    /// The resolver for indirect objects within the structure.
    pub resolver: &'a dyn Resolver,
}

impl<'a> LogicalStructure<'a> {
    /// Creates a new `LogicalStructure` wrapper for a structure tree root.
    pub fn new(dictionary: std::sync::Arc<BTreeMap<Vec<u8>, Object>>, resolver: &'a dyn Resolver) -> Self {
        Self { dictionary, resolver }
    }

    /// Resolves a structure type through the RoleMap if present (Clause 14.7.3).
    #[must_use] pub fn resolve_role(&self, role: &[u8]) -> Vec<u8> {
        let mut current_role = role.to_vec();
        let mut visited = std::collections::BTreeSet::new();
        visited.insert(current_role.clone());

        if let Some(role_map) = self.dictionary.get(b"RoleMap".as_ref()).and_then(|obj| self.resolver.resolve_if_ref(obj).ok()).and_then(|obj| obj.as_dict_arc()) {
            while let Some(mapped) = role_map.get(&current_role).and_then(|obj| obj.as_str()) {
                if visited.contains(mapped) { break; } // Prevent infinite recursion
                current_role = mapped.to_vec();
                visited.insert(current_role.clone());
            }
        }
        current_role
    }

    /// Retrieves attributes associated with a class name from the ClassMap (Clause 14.7.6).
    #[must_use] pub fn resolve_class(&self, class_name: &[u8]) -> Option<Object> {
        self.dictionary.get(b"ClassMap".as_ref())
            .and_then(|obj| self.resolver.resolve_if_ref(obj).ok())
            .and_then(|obj| obj.as_dict_arc())
            .and_then(|dict| dict.get(class_name).cloned())
            .and_then(|obj| self.resolver.resolve_if_ref(&obj).ok())
    }

    /// Returns an iterator over the immediate children of the structure tree root.
    #[must_use] pub fn kids<'b>(&'b self) -> StructureIterator<'a, 'b> {
        let mut stack = Vec::new();
        if let Some(kids_obj) = self.dictionary.get(b"K".as_ref()) {
            Self::push_kids_to_stack(&mut stack, kids_obj);
        }
        StructureIterator {
            stack,
            root: self,
        }
    }

    fn push_kids_to_stack(stack: &mut Vec<Object>, kids_obj: &Object) {
        match kids_obj {
            Object::Array(arr) => {
                // Push in reverse to maintain order when popping from stack
                for item in arr.iter().rev() {
                    stack.push(item.clone());
                }
            }
            _ => {
                stack.push(kids_obj.clone());
            }
        }
    }
}

/// Represents a Structure Element in the tree (Clause 14.7.2).
pub struct StructureElement<'a, 'b> {
    /// The structure element dictionary.
    pub dictionary: std::sync::Arc<BTreeMap<Vec<u8>, Object>>,
    /// The original reference of this element.
    pub reference: Reference,
    /// The structure tree root this element belongs to.
    pub root: &'b LogicalStructure<'a>,
}

impl<'a, 'b> StructureElement<'a, 'b> {
    /// Creates a new `StructureElement` wrapper.
    pub fn new(dictionary: std::sync::Arc<BTreeMap<Vec<u8>, Object>>, reference: Reference, root: &'b LogicalStructure<'a>) -> Self {
        Self { dictionary, reference, root }
    }

    /// Returns the Structure Type (S) of this element, resolved via RoleMap.
    #[must_use] pub fn structure_type(&self) -> Option<Vec<u8>> {
        self.dictionary.get(b"S".as_ref()).and_then(|obj| {
            if let Object::Name(n) = obj {
                Some(self.root.resolve_role(n))
            } else {
                None
            }
        })
    }

    /// Returns the attributes of this element, including those from classes (Clause 14.7.5).
    #[must_use] pub fn attributes(&self) -> Vec<std::sync::Arc<BTreeMap<Vec<u8>, Object>>> {
        let mut result = Vec::new();

        // 1. Resolve classes from /C entry
        if let Some(obj) = self.dictionary.get(b"C".as_ref()) {
            let obj = self.root.resolver.resolve_if_ref(obj).unwrap_or(obj.clone());
            match obj {
                Object::Name(name) => {
                    if let Some(attr_obj) = self.root.resolve_class(&name) {
                        Self::push_attributes(&mut result, &attr_obj);
                    }
                }
                Object::Array(arr) => {
                    for item in arr.iter() {
                        if let Some(name) = item.as_str() {
                            if let Some(attr_obj) = self.root.resolve_class(name) {
                                Self::push_attributes(&mut result, &attr_obj);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // 2. Add explicit attributes from /A entry
        if let Some(obj) = self.dictionary.get(b"A".as_ref()) {
            let obj = self.root.resolver.resolve_if_ref(obj).unwrap_or(obj.clone());
            Self::push_attributes(&mut result, &obj);
        }

    result
    }

    fn push_attributes(dest: &mut Vec<std::sync::Arc<BTreeMap<Vec<u8>, Object>>>, obj: &Object) {
        match obj {
            Object::Dictionary(dict) => dest.push(std::sync::Arc::clone(dict)),
            Object::Array(arr) => {
                for item in arr.iter() {
                    if let Object::Dictionary(dict) = item {
                        dest.push(std::sync::Arc::clone(dict));
                    }
                }
            }
            _ => {}
        }
    }

    /// Returns an iterator over the children of this structure element.
    #[must_use] pub fn kids(&self) -> StructureIterator<'a, 'b> {
        let mut stack = Vec::new();
        if let Some(kids_obj) = self.dictionary.get(b"K".as_ref()) {
            LogicalStructure::push_kids_to_stack(&mut stack, kids_obj);
        }
        StructureIterator {
            stack,
            root: self.root,
        }
    }
}

/// A non-recursive iterator over structure elements (Clause 14.7.2.2).
/// This iterator only returns `StructureElements`, skipping content items (marked content integers/MCIDs).
pub struct StructureIterator<'a, 'b> {
    /// Stack of objects to visit during iteration.
    stack: Vec<Object>,
    /// The structure tree root for resolving roles and creating elements.
    root: &'b LogicalStructure<'a>,
}

impl<'a, 'b> Iterator for StructureIterator<'a, 'b> {
    type Item = PdfResult<StructureElement<'a, 'b>>;

    fn next(&mut self) -> Option<Self::Item> {
        // RR-10 v2: Rule 9: Limit stack depth and total iterations to ensure deterministic behavior
        if self.stack.len() > 128 { return None; }
        
        let mut loop_count = 0;
        const MAX_ITER: usize = 1000;

        while let Some(obj) = self.stack.pop() {
            loop_count += 1;
            if loop_count > MAX_ITER { break; }

            match obj {
                Object::Reference(r) => {
                    match self.root.resolver.resolve(&r) {
                        Ok(Object::Dictionary(dict)) => {
                            if dict.contains_key(b"S".as_ref()) || dict.get(b"Type".as_ref()).and_then(|o| o.as_str()) == Some(b"StructElem") {
                                return Some(Ok(StructureElement::new(dict, r, self.root)));
                            }
                        }
                        Ok(_) => continue,
                        Err(e) => return Some(Err(e)),
                    }
                }
                Object::Dictionary(dict) => {
                    if dict.contains_key(b"S".as_ref()) {
                        let dummy_ref = Reference { id: 0, generation: 0 };
                        return Some(Ok(StructureElement::new(dict, dummy_ref, self.root)));
                    }
                }
                _ => continue,
            }
        }
        None
    }
}

/// Validator for Tagged PDF compliance (Clause 14.8).
pub struct TaggedPdfValidator<'a> {
    root: LogicalStructure<'a>,
}

impl<'a> TaggedPdfValidator<'a> {
    /// Creates a new validator that owns the logical structure.
    pub const fn new_owned(root: LogicalStructure<'a>) -> Self {
        Self { root }
    }

    /// Creates a new validator for the given logical structure (as a reference).
    /// Note: This is now a bit repetitive but kept for compatibility.
    pub fn new(root: &'a LogicalStructure<'a>) -> Self {
        Self { root: LogicalStructure::new(root.dictionary.clone(), root.resolver) }
    }

    /// Performs basic validation of Tagged PDF requirements.
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        // 1. Check for ParentTree (Clause 14.7.4.4) - Required for Tagged PDF
        if !self.root.dictionary.contains_key(b"ParentTree".as_ref()) {
            errors.push("Missing required /ParentTree in StructTreeRoot for Tagged PDF (Clause 14.8)".to_string());
        }

        // 2. Check for RoleMap if non-standard types are used
        
        // 3. Verify Kids structure
        let kids_count = self.root.kids().count();
        if kids_count == 0 {
            errors.push("StructTreeRoot has no children (Kids)".to_string());
        }

        // 4. Recursive check for structure elements
        self.validate_elements(&mut errors);

        errors
    }

    fn validate_elements(&self, errors: &mut Vec<String>) {
        let mut stack: Vec<StructureElement<'a, '_>> = self.root.kids().filter_map(|r| r.ok()).collect();
        let mut visited = std::collections::BTreeSet::new();
        let mut count = 0;
        const MAX_ELEMENTS: usize = 5000;

        while let Some(elem) = stack.pop() {
            count += 1;
            if count > MAX_ELEMENTS {
                errors.push("Exceeded maximum structure element limit during validation".to_string());
                break;
            }

            if !visited.insert(elem.reference) && elem.reference.id != 0 {
                continue; // Prevent cycles
            }

            // Check Structure Type (S)
            if elem.structure_type().is_none() {
                errors.push(format!("Structure element at {:?} missing required /S (Type) key", elem.reference));
            }

            // Add kids to stack
            for kid_result in elem.kids() {
                if let Ok(kid) = kid_result {
                    stack.push(kid);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Object, Reference, PdfResult};

    struct MockResolver;
    impl Resolver for MockResolver {
        fn resolve(&self, _r: &Reference) -> PdfResult<Object> {
            Ok(Object::Null)
        }
    }

    #[test]
    fn test_role_map_resolution() {
        let mut role_map = BTreeMap::new();
        role_map.insert(b"MyType".to_vec(), Object::new_name(b"P".to_vec()));
        role_map.insert(b"OtherType".to_vec(), Object::new_name(b"MyType".to_vec()));

        let mut dict = BTreeMap::new();
        dict.insert(b"RoleMap".to_vec(), Object::new_dict(role_map));

        let resolver = MockResolver;
        let root = LogicalStructure::new(std::sync::Arc::new(dict.clone()), &resolver);

        assert_eq!(root.resolve_role(b"MyType"), b"P".to_vec());
        assert_eq!(root.resolve_role(b"OtherType"), b"P".to_vec());
        assert_eq!(root.resolve_role(b"Standard"), b"Standard".to_vec());
    }

    #[test]
    fn test_class_map_resolution() {
        let mut class_dict = BTreeMap::new();
        let mut attrs = BTreeMap::new();
        attrs.insert(b"Color".to_vec(), Object::new_array(vec![Object::Integer(1), Object::Integer(0), Object::Integer(0)]));
        class_dict.insert(b"MyClass".to_vec(), Object::new_dict(attrs));

        let mut dict = BTreeMap::new();
        dict.insert(b"ClassMap".to_vec(), Object::new_dict(class_dict));

        let resolver = MockResolver;
        let root = LogicalStructure::new(std::sync::Arc::new(dict.clone()), &resolver);

        let resolved = root.resolve_class(b"MyClass").unwrap();
        assert!(matches!(resolved, Object::Dictionary(_)));
    }

    #[test]
    fn test_tagged_pdf_validation() {
        let dict = BTreeMap::new();
        // Validation fails if ParentTree is missing
        let resolver = MockResolver;
        let root = LogicalStructure::new(std::sync::Arc::new(dict.clone()), &resolver);
        let validator = TaggedPdfValidator::new(&root);
        let errors = validator.validate();
        assert!(!errors.is_empty());

        // Add ParentTree and Kids
        let mut dict = BTreeMap::new();
        dict.insert(b"ParentTree".to_vec(), Object::new_dict(BTreeMap::new()));
        dict.insert(b"K".to_vec(), Object::new_array(vec![]));
        
        let root = LogicalStructure::new(std::sync::Arc::new(dict), &resolver);
        let validator = TaggedPdfValidator::new(&root);
        let errors = validator.validate();
        // Still fails because Kids is empty
        assert!(errors.iter().any(|e| e.contains("no children")));
    }
}
