use std::str::from_utf8;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rask::parser::{raw_request::RawRequest, Method, ParseError, ParseResult};

const METHODS: [[u8; 8]; 8] = [
    [b'G', b'E', b'T', 0, 0, 0, 0, 0],
    [b'P', b'U', b'T', 0, 0, 0, 0, 0],
    [b'P', b'O', b'S', b'T', 0, 0, 0, 0],
    [b'H', b'E', b'A', b'D', 0, 0, 0, 0],
    [b'T', b'R', b'A', b'C', b'E', 0, 0, 0],
    [b'D', b'E', b'L', b'E', b'T', b'E', 0, 0],
    [b'O', b'P', b'T', b'I', b'O', b'N', b'S', 0],
    [b'C', b'O', b'N', b'N', b'E', b'C', b'T', 0],
];

fn benchmark(c: &mut Criterion) {
    for method in METHODS {
        c.bench_with_input(
            BenchmarkId::new(
                "method",
                from_utf8(&method).unwrap().trim_matches(char::is_control),
            ),
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
    let eight: [u8; 8] = buf.as_ref()[0..8]
        .try_into()
        .map_err(|_| ParseError::Method)?;
    let eight = u64::from_ne_bytes(eight);

    if eight & 0x0000_0000_00ff_ffff == u64::from_le_bytes([b'G', b'E', b'T', 0, 0, 0, 0, 0]) {
        buf.advance(3);
        buf.slice();
        Ok(Method::Get)
    } else if eight & 0x0000_0000_00ff_ffff == u64::from_le_bytes([b'P', b'U', b'T', 0, 0, 0, 0, 0])
    {
        buf.advance(3);
        buf.slice();
        Ok(Method::Put)
    } else if eight & 0x0000_0000_ffff_ffff
        == u64::from_le_bytes([b'P', b'O', b'S', b'T', 0, 0, 0, 0])
    {
        buf.advance(4);
        buf.slice();
        Ok(Method::Post)
    } else if eight & 0x0000_0000_ffff_ffff
        == u64::from_le_bytes([b'H', b'E', b'A', b'D', 0, 0, 0, 0])
    {
        buf.advance(4);
        buf.slice();
        Ok(Method::Head)
    } else if eight & 0x0000_00ff_ffff_ffff
        == u64::from_le_bytes([b'T', b'R', b'A', b'C', b'E', 0, 0, 0])
    {
        buf.advance(5);
        buf.slice();
        Ok(Method::Trace)
    } else if eight & 0x0000_ffff_ffff_ffff
        == u64::from_le_bytes([b'D', b'E', b'L', b'E', b'T', b'E', 0, 0])
    {
        buf.advance(6);
        buf.slice();
        Ok(Method::Delete)
    } else if eight & 0x00ff_ffff_ffff_ffff
        == u64::from_le_bytes([b'O', b'P', b'T', b'I', b'O', b'N', b'S', 0])
    {
        buf.advance(7);
        buf.slice();
        Ok(Method::Options)
    } else if eight & 0x00ff_ffff_ffff_ffff
        == u64::from_le_bytes([b'C', b'O', b'N', b'N', b'E', b'C', b'T', 0])
    {
        buf.advance(7);
        buf.slice();
        Ok(Method::Connect)
    } else {
        Err(ParseError::Method)
    }
}
