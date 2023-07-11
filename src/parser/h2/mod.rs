//! H2 Parser

/// HTTP/2 Frame
#[derive(Debug)]
pub struct Frame {
    /// 24 bits only - default max is 2^14
    length: u32,
    // should swap this for enum
    frame_type: u8,
    // should swap this for enum
    flags: u8,
    // 31 bits only (should this be i32 instead with only positive values allowed?)
    stream_id: u32,
}

// pub fn parse_frame(req: &[u8]) -> Frame {}

struct Stream {}
