// use std::{mem::MaybeUninit, str::from_utf8_unchecked};
//
// use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
// use rask::parser::{
//     h1::{
//         discard_newline, discard_whitespace,
//         request::Header,
//         tokens::{is_header_name_token, is_header_value_token},
//     },
//     raw_request::RawRequest,
//     ParseError, ParseResult,
// };
//
// const REQ: &[u8] = b"\
// Host: www.example.org\r\n\r\n";
//
// const REQ_MED: &[u8] = b"\
// Accept:*/*\r\n\
// Accept-Encoding:gzip,deflate,br\r\n\
// Accept-Language:en-US,en;q=0.5\r\n\
// Cache-Control:no-cache\r\n\
// Connection:keep-alive\r\n\
// DNT:1\r\n\
// Host: www.example.org\r\n\
// Pragma:no-cache\r\n\
// Referrer:https://www.example.org\r\n\
// Sec-Fetch-Dest:empty\r\n\
// Sec-Fetch-Mode:cors\r\n\
// Sec-Fetch-Site:same-origin\r\n\
// User-Agent:Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/109.0\r\n\r\n";
//
// const REQ_LONG: &[u8] = b"\
// Host: play.google.com\r\n\
// User-Agent: Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/109.0\r\n\
// Accept: */*\r\n\
// Accept-Language: en-US,en;q=0.5\r\n\
// Accept-Encoding: gzip, deflate, br\r\n\
// Referer: https://www.google.com/\r\n\
// Content-Type: application/x-www-form-urlencoded;charset=utf-8\r\n\
// Content-Length: 669\r\n\
// Origin: https://www.google.com\r\n\
// DNT: 1\r\n\
// Connection: keep-alive\r\n\
// Cookie: 1P_JAR=2023-01-24-14; AEC=xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx; NID=xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx; OGPC=xxxxxxxxxxx; ANID=xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx; OGP=xxxxxxxxxx\r\n\
// Sec-Fetch-Dest: empty\r\n\
// Sec-Fetch-Mode: cors\r\n\
// Sec-Fetch-Site: same-site\r\n\
// Pragma: no-cache\r\n\
// Cache-Control: no-cache\r\n\
// TE: trailers\r\n\r\n";
//
// const REQ_COMP: &[u8] = b"\
// Host: www.example.com\r\n\
// User-Agent: Mozilla/5.0 (Macintosh; U; Intel Mac OS X 10.6; ja-JP-mac; rv:1.9.2.3) Gecko/20100401 Firefox/3.6.3 Pathtraq/0.9\r\n\
// Accept: text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8\r\n\
// Accept-Language: ja,en-us;q=0.7,en;q=0.3\r\n\
// Accept-Encoding: gzip,deflate\r\n\
// Accept-Charset: Shift_JIS,utf-8;q=0.7,*;q=0.7\r\n\
// Keep-Alive: 115\r\n\
// Connection: keep-alive\r\n\
// Cookie: wp_ozh_wsa_visits=2; wp_ozh_wsa_visit_lasttime=xxxxxxxxxx; __utma=xxxxxxxxx.xxxxxxxxxx.xxxxxxxxxx.xxxxxxxxxx.xxxxxxxxxx.x; __utmz=xxxxxxxxx.xxxxxxxxxx.x.x.utmccn=(referral)|utmcsr=reader.livedoor.com|utmcct=/reader/|utmcmd=referral|padding=under256\r\n\r\n";
//
// const TARGETS: [&[u8]; 4] = [REQ, REQ_MED, REQ_COMP, REQ_LONG];
//
// fn benchmark(c: &mut Criterion) {
//     let mut group = c.benchmark_group("headers");
//     for target in TARGETS {
//         group.throughput(Throughput::Bytes(target.len() as u64));
//         group.bench_with_input(
//             BenchmarkId::new("vector", target.len() as u64),
//             black_box(&target),
//             |b, i| {
//                 b.iter(|| {
//                     let mut buf = RawRequest::new(i);
//                     unsafe {
//                         let mut headers: [MaybeUninit<Header<'_>>; 96] =
//                             MaybeUninit::uninit().assume_init();
//                         let headers = &mut headers as *mut [MaybeUninit<Header<'_>>];
//                         let _ = parse_headers_vector(&mut buf, &mut *headers);
//                     }
//                 })
//             },
//         );
//         group.bench_with_input(
//             BenchmarkId::new("scalar", target.len() as u64),
//             black_box(&target),
//             |b, i| {
//                 b.iter(|| {
//                     let mut buf = RawRequest::new(i);
//                     unsafe {
//                         let mut headers: [MaybeUninit<Header<'_>>; 96] =
//                             MaybeUninit::uninit().assume_init();
//                         let headers = &mut headers as *mut [MaybeUninit<Header<'_>>];
//                         let _ = parse_headers_scalar(&mut buf, &mut *headers);
//                     }
//                 })
//             },
//         );
//     }
//     group.finish();
// }
//
// criterion_group!(benches, benchmark);
// criterion_main!(benches);
//
// #[inline]
// #[allow(overflowing_literals)]
// unsafe fn validate_header_name_avx2(buf: &mut RawRequest<'_>) -> bool {
//     #[cfg(target_arch = "x86")]
//     use core::arch::x86::*;
//     #[cfg(target_arch = "x86_64")]
//     use core::arch::x86_64::*;
//
//     let row_map = _mm256_setr_epi8(
//         0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, // prevent fmt
//         0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // prevent fmt
//         0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, // prevent fmt
//         0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
//     );
//     let col_map = _mm256_setr_epi8(
//         0xe8, 0xfc, 0xf8, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, // prevent fmt
//         0xf8, 0xf8, 0xf4, 0x54, 0xd0, 0x54, 0xf4, 0x70, // prevent fmt
//         0xe8, 0xfc, 0xf8, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, // prevent fmt
//         0xf8, 0xf8, 0xf4, 0x54, 0xd0, 0x54, 0xf4, 0x70,
//     );
//     let lower_mask = _mm256_set1_epi8(0x0f);
//
//     while buf.as_ref().len() >= 32 {
//         let data = _mm256_lddqu_si256(buf.as_ref().as_ptr() as *const _);
//
//         // divide by 2^4 to get row and take lower half as shuffle control mask
//         let lower_div16 = _mm256_and_si256(lower_mask, _mm256_srli_epi16(data, 4));
//         let row_mask = _mm256_shuffle_epi8(row_map, lower_div16);
//         let col_mask = _mm256_shuffle_epi8(col_map, data);
//
//         let row_col = _mm256_and_si256(row_mask, col_mask);
//         let valid = _mm256_cmpeq_epi8(row_col, _mm256_setzero_si256());
//         let num_valid = (_mm256_movemask_epi8(valid) as u32).trailing_zeros();
//
//         buf.advance(num_valid as usize);
//
//         if num_valid != 32 {
//             return true;
//         }
//     }
//
//     false
// }
//
// #[inline]
// #[allow(overflowing_literals)]
// unsafe fn validate_header_name_ssse3(buf: &mut RawRequest<'_>) -> bool {
//     #[cfg(target_arch = "x86")]
//     use core::arch::x86::*;
//     #[cfg(target_arch = "x86_64")]
//     use core::arch::x86_64::*;
//
//     let row_map = _mm_setr_epi8(
//         0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, // prevent fmt
//         0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
//     );
//     let col_map = _mm_setr_epi8(
//         0xe8, 0xfc, 0xf8, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, // prevent fmt
//         0xf8, 0xf8, 0xf4, 0x54, 0xd0, 0x54, 0xf4, 0x70,
//     );
//     let lower_mask = _mm_set1_epi8(0x0f);
//
//     while buf.as_ref().len() >= 16 {
//         let data = _mm_lddqu_si128(buf.as_ref().as_ptr() as *const _);
//
//         // divide by 2^4 and only take lower half
//         let lower_div16 = _mm_and_si128(lower_mask, _mm_srli_epi16(data, 4));
//         let row_mask = _mm_shuffle_epi8(row_map, lower_div16);
//         let col_mask = _mm_shuffle_epi8(col_map, data);
//
//         let row_col = _mm_and_si128(row_mask, col_mask);
//         let valid = _mm_cmpeq_epi8(row_col, _mm_setzero_si128());
//         let num_valid = (0xffff_0000 | _mm_movemask_epi8(valid) as u32).trailing_zeros();
//
//         buf.advance(num_valid as usize);
//
//         if num_valid != 16 {
//             return true;
//         }
//     }
//
//     false
// }
//
// #[cfg(all(
//     target_feature = "avx2",
//     any(target_arch = "x86", target_arch = "x86_64")
// ))]
// #[inline]
// unsafe fn validate_header_value_avx2(buf: &mut RawRequest<'_>) -> bool {
//     #[cfg(target_arch = "x86")]
//     use core::arch::x86::*;
//     #[cfg(target_arch = "x86_64")]
//     use core::arch::x86_64::*;
//
//     let tab = _mm256_set1_epi8(0x09);
//     let del = _mm256_set1_epi8(0x7f);
//     let low = _mm256_set1_epi8(0x1f);
//
//     while buf.as_ref().len() >= 32 {
//         let data = _mm256_lddqu_si256(buf.as_ref().as_ptr() as *const _);
//
//         let is_tab = _mm256_cmpeq_epi8(data, tab);
//         let is_del = _mm256_cmpeq_epi8(data, del);
//         let above_low = _mm256_cmpgt_epi8(data, low);
//         let above_low_or_tab = _mm256_or_si256(above_low, is_tab);
//
//         let valid = _mm256_andnot_si256(is_del, above_low_or_tab);
//         let not_valid = _mm256_cmpeq_epi8(valid, _mm256_setzero_si256());
//         let num_valid = (_mm256_movemask_epi8(not_valid) as u32).trailing_zeros();
//
//         buf.advance(num_valid as usize);
//
//         if num_valid != 32 {
//             return true;
//         }
//     }
//
//     false
// }
//
// #[cfg(all(
//     target_feature = "ssse3",
//     any(target_arch = "x86", target_arch = "x86_64")
// ))]
// #[inline]
// unsafe fn validate_header_value_ssse3(buf: &mut RawRequest<'_>) -> bool {
//     #[cfg(target_arch = "x86")]
//     use core::arch::x86::*;
//     #[cfg(target_arch = "x86_64")]
//     use core::arch::x86_64::*;
//
//     let tab = _mm_set1_epi8(0x09);
//     let del = _mm_set1_epi8(0x7f);
//     let low = _mm_set1_epi8(0x1f);
//
//     while buf.as_ref().len() >= 16 {
//         let data = _mm_lddqu_si128(buf.as_ref().as_ptr() as *const _);
//
//         let is_tab = _mm_cmpeq_epi8(data, tab);
//         let is_del = _mm_cmpeq_epi8(data, del);
//         let above_low = _mm_cmpgt_epi8(data, low);
//         let above_low_or_tab = _mm_or_si128(above_low, is_tab);
//
//         let valid = _mm_andnot_si128(is_del, above_low_or_tab);
//         let not_valid = _mm_cmpeq_epi8(valid, _mm_setzero_si128());
//         let num_valid = (0xffff_0000 | _mm_movemask_epi8(not_valid) as u32).trailing_zeros();
//
//         buf.advance(num_valid as usize);
//
//         if num_valid != 16 {
//             return true;
//         }
//     }
//
//     false
// }
//
// #[inline(never)]
// fn parse_headers_vector<'b>(
//     buf: &mut RawRequest<'b>,
//     headers: &mut [MaybeUninit<Header<'b>>],
// ) -> ParseResult<usize> {
//     let mut idx: usize = 0;
//     loop {
//         let found = unsafe { validate_header_name_avx2(buf) || validate_header_name_ssse3(buf) };
//         let name = if found {
//             Some(buf.slice())
//         } else {
//             buf.take_until(|b| !is_header_name_token(b))
//         };
//
//         if let Some(name) = name {
//             if buf.next() != Some(&b':') {
//                 return Err(ParseError::HeaderName);
//             }
//
//             discard_whitespace(buf);
//
//             let found =
//                 unsafe { validate_header_value_avx2(buf) || validate_header_value_ssse3(buf) };
//
//             let value = if found {
//                 buf.slice()
//             } else {
//                 buf.take_until(|b| !is_header_value_token(b))
//                     .ok_or(ParseError::HeaderValue)?
//             };
//
//             let (name, value) = unsafe { (from_utf8_unchecked(name), from_utf8_unchecked(value)) };
//
//             headers[idx].write(Header { name, value });
//             idx += 1;
//
//             discard_newline(buf);
//         } else if buf.next() == Some(&b'\r') && buf.next() == Some(&b'\n') {
//             buf.slice();
//             return Ok(idx);
//         } else {
//             return Err(ParseError::HeaderName);
//         }
//     }
// }
//
// #[inline(never)]
// fn parse_headers_scalar<'b>(
//     buf: &mut RawRequest<'b>,
//     headers: &mut [MaybeUninit<Header<'b>>],
// ) -> ParseResult<usize> {
//     let mut idx: usize = 0;
//     loop {
//         if let Some(name) = buf.take_until(|b| !is_header_name_token(b)) {
//             if buf.next() != Some(&b':') {
//                 return Err(ParseError::HeaderName);
//             }
//
//             discard_whitespace(buf);
//
//             let value = buf
//                 .take_until(|b| !is_header_value_token(b))
//                 .ok_or(ParseError::HeaderValue)?;
//
//             let (name, value) = unsafe { (from_utf8_unchecked(name), from_utf8_unchecked(value)) };
//
//             headers[idx].write(Header { name, value });
//             idx += 1;
//
//             discard_newline(buf);
//         } else if buf.next() == Some(&b'\r') && buf.next() == Some(&b'\n') {
//             buf.slice();
//             return Ok(idx);
//         } else {
//             return Err(ParseError::HeaderName);
//         }
//     }
// }

fn main() {}
