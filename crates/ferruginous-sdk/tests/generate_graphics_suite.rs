use ferruginous_sdk::core::{Object, Reference};
use ferruginous_sdk::serialize::{write_indirect_object, write_xref_section, write_trailer};
use ferruginous_sdk::xref::XRefEntry;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;

#[test]
fn generate_graphics_suite() {
    let mut data = Vec::new();
    data.extend(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
    let total_pages = 8;

    let mut offsets = BTreeMap::new();
    let mut next_id = 1;

    // Helper to write an object and track offset
    let mut write_obj = |id: u32, obj: &Object, data: &mut Vec<u8>, offsets: &mut BTreeMap<u32, u64>| {
        offsets.insert(id, data.len() as u64);
        write_indirect_object(data, id, 0, obj).unwrap();
    };

    let catalog_id = next_id; next_id += 1;
    let pages_id = next_id; next_id += 1;
    let font_id = next_id; next_id += 1;
    let gs_id = next_id; next_id += 1; // ExtGState for transparency
    let shading_id = next_id; next_id += 1;
    let function_id = next_id; next_id += 1;
    let group_id = next_id; next_id += 1;
    let pattern_id = next_id; next_id += 1;

    let mut page_ids = Vec::new();
    let mut content_ids = Vec::new();
    for _ in 0..total_pages {
        page_ids.push(next_id); next_id += 1;
        content_ids.push(next_id); next_id += 1;
    }

    // 1. Catalog
    let mut catalog = BTreeMap::new();
    catalog.insert(b"Type".to_vec(), Object::new_name(b"Catalog".to_vec()));
    catalog.insert(b"Pages".to_vec(), Object::Reference(Reference { id: pages_id, generation: 0 }));
    write_obj(catalog_id, &Object::new_dict(catalog), &mut data, &mut offsets);

    // 2. Pages Root
    let mut pages = BTreeMap::new();
    pages.insert(b"Type".to_vec(), Object::new_name(b"Pages".to_vec()));
    pages.insert(b"Count".to_vec(), Object::Integer(total_pages as i64));
    pages.insert(b"Kids".to_vec(), Object::new_array(page_ids.iter().map(|&id| Object::Reference(Reference { id, generation: 0 })).collect()));
    write_obj(pages_id, &Object::new_dict(pages), &mut data, &mut offsets);

    // 3. Shared Font
    let mut font = BTreeMap::new();
    font.insert(b"Type".to_vec(), Object::new_name(b"Font".to_vec()));
    font.insert(b"Subtype".to_vec(), Object::new_name(b"Type1".to_vec()));
    font.insert(b"BaseFont".to_vec(), Object::new_name(b"Helvetica-Bold".to_vec()));
    font.insert(b"FirstChar".to_vec(), Object::Integer(32));
    font.insert(b"LastChar".to_vec(), Object::Integer(122));
    let mut widths = Vec::new();
    for _ in 32..=122 { widths.push(Object::Real(600.0)); } // Simplified uniform width
    font.insert(b"Widths".to_vec(), Object::new_array(widths));
    write_obj(font_id, &Object::new_dict(font), &mut data, &mut offsets);

    // 4. ExtGState for Transparency
    let mut gs_dict = BTreeMap::new();
    gs_dict.insert(b"Type".to_vec(), Object::new_name(b"ExtGState".to_vec()));
    gs_dict.insert(b"ca".to_vec(), Object::Real(0.5)); // Non-stroking alpha
    gs_dict.insert(b"CA".to_vec(), Object::Real(0.7)); // Stroking alpha
    gs_dict.insert(b"BM".to_vec(), Object::new_name(b"Multiply".to_vec()));
    write_obj(gs_id, &Object::new_dict(gs_dict), &mut data, &mut offsets);
    
    // 5. Axial Shading (Linear Gradient)
    let mut func_dict = BTreeMap::new();
    func_dict.insert(b"FunctionType".to_vec(), Object::Integer(2));
    func_dict.insert(b"Domain".to_vec(), Object::new_array(vec![Object::Real(0.0), Object::Real(1.0)]));
    func_dict.insert(b"C0".to_vec(), Object::new_array(vec![Object::Real(1.0), Object::Real(0.0), Object::Real(0.0)])); // Red
    func_dict.insert(b"C1".to_vec(), Object::new_array(vec![Object::Real(0.0), Object::Real(0.0), Object::Real(1.0)])); // Blue
    func_dict.insert(b"N".to_vec(), Object::Real(1.0));
    write_obj(function_id, &Object::new_dict(func_dict), &mut data, &mut offsets);

    let mut sh_dict = BTreeMap::new();
    sh_dict.insert(b"ShadingType".to_vec(), Object::Integer(2)); // Axial
    sh_dict.insert(b"ColorSpace".to_vec(), Object::new_name(b"DeviceRGB".to_vec()));
    sh_dict.insert(b"Coords".to_vec(), Object::new_array(vec![
        Object::Real(100.0), Object::Real(100.0), Object::Real(300.0), Object::Real(300.0)
    ]));
    sh_dict.insert(b"Function".to_vec(), Object::Reference(Reference { id: function_id, generation: 0 }));
    write_obj(shading_id, &Object::new_dict(sh_dict), &mut data, &mut offsets);
    
    // 6. Form XObject (Transparency Group)
    let mut group_attr = BTreeMap::new();
    group_attr.insert(b"Type".to_vec(), Object::new_name(b"Group".to_vec()));
    group_attr.insert(b"S".to_vec(), Object::new_name(b"Transparency".to_vec()));
    group_attr.insert(b"I".to_vec(), Object::Boolean(true)); // Isolated
    
    let mut group_dict = BTreeMap::new();
    group_dict.insert(b"Type".to_vec(), Object::new_name(b"XObject".to_vec()));
    group_dict.insert(b"Subtype".to_vec(), Object::new_name(b"Form".to_vec()));
    group_dict.insert(b"BBox".to_vec(), Object::new_array(vec![Object::Integer(0), Object::Integer(0), Object::Integer(200), Object::Integer(200)]));
    group_dict.insert(b"Group".to_vec(), Object::new_dict(group_attr));
    group_dict.insert(b"Resources".to_vec(), Object::new_dict(BTreeMap::new())); // Empty resources for now
    
    let group_stream = b"1 0 0 rg 0 0 100 100 re f 0 1 0 rg 50 50 100 100 re f";
    write_obj(group_id, &Object::new_stream(group_dict, group_stream.to_vec()), &mut data, &mut offsets);

    // 7. Tiling Pattern
    let mut pattern_dict = BTreeMap::new();
    pattern_dict.insert(b"Type".to_vec(), Object::new_name(b"Pattern".to_vec()));
    pattern_dict.insert(b"PatternType".to_vec(), Object::Integer(1)); // Tiling
    pattern_dict.insert(b"PaintType".to_vec(), Object::Integer(1)); // Colored
    pattern_dict.insert(b"TilingType".to_vec(), Object::Integer(1));
    pattern_dict.insert(b"BBox".to_vec(), Object::new_array(vec![Object::Integer(0), Object::Integer(0), Object::Integer(10), Object::Integer(10)]));
    pattern_dict.insert(b"XStep".to_vec(), Object::Integer(10));
    pattern_dict.insert(b"YStep".to_vec(), Object::Integer(10));
    pattern_dict.insert(b"Resources".to_vec(), Object::new_dict(BTreeMap::new()));
    
    let pattern_stream = b"1 0 0 rg 0 0 5 5 re f 0 0 1 rg 5 5 5 5 re f";
    write_obj(pattern_id, &Object::new_stream(pattern_dict, pattern_stream.to_vec()), &mut data, &mut offsets);

    // Common Resources
    let mut resources = BTreeMap::new();
    let mut font_res = BTreeMap::new();
    font_res.insert(b"F1".to_vec(), Object::Reference(Reference { id: font_id, generation: 0 }));
    resources.insert(b"Font".to_vec(), Object::new_dict(font_res));
    let mut gs_res = BTreeMap::new();
    gs_res.insert(b"GS1".to_vec(), Object::Reference(Reference { id: gs_id, generation: 0 }));
    resources.insert(b"ExtGState".to_vec(), Object::new_dict(gs_res));
    let mut sh_res = BTreeMap::new();
    sh_res.insert(b"SH1".to_vec(), Object::Reference(Reference { id: shading_id, generation: 0 }));
    resources.insert(b"Shading".to_vec(), Object::new_dict(sh_res));
    let mut x_res = BTreeMap::new();
    x_res.insert(b"Group1".to_vec(), Object::Reference(Reference { id: group_id, generation: 0 }));
    resources.insert(b"XObject".to_vec(), Object::new_dict(x_res));
    let mut p_res = BTreeMap::new();
    p_res.insert(b"P1".to_vec(), Object::Reference(Reference { id: pattern_id, generation: 0 }));
    resources.insert(b"Pattern".to_vec(), Object::new_dict(p_res));
    let resources_obj = Object::new_dict(resources);

    // Metadata for each page
    let contents = [
        // Page 1: Basic Shapes
        "q 1 0 0 rg 50 50 100 100 re f 0 0 1 RG 200 50 m 300 150 l S 0 1 0 rg 50 200 m 150 200 l 100 300 l h f Q BT /F1 18 Tf 50 800 Td (Page 1: Basic Shapes) Tj ET",
        // Page 2: Curves & Clipping
        "q 0 0 0 rg 50 50 300 300 re W n 1 0 0 rg 100 100 m 100 200 200 200 200 100 c f Q BT /F1 18 Tf 50 800 Td (Page 2: Curves & Clipping) Tj ET",
        // Page 3: Line Styles
        "q 0.5 g 5 w 1 J 1 j [10 5] 0 d 50 100 m 500 100 l S [2 2] 0 d 50 150 m 500 150 l S 10 w 0 J 0 j [] 0 d 50 200 m 500 200 l S Q BT /F1 18 Tf 50 800 Td (Page 3: Line Styles) Tj ET",
        // Page 4: Colors (RGB, Gray, CMYK)
        "q 1 0 0 rg 50 50 50 50 re f 0.5 g 150 50 50 50 re f 0 1 1 0 k 250 50 50 50 re f Q BT /F1 18 Tf 50 800 Td (Page 4: Colorspaces) Tj ET",
        // Page 5: Transparency
        "q /GS1 gs 1 0 0 rg 100 100 200 200 re f 0 0 1 rg 150 150 200 200 re f Q BT /F1 18 Tf 50 800 Td (Page 5: Transparency) Tj ET",
        // Page 6: Shading
        "q /SH1 sh Q BT /F1 18 Tf 50 800 Td (Page 6: Shading) Tj ET",
        "q /Group1 Do Q BT /F1 18 Tf 50 800 Td (Page 7: Transparency Group) Tj ET",
        "q /Pattern cs /P1 scn 50 50 300 300 re f Q BT /F1 18 Tf 50 800 Td (Page 8: Tiling Pattern) Tj ET",
    ];

    for i in 0..total_pages {
        // Page Object
        let mut page = BTreeMap::new();
        page.insert(b"Type".to_vec(), Object::new_name(b"Page".to_vec()));
        page.insert(b"Parent".to_vec(), Object::Reference(Reference { id: pages_id, generation: 0 }));
        page.insert(b"MediaBox".to_vec(), Object::new_array(vec![
            Object::Integer(0), Object::Integer(0), Object::Integer(595), Object::Integer(842)
        ]));
        page.insert(b"Contents".to_vec(), Object::Reference(Reference { id: content_ids[i], generation: 0 }));
        page.insert(b"Resources".to_vec(), resources_obj.clone());
        write_obj(page_ids[i], &Object::new_dict(page), &mut data, &mut offsets);

        // Content Stream
        write_obj(content_ids[i], &Object::new_stream(BTreeMap::new(), contents[i].as_bytes().to_vec()), &mut data, &mut offsets);
    }

    // Finalize PDF
    let xref_start = data.len() as u64;
    let mut entries = Vec::new();
    entries.push(XRefEntry::Free { next: 0, generation: 65535 });
    for id in 1..next_id {
        let offset = *offsets.get(&id).expect("ID missing");
        entries.push(XRefEntry::InUse { offset, generation: 0 });
    }
    write_xref_section(&mut data, &[(0, entries)]).unwrap();
    
    let mut trailer = BTreeMap::new();
    trailer.insert(b"Size".to_vec(), Object::Integer(next_id as i64));
    trailer.insert(b"Root".to_vec(), Object::Reference(Reference { id: catalog_id, generation: 0 }));
    write_trailer(&mut data, &trailer.into(), xref_start).unwrap();

    std::fs::create_dir_all("../../samples").unwrap();
    let mut file = File::create("../../samples/graphics-suite.pdf").unwrap();
    file.write_all(&data).unwrap();
    println!("Generated samples/graphics-suite.pdf");
}
