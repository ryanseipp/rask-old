//! Response model

use std::io::Write;

use crate::{
    first::buffer::Buffer,
    parser::{status::Status, Version},
};

use super::request::Header;

/// Response model
#[derive(Debug)]
pub struct Response {
    version: Version,
    status: Status,
    headers: Option<Vec<Header>>,
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
    pub fn get_serialized(&self) -> &str {
        "HTTP/1.1 204\r\nServer: rask/0.0.1\r\nConnection: keep-alive\r\n\r\n"
    }

    /// TODO
    pub fn write_to_buf(&self, buf: &mut Buffer) -> std::io::Result<usize> {
        let pos = buf.write_pos();
        write!(
            buf,
            "HTTP/1.1 204\r\nServer: rask/0.0.1\r\nConnection: keep-alive\r\n\r\n",
        )?;
        Ok(buf.write_pos() - pos)
    }
}
