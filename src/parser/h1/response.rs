//! Response model

use crate::parser::{status::Status, Version};

use super::request::Header;

/// Response model
#[derive(Debug)]
pub struct Response {
    version: Version,
    status: Status,
    headers: Option<Vec<Header<'static>>>,
    body: String,
}
