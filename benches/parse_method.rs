use std::str::from_utf8;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rask::parser::{raw_request::RawRequest, Method, ParseError, ParseResult};

const METHODS: [&[u8]; 8] = [
    b"GET", b"PUT", b"POST", b"HEAD", b"TRACE", b"DELETE", b"CONNECT", b"OPTIONS",
];

fn benchmark(c: &mut Criterion) {
    for method in METHODS {
        c.bench_with_input(
            BenchmarkId::new("method", from_utf8(method).unwrap()),
            black_box(&method),
            |b, i| {
                b.iter(|| {
                    let mut buf = RawRequest::new(i);
                    let _ = parse_method(&mut buf);
                })
            },
        );
    }
}

criterion_group!(benches, benchmark);
criterion_main!(benches);

#[inline(never)]
fn parse_method(buf: &mut RawRequest<'_>) -> ParseResult<Method> {
    loop {
        let b = buf.next().ok_or(ParseError::Method)?;
        if *b == b' ' {
            return Method::try_from(buf.slice_skip(1).map_err(|_| ParseError::Method)?);
        } else if !b.is_ascii_uppercase() {
            return Err(ParseError::Method);
        }
    }
}
