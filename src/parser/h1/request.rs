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
use std::str::from_utf8_unchecked;

use super::tokens::{is_header_name_token, is_header_value_token, is_request_target_token};
use super::{discard_newline, discard_whitespace, ParseError, ParseResult};
use crate::parser::raw_request::RawRequest;
use crate::parser::{HttpMethod, HttpVersion};

/// TODO
#[derive(Debug, PartialEq, Eq)]
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
    pub method: Option<HttpMethod>,
    /// target
    pub target: Option<&'buf str>,
    /// version
    pub version: Option<HttpVersion>,
    /// headers
    pub headers: Option<Vec<Header<'buf>>>,
}

impl<'b> H1Request<'b> {
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
    /// # use rask::parser::h1::request::{H1Request, Header};
    /// # fn main() -> Result<(), ParseError> {
    /// let mut req = H1Request::new();
    /// req.parse(b"GET / HTTP/1.1\r\nHost:http://www.example.org\r\n\r\n")?;
    /// assert_eq!(Some(HttpMethod::Get), req.method);
    /// assert_eq!(Some("/"), req.target);
    /// assert_eq!(Some(HttpVersion::H1_1), req.version);
    /// assert!(req.headers.is_some());
    /// assert_eq!(Header {name: "Host", value: "http://www.example.org"}, req.headers.unwrap()[0]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn parse(&mut self, buf: &'b [u8]) -> ParseResult<()> {
        let mut req = RawRequest::new(buf);
        self.set_method(&mut req)?;
        // pos = get_non_whitespace_pos(buf, pos).unwrap_or(pos);
        self.set_target(&mut req)?;
        // pos = get_non_whitespace_pos(buf, pos).unwrap_or(pos);
        self.set_version(&mut req)?;
        discard_newline(&mut req);
        self.set_headers(&mut req)?;

        Ok(())
    }

    // TODO: This may have way too many branches. Control flow looks insane https://godbolt.org/z/jhx8Ga4d3
    #[inline]
    fn set_method(&mut self, buf: &mut RawRequest<'_>) -> ParseResult<()> {
        if let Some(slice) = buf.take_until(|b| !b.is_ascii_uppercase()) {
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

            discard_whitespace(buf);
            self.method = Some(res);
            return Ok(());
        }

        Err(ParseError::Method)
    }

    #[inline]
    fn set_target(&mut self, buf: &mut RawRequest<'b>) -> ParseResult<()> {
        if let Some(target) = buf.take_until(|b| !is_request_target_token(b)) {
            if buf.peek() == Some(b' ') || buf.peek() == Some(b'\t') {
                discard_whitespace(buf);

                // SAFETY: slice has been checked for valid ASCII in this range, which makes this valid utf8
                self.target = unsafe { Some(from_utf8_unchecked(target)) };
                return Ok(());
            }
        }

        Err(ParseError::Target)
    }

    #[inline]
    fn set_version(&mut self, buf: &mut RawRequest<'_>) -> ParseResult<()> {
        if let Some(version) = buf.take_until(|b| b.is_ascii_whitespace()) {
            let res = match version {
                b"HTTP/1.0" => Ok(HttpVersion::H1_0),
                b"HTTP/1.1" => Ok(HttpVersion::H1_1),
                b"HTTP/2" => Ok(HttpVersion::H2),
                b"HTTP/3" => Ok(HttpVersion::H3),
                _ => Err(ParseError::Version),
            }?;

            discard_whitespace(buf);
            self.version = Some(res);
            Ok(())
        } else {
            Err(ParseError::Version)
        }
    }

    #[inline]
    fn set_headers(&mut self, buf: &mut RawRequest<'b>) -> ParseResult<()> {
        loop {
            if let Some(name) = buf.take_until(|b| !is_header_name_token(b)) {
                match buf.next() {
                    Some(&b) if b != b':' => {
                        return Err(ParseError::HeaderName);
                    }
                    Some(_) => {}
                    None => return Err(ParseError::HeaderName),
                }

                discard_whitespace(buf);

                if let Some(value) = buf.take_until(|b| !is_header_value_token(b)) {
                    let headers = self.headers.get_or_insert(Vec::default());
                    // SAFETY: slices have been checked for valid ASCII, which makes this valid
                    // UTF8
                    let (name, value) =
                        unsafe { (from_utf8_unchecked(name), from_utf8_unchecked(value)) };

                    headers.push(Header { name, value });

                    discard_newline(buf);
                } else {
                    return Err(ParseError::HeaderValue);
                }
            } else if buf.next() == Some(&b'\r') && buf.next() == Some(&b'\n') {
                buf.slice();
                return Ok(());
            } else {
                return Err(ParseError::HeaderName);
            }
        }
    }
}

impl Display for H1Request<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{} {} {}\n",
            self.method.as_ref().unwrap_or(&HttpMethod::Get),
            self.target.unwrap_or(""),
            self.version.as_ref().unwrap_or(&HttpVersion::H1_0)
        ))?;

        for header in self.headers.as_ref().unwrap_or(&Vec::default()).iter() {
            f.write_fmt(format_args!("{}: {}\n", header.name, header.value))?;
        }

        Ok(())
    }
}
