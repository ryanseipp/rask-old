# Rask

## An HTTP framework for rapid API development

This project's sole role is to help me learn the fundamentals of networking and performance optimization. As such,
even though the goal is to be fully compliant, this library should probably not be used in production applications.
If you're looking for a low-level HTTP2 implementation, check out [h2](https://github.com/hyperium/h2). For an
experimental HTTP3 implementation, check out [h3](https://github.com/hyperium/h3).

If you happen like something you see in this crate, please reach out. I'd be happy to contribute to other open-source
crates doing something similar.

## Goals

- Learn the HTTP stack, understanding what's happening at a lower level
- Accept and understand TCP/UDP connections, and how the OS handles them
- Negotiate TLS handshakes, and understand how SSL operates
- Support HTTP through HTTP3, and understand how negotiation of supported versions work
- Should be performant. This is an opportunity to understand async I/O at a deep level

As this library is intended for deeper understanding of some very low level concepts, the use of other crates should
be minimal. Therefore, I will likely only build on top of crates that abstract OS-level primitives, or provide
functionality that is not critical to the understanding of these concepts.

This library will start with implementation built on top of [tokio's mio](https://github.com/tokio-rs/mio) to minimize
what's being abstracted. Later, I will consider switching to utilizing tokio, or at least providing an API that is
supported under the tokio runtime.

## Relevant RFCs

1. [RFC 9110 HTTP Semantics](https://www.rfc-editor.org/rfc/rfc9110)
2. [RFC 9111 HTTP Caching](https://www.rfc-editor.org/rfc/rfc9111)
3. [RFC 9112 HTTP/1.1](https://www.rfc-editor.org/rfc/rfc9112)
4. [RFC 9113 HTTP/2](https://www.rfc-editor.org/rfc/rfc9113)
5. [RFC 9114 HTTP/3](https://www.rfc-editor.org/rfc/rfc9114)
