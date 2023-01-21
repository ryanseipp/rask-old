use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rask::parser::h1::request::H1Request;

fn benchmark(c: &mut Criterion) {
    let input: &[u8] = b"GET /api/v1.0/weather/forecast HTTP/1.1\r\nHost: www.example.org\r\n\r\n";

    c.bench_with_input(BenchmarkId::new("parse", "GET"), &input, |b, &i| {
        b.iter(|| {
            let mut req = H1Request::new();
            let _ = req.parse(i);
        })
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
