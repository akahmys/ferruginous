use ferruginous_sdk::loader::load_document_structure;
use ferruginous_sdk::core::{Object, Resolver};
use ferruginous_sdk::page::Page;
use std::path::Path;

#[test]
fn test_dump_graphics_ops() {
    println!("CWD: {:?}", std::env::current_dir().unwrap());
    let pdf_path = "../../tests/fixtures/pdfs/graphics-test.pdf";
    let data = std::fs::read(pdf_path).expect("failed to read pdf");
    let doc = load_document_structure(&data).unwrap();
    let catalog = doc.catalog().unwrap();
    let page_tree = doc.page_tree().unwrap();
    
    for i in 0..page_tree.get_count() {
        let page = page_tree.get_page(i).unwrap();
        let contents = page.get_display_list().unwrap();
        
        println!("--- Page {} Display List ---", i + 1);
        for cmd in contents {
            println!("{:?}", cmd.op);
        }
    }
}

fn dump_node(node: &ferruginous_sdk::content::ContentNode, depth: usize) {
    let indent = " ".repeat(depth * 2);
    match node {
        ferruginous_sdk::content::ContentNode::Operation(op) => {
            let op_name = String::from_utf8_lossy(&op.operator);
            println!("{}{}: {:?}", indent, op_name, op.operands);
        }
        ferruginous_sdk::content::ContentNode::Block(start, children) => {
            println!("{}Block ({}):", indent, String::from_utf8_lossy(start));
            for child in children {
                dump_node(child, depth + 1);
            }
        }
        ferruginous_sdk::content::ContentNode::Operation(_) => {}
        ferruginous_sdk::content::ContentNode::TransparencyGroup(_, children) => {
            println!("{}TransparencyGroup:", indent);
            for child in children {
                dump_node(child, depth + 1);
            }
        }
    }
}
