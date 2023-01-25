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

//! H1 parser implementation

use core::fmt::Display;

use super::raw_request::RawRequest;

pub mod request;
mod tokens;

/// Represents possible failures while parsing
#[derive(Debug)]
pub enum ParseError {
    /// Invalid byte in method.
    Method,
    /// Invalid byte in target.
    Target,
    /// Invalid HTTP version.
    Version,
    /// Invalid byte in header name.
    HeaderName,
    /// Invalid byte in header value.
    HeaderValue,
    /// Invalid or missing new line.
    NewLine,
    /// Invalid whitespace
    Whitespace,
}

impl ParseError {
    fn description_str(&self) -> &'static str {
        match *self {
            ParseError::Method => "Invalid token in method",
            ParseError::Target => "Invalid token in target",
            ParseError::Version => "Invalid version",
            ParseError::HeaderName => "Invalid token in header name",
            ParseError::HeaderValue => "Invalid token in header value",
            ParseError::NewLine => "Invalid or missing new line",
            ParseError::Whitespace => "Invalid whitespace",
        }
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.description_str())
    }
}

impl std::error::Error for ParseError {}

/// Result whose Err variant is `ParseError`
pub type ParseResult<T> = std::result::Result<T, ParseError>;

/// Consumes whitespace characters from `buf`.
/// Whitespace is defined by RFC 9110 Secion 5.6.3 by ABNF
/// ```abnf
/// OWS = *( SP / HTAB )
/// ```
#[inline]
pub fn discard_whitespace(buf: &mut RawRequest<'_>) {
    buf.take_until(|b| b != b' ' && b != b'\t');
}

/// TODO
#[inline]
pub fn skip_whitespace(buf: &mut RawRequest<'_>) {
    buf.find(|&&b| b != b' ' && b != b'\t');
}

/// Consumes whitespace characters from `buf`. Requires that at least one whitespace character is
/// encountered.
/// Whitespace is defined by RFC 9110 Secion 5.6.3 by ABNF
/// ```abnf
/// RWS = 1*( SP / HTAB )
/// ```
#[inline]
pub fn discard_required_whitespace(buf: &mut RawRequest<'_>) -> ParseResult<()> {
    let pos = buf.pos();

    buf.take_until(|b| b != b' ' && b != b'\t');
    if pos == buf.pos() {
        return Err(ParseError::Whitespace);
    }

    Ok(())
}

/// Consumes `buf` to the end of a new-line character sequence `b"\r\n"`
#[inline]
pub fn discard_newline(buf: &mut RawRequest<'_>) {
    loop {
        buf.take_until(|b| b == b'\r');
        buf.next();
        if buf.next() == Some(&b'\n') {
            buf.slice();
            return;
        }
    }
}

/// TODO
#[inline]
pub fn skip_newline(buf: &mut RawRequest<'_>) {
    loop {
        buf.find(|&&b| b == b'\r');
        if buf.next() == Some(&b'\n') {
            println!("{}", buf.pos());
            return;
        }
    }
}
