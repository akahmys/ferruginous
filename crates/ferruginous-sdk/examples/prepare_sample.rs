use ferruginous_sdk::core::{Object, Reference, PdfResult};
use ferruginous_sdk::serialize::{write_indirect_object, write_xref_section, write_trailer};
use ferruginous_sdk::xref::XRefEntry;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;

fn main() -> PdfResult<()> {
    let mut data = Vec::new();
    data.extend(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");

    let mut offsets = BTreeMap::new();
    write_body(&mut data, &mut offsets)?;

    let xref_start = data.len() as u64;
    write_meta(&mut data, &offsets, xref_start)?;

    std::fs::create_dir_all("samples").map_err(ferruginous_sdk::core::PdfError::from)?;
    let mut file = File::create("samples/text-sample.pdf").map_err(ferruginous_sdk::core::PdfError::from)?;
    file.write_all(&data).map_err(ferruginous_sdk::core::PdfError::from)?;
    
    println!("Generated: samples/text-sample.pdf");
    Ok(())
}

fn write_body(data: &mut Vec<u8>, offsets: &mut BTreeMap<u32, u64>) -> PdfResult<()> {
    // 1 0 obj: Catalog
    offsets.insert(1, data.len() as u64);
    let mut catalog = BTreeMap::new();
    catalog.insert(b"Type".to_vec(), Object::new_name(b"Catalog".to_vec()));
    catalog.insert(b"Pages".to_vec(), Object::Reference(Reference { id: 2, generation: 0 }));
    write_indirect_object(data, 1, 0, &Object::new_dict(catalog))?;

    // 2 0 obj: Pages
    offsets.insert(2, data.len() as u64);
    let mut pages = BTreeMap::new();
    pages.insert(b"Type".to_vec(), Object::new_name(b"Pages".to_vec()));
    pages.insert(b"Kids".to_vec(), Object::new_array(vec![Object::Reference(Reference { id: 3, generation: 0 })]));
    pages.insert(b"Count".to_vec(), Object::Integer(1));
    write_indirect_object(data, 2, 0, &Object::new_dict(pages))?;

    write_page_and_content(data, offsets)?;
    Ok(())
}

fn write_page_and_content(data: &mut Vec<u8>, offsets: &mut BTreeMap<u32, u64>) -> PdfResult<()> {
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
    write_indirect_object(data, 3, 0, &Object::new_dict(page))?;

    // 4 0 obj: Contents (Stream)
    offsets.insert(4, data.len() as u64);
    let content = b"BT /F1 24 Tf 100 700 Td (Ferruginous PDF Tool Test) Tj ET";
    write_indirect_object(data, 4, 0, &Object::new_stream(BTreeMap::new(), content.to_vec()))?;

    // 5 0 obj: Font (Helvetica)
    offsets.insert(5, data.len() as u64);
    let font = create_font_dict();
    write_indirect_object(data, 5, 0, &Object::new_dict(font))?;
    Ok(())
}

fn create_font_dict() -> BTreeMap<Vec<u8>, Object> {
    let mut font = BTreeMap::new();
    font.insert(b"Type".to_vec(), Object::new_name(b"Font".to_vec()));
    font.insert(b"Subtype".to_vec(), Object::new_name(b"Type1".to_vec()));
    font.insert(b"BaseFont".to_vec(), Object::new_name(b"Helvetica".to_vec()));
    font.insert(b"FirstChar".to_vec(), Object::Integer(32));
    font.insert(b"LastChar".to_vec(), Object::Integer(126));
    let mut widths = Vec::new();
    for _ in 32..=126 {
        widths.push(Object::Integer(600)); 
    }
    font.insert(b"Widths".to_vec(), Object::new_array(widths));
    font
}

fn write_meta(data: &mut Vec<u8>, offsets: &BTreeMap<u32, u64>, xref_start: u64) -> PdfResult<()> {
    let mut entries = Vec::new();
    entries.push(XRefEntry::Free { next: 0, generation: 65535 });
    for id in 1..=5 {
        let offset = *offsets.get(&id).ok_or_else(|| ferruginous_sdk::core::PdfError::ContentError("ID missing in offsets".into()))?;
        entries.push(XRefEntry::InUse { offset, generation: 0 });
    }
    
    write_xref_section(data, &[(0, entries)])?;
    
    let mut trailer = BTreeMap::new();
    trailer.insert(b"Size".to_vec(), Object::Integer(6));
    trailer.insert(b"Root".to_vec(), Object::Reference(Reference { id: 1, generation: 0 }));
    write_trailer(data, &trailer, xref_start)?;
    Ok(())
}

