use ferruginous_core::{Document, Object};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let doc = Document::load(Path::new("samples/volvo_xc90.pdf"))?;
    let page = doc.get_page(149)?;
    let res_h = page.resources_handle();
    let res_dict = doc.arena().get_dict(res_h).unwrap();

    if let Some(xobj_obj) = res_dict.get(&doc.arena().name("XObject")) {
        if let Some(xobj_dh) = xobj_obj.resolve(doc.arena()).as_dict_handle() {
            let xobj_dict = doc.arena().get_dict(xobj_dh).unwrap();
            for (nk, val) in xobj_dict {
                let name = doc.arena().get_name(nk).unwrap();
                println!("XObject: /{}", name.as_str());
                if let Object::Stream(dh, _) = val.resolve(doc.arena()) {
                    let d = doc.arena().get_dict(dh).unwrap();
                    if let Some(cs_obj) = d.get(&doc.arena().name("ColorSpace")) {
                        let cs = cs_obj.resolve(doc.arena());
                        println!("  ColorSpace: {:?}", cs);
                        if let Object::Array(ah) = cs {
                            let arr = doc.arena().get_array(ah).unwrap();
                            for (i, v) in arr.iter().enumerate() {
                                println!("    [{}] -> {:?}", i, v);
                                if i == 0 {
                                    if let Some(nh) = v.resolve(doc.arena()).as_name() {
                                        println!(
                                            "    Name[0]: {}",
                                            doc.arena().get_name(nh).unwrap().as_str()
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
