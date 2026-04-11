use ferruginous_sdk::core::{Object, Reference};
use ferruginous_sdk::serialize::{write_indirect_object, write_xref_section, write_trailer};
use ferruginous_sdk::xref::XRefEntry;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;

#[test]
fn simple_gen() {
    let mut data = Vec::new();
    data.extend(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");

    let mut offsets = BTreeMap::new();

    // 1 0 obj: Catalog
    offsets.insert(1, data.len() as u64);
    let mut catalog = BTreeMap::new();
    catalog.insert(b"Type".to_vec(), Object::new_name(b"Catalog".to_vec()));
    catalog.insert(b"Pages".to_vec(), Object::Reference(Reference { id: 2, generation: 0 }));
    write_indirect_object(&mut data, 1, 0, &Object::new_dict(catalog)).unwrap();

    // 2 0 obj: Pages
    offsets.insert(2, data.len() as u64);
    let mut pages = BTreeMap::new();
    pages.insert(b"Type".to_vec(), Object::new_name(b"Pages".to_vec()));
    pages.insert(b"Kids".to_vec(), Object::new_array(vec![Object::Reference(Reference { id: 3, generation: 0 })]));
    pages.insert(b"Count".to_vec(), Object::Integer(1));
    write_indirect_object(&mut data, 2, 0, &Object::new_dict(pages)).unwrap();

    // 3 0 obj: Page
    offsets.insert(3, data.len() as u64);
    let mut page = BTreeMap::new();
    page.insert(b"Type".to_vec(), Object::new_name(b"Page".to_vec()));
    page.insert(b"Parent".to_vec(), Object::Reference(Reference { id: 2, generation: 0 }));
    page.insert(b"MediaBox".to_vec(), Object::new_array(vec![
        Object::Integer(0), Object::Integer(0), Object::Integer(595), Object::Integer(842)
    ]));
    page.insert(b"Contents".to_vec(), Object::Reference(Reference { id: 4, generation: 0 }));
    
    let mut resources = BTreeMap::new();
    let mut font_dict = BTreeMap::new();
    font_dict.insert(b"F1".to_vec(), Object::Reference(Reference { id: 5, generation: 0 }));
    resources.insert(b"Font".to_vec(), Object::new_dict(font_dict));
    page.insert(b"Resources".to_vec(), Object::new_dict(resources));
    
    write_indirect_object(&mut data, 3, 0, &Object::new_dict(page)).unwrap();

    // 4 0 obj: Contents (Stream)
    offsets.insert(4, data.len() as u64);
    let content = b"BT /F1 24 Tf 100 700 Td (Hello Ferruginous PDF!) Tj ET";
    write_indirect_object(&mut data, 4, 0, &Object::new_stream(BTreeMap::new(), content.to_vec())).unwrap();

    // 5 0 obj: Font (Helvetica)
    offsets.insert(5, data.len() as u64);
    let mut font = BTreeMap::new();
    font.insert(b"Type".to_vec(), Object::new_name(b"Font".to_vec()));
    font.insert(b"Subtype".to_vec(), Object::new_name(b"Type1".to_vec()));
    font.insert(b"BaseFont".to_vec(), Object::new_name(b"Helvetica".to_vec()));
    font.insert(b"FirstChar".to_vec(), Object::Integer(32));
    font.insert(b"LastChar".to_vec(), Object::Integer(126));
    let mut widths = Vec::new();
    for _ in 32..=126 {
        widths.push(Object::Integer(600)); // Default fallback width
    }
    font.insert(b"Widths".to_vec(), Object::new_array(widths));
    write_indirect_object(&mut data, 5, 0, &Object::new_dict(font)).unwrap();

    let xref_start = data.len() as u64;
    let mut entries = Vec::new();
    // ID 0
    entries.push(XRefEntry::Free { next: 0, generation: 65535 });
    // ID 1 to 5
    for id in 1..=5 {
        let offset = *offsets.get(&id).expect("ID missing in offsets");
        entries.push(XRefEntry::InUse { offset, generation: 0 });
    }
    
    write_xref_section(&mut data, &[(0, entries)]).unwrap();
    
    let mut trailer = BTreeMap::new();
    trailer.insert(b"Size".to_vec(), Object::Integer(6));
    trailer.insert(b"Root".to_vec(), Object::Reference(Reference { id: 1, generation: 0 }));
    write_trailer(&mut data, &trailer.into(), xref_start).unwrap();

    std::fs::create_dir_all("samples").unwrap();
    let mut file = File::create("samples/simple.pdf").unwrap();
    file.write_all(&data).unwrap();
}
