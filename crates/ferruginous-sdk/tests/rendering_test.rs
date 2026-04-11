use ferruginous_sdk::content::{Processor, parse_content_stream};
use ferruginous_sdk::graphics::{DrawOp, Color, ClippingRule};
use kurbo::Affine;

#[test]
fn test_draw_op_generation_basic() {
    let content = b"10 20 m 30 40 l S";
    let nodes = parse_content_stream(content).unwrap();
    
    // Processor with 200x100 mediabox (height = 100)
    let mut processor = Processor::new(None, Some([0.0, 0.0, 200.0, 100.0]), None);
    processor.process_nodes(&nodes).unwrap();
    
    let dl = processor.display_list;
    
    // 1. SetTransform (Normalization: height=100) -> [1 0 0 -1 0 100]
    // 2. StrokePath
    assert_eq!(dl.len(), 2);
    
    match &dl[0] {
        DrawOp::SetTransform(m) => {
            let c = m.as_coeffs();
            assert_eq!(c[0], 1.0); // a
            assert_eq!(c[3], -1.0); // d
            assert_eq!(c[5], 100.0); // f
        }
        _ => panic!("Expected SetTransform at index 0"),
    }
    
    match &dl[1] {
        DrawOp::StrokePath { path, color, width, .. } => {
            assert_eq!(path.elements().len(), 2);
            assert_eq!(*color, Color::RGB(0.0, 0.0, 0.0));
            assert_eq!(*width, 1.0);
        }
        _ => panic!("Expected StrokePath at index 1"),
    }
}

#[test]
fn test_draw_op_state_stack() {
    let content = b"q 1 0 0 1 10 20 cm S Q";
    let nodes = parse_content_stream(content).unwrap();
    
    let mut processor = Processor::new(None, None, None);
    processor.process_nodes(&nodes).unwrap();
    
    let dl = processor.display_list;
    
    // 1. PushState (q)
    // 2. SetTransform (cm)
    // 3. StrokePath (S)
    // 4. PopState (Q)
    assert_eq!(dl.len(), 4);
    assert!(matches!(dl[0], DrawOp::PushState));
    assert!(matches!(dl[1], DrawOp::SetTransform(_)));
    assert!(matches!(dl[2], DrawOp::StrokePath { .. }));
    assert!(matches!(dl[3], DrawOp::PopState));
}

#[test]
fn test_draw_op_fill_rules() {
    let content = b"0 0 m 10 10 l f*";
    let nodes = parse_content_stream(content).unwrap();
    
    let mut processor = Processor::new(None, None, None);
    processor.process_nodes(&nodes).unwrap();
    
    let dl = processor.display_list;
    
    match &dl[0] {
        DrawOp::FillPath { rule, .. } => {
            assert_eq!(*rule, ClippingRule::EvenOdd);
        }
        _ => panic!("Expected FillPath"),
    }
}
