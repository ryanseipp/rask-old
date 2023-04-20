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
use std::io::{self, ErrorKind, Read};
use std::mem::MaybeUninit;
use std::ops::Range;
use std::str::from_utf8;

use super::tokens::{is_header_name_token, is_header_value_token, is_request_target_token};
use super::{
    discard_required_newline, discard_required_whitespace, discard_whitespace, ParseError,
    ParseResult,
};
use crate::parser::{Method, Status, Version};

/// TODO
#[derive(Debug, PartialEq, Eq, Default, Clone)]
pub struct Header {
    /// Header name
    pub name: Range<usize>,
    /// Header value
    pub value: Range<usize>,
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
pub struct H1Request {
    data: Vec<u8>,
    /// TODO
    pub complete: bool,
    /// TODO
    pub method: Option<Method>,
    /// TODO
    pub target: Option<Range<usize>>,
    /// TODO
    pub version: Option<Version>,
    /// TODO
    pub headers: Option<&'static [Header]>,
}

// TODO: PROBABLE UNDEFINED BEHAVIOR WITH HEADERS!!!!!!!!!

// impl Default for H1Request {
//     fn default() -> Self {
//         Self {
//             data: Vec::new(),
//             complete: false,
//             method: None,
//             target: None,
//             version: None,
//             headers: None,
//         }
//     }
// }

impl Display for H1Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{} ", &self.method.as_ref().unwrap()))?;
        f.write_fmt(format_args!(
            "{} ",
            from_utf8(&self.data[self.target.clone().unwrap()]).unwrap()
        ))?;
        f.write_fmt(format_args!("{}\r\n", &self.version.as_ref().unwrap()))?;

        for header in *self.headers.as_ref().unwrap() {
            f.write_fmt(format_args!(
                "{}: {}\r\n",
                from_utf8(&self.data[header.name.clone()]).unwrap(),
                from_utf8(&self.data[header.value.clone()]).unwrap()
            ))?;
        }

        f.write_str("\r\n")
    }
}

impl H1Request {
    /// Creates a new HTTP/1.1 request
    pub fn new() -> Self {
        Self::default()
    }

    /// Fills the request buffer with data received for the connection
    pub fn fill<R: Read>(&mut self, reader: &mut R) -> io::Result<usize> {
        let mut total_read = 0;
        let mut bytes = [0u8; 4096];
        loop {
            match reader.read(&mut bytes) {
                Ok(0) => return Ok(0),
                Ok(n) => {
                    total_read += n;
                    self.data.extend_from_slice(&bytes[..n]);
                }
                Err(e) => match e.kind() {
                    ErrorKind::WouldBlock => {
                        if total_read == 0 {
                            return Err(e);
                        } else {
                            return Ok(total_read);
                        }
                    }
                    ErrorKind::Interrupted => {}
                    _ => return Err(e),
                },
            }
        }

        // TODO: This doesn't work, as we can only read into an _initialized_ region owned by the
        // vec.
        // println!("filling");
        // let mut read: usize = 0;
        // loop {
        //     if self.data.capacity() - self.data.len() < 4096 {
        //         let len = self.data.len().saturating_sub(1);
        //         self.data.resize(len + 4096, 0);
        //     }
        //
        //     let pos = self.data.len().saturating_sub(1);
        //     match reader.read(&mut self.data[pos..]) {
        //         Ok(0) => {
        //             println!("read 0");
        //             return Ok(read);
        //         }
        //         Ok(n) => {
        //             println!("read {}", n);
        //             read += n;
        //         }
        //         Err(e) => {
        //             println!("err {:?}", e);
        //             match e.kind() {
        //                 ErrorKind::WouldBlock => {
        //                     if read == 0 {
        //                         return Err(e);
        //                     } else {
        //                         println!("read total {}", read);
        //                         return Ok(read);
        //                     }
        //                 }
        //                 ErrorKind::Interrupted => {}
        //                 _ => return Err(e),
        //             }
        //         }
        //     }
        // }
    }

    /// Fills the request buffer with exactly N bytes
    pub fn fill_exact<R: Read>(&mut self, reader: &mut R, n: usize) -> io::Result<()> {
        // buffer may have write capacity left. To avoid blocking, resize correctly
        let len = self.data.len().saturating_sub(1);
        self.data.resize(len + n, 0);
        reader.read_exact(&mut self.data)
    }

    /// Parses a request
    ///
    /// # Example
    /// ```
    /// # use rask::parser::{Method, Version, ParseError};
    /// # use rask::parser::h1::request::{H1Request, Header};
    /// # fn main() -> Result<(), ParseError> {
    /// let mut req = H1Request::new();
    /// let mut req_buffer: &[u8] = b"GET / HTTP/1.1\r\nHost:www.example.org\r\n\r\n";
    ///
    /// req.fill(&mut req_buffer).unwrap();
    /// req.parse()?;
    ///
    /// assert_eq!(Some(Method::Get), req.method);
    /// assert_eq!(Some(4..5), req.target);
    /// assert_eq!(Some(Version::H1_1), req.version);
    /// assert!(req.headers.is_some());
    /// assert_eq!(Header {name: 16..20, value: 21..36}, req.headers.unwrap()[0]);
    /// assert_eq!(true, req.complete);
    /// # Ok(())
    /// # }
    /// ```
    pub fn parse(&mut self) -> ParseResult<usize> {
        let mut pos: usize;

        match parse_method(&self.data) {
            Ok(Status::Complete((read, method))) => {
                pos = read;
                self.method = Some(method)
            }
            Ok(Status::Partial) => return Ok(Status::Partial),
            Err(err) => return Err(err),
        };

        match discard_required_whitespace(&self.data, pos, ParseError::Method) {
            Ok(Status::Complete(n)) => pos = n,
            Ok(Status::Partial) => return Ok(Status::Partial),
            Err(err) => return Err(err),
        };

        match parse_target(&self.data, pos) {
            Ok(Status::Complete((read, target))) => {
                pos = read;
                self.target = Some(target);
            }
            Ok(Status::Partial) => return Ok(Status::Partial),
            Err(err) => return Err(err),
        }

        match discard_required_whitespace(&self.data, pos, ParseError::Method) {
            Ok(Status::Complete(n)) => pos = n,
            Ok(Status::Partial) => return Ok(Status::Partial),
            Err(err) => return Err(err),
        };

        match parse_version(&self.data, pos) {
            Ok(Status::Complete((read, version))) => {
                pos = read;
                self.version = Some(version);
            }
            Ok(Status::Partial) => return Ok(Status::Partial),
            Err(err) => return Err(err),
        };

        match discard_required_newline(&self.data, pos, ParseError::NewLine) {
            Ok(Status::Complete(n)) => pos = n,
            Ok(Status::Partial) => return Ok(Status::Partial),
            Err(err) => return Err(err),
        };

        unsafe {
            let mut headers: [MaybeUninit<Header>; 96] = MaybeUninit::uninit().assume_init();
            let headers = &mut headers as *mut [MaybeUninit<Header>];
            match parse_headers(&self.data, pos, &mut *headers) {
                Ok(status) => {
                    let headers = &*(headers as *mut [Header]);
                    match status {
                        HeaderStatus::Complete((read, num_headers)) => {
                            self.headers = Some(&headers[0..num_headers]);
                            pos = read;
                        }
                        HeaderStatus::Partial(num_headers) => {
                            self.headers = Some(&headers[0..num_headers]);
                            return Ok(Status::Partial);
                        }
                    }
                }
                Err(err) => {
                    std::mem::take(&mut &mut *headers);
                    return Err(err);
                }
            }
        }

        match discard_required_newline(&self.data, pos, ParseError::NewLine) {
            Ok(Status::Complete(n)) => pos = n,
            Ok(Status::Partial) => return Ok(Status::Partial),
            Err(err) => return Err(err),
        };

        self.complete = true;

        Ok(Status::Complete(pos))
    }
}

#[inline]
fn parse_method(buf: &[u8]) -> ParseResult<(usize, Method)> {
    if buf.len() < 8 {
        return Ok(Status::Partial);
    }

    let eight: [u8; 8] = buf[..8].try_into().map_err(|_| ParseError::Method)?;
    let eight = u64::from_ne_bytes(eight);

    if eight & 0x0000_0000_00ff_ffff == u64::from_le_bytes([b'G', b'E', b'T', 0, 0, 0, 0, 0]) {
        Ok(Status::Complete((3, Method::Get)))
    } else if eight & 0x0000_0000_00ff_ffff == u64::from_le_bytes([b'P', b'U', b'T', 0, 0, 0, 0, 0])
    {
        Ok(Status::Complete((3, Method::Put)))
    } else if eight & 0x0000_0000_ffff_ffff
        == u64::from_le_bytes([b'P', b'O', b'S', b'T', 0, 0, 0, 0])
    {
        Ok(Status::Complete((4, Method::Post)))
    } else if eight & 0x0000_0000_ffff_ffff
        == u64::from_le_bytes([b'H', b'E', b'A', b'D', 0, 0, 0, 0])
    {
        Ok(Status::Complete((4, Method::Head)))
    } else if eight & 0x0000_00ff_ffff_ffff
        == u64::from_le_bytes([b'T', b'R', b'A', b'C', b'E', 0, 0, 0])
    {
        Ok(Status::Complete((5, Method::Trace)))
    } else if eight & 0x0000_ffff_ffff_ffff
        == u64::from_le_bytes([b'D', b'E', b'L', b'E', b'T', b'E', 0, 0])
    {
        Ok(Status::Complete((6, Method::Delete)))
    } else if eight & 0x00ff_ffff_ffff_ffff
        == u64::from_le_bytes([b'O', b'P', b'T', b'I', b'O', b'N', b'S', 0])
    {
        Ok(Status::Complete((7, Method::Options)))
    } else if eight & 0x00ff_ffff_ffff_ffff
        == u64::from_le_bytes([b'C', b'O', b'N', b'N', b'E', b'C', b'T', 0])
    {
        Ok(Status::Complete((7, Method::Connect)))
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

#[inline]
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
            return Ok(Status::Complete((pos, start..pos)));
        }

        pos += 1;
    }

    Ok(Status::Partial)
}

#[inline]
fn parse_version(buf: &[u8], pos: usize) -> ParseResult<(usize, Version)> {
    if buf[pos..].len() < 8 {
        return Ok(Status::Partial);
    }

    const SIX_BYTE_MASK: u64 = 0x0000_ffff_ffff_ffff;
    let eight: [u8; 8] = buf[pos..pos + 8]
        .try_into()
        .map_err(|_| ParseError::Version)?;
    let eight = u64::from_ne_bytes(eight);

    if eight == u64::from_le_bytes([b'H', b'T', b'T', b'P', b'/', b'1', b'.', b'1']) {
        Ok(Status::Complete((pos + 8, Version::H1_1)))
    } else if eight == u64::from_le_bytes([b'H', b'T', b'T', b'P', b'/', b'1', b'.', b'0']) {
        Ok(Status::Complete((pos + 8, Version::H1_0)))
    } else if eight & SIX_BYTE_MASK
        == u64::from_le_bytes([b'H', b'T', b'T', b'P', b'/', b'2', 0, 0])
    {
        Ok(Status::Complete((pos + 6, Version::H2)))
    } else if eight & SIX_BYTE_MASK
        == u64::from_le_bytes([b'H', b'T', b'T', b'P', b'/', b'3', 0, 0])
    {
        Ok(Status::Complete((pos + 6, Version::H3)))
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
fn validate_header_name_avx2(buf: &[u8], mut pos: usize) -> Result<usize, usize> {
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
fn validate_header_name_ssse3(buf: &[u8], mut pos: usize) -> Result<usize, usize> {
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

#[cfg(all(
    target_feature = "avx2",
    any(target_arch = "x86", target_arch = "x86_64")
))]
#[inline]
fn validate_header_value_avx2(buf: &[u8], mut pos: usize) -> Result<usize, usize> {
    #[cfg(target_arch = "x86")]
    use core::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use core::arch::x86_64::*;

    unsafe {
        let tab = _mm256_set1_epi8(0x09);
        let del = _mm256_set1_epi8(0x7f);
        let low = _mm256_set1_epi8(0x1f);

        while buf[pos..].len() >= 32 {
            let data = _mm256_lddqu_si256(buf[pos..].as_ptr() as *const _);

            let is_tab = _mm256_cmpeq_epi8(data, tab);
            let is_del = _mm256_cmpeq_epi8(data, del);
            let above_low = _mm256_cmpgt_epi8(data, low);
            let above_low_or_tab = _mm256_or_si256(above_low, is_tab);

            let valid = _mm256_andnot_si256(is_del, above_low_or_tab);
            let not_valid = _mm256_cmpeq_epi8(valid, _mm256_setzero_si256());
            let num_valid = (_mm256_movemask_epi8(not_valid) as u32).trailing_zeros();

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
fn validate_header_value_ssse3(buf: &[u8], mut pos: usize) -> Result<usize, usize> {
    #[cfg(target_arch = "x86")]
    use core::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use core::arch::x86_64::*;

    unsafe {
        let tab = _mm_set1_epi8(0x09);
        let del = _mm_set1_epi8(0x7f);
        let low = _mm_set1_epi8(0x1f);

        while buf[pos..].len() >= 16 {
            let data = _mm_lddqu_si128(buf[pos..].as_ptr() as *const _);

            let is_tab = _mm_cmpeq_epi8(data, tab);
            let is_del = _mm_cmpeq_epi8(data, del);
            let above_low = _mm_cmpgt_epi8(data, low);
            let above_low_or_tab = _mm_or_si128(above_low, is_tab);

            let valid = _mm_andnot_si128(is_del, above_low_or_tab);
            let not_valid = _mm_cmpeq_epi8(valid, _mm_setzero_si128());
            let num_valid = (0xffff_0000 | _mm_movemask_epi8(not_valid) as u32).trailing_zeros();

            pos += num_valid as usize;

            if num_valid != 16 {
                return Ok(pos);
            }
        }
    }

    Err(pos)
}

#[inline]
fn get_header_name(buf: &[u8], mut pos: usize) -> ParseResult<(usize, Range<usize>)> {
    let start = pos;

    #[cfg(all(
        target_feature = "avx2",
        any(target_arch = "x86", target_arch = "x86_64")
    ))]
    match validate_header_name_avx2(buf, pos) {
        Ok(n) => return Ok(Status::Complete((n, start..n))),
        Err(n) => pos = n,
    };

    #[cfg(all(
        target_feature = "ssse3",
        any(target_arch = "x86", target_arch = "x86_64")
    ))]
    match validate_header_name_ssse3(buf, pos) {
        Ok(n) => return Ok(Status::Complete((n, start..n))),
        Err(n) => pos = n,
    };

    for &b in &buf[pos..] {
        if !is_header_name_token(b) {
            if start == pos {
                return Err(ParseError::HeaderName);
            }

            return Ok(Status::Complete((pos, start..pos)));
        }

        pos += 1;
    }

    Ok(Status::Partial)
}

#[inline]
fn get_header_value(buf: &[u8], mut pos: usize) -> ParseResult<(usize, Range<usize>)> {
    let start = pos;

    #[cfg(all(
        target_feature = "avx2",
        any(target_arch = "x86", target_arch = "x86_64")
    ))]
    match validate_header_value_avx2(buf, pos) {
        Ok(n) => return Ok(Status::Complete((n, start..n))),
        Err(n) => pos = n,
    };

    #[cfg(all(
        target_feature = "ssse3",
        any(target_arch = "x86", target_arch = "x86_64")
    ))]
    match validate_header_value_ssse3(buf, pos) {
        Ok(n) => return Ok(Status::Complete((n, start..n))),
        Err(n) => pos = n,
    };

    for &b in &buf[pos..] {
        if !is_header_value_token(b) {
            if start == pos {
                return Err(ParseError::HeaderValue);
            }

            return Ok(Status::Complete((pos, start..pos)));
        }

        pos += 1;
    }

    Ok(Status::Partial)
}

#[derive(Debug)]
enum HeaderStatus {
    Complete((usize, usize)),
    Partial(usize),
}

#[inline]
fn parse_headers(
    buf: &[u8],
    pos: usize,
    headers: &mut [MaybeUninit<Header>],
) -> Result<HeaderStatus, ParseError> {
    let mut idx: usize = 0;
    let mut pos = pos;
    loop {
        let name = match get_header_name(buf, pos) {
            Ok(Status::Complete((read, name))) => {
                pos = read;
                name
            }
            Ok(Status::Partial) => return Ok(HeaderStatus::Partial(idx)),
            Err(err) => {
                if buf[pos..].len() >= 2 && buf[pos..pos + 2].cmp(b"\r\n").is_eq() {
                    return Ok(HeaderStatus::Complete((pos, idx)));
                }
                return Err(err);
            }
        };

        if buf[pos] == b':' {
            pos += 1;
        } else {
            return Err(ParseError::HeaderName);
        }

        match discard_whitespace(buf, pos) {
            Some(n) => pos = n,
            None => return Ok(HeaderStatus::Partial(idx)),
        };

        let value = match get_header_value(buf, pos) {
            Ok(Status::Complete((read, value))) => {
                pos = read;
                value
            }
            Ok(Status::Partial) => return Ok(HeaderStatus::Partial(idx)),
            Err(err) => return Err(err),
        };

        headers[idx].write(Header { name, value });
        idx += 1;

        match discard_whitespace(buf, pos) {
            Some(n) => pos = n,
            None => return Ok(HeaderStatus::Partial(idx)),
        };

        match discard_required_newline(buf, pos, ParseError::HeaderValue) {
            Ok(Status::Complete(n)) => pos = n,
            Ok(Status::Partial) => return Ok(HeaderStatus::Partial(idx)),
            Err(err) => return Err(err),
        };
    }
}

#[cfg(test)]
mod test {
    use std::str::from_utf8;

    use crate::parser::{h1::request::Header, Method, Status, Version};

    use super::H1Request;

    const REQ: &[u8] = b"\
GET /api/v1.0/weather/forecast HTTP/1.1\r\n\
Host: www.example.org\r\n\r\n";

    const REQ_MED: &[u8] = b"\
GET /api/v1.0/weather/forecast HTTP/1.1\r\n\
Accept: */*\r\n\
Accept-Encoding: gzip,deflate,br\r\n\
Accept-Language: en-US,en;q=0.5\r\n\
Cache-Control: no-cache\r\n\
Connection: keep-alive\r\n\
DNT: 1\r\n\
Host: www.example.org\r\n\
Pragma: no-cache\r\n\
Referrer: https://www.example.org\r\n\
Sec-Fetch-Dest: empty\r\n\
Sec-Fetch-Mode: cors\r\n\
Sec-Fetch-Site: same-origin\r\n\
User-Agent: Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/109.0\r\n\r\n";

    const REQ_LONG: &[u8] = b"POST /log?format=json&hasfast=true HTTP/1.1\r\n\
Host: play.google.com\r\n\
User-Agent: Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/109.0\r\n\
Accept: */*\r\n\
Accept-Language: en-US,en;q=0.5\r\n\
Accept-Encoding: gzip, deflate, br\r\n\
Referer: https://www.google.com/\r\n\
Content-Type: application/x-www-form-urlencoded;charset=utf-8\r\n\
Content-Length: 669\r\n\
Origin: https://www.google.com\r\n\
DNT: 1\r\n\
Connection: keep-alive\r\n\
Cookie: 1P_JAR=2023-01-24-14; AEC=xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx; NID=xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx; OGPC=xxxxxxxxxxx; ANID=xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx; OGP=xxxxxxxxxx\r\n\
Sec-Fetch-Dest: empty\r\n\
Sec-Fetch-Mode: cors\r\n\
Sec-Fetch-Site: same-site\r\n\
Pragma: no-cache\r\n\
Cache-Control: no-cache\r\n\
TE: trailers\r\n\r\n";

    const REQ_COMP: &[u8] = b"\
GET /wp-content/uploads/2010/03/darth-vader-jedi-battle-lightsaber.jpg HTTP/1.1\r\n\
Host: www.example.org\r\n\
User-Agent: Mozilla/5.0 (Macintosh; U; Intel Mac OS X 10.6; ja-JP-mac; rv:1.9.2.3) Gecko/20100401 Firefox/3.6.3 Pathtraq/0.9\r\n\
Accept: text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8\r\n\
Accept-Language: ja,en-us;q=0.7,en;q=0.3\r\n\
Accept-Encoding: gzip,deflate\r\n\
Accept-Charset: Shift_JIS,utf-8;q=0.7,*;q=0.7\r\n\
Keep-Alive: 115\r\n\
Connection: keep-alive\r\n\
Cookie: wp_ozh_wsa_visits=2; wp_ozh_wsa_visit_lasttime=xxxxxxxxxx; __utma=xxxxxxxxx.xxxxxxxxxx.xxxxxxxxxx.xxxxxxxxxx.xxxxxxxxxx.x; __utmz=xxxxxxxxx.xxxxxxxxxx.x.x.utmccn=(referral)|utmcsr=reader.livedoor.com|utmcct=/reader/|utmcmd=referral|padding=under256\r\n\r\n";

    #[test]
    pub fn test_req() {
        let mut req = H1Request::new();
        let mut buf = REQ;
        assert_eq!(REQ.len(), req.fill(&mut buf).unwrap());
        assert_eq!(Ok(Status::Complete(REQ.len())), req.parse());
        assert_eq!(Some(Method::Get), req.method);
        assert_eq!(&REQ[4..30], b"/api/v1.0/weather/forecast");
        assert_eq!(Some(4..30), req.target);
        assert_eq!(Some(Version::H1_1), req.version);
        assert!(req.headers.is_some());
        assert_eq!(&REQ[41..45], b"Host");
        assert_eq!(&REQ[47..62], b"www.example.org");
        assert_eq!(
            Header {
                name: 41..45,
                value: 47..62
            },
            req.headers.unwrap()[0]
        );
    }

    #[test]
    pub fn test_req_med() {
        let mut req = H1Request::new();
        let mut buf = REQ_MED;
        assert_eq!(REQ_MED.len(), req.fill(&mut buf).unwrap());
        assert_eq!(Ok(Status::Complete(REQ_MED.len())), req.parse());
        assert_eq!(Some(Method::Get), req.method);
        assert_eq!(&REQ[4..30], b"/api/v1.0/weather/forecast");
        assert_eq!(Some(4..30), req.target);
        assert_eq!(Some(Version::H1_1), req.version);
        assert!(req.headers.is_some());
        println!("{}", req);
        println!("{:?}", req.headers.unwrap()[0]);
        assert_eq!(
            Header {
                name: 41..47,
                value: 49..52
            },
            req.headers.unwrap()[0]
        );
        assert_eq!(&REQ_MED[41..47], b"Accept");
        assert_eq!(&REQ_MED[49..52], b"*/*");
    }

    #[test]
    pub fn test_req_long() {
        let mut req = H1Request::new();
        let mut buf = REQ_LONG;
        assert_eq!(REQ_LONG.len(), req.fill(&mut buf).unwrap());
        assert_eq!(Ok(Status::Complete(REQ_LONG.len())), req.parse());
        assert_eq!(format!("{}", req), from_utf8(REQ_LONG).unwrap());
    }

    #[test]
    pub fn test_req_comp() {
        let mut req = H1Request::new();
        let mut buf = REQ_COMP;
        assert_eq!(REQ_COMP.len(), req.fill(&mut buf).unwrap());
        assert_eq!(Ok(Status::Complete(REQ_COMP.len())), req.parse());
        assert_eq!(format!("{}", req), from_utf8(REQ_COMP).unwrap());
    }

    #[test]
    pub fn test_multiple() {
        let inputs = [REQ, REQ_MED, REQ_COMP, REQ_LONG];

        for &input in inputs.iter() {
            let mut i = Vec::default();
            i.extend_from_slice(input);

            let mut i: &[u8] = &i;
            let mut req = H1Request::new();
            req.fill(&mut i).unwrap();
            req.parse().unwrap();
            println!("{}\n{}", from_utf8(input).unwrap(), req);
            assert_eq!(from_utf8(input).unwrap(), format!("{}", req));
        }
    }
}
