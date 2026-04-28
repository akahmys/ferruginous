use ferruginous_core::{Document, PdfName, Object};
use std::fs;

fn main() {
    let data = fs::read("/Users/jun/Downloads/converted/bokutokitan.pdf").unwrap();
    let doc = Document::parse(&data).unwrap();
    let pages = doc.pages().unwrap();
    let page2 = doc.arena().get_dict(pages[1]).unwrap();
    let res_key = doc.arena().intern_name(PdfName::new("Resources"));
    let xobj_key = doc.arena().intern_name(PdfName::new("XObject"));
    if let Some(res) = page2.get(&res_key).and_then(|o| o.resolve(doc.arena()).as_dict(doc.arena())) {
        if let Some(xobj) = res.get(&xobj_key).and_then(|o| o.resolve(doc.arena()).as_dict(doc.arena())) {
            for (k, v) in xobj.iter() {
                let name = doc.arena().get_name(*k).unwrap().as_str();
                println!("XObject: {}", name);
                let obj = v.resolve(doc.arena());
                if let Object::Stream(dh, _) = obj {
                    let dict = doc.arena().get_dict(dh).unwrap();
                    for (dk, dv) in dict.iter() {
                        let dv_res = dv.resolve(doc.arena());
                        let val_str = match dv_res {
                            Object::Integer(i) => i.to_string(),
                            Object::Name(n) => doc.arena().get_name(n).unwrap().as_str().to_string(),
                            Object::Array(a) => format!("Array of len {}", doc.arena().get_array(a).unwrap().len()),
                            _ => format!("{:?}", dv_res),
                        };
                        println!("  {} = {}", doc.arena().get_name(*dk).unwrap().as_str(), val_str);
                    }
                }
            }
        }
    }
}
