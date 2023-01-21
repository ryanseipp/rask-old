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

use core::str::from_utf8_unchecked;

use super::tokens::is_request_target_token;
use super::{take_after_newline, Header, ParseError};
use crate::parser::raw_request::RawRequest;
use crate::parser::{HttpMethod, HttpVersion};

// TODO: I don't think we can hold onto &str, as we may receive requests over multiple TCP packets.
// This would require such a mashup of lifetimes that would be impossible to reason about. How do
// we avoid the need to allocate a ton of strings? Would cost two heap allocations per header...
// Can we potentially just keep a buffer for the entire request received over multiple packets, and
// indexes into the important parts, deferring parsing until it's actually needed/used? Would mean
// one heap allocation per packet rather than tons
/// Parsed H1 Request
/// IETF RFC 9112
#[derive(Debug, Default)]
pub struct H1Request<'buf, 'headers> {
    /// method
    pub method: Option<HttpMethod>,
    /// target
    pub target: Option<&'buf str>,
    /// version
    pub version: Option<HttpVersion>,
    /// headers
    pub headers: Option<&'headers mut [Header<'buf>]>,
}

impl<'b, 'h> H1Request<'b, 'h> {
    /// Creates a new HTTP/1.1 request
    pub fn new() -> Self {
        H1Request {
            method: None,
            target: None,
            version: None,
            headers: None,
        }
    }

    /// Parses a request
    ///
    /// # Example
    /// ```
    /// # use rask::parser::{HttpMethod, HttpVersion};
    /// # use rask::parser::h1::ParseError;
    /// # use rask::parser::h1::request::H1Request;
    /// # fn main() -> Result<(), ParseError> {
    /// let mut req = H1Request::new();
    /// req.parse(b"GET / HTTP/1.1\r\n\r\n")?;
    /// assert_eq!(Some(HttpMethod::Get), req.method);
    /// assert_eq!(Some("/"), req.target);
    /// assert_eq!(Some(HttpVersion::H1_1), req.version);
    /// # Ok(())
    /// # }
    /// ```
    pub fn parse(&mut self, buf: &'b [u8]) -> Result<(), ParseError> {
        let mut req = RawRequest::new(buf);
        self.set_method(&mut req)?;
        // pos = get_non_whitespace_pos(buf, pos).unwrap_or(pos);
        self.set_target(&mut req)?;
        // pos = get_non_whitespace_pos(buf, pos).unwrap_or(pos);
        self.set_version(&mut req)?;
        take_after_newline(&mut req)?;
        self.set_headers(&mut req)?;

        Ok(())
    }

    // TODO: This may have way too many branches. Control flow looks insane https://godbolt.org/z/jhx8Ga4d3
    fn set_method(&mut self, buf: &mut RawRequest<'b>) -> Result<(), ParseError> {
        if buf.any(|&b| !b.is_ascii_uppercase()) {
            if let Ok(slice) = buf.slice_skip(1) {
                let res = match slice {
                    b"GET" => Ok(HttpMethod::Get),
                    b"HEAD" => Ok(HttpMethod::Head),
                    b"POST" => Ok(HttpMethod::Post),
                    b"PUT" => Ok(HttpMethod::Put),
                    b"DELETE" => Ok(HttpMethod::Delete),
                    b"CONNECT" => Ok(HttpMethod::Connect),
                    b"OPTIONS" => Ok(HttpMethod::Options),
                    b"TRACE" => Ok(HttpMethod::Trace),
                    _ => Err(ParseError::Method),
                }?;

                self.method = Some(res);
                return Ok(());
            }
        }

        Err(ParseError::Method)
    }

    fn set_target(&mut self, buf: &mut RawRequest<'b>) -> Result<(), ParseError> {
        for &b in &mut *buf {
            if b == b' ' {
                if let Ok(slice) = buf.slice_skip(1) {
                    // SAFETY: slice has been checked for valid ASCII in this range, which makes this valid utf8
                    self.target = Some(unsafe { from_utf8_unchecked(slice) });
                    return Ok(());
                }

                break;
            } else if !is_request_target_token(b) {
                break;
            }
        }

        Err(ParseError::Target)
    }

    fn set_version(&mut self, buf: &mut RawRequest<'b>) -> Result<(), ParseError> {
        let result = if !buf.take(5).eq(b"HTTP/".iter()) {
            Err(ParseError::Version)
        } else {
            match buf.next() {
                Some(b'1') => {
                    if buf.next() == Some(&b'.') {
                        match buf.next() {
                            Some(b'0') => Ok(HttpVersion::H1_0),
                            Some(b'1') => Ok(HttpVersion::H1_1),
                            _ => Err(ParseError::Version),
                        }
                    } else {
                        Err(ParseError::Version)
                    }
                }
                Some(b'2') => Ok(HttpVersion::H2),
                Some(b'3') => Ok(HttpVersion::H3),
                _ => Err(ParseError::Version),
            }
        };

        buf.slice();

        match result {
            Ok(version) => {
                self.version = Some(version);
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    fn set_headers(&mut self, _buf: &mut RawRequest<'b>) -> Result<(), ParseError> {
        Ok(())
    }
}
