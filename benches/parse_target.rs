use std::ops::Range;

use criterion::{
    black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput::Bytes,
};
use rask::parser::{h1::tokens::is_request_target_token, ParseError, ParseResult, Status};

const TARGETS: [&[u8]; 4] = [
    b"/",
    b"/api/v1.0/weather/forecast/days/16",
    b"/wp-content/uploads/2010/03/hello-kitty-darth-vader-pink.jpg",
    b"/nvidia_web_services/controller.gfeclientcontent.php/com.nvidia.services.GFEClientContent.getShieldReady/{\"gcV\":\"2.2.2.0\",\"dID\":\"1341\",\"osC\":\"6.20\",\"is6\":\"1\",\"lg\":\"1033\",\"GFPV\":\"389.08\",\"isO\":\"1\",\"sM\":\"16777216\"}"
];

fn benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("target");
    for target in TARGETS {
        group.throughput(Bytes(target.len() as u64));

        group.bench_with_input(
            BenchmarkId::new("vector", target.len()),
            black_box(&target),
            |b, i| {
                b.iter(|| {
                    let _ = parse_target(black_box(i), 0);
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("autovector", target.len()),
            black_box(&target),
            |b, i| {
                b.iter(|| {
                    let _ = parse_target2(black_box(i), 0);
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("scalar", target.len()),
            black_box(&target),
            |b, i| {
                b.iter(|| {
                    let _ = parse_target_scalar(black_box(i), 0);
                })
            },
        );
    }
    group.finish();
}

#[cfg(all(
    target_feature = "avx2",
    any(target_arch = "x86", target_arch = "x86_64")
))]
#[inline]
#[allow(overflowing_literals)]
fn parse_target_vectorized_avx2(buf: &[u8], mut pos: usize) -> Result<usize, usize> {
    #[cfg(target_arch = "x86")]
    use core::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use core::arch::x86_64::*;

    unsafe {
        let row_map = _mm256_setr_epi8(
            0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, // prevent fmt
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // prevent fmt
            0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, // prevent fmt
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        );
        let col_map = _mm256_setr_epi8(
            0xf8, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, // prevent fmt
            0xfc, 0xfc, 0xfc, 0xfc, 0xf4, 0xfc, 0xf4, 0x7c, // prevent fmt
            0xf8, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, // prevent fmt
            0xfc, 0xfc, 0xfc, 0xfc, 0xf4, 0xfc, 0xf4, 0x7c,
        );
        let lower_mask = _mm256_set1_epi8(0x0f);

        while buf[pos..].len() >= 32 {
            let data = _mm256_lddqu_si256(buf[pos..].as_ptr() as *const _);

            // divide by 2^4 to get row and take lower half as shuffle control mask
            let lower_div16 = _mm256_and_si256(lower_mask, _mm256_srli_epi16(data, 4));
            let row_mask = _mm256_shuffle_epi8(row_map, lower_div16);
            let col_mask = _mm256_shuffle_epi8(col_map, data);

            let row_col = _mm256_and_si256(row_mask, col_mask);
            let valid = _mm256_cmpeq_epi8(row_col, _mm256_setzero_si256());
            let num_valid = (_mm256_movemask_epi8(valid) as u32).trailing_zeros();

            pos += num_valid as usize;

            if num_valid != 32 {
                return Ok(pos);
            }
        }
    }

    Err(pos)
}

#[cfg(all(
    target_feature = "ssse3",
    any(target_arch = "x86", target_arch = "x86_64")
))]
#[inline]
#[allow(overflowing_literals)]
fn parse_target_vectorized_ssse3(buf: &[u8], mut pos: usize) -> Result<usize, usize> {
    #[cfg(target_arch = "x86")]
    use core::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use core::arch::x86_64::*;

    unsafe {
        let row_map: __m128i = _mm_setr_epi8(
            0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, // prevent fmt
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        );
        let col_map: __m128i = _mm_setr_epi8(
            0xf8, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, // prevent fmt
            0xfc, 0xfc, 0xfc, 0xfc, 0xf4, 0xfc, 0xf4, 0x7c,
        );
        let lower_mask: __m128i = _mm_set1_epi8(0x0f);

        while buf[pos..].len() >= 16 {
            let data = _mm_lddqu_si128(buf[pos..].as_ptr() as *const _);

            // divide by 2^4 and only take lower half
            let lower_div16 = _mm_and_si128(lower_mask, _mm_srli_epi16(data, 4));
            let row_mask = _mm_shuffle_epi8(row_map, lower_div16);
            let col_mask = _mm_shuffle_epi8(col_map, data);

            let row_col = _mm_and_si128(row_mask, col_mask);
            let valid = _mm_cmpeq_epi8(row_col, _mm_setzero_si128());
            let num_valid = (0xffff_0000 | _mm_movemask_epi8(valid) as u32).trailing_zeros();

            pos += num_valid as usize;

            if num_valid != 16 {
                return Ok(pos);
            }
        }
    }

    Err(pos)
}

fn parse_target(buf: &[u8], mut pos: usize) -> ParseResult<(usize, Range<usize>)> {
    let start = pos;

    #[cfg(all(
        target_feature = "avx2",
        any(target_arch = "x86", target_arch = "x86_64")
    ))]
    match parse_target_vectorized_avx2(buf, pos) {
        Ok(n) => return Ok(Status::Complete((n, start..n))),
        Err(n) => pos = n,
    };

    #[cfg(all(
        target_feature = "ssse3",
        any(target_arch = "x86", target_arch = "x86_64")
    ))]
    match parse_target_vectorized_ssse3(buf, pos) {
        Ok(n) => return Ok(Status::Complete((n, start..n))),
        Err(n) => pos = n,
    };

    for &b in &buf[pos..] {
        if !is_request_target_token(b) {
            if pos == start {
                return Err(ParseError::Target);
            }

            return Ok(Status::Complete((pos, start..pos)));
        }

        pos += 1;
    }

    Ok(Status::Partial)
}

// https://godbolt.org/z/cqvf3j1eE
fn parse_target2(buf: &[u8], mut pos: usize) -> ParseResult<(usize, Range<usize>)> {
    let start = pos;

    for window in buf[start..].chunks(32) {
        let res = window.iter().enumerate().fold(0, |acc, (i, b)| {
            (((*b == b'=' || (b'!'..=b';').contains(b) || (b'?'..=b'~').contains(b)) as u32) << i)
                | acc
        });

        let num_valid = res.trailing_ones();
        pos += num_valid as usize;

        if num_valid != 32 {
            if pos == start {
                return Err(ParseError::Target);
            }

            return Ok(Status::Complete((pos, start..pos)));
        }
    }

    Ok(Status::Partial)
}

fn parse_target_scalar(buf: &[u8], mut pos: usize) -> ParseResult<(usize, Range<usize>)> {
    let start = pos;

    for &b in &buf[pos..] {
        if !is_request_target_token(b) {
            return Ok(Status::Complete((pos, start..pos)));
        }

        pos += 1;
    }

    Ok(Status::Partial)
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
