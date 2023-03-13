//! Response model

use std::io::Write;

use crate::{
    buffer::Buffer,
    parser::{status::Status, Version},
};

use super::request::Header;

/// Response model
#[derive(Debug)]
pub struct Response {
    version: Version,
    status: Status,
    headers: Option<Vec<Header<'static>>>,
    body: String,
}

impl Response {
    /// TODO
    pub fn new_with_status_line(version: Version, status: Status) -> Self {
        Response {
            version,
            status,
            headers: None,
            body: String::new(),
        }
    }

    /// TODO
    pub fn write_to_buf(&self, buf: &mut Buffer) -> std::io::Result<usize> {
        let pos = buf.write_pos();
        write!(buf, "{} {}\r\n\r\n", self.version, self.status)?;
        Ok(buf.write_pos() - pos)
    }
}
