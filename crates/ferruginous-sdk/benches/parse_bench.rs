use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ferruginous_sdk::lexer::parse_object;

fn bench_parse_simple_objects(c: &mut Criterion) {
    let input = b"<< /Type /Catalog /Pages [ 1 0 R 2 0 R ] /ID [ <12345> <67890> ] >>";
    c.bench_function("parse_dictionary", |b| b.iter(|| parse_object(black_box(input))));
}

fn bench_parse_large_stream(c: &mut Criterion) {
    let data = vec![b'a'; 1024 * 1024]; // 1MB stream
    let input = format!("<< /Length {} >>\nstream\n", data.len()).into_bytes();
    let mut full_input = input;
    full_input.extend_from_slice(&data);
    full_input.extend_from_slice(b"\nendstream");
    
    c.bench_function("parse_1mb_stream", |b| b.iter(|| parse_object(black_box(&full_input))));
}

criterion_group!(benches, bench_parse_simple_objects, bench_parse_large_stream);
criterion_main!(benches);
