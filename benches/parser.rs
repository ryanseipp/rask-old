use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rask::parser::h1::request::H1Request;

fn benchmark(c: &mut Criterion) {
    let input: &[u8] = b"GET /api/v1.0/weather/forecast HTTP/1.1\r\nHost: www.example.org\r\n\r\n";
    let input_long: &[u8] = b"GET /api/v1.0/weather/forecast HTTP/1.1\r\nAccept:*/*\r\nAccept-Encoding:gzip,deflate,br\r\nAccept-Language:en-US,en;q=0.5\r\nCache-Control:no-cache\r\nConnection:keep-alive\r\nDNT:1\r\nHost: www.example.org\r\nPragma:no-cache\r\nReferrer:https://www.example.org\r\nSec-Fetch-Dest:empty\r\nSec-Fetch-Mode:cors\r\nSec-Fetch-Site:same-origin\r\nUser-Agent:Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/109.0\r\n";

    let input_longer: &[u8] = b"POST /log?format=json&hasfast=true HTTP/3\r\nHost: play.google.com\r\nUser-Agent: Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/109.0\r\nAccept: */*\r\nAccept-Language: en-US,en;q=0.5\r\nAccept-Encoding: gzip, deflate, br\r\nReferer: https://www.google.com/\r\nContent-Type: application/x-www-form-urlencoded;charset=utf-8\r\nContent-Length: 669\r\nOrigin: https://www.google.com\r\nDNT: 1\r\nConnection: keep-alive\r\nCookie: 1P_JAR=2023-01-24-14; AEC=ARSKqsJBkGg7byaK-h9Pg8UFqNa_tQWYQoxoYyziDbv4vMk5090aYJFoxSc; NID=511=S4cpsYC3bZaeO8vaqNVlIGUx5wBnFk4p_492D3aw8mqT-x5VVn7d3W_BypUHVO83MBi9c_9DaG2Oj3zkgKJ7fYhgGOZ5wT5PLhaZVZhcshiEK1W0EENABLpYPeE-ts09STmkHKlhACGMxYwYXHeVHxfMKRBjS5lABOvDDTggjyg; OGPC=19027681-1:; ANID=AHWqTUnwDcnsbvsZ3b7zB5etOr6vhiYiypkD5MdfeBpk2xk38RVfqbUtklKUT8qp; OGP=-19027681:\r\nSec-Fetch-Dest: empty\r\nSec-Fetch-Mode: cors\r\nSec-Fetch-Site: same-site\r\nPragma: no-cache\r\nCache-Control: no-cache\r\nTE: trailers\r\n\r\n";

    let mut group = c.benchmark_group("parse");
    for &input in [input, input_long, input_longer].iter() {
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(input.len()), input, |b, i| {
            b.iter(|| {
                let mut req = H1Request::new();
                let _ = req.parse(i);
            })
        });
    }
    group.finish();
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
