#![allow(clippy::all, missing_docs)]
//! Test module

use ferruginous_sdk::core::Object;
use ferruginous_sdk::lexer::parse_object;
use ferruginous_sdk::writer::write_object;

#[test]
fn test_object_round_trip() {
    let test_cases = vec![
        "true",
        "false",
        "null",
        "42",
        "-123",
        "3.1416",
        "/Name",
        "/Name#20With#20Spaces",
        "(Literal String)",
        "(String with (nested) parens)",
        "<4E6F76>", // Hex string "Nov"
        "[1 2 3]",
        "[1 /Name (String)]",
        "<< /Key /Value /Num 10 >>",
        "<< /Sub << /K /V >> >>",
        "10 0 R",
    ];

    for case in test_cases {
        // 1. Parse
        let input = case.as_bytes();
        let (_, obj) = parse_object(input).unwrap_or_else(|_| panic!("Failed to parse: {case}"));

        // 2. Serialize
        let mut buf = Vec::new();
        write_object(&mut buf, &obj).unwrap_or_else(|_| panic!("Failed to write: {obj:?}"));

        // 3. Re-parse
        let (_, re_obj) = parse_object(&buf).unwrap_or_else(|_| panic!("Failed to re-parse: {}", String::from_utf8_lossy(&buf)));

        // 4. Compare
        assert_eq!(obj, re_obj, "Round-trip failed for: {case}");
    }
}

#[test]
fn test_stream_round_trip() {
    let stream_input = b"<< /Length 5 >>\nstream\nabcde\nendstream";
    let (_, obj) = parse_object(stream_input).unwrap();

    let mut buf = Vec::new();
    write_object(&mut buf, &obj).unwrap();

    let (_, re_obj) = parse_object(&buf).unwrap_or_else(|_| panic!("Failed to re-parse stream: {}", String::from_utf8_lossy(&buf)));
    
    if let (Object::Stream(_, data1), Object::Stream(_, data2)) = (obj, re_obj) {
        assert_eq!(data1, data2);
    } else {
        panic!("Expected streams");
    }
}
