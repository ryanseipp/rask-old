use std::{
    io::{Read, Write},
    net::TcpStream,
};

fn main() -> std::io::Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:8080")?;
    let _ = stream.write(b"GET /api/v1.0/weather/forecast HTTP/1.1\r\nAccept:*/*\r\nAccept-Encoding:gzip,deflate,br\r\nAccept-Language:en-US,en;q=0.5\r\nCache-Control:no-cache\r\nConnection:keep-alive\r\nDNT:1\r\nHost: www.example.org\r\nPragma:no-cache\r\nReferrer:https://www.example.org\r\nSec-Fetch-Dest:empty\r\nSec-Fetch-Mode:cors\r\nSec-Fetch-Site:same-origin\r\nUser-Agent:Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/109.0\r\n\r\n")?;

    stream.flush()?;
    let _ = stream.read(&mut [0; 128])?;

    Ok(())
}
