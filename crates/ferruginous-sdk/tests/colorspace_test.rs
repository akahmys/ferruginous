use ferruginous_sdk::colorspace::ColorSpace;
use std::sync::Arc;
use ferruginous_sdk::core::{Object, Resolver, Reference, PdfResult};
use std::collections::BTreeMap;

struct MockResolver {
    objects: BTreeMap<Reference, Object>,
}

impl Resolver for MockResolver {
    fn resolve(&self, r: &Reference) -> PdfResult<Object> {
        self.objects.get(r).cloned().ok_or(ferruginous_sdk::core::PdfError::ResourceError("Not found".into()))
    }
}

#[test]
fn test_parse_device_colorspaces() {
    let resolver = MockResolver { objects: BTreeMap::new() };
    
    let cs_rgb = ColorSpace::from_object(&Object::new_name(b"DeviceRGB".to_vec()), &resolver).unwrap();
    assert_eq!(cs_rgb, ColorSpace::DeviceRGB);
    assert_eq!(cs_rgb.components(), 3);

    let cs_cmyk = ColorSpace::from_object(&Object::new_name(b"DeviceCMYK".to_vec()), &resolver).unwrap();
    assert_eq!(cs_cmyk, ColorSpace::DeviceCMYK);
    assert_eq!(cs_cmyk.components(), 4);
}

#[test]
fn test_parse_calibrated_gray() {
    let resolver = MockResolver { objects: BTreeMap::new() };
    let mut dict = BTreeMap::new();
    dict.insert(b"WhitePoint".to_vec(), Object::new_array(vec![Object::Real(1.0), Object::Real(1.0), Object::Real(1.0)]));
    dict.insert(b"Gamma".to_vec(), Object::Real(2.2));
    
    let cs_obj = Object::new_array(vec![
        Object::new_name(b"CalGray".to_vec()),
        Object::new_dict(dict)
    ]);
    
    let cs = ColorSpace::from_object(&cs_obj, &resolver).unwrap();
    if let ColorSpace::CalGray { white_point, gamma, .. } = cs {
        assert_eq!(white_point, [1.0, 1.0, 1.0]);
        assert_eq!(gamma, 2.2);
    } else {
        panic!("Expected CalGray, got {:?}", cs);
    }
}

#[test]
fn test_parse_icc_based() {
    let mut resolver = MockResolver { objects: BTreeMap::new() };
    let mut dict = BTreeMap::new();
    dict.insert(b"N".to_vec(), Object::Integer(3));
    dict.insert(b"Alternate".to_vec(), Object::new_name(b"DeviceRGB".to_vec()));
    
    let mut icc_data = vec![0x00; 132]; // Mock ICC profile data (header + tag count)
    icc_data[36..40].copy_from_slice(b"acsp"); // Magic number
    let stream = Object::new_stream(dict, icc_data);
    let r = Reference { id: 10, generation: 0 };
    resolver.objects.insert(r, stream);

    let cs_obj = Object::new_array(vec![
        Object::new_name(b"ICCBased".to_vec()),
        Object::Reference(r)
    ]);

    let cs = ColorSpace::from_object(&cs_obj, &resolver).unwrap();
    if let ColorSpace::ICCBased(profile) = cs {
        assert_eq!(profile.components, 3);
        assert_eq!(profile.data.len(), 132);
        assert_eq!(profile.alternate, Some(Box::new(ColorSpace::DeviceRGB)));
    } else {
        panic!("Expected ICCBased, got {:?}", cs);
    }
}
