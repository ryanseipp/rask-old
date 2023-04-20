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

use super::{ParseError, ParseResult, Status};

pub mod request;
pub mod response;
pub mod tokens;

/// Consumes whitespace characters from `buf`.
/// Whitespace is defined by RFC 9110 Secion 5.6.3 by ABNF
/// ```abnf
/// OWS = *( SP / HTAB )
/// ```
#[inline]
pub fn discard_whitespace(buf: &[u8], pos: usize) -> Option<usize> {
    let mut pos = pos;
    for &byte in &buf[pos..] {
        if byte != b' ' && byte != b'\t' {
            return Some(pos);
        }

        pos += 1;
    }

    None
}

/// Consumes whitespace characters from `buf`. Requires that at least one whitespace character is
/// encountered.
/// Whitespace is defined by RFC 9110 Secion 5.6.3 by ABNF
/// ```abnf
/// RWS = 1*( SP / HTAB )
/// ```
#[inline]
pub fn discard_required_whitespace(
    buf: &[u8],
    pos: usize,
    err_type: ParseError,
) -> ParseResult<usize> {
    let mut pos = pos;
    if buf[pos] != b' ' && buf[pos] != b'\t' {
        return Err(err_type);
    }

    pos += 1;

    for &byte in &buf[pos..] {
        if byte != b' ' && byte != b'\t' {
            return Ok(Status::Complete(pos));
        }

        pos += 1;
    }

    Ok(Status::Partial)
}

/// Verifies the placement of a required newline sequence of bytes.
/// Returns the position after the newline sequence.
/// Takes a ParseError to be returned should the newline sequence not be found.
///
/// ```rust
/// # use rask::parser::{Status, ParseError};
/// # use rask::parser::h1::discard_required_newline;
/// let buf: &[u8] = b"Hello\r\nWorld!";
/// assert_eq!(Ok(Status::Complete(7)), discard_required_newline(buf, 5, ParseError::NewLine))
/// ```
#[inline]
pub fn discard_required_newline(
    buf: &[u8],
    pos: usize,
    err_type: ParseError,
) -> ParseResult<usize> {
    if buf.len() - pos < 2 {
        return Ok(Status::Partial);
    }

    if &buf[pos..pos + 2] != b"\r\n" {
        return Err(err_type);
    }

    Ok(Status::Complete(pos + 2))
}
