#![allow(clippy::all, missing_docs)]
//! Test module

use ferruginous_sdk::core::{Object, Resolver, PdfResult, Reference};
use ferruginous_sdk::content::{Processor, Operation};
use ferruginous_sdk::ocg::OCContext;
use std::collections::BTreeMap;

struct MockResolver;
impl Resolver for MockResolver {
    fn resolve(&self, reference: &Reference) -> PdfResult<Object> {
        let mut dict = BTreeMap::new();
        dict.insert(b"Type".to_vec(), Object::new_name(b"OCG".to_vec()));
        if reference.id == 10 {
            dict.insert(b"Name".to_vec(), Object::new_string(b"Layer1".to_vec()));
        } else if reference.id == 11 {
            dict.insert(b"Name".to_vec(), Object::new_string(b"Layer2".to_vec()));
        }
        Ok(Object::new_dict(dict))
    }
}

#[test]
fn test_ocg_visibility_filtering() -> PdfResult<()> {
    // 1. Create a mock OCG setup
    let ref1 = Reference::new(10, 0);
    let ref2 = Reference::new(11, 0);
    
    let mut states = BTreeMap::new();
    states.insert(ref1, true);
    states.insert(ref2, false);
    
    let ctx = OCContext::new(states);

    // 2. Setup Resources with /Properties mapping
    let mut props = BTreeMap::new();
    props.insert(b"Layer1".to_vec(), Object::Reference(ref1));
    props.insert(b"Layer2".to_vec(), Object::Reference(ref2));
    
    let mut res_dict = BTreeMap::new();
    res_dict.insert(b"Properties".to_vec(), Object::new_dict(props));
    
    let resolver = MockResolver;
    // We don't need a real Resources object, but Processor needs one
    let resources = ferruginous_sdk::resources::Resources::new(res_dict.into(), &resolver);
    
    let media_box = Some([0.0, 0.0, 100.0, 100.0]);
    let mut processor = Processor::new(Some(resources), media_box, Some(ctx));

    let l1_name = b"Layer1".to_vec();
    let l2_name = b"Layer2".to_vec();

    // Simulate BDC for Layer 1 (Visible)
    let bdc1 = Operation {
        operator: b"BDC".to_vec(),
        operands: vec![Object::new_name(b"OC".to_vec()), Object::new_name(l1_name)],
    };
    processor.execute_operation(&bdc1)?;
    
    // Operation inside Layer 1
    let op1 = Operation {
        operator: b"m".to_vec(),
        operands: vec![Object::Integer(10), Object::Integer(10)],
    };
    processor.execute_operation(&op1)?;
    
    let emc = Operation {
        operator: b"EMC".to_vec(),
        operands: vec![],
    };
    processor.execute_operation(&emc)?;

    // Simulate BDC for Layer 2 (Hidden)
    let bdc2 = Operation {
        operator: b"BDC".to_vec(),
        operands: vec![Object::new_name(b"OC".to_vec()), Object::new_name(l2_name)],
    };
    processor.execute_operation(&bdc2)?;
    
    // Operation inside Layer 2 (should be ignored)
    let op2 = Operation {
        operator: b"m".to_vec(),
        operands: vec![Object::Integer(20), Object::Integer(20)],
    };
    processor.execute_operation(&op2)?;
    processor.execute_operation(&emc)?;

    // 3. Verify display list
    // Only one StrokePath should exist
    processor.execute_operation(&bdc1)?;
    processor.execute_operation(&op1)?;
    let op_s = Operation { operator: b"S".to_vec(), operands: vec![] };
    processor.execute_operation(&op_s)?;
    processor.execute_operation(&emc)?;

    processor.execute_operation(&bdc2)?;
    processor.execute_operation(&op2)?;
    processor.execute_operation(&op_s)?;
    processor.execute_operation(&emc)?;

    let dl = &processor.display_list;
    
    // Only one StrokePath should exist
    let stroke_count = dl.iter().filter(|op| matches!(op.op, ferruginous_sdk::graphics::DrawOp::StrokePath { .. })).count();
    assert_eq!(stroke_count, 1, "Only visible layer should emit DrawOp");
    
    Ok(())
}
