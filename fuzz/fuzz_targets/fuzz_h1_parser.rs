#![no_main]

use libfuzzer_sys::fuzz_target;
use rask::parser::h1::request::H1Request;

fuzz_target!(|data: &[u8]| {
    let mut request = H1Request::new();
    let _ = request.parse(data);
});
