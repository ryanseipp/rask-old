// Copyright 2022 Ryan Seipp
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! HTTP/1.1 Request

use std::fmt::Display;
use std::mem::MaybeUninit;
use std::str::from_utf8_unchecked;

use super::tokens::{is_header_name_token, is_header_value_token, is_request_target_token};
use super::{discard_newline, discard_whitespace, ParseError, ParseResult};
use crate::parser::raw_request::RawRequest;
use crate::parser::{Method, Version};

/// TODO
#[derive(Debug, PartialEq, Eq, Default, Clone)]
pub struct Header<'buf> {
    /// Header name
    pub name: &'buf str,
    /// Header value
    pub value: &'buf str,
}

// TODO: I don't think we can hold onto &str, as we may receive requests over multiple TCP packets.
// This would require such a mashup of lifetimes that would be impossible to reason about. How do
// we avoid the need to allocate a ton of strings? Would cost two heap allocations per header...
// Can we potentially just keep a buffer for the entire request received over multiple packets, and
// indexes into the important parts, deferring parsing until it's actually needed/used? Would mean
// one heap allocation per packet rather than tons
/// Parsed H1 Request
/// IETF RFC 9112
#[derive(Debug, Default)]
pub struct H1Request<'buf> {
    /// method
    pub method: Option<Method>,
    /// target
    pub target: Option<&'buf str>,
    /// version
    pub version: Option<Version>,
    ///
    pub headers: Option<&'buf [Header<'buf>]>,
}

impl<'b> H1Request<'b> {
    /// Creates a new HTTP/1.1 request
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses a request
    ///
    /// # Example
    /// ```
    /// # use rask::parser::{Method, Version};
    /// # use rask::parser::ParseError;
    /// # use rask::parser::h1::request::{H1Request, Header};
    /// # fn main() -> Result<(), ParseError> {
    /// let mut req = H1Request::new();
    /// req.parse(b"GET / HTTP/1.1\r\nHost:http://www.example.org\r\n\r\n")?;
    /// assert_eq!(Some(Method::Get), req.method);
    /// assert_eq!(Some("/"), req.target);
    /// assert_eq!(Some(Version::H1_1), req.version);
    /// assert!(req.headers.is_some());
    /// assert_eq!(Header {name: "Host", value: "http://www.example.org"}, req.headers.unwrap()[0]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn parse(&mut self, buf: &'b [u8]) -> ParseResult<()> {
        let mut req = RawRequest::new(buf);
        self.method = Some(parse_method(&mut req)?);
        discard_whitespace(&mut req);
        self.target = Some(parse_target(&mut req)?);
        discard_whitespace(&mut req);
        self.version = Some(parse_version(&mut req)?);
        discard_newline(&mut req);

        unsafe {
            let mut headers: [MaybeUninit<Header<'b>>; 96] = MaybeUninit::uninit().assume_init();
            let headers = &mut headers as *mut [MaybeUninit<Header<'b>>];
            match parse_headers(&mut req, &mut *headers) {
                Ok(num) => {
                    let headers = &*(headers as *mut [Header<'b>]);
                    self.headers = Some(&headers[0..num]);
                    Ok(())
                }
                Err(err) => {
                    std::mem::take(&mut &mut *headers);
                    Err(err)
                }
            }?;
        }

        Ok(())
    }
}

impl Display for H1Request<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{} {} {}\n",
            self.method.as_ref().unwrap_or(&Method::Get),
            self.target.unwrap_or(""),
            self.version.as_ref().unwrap_or(&Version::H1_0)
        ))?;

        if let Some(headers) = &self.headers {
            for header in headers.iter().filter(|h| !h.name.is_empty()) {
                f.write_fmt(format_args!("{}: {}\n", header.name, header.value))?;
            }
        }

        Ok(())
    }
}

#[inline]
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

#[cfg(all(
    target_feature = "avx2",
    any(target_arch = "x86", target_arch = "x86_64")
))]
#[inline]
#[allow(overflowing_literals)]
fn parse_target_vectorized_avx2(buf: &mut RawRequest<'_>) -> bool {
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
    }

    false
}

#[cfg(all(
    target_feature = "ssse3",
    any(target_arch = "x86", target_arch = "x86_64")
))]
#[inline]
#[allow(overflowing_literals)]
fn parse_target_vectorized_ssse3(buf: &mut RawRequest<'_>) -> bool {
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
    }

    false
}

#[inline]
fn parse_target<'b>(buf: &mut RawRequest<'b>) -> ParseResult<&'b str> {
    let mut found = false;
    #[cfg(all(
        target_feature = "avx2",
        any(target_arch = "x86", target_arch = "x86_64")
    ))]
    if !found {
        found = parse_target_vectorized_avx2(buf);
    }

    #[cfg(all(
        target_feature = "ssse3",
        any(target_arch = "x86", target_arch = "x86_64")
    ))]
    if !found {
        parse_target_vectorized_ssse3(buf);
    };

    loop {
        let b = buf.next().ok_or(ParseError::Target)?;
        if !is_request_target_token(*b) {
            return Ok(unsafe {
                from_utf8_unchecked(buf.slice_skip(1).map_err(|_| ParseError::Target)?)
            });
        }
    }
}

#[inline]
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

#[cfg(all(
    target_feature = "avx2",
    any(target_arch = "x86", target_arch = "x86_64")
))]
#[inline]
#[allow(overflowing_literals)]
fn validate_header_name_avx2(buf: &mut RawRequest<'_>) -> bool {
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
            0xe8, 0xfc, 0xf8, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, // prevent fmt
            0xf8, 0xf8, 0xf4, 0x54, 0xd0, 0x54, 0xf4, 0x70, // prevent fmt
            0xe8, 0xfc, 0xf8, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, // prevent fmt
            0xf8, 0xf8, 0xf4, 0x54, 0xd0, 0x54, 0xf4, 0x70,
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
    }

    false
}

#[cfg(all(
    target_feature = "ssse3",
    any(target_arch = "x86", target_arch = "x86_64")
))]
#[inline]
#[allow(overflowing_literals)]
fn validate_header_name_ssse3(buf: &mut RawRequest<'_>) -> bool {
    #[cfg(target_arch = "x86")]
    use core::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use core::arch::x86_64::*;

    unsafe {
        let row_map = _mm_setr_epi8(
            0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, // prevent fmt
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        );
        let col_map = _mm_setr_epi8(
            0xe8, 0xfc, 0xf8, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, // prevent fmt
            0xf8, 0xf8, 0xf4, 0x54, 0xd0, 0x54, 0xf4, 0x70,
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
    }

    false
}

#[cfg(all(
    target_feature = "avx2",
    any(target_arch = "x86", target_arch = "x86_64")
))]
#[inline]
fn validate_header_value_avx2(buf: &mut RawRequest<'_>) -> bool {
    #[cfg(target_arch = "x86")]
    use core::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use core::arch::x86_64::*;

    unsafe {
        let tab = _mm256_set1_epi8(0x09);
        let del = _mm256_set1_epi8(0x7f);
        let low = _mm256_set1_epi8(0x1f);

        while buf.as_ref().len() >= 32 {
            let data = _mm256_lddqu_si256(buf.as_ref().as_ptr() as *const _);

            let is_tab = _mm256_cmpeq_epi8(data, tab);
            let is_del = _mm256_cmpeq_epi8(data, del);
            let above_low = _mm256_cmpgt_epi8(data, low);
            let above_low_or_tab = _mm256_or_si256(above_low, is_tab);

            let valid = _mm256_andnot_si256(is_del, above_low_or_tab);
            let not_valid = _mm256_cmpeq_epi8(valid, _mm256_setzero_si256());
            let num_valid = (_mm256_movemask_epi8(not_valid) as u32).trailing_zeros();

            buf.advance(num_valid as usize);

            if num_valid != 32 {
                return true;
            }
        }
    }

    false
}

#[cfg(all(
    target_feature = "ssse3",
    any(target_arch = "x86", target_arch = "x86_64")
))]
#[inline]
fn validate_header_value_ssse3(buf: &mut RawRequest<'_>) -> bool {
    #[cfg(target_arch = "x86")]
    use core::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use core::arch::x86_64::*;

    unsafe {
        let tab = _mm_set1_epi8(0x09);
        let del = _mm_set1_epi8(0x7f);
        let low = _mm_set1_epi8(0x1f);

        while buf.as_ref().len() >= 16 {
            let data = _mm_lddqu_si128(buf.as_ref().as_ptr() as *const _);

            let is_tab = _mm_cmpeq_epi8(data, tab);
            let is_del = _mm_cmpeq_epi8(data, del);
            let above_low = _mm_cmpgt_epi8(data, low);
            let above_low_or_tab = _mm_or_si128(above_low, is_tab);

            let valid = _mm_andnot_si128(is_del, above_low_or_tab);
            let not_valid = _mm_cmpeq_epi8(valid, _mm_setzero_si128());
            let num_valid = (0xffff_0000 | _mm_movemask_epi8(not_valid) as u32).trailing_zeros();

            buf.advance(num_valid as usize);

            if num_valid != 16 {
                return true;
            }
        }
    }

    false
}

#[inline]
fn get_header_name<'b>(buf: &mut RawRequest<'b>) -> Option<&'b [u8]> {
    let mut found = false;
    #[cfg(all(
        target_feature = "avx2",
        any(target_arch = "x86", target_arch = "x86_64")
    ))]
    if !found {
        found = validate_header_name_avx2(buf);
    }

    #[cfg(all(
        target_feature = "ssse3",
        any(target_arch = "x86", target_arch = "x86_64")
    ))]
    if !found {
        found = validate_header_name_ssse3(buf);
    }

    return if found {
        Some(buf.slice())
    } else {
        buf.take_until(|b| !is_header_name_token(b))
    };
}

#[inline]
fn get_header_value<'b>(buf: &mut RawRequest<'b>) -> Option<&'b [u8]> {
    let mut found = false;
    #[cfg(all(
        target_feature = "avx2",
        any(target_arch = "x86", target_arch = "x86_64")
    ))]
    if !found {
        found = validate_header_value_avx2(buf);
    }

    #[cfg(all(
        target_feature = "ssse3",
        any(target_arch = "x86", target_arch = "x86_64")
    ))]
    if !found {
        found = validate_header_value_ssse3(buf);
    }

    return if found {
        Some(buf.slice())
    } else {
        buf.take_until(|b| !is_header_value_token(b))
    };
}

#[inline]
fn parse_headers<'b>(
    buf: &mut RawRequest<'b>,
    headers: &mut [MaybeUninit<Header<'b>>],
) -> ParseResult<usize> {
    let mut idx: usize = 0;
    loop {
        if let Some(name) = get_header_name(buf) {
            if buf.next() != Some(&b':') {
                return Err(ParseError::HeaderName);
            }

            discard_whitespace(buf);

            let value = get_header_value(buf).ok_or(ParseError::HeaderValue)?;
            let (name, value) = unsafe { (from_utf8_unchecked(name), from_utf8_unchecked(value)) };

            headers[idx].write(Header { name, value });
            idx += 1;

            discard_newline(buf);
        } else if buf.next() == Some(&b'\r') && buf.next() == Some(&b'\n') {
            buf.slice();
            return Ok(idx);
        } else {
            return Err(ParseError::HeaderName);
        }
    }
}
