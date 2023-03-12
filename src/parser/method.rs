//! Representation of HTTP method
use std::fmt::Display;

/// Representation of the requested HTTP Method
/// [IETF RFC 9110 Section 9](https://www.rfc-editor.org/rfc/rfc9110#section-9)
#[derive(Debug, PartialEq, Eq)]
pub enum Method {
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

impl Display for Method {
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
