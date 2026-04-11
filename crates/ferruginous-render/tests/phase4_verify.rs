use ferruginous_sdk::graphics::{DrawOp, Color, ClippingRule};
use ferruginous_sdk::filter::decode_stream;
use ferruginous_sdk::core::Object;
use ferruginous_render::Renderer;
use std::collections::BTreeMap;
use std::sync::Arc;
use vello::kurbo::{Rect, BezPath, Affine, Shape};

#[test]
fn test_phase4_integration_basic_render() {
    let mut renderer = Renderer::new();
    
    // 1. Test Filter + Image Data Flow
    // Compressed RGB data (1x1 red pixel)
    // Red: 255, 0, 0
    let raw_data = vec![255, 0, 0];
    let compressed = miniz_oxide::deflate::compress_to_vec_zlib(&raw_data, 6);
    
    let mut dict = BTreeMap::new();
    dict.insert(b"Filter".to_vec(), Object::new_name(b"FlateDecode".to_vec()));
    dict.insert(b"Width".to_vec(), Object::Integer(1));
    dict.insert(b"Height".to_vec(), Object::Integer(1));
    dict.insert(b"ColorSpace".to_vec(), Object::new_name(b"DeviceRGB".to_vec()));
    
    let decoded = decode_stream(&dict, &compressed).expect("Failed to decode image data");
    assert_eq!(decoded, raw_data);

    // 2. Build DisplayList with various operations
    let green_rect_path = Rect::new(0.0, 0.0, 100.0, 100.0).to_path(0.1);
    let stroke_rect_path = Rect::new(10.0, 10.0, 90.0, 90.0).to_path(0.1);

    let display_list = vec![
        DrawOp::PushState,
        DrawOp::SetTransform(Affine::IDENTITY),
        DrawOp::FillPath {
            path: Arc::new(green_rect_path),
            color: Color::RGB(0.0, 1.0, 0.0),
            rule: ClippingRule::NonZeroWinding,
            blend_mode: Default::default(),
            alpha: 1.0,
        },
        DrawOp::StrokePath {
            path: Arc::new(stroke_rect_path),
            color: Color::Gray(0.5),
            width: 2.0,
            blend_mode: Default::default(),
            alpha: 1.0,
        },
        DrawOp::DrawImage {
            data: Arc::new(decoded),
            width: 1,
            height: 1,
            components: 3,
            rect: Rect::new(20.0, 20.0, 80.0, 80.0),
            blend_mode: Default::default(),
            alpha: 1.0,
        },
        DrawOp::PopState,
    ];

    // 3. Verify Renderer can build scene without panicking
    renderer.render_display_list(&display_list, Affine::IDENTITY);
    
    // In Vello 0.8.0+, the scene is an opaque record of encodings. 
    // If it reaches here without panicking, the construction is considered successful.
    let _scene = renderer.scene();
}

#[test]
fn test_phase4_predictor_integration() {
    // TIFF Predictor (Predictor 2)
    // Row 1: 10, 20, 30 -> (after prediction) 10, 30, 60
    let data = vec![10, 20, 30];
    let mut dict = BTreeMap::new();
    dict.insert(b"Filter".to_vec(), Object::new_name(b"FlateDecode".to_vec()));
    let mut params = BTreeMap::new();
    params.insert(b"Predictor".to_vec(), Object::Integer(2));
    params.insert(b"Columns".to_vec(), Object::Integer(3));
    params.insert(b"Colors".to_vec(), Object::Integer(1));
    params.insert(b"BitsPerComponent".to_vec(), Object::Integer(8));
    dict.insert(b"DecodeParms".to_vec(), Object::new_dict(params));

    let compressed = miniz_oxide::deflate::compress_to_vec_zlib(&data, 6);
    let decoded = decode_stream(&dict, &compressed).expect("TIFF Predictor decode failed");
    
    assert_eq!(decoded, vec![10, 30, 60]);
}
