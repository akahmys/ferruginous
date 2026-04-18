use ferruginous_sdk::PdfResult;
use ferruginous_sdk::core::{Object, Reference};
use std::collections::BTreeMap;
use std::io::{BufWriter, Write, Seek};

fn main() -> PdfResult<()> {
    let output_path = "samples/pdf20/jp-harness.pdf";
    std::fs::create_dir_all("samples/pdf20").map_err(ferruginous_sdk::PdfError::IoError)?;
    let file = std::fs::File::create(output_path).map_err(ferruginous_sdk::PdfError::IoError)?;
    let mut writer = BufWriter::new(file);

    writer.write_all(b"%PDF-2.0\n%\xBD\xBE\xBC\xAF\n").map_err(ferruginous_sdk::PdfError::IoError)?;

    let mut offsets = Vec::new();
    write_structure_objects(&mut writer, &mut offsets)?;
    write_font_objects(&mut writer, &mut offsets)?;
    write_xref_and_trailer(&mut writer, &offsets)?;

    println!("Manual harness generated successfully: {}", output_path);
    Ok(())
}

fn write_structure_objects(writer: &mut BufWriter<std::fs::File>, offsets: &mut Vec<(u32, u64)>) -> PdfResult<()> {
    // 1. Catalog
    offsets.push((1, writer.stream_position().map_err(ferruginous_sdk::PdfError::IoError)?));
    let mut catalog = BTreeMap::new();
    catalog.insert(b"Type".to_vec(), Object::new_name(b"Catalog".to_vec()));
    catalog.insert(b"Pages".to_vec(), Object::Reference(Reference { id: 2, generation: 0 }));
    ferruginous_sdk::writer::write_indirect_object(writer, 1, 0, &Object::new_dict(catalog))?;

    // 2. Pages Root
    offsets.push((2, writer.stream_position().map_err(ferruginous_sdk::PdfError::IoError)?));
    let mut pages = BTreeMap::new();
    pages.insert(b"Type".to_vec(), Object::new_name(b"Pages".to_vec()));
    pages.insert(b"Count".to_vec(), Object::Integer(1));
    pages.insert(b"Kids".to_vec(), Object::new_array(vec![Object::Reference(Reference { id: 3, generation: 0 })]));
    ferruginous_sdk::writer::write_indirect_object(writer, 2, 0, &Object::new_dict(pages))?;

    // 3. Page 1
    offsets.push((3, writer.stream_position().map_err(ferruginous_sdk::PdfError::IoError)?));
    let mut page = BTreeMap::new();
    page.insert(b"Type".to_vec(), Object::new_name(b"Page".to_vec()));
    page.insert(b"Parent".to_vec(), Object::Reference(Reference { id: 2, generation: 0 }));
    page.insert(b"MediaBox".to_vec(), Object::new_array(vec![Object::Integer(0), Object::Integer(0), Object::Integer(595), Object::Integer(842)]));
    page.insert(b"Contents".to_vec(), Object::Reference(Reference { id: 4, generation: 0 }));
    
    let mut fonts = BTreeMap::new();
    fonts.insert(b"F1".to_vec(), Object::Reference(Reference { id: 5, generation: 0 }));
    let mut res = BTreeMap::new();
    res.insert(b"Font".to_vec(), Object::new_dict(fonts));
    page.insert(b"Resources".to_vec(), Object::new_dict(res));
    ferruginous_sdk::writer::write_indirect_object(writer, 3, 0, &Object::new_dict(page))?;

    // 4. Content Stream
    offsets.push((4, writer.stream_position().map_err(ferruginous_sdk::PdfError::IoError)?));
    let content = b"BT /F1 10 Tf 1 0 0 -1 100 700 Tm <00210021> Tj 1 0 0 1 200 700 Tm <00210021> Tj ET";
    ferruginous_sdk::writer::write_indirect_object(writer, 4, 0, &Object::new_stream(BTreeMap::new(), content.to_vec()))?;
    Ok(())
}

fn write_font_objects(writer: &mut BufWriter<std::fs::File>, offsets: &mut Vec<(u32, u64)>) -> PdfResult<()> {
    // 5. Type 0 Font
    offsets.push((5, writer.stream_position().map_err(ferruginous_sdk::PdfError::IoError)?));
    let mut f_dict = BTreeMap::new();
    f_dict.insert(b"Type".to_vec(), Object::new_name(b"Font".to_vec()));
    f_dict.insert(b"Subtype".to_vec(), Object::new_name(b"Type0".to_vec()));
    f_dict.insert(b"BaseFont".to_vec(), Object::new_name(b"Harness-Mincho-V".to_vec()));
    f_dict.insert(b"Encoding".to_vec(), Object::new_name(b"Identity-V".to_vec()));
    f_dict.insert(b"DescendantFonts".to_vec(), Object::new_array(vec![Object::Reference(Reference { id: 6, generation: 0 })]));
    ferruginous_sdk::writer::write_indirect_object(writer, 5, 0, &Object::new_dict(f_dict))?;

    // 6. CIDFont
    offsets.push((6, writer.stream_position().map_err(ferruginous_sdk::PdfError::IoError)?));
    let mut c_dict = BTreeMap::new();
    c_dict.insert(b"Type".to_vec(), Object::new_name(b"Font".to_vec()));
    c_dict.insert(b"Subtype".to_vec(), Object::new_name(b"CIDFontType2".to_vec()));
    c_dict.insert(b"BaseFont".to_vec(), Object::new_name(b"Harness-Mincho-V".to_vec()));
    c_dict.insert(b"DW".to_vec(), Object::Integer(1000));
    c_dict.insert(b"DW2".to_vec(), Object::new_array(vec![Object::Integer(880), Object::Integer(-1000)]));
    c_dict.insert(b"W2".to_vec(), Object::new_array(vec![
        Object::Integer(33), 
        Object::new_array(vec![Object::Integer(-1000), Object::Integer(500), Object::Integer(880)])
    ]));
    ferruginous_sdk::writer::write_indirect_object(writer, 6, 0, &Object::new_dict(c_dict))?;
    Ok(())
}

fn write_xref_and_trailer(writer: &mut BufWriter<std::fs::File>, offsets: &[(u32, u64)]) -> PdfResult<()> {
    let xref_pos = writer.stream_position().map_err(ferruginous_sdk::PdfError::IoError)?;
    writer.write_all(b"xref\n0 ").map_err(ferruginous_sdk::PdfError::IoError)?;
    writeln!(writer, "{}", offsets.len() + 1).map_err(ferruginous_sdk::PdfError::IoError)?;
    writer.write_all(b"0000000000 65535 f \n").map_err(ferruginous_sdk::PdfError::IoError)?;
    for (_, off) in offsets {
        writeln!(writer, "{:010} 00000 n ", off).map_err(ferruginous_sdk::PdfError::IoError)?;
    }

    let mut trailer = BTreeMap::new();
    trailer.insert(b"Size".to_vec(), Object::Integer((offsets.len() + 1) as i64));
    trailer.insert(b"Root".to_vec(), Object::Reference(Reference { id: 1, generation: 0 }));
    
    writer.write_all(b"trailer\n").map_err(ferruginous_sdk::PdfError::IoError)?;
    ferruginous_sdk::writer::write_object(writer, &Object::new_dict(trailer))?;
    write!(writer, "\nstartxref\n{xref_pos}\n%%EOF\n").map_err(ferruginous_sdk::PdfError::IoError)?;
    Ok(())
}
