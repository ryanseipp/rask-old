//! Representation of HTTP method
use std::fmt::Display;

use super::ParseError;

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

impl TryFrom<&[u8]> for Method {
    type Error = ParseError;

    #[inline]
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        match value {
            b"GET" => Ok(Self::Get),
            b"HEAD" => Ok(Self::Head),
            b"POST" => Ok(Self::Post),
            b"PUT" => Ok(Self::Put),
            b"DELETE" => Ok(Self::Delete),
            b"CONNECT" => Ok(Self::Connect),
            b"OPTIONS" => Ok(Self::Options),
            b"TRACE" => Ok(Self::Trace),
            _ => Err(Self::Error::Method),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn http_method_try_from_get() {
        assert_eq!(Ok(Method::Get), Method::try_from(b"GET" as &[u8]))
    }

    #[test]
    fn http_method_try_from_put() {
        assert_eq!(Ok(Method::Put), Method::try_from(b"PUT" as &[u8]))
    }

    #[test]
    fn http_method_try_from_post() {
        assert_eq!(Ok(Method::Post), Method::try_from(b"POST" as &[u8]))
    }

    #[test]
    fn http_method_try_from_delete() {
        assert_eq!(Ok(Method::Delete), Method::try_from(b"DELETE" as &[u8]))
    }

    #[test]
    fn http_method_try_from_options() {
        assert_eq!(Ok(Method::Options), Method::try_from(b"OPTIONS" as &[u8]))
    }

    #[test]
    fn http_method_try_from_head() {
        assert_eq!(Ok(Method::Head), Method::try_from(b"HEAD" as &[u8]))
    }

    #[test]
    fn http_method_try_from_connect() {
        assert_eq!(Ok(Method::Connect), Method::try_from(b"CONNECT" as &[u8]))
    }

    #[test]
    fn http_method_try_from_trace() {
        assert_eq!(Ok(Method::Trace), Method::try_from(b"TRACE" as &[u8]))
    }

    #[test]
    fn http_method_try_from_fails_if_too_long() {
        assert!(Method::try_from(b"GET " as &[u8]).is_err());
    }
}
