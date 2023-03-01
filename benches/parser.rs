use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rask::parser::h1::request::H1Request;

const REQ: &[u8] = b"\
GET /api/v1.0/weather/forecast HTTP/1.1\r\n\
Host: www.example.org\r\n\r\n";

const REQ_MED: &[u8] = b"\
GET /api/v1.0/weather/forecast HTTP/1.1\r\n\
Accept:*/*\r\n\
Accept-Encoding:gzip,deflate,br\r\n\
Accept-Language:en-US,en;q=0.5\r\n\
Cache-Control:no-cache\r\n\
Connection:keep-alive\r\n\
DNT:1\r\n\
Host: www.example.org\r\n\
Pragma:no-cache\r\n\
Referrer:https://www.example.org\r\n\
Sec-Fetch-Dest:empty\r\n\
Sec-Fetch-Mode:cors\r\n\
Sec-Fetch-Site:same-origin\r\n\
User-Agent:Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/109.0\r\n\r\n";

const REQ_LONG: &[u8] = b"POST /log?format=json&hasfast=true HTTP/3\r\n\
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

const REQ_COMP: &[u8] = b"\
GET /wp-content/uploads/2010/03/darth-vader-jedi-battle-lightsaber.jpg HTTP/1.1\r\n\
Host: www.example.org\r\n\
User-Agent: Mozilla/5.0 (Macintosh; U; Intel Mac OS X 10.6; ja-JP-mac; rv:1.9.2.3) Gecko/20100401 Firefox/3.6.3 Pathtraq/0.9\r\n\
Accept: text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8\r\n\
Accept-Language: ja,en-us;q=0.7,en;q=0.3\r\n\
Accept-Encoding: gzip,deflate\r\n\
Accept-Charset: Shift_JIS,utf-8;q=0.7,*;q=0.7\r\n\
Keep-Alive: 115\r\n\
Connection: keep-alive\r\n\
Cookie: wp_ozh_wsa_visits=2; wp_ozh_wsa_visit_lasttime=xxxxxxxxxx; __utma=xxxxxxxxx.xxxxxxxxxx.xxxxxxxxxx.xxxxxxxxxx.xxxxxxxxxx.x; __utmz=xxxxxxxxx.xxxxxxxxxx.x.x.utmccn=(referral)|utmcsr=reader.livedoor.com|utmcct=/reader/|utmcmd=referral|padding=under256\r\n\r\n";

fn benchmark(c: &mut Criterion) {
    let inputs = [REQ, REQ_MED, REQ_COMP, REQ_LONG];

    let mut group = c.benchmark_group("parse");
    for &input in inputs.iter() {
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("all", input.len() as u64),
            input,
            |b, i| {
                b.iter(|| {
                    let mut req = H1Request::new();
                    let _ = req.parse(i);
                })
            },
        );
    }
    group.finish();
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
