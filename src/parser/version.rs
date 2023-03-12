//! Representation of the requested HTTP version

use std::fmt::Display;

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
