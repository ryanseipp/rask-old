use std::str::from_utf8;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rask::parser::{raw_request::RawRequest, ParseError, ParseResult, Version};

const METHODS: [[u8; 8]; 4] = [
    [b'H', b'T', b'T', b'P', b'/', b'1', b'.', b'1'],
    [b'H', b'T', b'T', b'P', b'/', b'1', b'.', b'0'],
    [b'H', b'T', b'T', b'P', b'/', b'2', 0, 0],
    [b'H', b'T', b'T', b'P', b'/', b'3', 0, 0],
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
                    let _ = parse_version(&mut buf);
                })
            },
        );
    }
}

criterion_group!(benches, benchmark);
criterion_main!(benches);

#[inline(never)]
fn parse_version(buf: &mut RawRequest<'_>) -> ParseResult<Version> {
    const SIX_BYTE_MASK: u64 = 0x0000_ffff_ffff_ffff;
    let eight: [u8; 8] = buf.as_ref()[0..8]
        .try_into()
        .map_err(|_| ParseError::Version)?;
    let eight = u64::from_ne_bytes(eight);

    if eight == u64::from_le_bytes([b'H', b'T', b'T', b'P', b'/', b'1', b'.', b'1']) {
        buf.advance(8);
        buf.slice();
        Ok(Version::H1_1)
    } else if eight == u64::from_le_bytes([b'H', b'T', b'T', b'P', b'/', b'1', b'.', b'0']) {
        buf.advance(8);
        buf.slice();
        Ok(Version::H1_0)
    } else if eight & SIX_BYTE_MASK
        == u64::from_le_bytes([b'H', b'T', b'T', b'P', b'/', b'2', 0, 0])
    {
        buf.advance(6);
        buf.slice();
        Ok(Version::H2)
    } else if eight & SIX_BYTE_MASK
        == u64::from_le_bytes([b'H', b'T', b'T', b'P', b'/', b'3', 0, 0])
    {
        buf.advance(6);
        buf.slice();
        Ok(Version::H3)
    } else {
        Err(ParseError::Version)
    }
}
