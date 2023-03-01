use std::str::from_utf8_unchecked;

use iai::black_box;
use rask::parser::{
    h1::{
        discard_newline, discard_whitespace,
        request::{H1Request, Header},
        tokens::{is_header_name_token, is_header_value_token, is_request_target_token},
    },
    raw_request::RawRequest,
    Method, ParseError, ParseResult, Version,
};

const REQ: &[u8] = b"POST /log?format=json&hasfast=true HTTP/3\r\n\
Host: play.google.com\r\n\
User-Agent: Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/109.0\r\n\
Accept: */*\r\n\
Accept-Language: en-US,en;q=0.5\r\n\
Accept-Encoding: gzip, deflate, br\r\n\
Referer: https://www.google.com/\r\n\
Content-Type: application/x-www-form-urlencoded;charset=utf-8\r\n\
Content-Length: 669\r\n\
Origin: https://www.google.com\r\n\
DNT: 1\r\n\
Connection: keep-alive\r\n\
Cookie: 1P_JAR=2023-01-24-14; AEC=xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx; NID=xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx; OGPC=xxxxxxxxxxx; ANID=xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx; OGP=xxxxxxxxxx\r\n\
Sec-Fetch-Dest: empty\r\n\
Sec-Fetch-Mode: cors\r\n\
Sec-Fetch-Site: same-site\r\n\
Pragma: no-cache\r\n\
Cache-Control: no-cache\r\n\
TE: trailers\r\n\r\n";

fn parser() {
    let mut req = H1Request::new();
    let _ = req.parse(black_box(REQ));
}

fn method() {
    let mut buf = RawRequest::new(black_box(REQ));
    let _ = parse_method(&mut buf);
}

fn target() {
    let mut buf = RawRequest::new(black_box(&REQ[5..]));
    let _ = parse_target(&mut buf);
}

fn version() {
    let mut buf = RawRequest::new(black_box(&REQ[35..]));
    let _ = parse_version(&mut buf);
}

fn headers() {
    let mut buf = RawRequest::new(black_box(&REQ[45..]));
    let _ = parse_headers(&mut buf);
}

#[inline]
fn parse_method(buf: &mut RawRequest<'_>) -> ParseResult<Method> {
    loop {
        let b = buf.next().ok_or(ParseError::Method)?;
        if *b == b' ' {
            return Method::try_from(buf.slice_skip(1).map_err(|_| ParseError::Method)?);
        } else if !b.is_ascii_uppercase() {
            return Err(ParseError::Method);
        }
    }
}

#[inline]
fn parse_target<'b>(buf: &mut RawRequest<'b>) -> ParseResult<&'b str> {
    loop {
        let b = buf.next().ok_or(ParseError::Target)?;
        if !is_request_target_token(*b) {
            return Ok(unsafe {
                from_utf8_unchecked(buf.slice_skip(1).map_err(|_| ParseError::Target)?)
            });
        }
    }
    // let target = buf
    //     .take_until(|b| !is_request_target_token(b))
    //     .ok_or(ParseError::Target)?;
    //
    // discard_whitespace(buf);
    //
    // // SAFETY: slice has been checked for valid ASCII in this range, which makes this valid utf8
    // self.target = unsafe { Some(from_utf8_unchecked(target)) };
    // Ok(())
}

#[inline]
fn parse_version(buf: &mut RawRequest<'_>) -> ParseResult<Version> {
    let slice = buf
        .take_until(|b| b.is_ascii_whitespace())
        .ok_or(ParseError::Version)?;
    let version = Version::try_from(slice)?;

    discard_whitespace(buf);
    Ok(version)
}

#[inline]
fn parse_headers<'b>(buf: &mut RawRequest<'b>) -> ParseResult<Vec<Header<'b>>> {
    let mut headers = Vec::default();
    loop {
        if let Some(name) = buf.take_until(|b| !is_header_name_token(b)) {
            if buf.next() != Some(&b':') {
                return Err(ParseError::HeaderName);
            }

            discard_whitespace(buf);

            let value = buf
                .take_until(|b| !is_header_value_token(b))
                .ok_or(ParseError::HeaderValue)?;

            let (name, value) = unsafe { (from_utf8_unchecked(name), from_utf8_unchecked(value)) };

            headers.push(Header { name, value });

            discard_newline(buf);
        } else if buf.next() == Some(&b'\r') && buf.next() == Some(&b'\n') {
            buf.slice();
            return Ok(headers);
        } else {
            return Err(ParseError::HeaderName);
        }
    }
}

iai::main!(parser, method, target, version, headers);
