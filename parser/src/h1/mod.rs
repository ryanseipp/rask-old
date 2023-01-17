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

use crate::raw_request::RawRequest;

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
        }
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.description_str())
    }
}

impl std::error::Error for ParseError {}

/// Finds offset of next non-whitespace character.
///
/// In RFC 9112 Section 3, this is defined as any SP, HTAB, VT, FF, or CR without LF.
pub fn get_non_whitespace_pos(buf: &[u8], start: usize) -> Option<usize> {
    let mut buf_iter = buf.iter().skip(start).peekable();
    let mut pos = start;

    loop {
        if let Some(&b) = buf_iter.next() {
            pos += 1;

            if b == b'\r' {
                // b"\r\n" is considered newline, and not whitespace
                if buf_iter.peek() == Some(&&b'\n') {
                    return Some(pos - 1);
                }
            }

            if b != b' ' && b != b'\t' && !(b >= 0x11 && b <= b'\r') {
                return Some(pos);
            }
        } else {
            return None;
        }
    }
}

/// Consumes `buf` to the end of a new-line character sequence `b"\r\n"`
pub fn take_after_newline(buf: &mut RawRequest<'_>) -> Result<(), ParseError> {
    loop {
        match buf.next() {
            Some(&b) => {
                if b == b'\r' && buf.peek() == Some(b'\n') {
                    // trim the buffer, effectively discarding everything we iterated over
                    buf.slice();
                    return Ok(());
                }
            }
            None => return Err(ParseError::NewLine),
        }
    }
}

/// TODO!
#[derive(Debug)]
pub struct Header<'a> {
    name: &'a str,
    value: &'a [u8],
}
