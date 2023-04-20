use std::str::from_utf8_unchecked;

use criterion::{
    black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput::Bytes,
};
use rask::parser::{
    h1::tokens::is_request_target_token, raw_request::RawRequest, ParseError, ParseResult, Status,
};

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
                    let mut buf = RawRequest::new(i);
                    let _ = parse_target_vector(&mut buf);
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("scalar", target.len()),
            black_box(&target),
            |b, i| {
                b.iter(|| {
                    let mut buf = RawRequest::new(i);
                    let _ = parse_target_scalar(&mut buf);
                })
            },
        );
    }
    group.finish();
}

#[inline]
#[allow(overflowing_literals)]
unsafe fn parse_target_vectorized_avx2(buf: &mut RawRequest<'_>) -> bool {
    use core::arch::x86_64::*;

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

    while buf.as_ref().len() >= 32 {
        let data = _mm256_lddqu_si256(buf.as_ref().as_ptr() as *const _);

        // divide by 2^4 to get row and take lower half as shuffle control mask
        let lower_div16 = _mm256_and_si256(lower_mask, _mm256_srli_epi16(data, 4));
        let row_mask = _mm256_shuffle_epi8(row_map, lower_div16);
        let col_mask = _mm256_shuffle_epi8(col_map, data);

        let row_col = _mm256_and_si256(row_mask, col_mask);
        let valid = _mm256_cmpeq_epi8(row_col, _mm256_setzero_si256());
        let num_valid = (_mm256_movemask_epi8(valid) as u32).trailing_zeros();

        buf.advance(num_valid as usize);

        if num_valid != 32 {
            return true;
        }
    }

    false
}

#[inline]
#[allow(overflowing_literals)]
unsafe fn parse_target_vectorized_ssse3(buf: &mut RawRequest<'_>) -> bool {
    use core::arch::x86_64::*;

    let row_map = _mm_setr_epi8(
        0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, // prevent fmt
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    );
    let col_map = _mm_setr_epi8(
        0xf8, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, // prevent fmt
        0xfc, 0xfc, 0xfc, 0xfc, 0xf4, 0xfc, 0xf4, 0x7c,
    );
    let lower_mask = _mm_set1_epi8(0x0f);

    while buf.as_ref().len() >= 16 {
        let data = _mm_lddqu_si128(buf.as_ref().as_ptr() as *const _);

        // divide by 2^4 and only take lower half
        let lower_div16 = _mm_and_si128(lower_mask, _mm_srli_epi16(data, 4));
        let row_mask = _mm_shuffle_epi8(row_map, lower_div16);
        let col_mask = _mm_shuffle_epi8(col_map, data);

        let row_col = _mm_and_si128(row_mask, col_mask);
        let valid = _mm_cmpeq_epi8(row_col, _mm_setzero_si128());
        let num_valid = (0xffff_0000 | _mm_movemask_epi8(valid) as u32).trailing_zeros();

        buf.advance(num_valid as usize);

        if num_valid != 16 {
            return true;
        }
    }

    false
}

#[inline(never)]
fn parse_target_vector<'b>(buf: &mut RawRequest<'b>) -> ParseResult<&'b str> {
    unsafe { parse_target_vectorized_avx2(buf) || parse_target_vectorized_ssse3(buf) };
    loop {
        let b = buf.next().ok_or(ParseError::Target)?;
        if !is_request_target_token(*b) {
            return Ok(Status::Complete(unsafe {
                from_utf8_unchecked(buf.slice_skip(1).map_err(|_| ParseError::Target)?)
            }));
        }
    }
}

#[inline(never)]
fn parse_target_scalar<'b>(buf: &mut RawRequest<'b>) -> ParseResult<&'b str> {
    loop {
        let b = buf.next().ok_or(ParseError::Target)?;
        if !is_request_target_token(*b) {
            return Ok(Status::Complete(unsafe {
                from_utf8_unchecked(buf.slice_skip(1).map_err(|_| ParseError::Target)?)
            }));
        }
    }
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
