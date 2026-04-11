use ferruginous_sdk::core::{Object, Resolver, PdfResult, Reference};
use ferruginous_sdk::forms::AcroForm;
use std::collections::BTreeMap;

struct MockResolver;
impl Resolver for MockResolver {
    fn resolve(&self, reference: &Reference) -> PdfResult<Object> {
        let mut dict = BTreeMap::new();
        if reference.id == 100 {
            // Parent field
            dict.insert(b"T".to_vec(), Object::new_string(b"Parent".to_vec()));
            dict.insert(b"FT".to_vec(), Object::new_name(b"Tx".to_vec())); // Inheritable Field Type: Text
            dict.insert(b"Kids".to_vec(), Object::new_array(vec![Object::Reference(Reference::new(101, 0))]));
        } else if reference.id == 101 {
            // Child field (Terminal)
            dict.insert(b"T".to_vec(), Object::new_string(b"Child".to_vec()));
            dict.insert(b"V".to_vec(), Object::new_string(b"Hello".to_vec()));
        }
        Ok(Object::new_dict(dict))
    }
}

#[test]
fn test_acroform_field_extraction() -> PdfResult<()> {
    // 1. Setup AcroForm with a hierarchy
    let mut root_dict = BTreeMap::new();
    root_dict.insert(b"Fields".to_vec(), Object::new_array(vec![Object::Reference(Reference::new(100, 0))]));
    
    let resolver = MockResolver;
    let acroform = AcroForm::new(root_dict.into(), &resolver);

    // 2. Extract fields
    let fields = acroform.all_fields()?;
    
    // 3. Verify extraction and inheritance
    println!("Extracted fields count: {}", fields.len());
    for (i, f) in fields.iter().enumerate() {
        println!("Field {}: name={:?}", i, String::from_utf8_lossy(&f.name));
    }

    assert_eq!(fields.len(), 1);
    let field = &fields[0];
    
    // Full name should be "Parent.Child"
    assert_eq!(field.full_name, b"Parent.Child");
    
    // Field type should be inherited from parent
    assert!(field.is_text());
    
    // Value should be correct
    if let Some(Object::String(ref s)) = field.value {
        assert_eq!(s.as_ref(), b"Hello");
    } else {
        panic!("Field value missing or wrong type");
    }

    Ok(())
}
