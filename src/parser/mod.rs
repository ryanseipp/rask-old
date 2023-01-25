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
mod raw_request;

// TODO: What goes here?
/// Parser trait
pub trait Parser {}

/// Representation of the requested HTTP Method
/// [IETF RFC 9110 Section 9](https://www.rfc-editor.org/rfc/rfc9110#section-9)
#[derive(Debug, PartialEq, Eq)]
pub enum HttpMethod {
    /// RFC 9110 9.3.1
    Get,
    /// RFC 9110 9.3.2
    Head,
    /// RFC 9110 9.3.3
    Post,
    /// RFC 9110 9.3.4
    Put,
    /// RFC 9110 9.3.5
    Delete,
    /// RFC 9110 9.3.6
    Connect,
    /// RFC 9110 9.3.7
    Options,
    /// RFC 9110 9.3.8
    Trace,
}

impl Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Get => "GET",
            Self::Head => "HEAD",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
            Self::Connect => "CONNECT",
            Self::Options => "OPTIONS",
            Self::Trace => "TRACE",
        })
    }
}

/// Representation of the requested HTTP version
#[derive(Debug, PartialEq, Eq)]
pub enum HttpVersion {
    /// HTTP/1.0
    H1_0,
    /// HTTP/1.1
    H1_1,
    /// HTTP/2
    H2,
    /// HTTP/3
    H3,
}

impl Display for HttpVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::H1_0 => "HTTP/1.0",
            Self::H1_1 => "HTTP/1.1",
            Self::H2 => "HTTP/2",
            Self::H3 => "HTTP/3",
        })
    }
}
