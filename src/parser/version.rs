//! Representation of the requested HTTP version

use std::fmt::Display;

use super::ParseError;

/// Representation of the requested HTTP version
#[derive(Debug, PartialEq, Eq)]
pub enum Version {
    /// HTTP/1.0
    H1_0,
    /// HTTP/1.1
    H1_1,
    /// HTTP/2
    H2,
    /// HTTP/3
    H3,
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::H1_0 => "HTTP/1.0",
            Self::H1_1 => "HTTP/1.1",
            Self::H2 => "HTTP/2",
            Self::H3 => "HTTP/3",
        })
    }
}

impl TryFrom<&[u8]> for Version {
    type Error = ParseError;

    #[inline]
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        match value {
            b"HTTP/1.0" => Ok(Self::H1_0),
            b"HTTP/1.1" => Ok(Self::H1_1),
            b"HTTP/2" => Ok(Self::H2),
            b"HTTP/3" => Ok(Self::H3),
            _ => Err(Self::Error::Version),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn http_method_try_from_1_0() {
        assert_eq!(Ok(Version::H1_0), Version::try_from(b"HTTP/1.0" as &[u8]));
    }

    #[test]
    fn http_method_try_from_1_1() {
        assert_eq!(Ok(Version::H1_1), Version::try_from(b"HTTP/1.1" as &[u8]));
    }

    #[test]
    fn http_method_try_from_2() {
        assert_eq!(Ok(Version::H2), Version::try_from(b"HTTP/2" as &[u8]));
    }

    #[test]
    fn http_method_try_from_3() {
        assert_eq!(Ok(Version::H3), Version::try_from(b"HTTP/3" as &[u8]));
    }

    #[test]
    fn http_method_try_from_fails_if_too_long() {
        assert!(Version::try_from(b"HTTP/2 " as &[u8]).is_err());
    }

    #[test]
    fn http_method_try_from_fails_if_gibberish() {
        assert!(Version::try_from(b"ABCD" as &[u8]).is_err())
    }
}
