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

//! Parser implementations for HTTP

use std::fmt::Display;

pub mod h1;
pub mod method;
pub mod raw_request;
pub mod status;
pub mod version;

pub use method::Method;
pub use version::Version;

// TODO: What goes here?
/// Parser trait
pub trait Parser {}

/// Represents possible failures while parsing
#[derive(Debug, PartialEq, Eq)]
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

/// Status of a parse operation. Determines if the operation completed, or reached the end of the
/// buffer.
#[derive(Debug, PartialEq, Eq)]
pub enum Status<T> {
    /// The parse operation completed
    Complete(T),
    /// The parse operation encountered the end of the buffer
    Partial,
}

/// Result whose Err variant is `ParseError`
pub type ParseResult<T> = std::result::Result<Status<T>, ParseError>;
