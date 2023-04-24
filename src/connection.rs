//! TODO
use std::{
    fmt::Debug,
    io::{self, ErrorKind, Read, Result, Write},
    sync::Arc,
};

use mio::{event::Source, Interest, Registry, Token};
use rustls::{IoState, ServerConfig, ServerConnection};

use crate::parser::{
    h1::{request::H1Request, response::Response},
    ParseError, ParseResult, Status,
};

use super::net::tcp_stream::TcpStream;

/// TODO
#[derive(Debug)]
pub enum ConnectionType<S>
where
    S: TcpStream + Read + Write + Source,
{
    /// TODO
    Plain(PlainConnection<S>),
    /// TODO
    Tls(TlsConnection<S>),
}

/// TODO
#[derive(Debug)]
pub enum ConnectionVersion {
    /// TODO
    Http11(H1Request),
    /// TODO
    H2,
    /// TODO
    H3,
}

/// TODO
pub trait Connection {
    /// TODO
    fn read(&mut self) -> Result<()>;
    /// TODO
    fn write(&mut self) -> Result<usize>;
    /// TODO
    fn parse(&mut self) -> ParseResult<usize>;
    /// TODO
    fn prepare_response(&mut self, response: Response);
    /// TODO
    fn is_closed(&self) -> bool;
    /// TODO
    fn token(&self) -> Token;
    /// TODO
    fn register(&mut self, registry: &Registry) -> Result<()>;
    /// TODO
    fn reregister(&mut self, registry: &Registry) -> Result<()>;
    /// TODO
    fn deregister(&mut self, registry: &Registry) -> Result<()>;
}

/// TODO
#[derive(Debug)]
pub struct ConnectionBuilder<S> {
    stream: S,
    token: Token,
}

impl<S> ConnectionBuilder<S>
where
    S: TcpStream + Read + Write + Source,
{
    /// TODO
    pub fn new(stream: S, token: Token) -> Self {
        Self { stream, token }
    }

    /// TODO
    pub fn with_plaintext(self) -> PlaintextConnectionBuilder<S> {
        PlaintextConnectionBuilder::new(self.stream, self.token)
    }

    /// TODO
    pub fn with_tls(self, config: Arc<ServerConfig>) -> TlsConnectionBuilder<S> {
        TlsConnectionBuilder::new(self.stream, self.token, config)
    }
}

/// TODO
#[derive(Debug)]
pub struct PlaintextConnectionBuilder<S> {
    stream: S,
    token: Token,
}

impl<S> PlaintextConnectionBuilder<S>
where
    S: TcpStream + Read + Write + Source,
{
    fn new(stream: S, token: Token) -> Self {
        PlaintextConnectionBuilder { stream, token }
    }

    /// TODO
    pub fn build(self) -> PlainConnection<S> {
        PlainConnection::new(self.token, self.stream)
    }
}

/// TODO
#[derive(Debug)]
pub struct TlsConnectionBuilder<S> {
    stream: S,
    token: Token,
    config: Arc<ServerConfig>,
}

impl<S> TlsConnectionBuilder<S>
where
    S: TcpStream + Read + Write + Source,
{
    fn new(stream: S, token: Token, config: Arc<ServerConfig>) -> Self {
        TlsConnectionBuilder {
            stream,
            token,
            config,
        }
    }

    /// TODO
    pub fn build(self) -> std::result::Result<TlsConnection<S>, rustls::Error> {
        let tls = ServerConnection::new(self.config)?;
        Ok(TlsConnection::new(self.token, self.stream, tls))
    }
}

/// TODO
#[derive(Debug)]
pub struct PlainConnection<S>
where
    S: TcpStream + Read + Write + Source,
{
    stream: S,
    token: Token,
    closed: bool,
    responses: Vec<Response>,
    /// TODO
    pub state: Option<ConnectionVersion>,
}

impl<S> PlainConnection<S>
where
    S: TcpStream + Read + Write + Source,
{
    /// TODO
    pub fn new(token: Token, stream: S) -> Self {
        Self {
            stream,
            token,
            closed: false,
            responses: Vec::default(),
            state: None,
        }
    }

    #[inline]
    fn event_set(&self) -> Interest {
        if !self.responses.is_empty() {
            Interest::READABLE | Interest::WRITABLE
        } else {
            Interest::READABLE
        }
    }
}

impl<S> Connection for PlainConnection<S>
where
    S: TcpStream + Read + Write + Source,
{
    #[inline]
    fn read(&mut self) -> Result<()> {
        let mut done = false;

        if self.state.is_none() {
            const H2_PREFACE: &[u8] = b"PRI * HTTP/2";
            let mut preface_buf = [0; 12];

            self.state = if self.stream.peek(&mut preface_buf).is_ok() && preface_buf == H2_PREFACE
            {
                Some(ConnectionVersion::H2)
            } else {
                Some(ConnectionVersion::Http11(H1Request::default()))
            };
        }

        if let Some(ref mut state) = self.state {
            done = match state {
                ConnectionVersion::Http11(ref mut request) => request.fill(&mut self.stream)? == 0,
                ConnectionVersion::H2 => true,
                ConnectionVersion::H3 => true,
            }
        }

        if done {
            self.closed = true;
        }

        Ok(())
    }

    #[inline]
    fn write(&mut self) -> io::Result<usize> {
        let mut total = 0;
        for response in self.responses.drain(..) {
            let write_buf = response.get_serialized();
            total += write_buf.as_bytes().len();
            self.stream.write_all(write_buf.as_bytes())?;
            self.stream.flush()?;
        }

        Ok(total)
    }

    fn parse(&mut self) -> ParseResult<usize> {
        if let Some(ref mut state) = self.state {
            match state {
                ConnectionVersion::Http11(ref mut request) => request.parse(),
                ConnectionVersion::H2 => Ok(Status::Partial),
                ConnectionVersion::H3 => Ok(Status::Partial),
            }
        } else {
            Err(ParseError::Method)
        }
    }

    #[inline]
    fn prepare_response(&mut self, response: Response) {
        self.responses.push(response);
        self.state = None;
    }

    fn is_closed(&self) -> bool {
        self.closed
    }

    #[inline]
    fn register(&mut self, registry: &Registry) -> Result<()> {
        let interest = self.event_set();
        registry.register(&mut self.stream, self.token, interest)
    }

    #[inline]
    fn reregister(&mut self, registry: &Registry) -> Result<()> {
        let interest = self.event_set();
        registry.reregister(&mut self.stream, self.token, interest)
    }

    #[inline]
    fn deregister(&mut self, registry: &Registry) -> Result<()> {
        registry.deregister(&mut self.stream)
    }

    fn token(&self) -> Token {
        self.token
    }
}

/// TODO
#[derive(Debug)]
pub struct TlsConnection<S>
where
    S: TcpStream + Read + Write + Source,
{
    stream: S,
    tls: Box<ServerConnection>,
    token: Token,
    closed: bool,
    /// TODO
    pub state: Option<ConnectionVersion>,
}

impl<S> TlsConnection<S>
where
    S: TcpStream + Read + Write + Source,
{
    /// TODO
    pub fn new(token: Token, stream: S, tls: ServerConnection) -> Self {
        Self {
            stream,
            tls: Box::new(tls),
            token,
            closed: false,
            state: None,
        }
    }

    #[inline]
    fn read_tls(&mut self) -> Result<usize> {
        let mut read = 0;
        loop {
            match self.tls.read_tls(&mut self.stream) {
                Ok(0) => return Ok(0),
                Ok(n) => read += n,
                Err(e) => match e.kind() {
                    ErrorKind::WouldBlock => {
                        if read == 0 {
                            return Err(e);
                        } else {
                            return Ok(read);
                        }
                    }
                    ErrorKind::Interrupted => {}
                    _ => return Err(e),
                },
            }
        }
    }

    #[inline]
    fn read_plaintext(&mut self, tls_state: IoState) -> Result<()> {
        if tls_state.plaintext_bytes_to_read() > 0 {
            if let Some(ref mut state) = self.state {
                return match state {
                    ConnectionVersion::Http11(ref mut request) => request
                        .fill_exact(&mut self.tls.reader(), tls_state.plaintext_bytes_to_read()),
                    ConnectionVersion::H2 => Ok(()),
                    ConnectionVersion::H3 => Ok(()),
                };
            }
        }

        Ok(())
    }

    #[inline]
    fn event_set(&self) -> Interest {
        let read = self.tls.wants_read();
        let write = self.tls.wants_write();

        if read && write {
            Interest::READABLE | Interest::WRITABLE
        } else if write {
            Interest::WRITABLE
        } else {
            Interest::READABLE
        }
    }
}

impl<S> Connection for TlsConnection<S>
where
    S: TcpStream + Read + Write + Source,
{
    #[inline]
    fn read(&mut self) -> Result<()> {
        if self.state.is_none() {
            if let Some(protos) = self.tls.alpn_protocol() {
                if protos.windows(2).any(|w| w == b"h2") {
                    self.state = Some(ConnectionVersion::H2);
                }
            }

            if self.state.is_none() {
                self.state = Some(ConnectionVersion::Http11(H1Request::default()));
            }
        }

        let mut done = self.read_tls()? == 0;

        if !done {
            match self.tls.process_new_packets() {
                Ok(tls_state) => self.read_plaintext(tls_state)?,
                Err(_) => done = true,
            }
        }

        if done {
            self.closed = true;
        }

        Ok(())
    }

    #[inline]
    fn write(&mut self) -> io::Result<usize> {
        // TODO: this may be supressing errors
        self.tls.write_tls(&mut self.stream)
    }

    fn parse(&mut self) -> ParseResult<usize> {
        if let Some(ref mut state) = self.state {
            match state {
                ConnectionVersion::Http11(ref mut request) => request.parse(),
                ConnectionVersion::H2 => Ok(Status::Partial),
                ConnectionVersion::H3 => Ok(Status::Partial),
            }
        } else {
            Err(ParseError::Method)
        }
    }

    #[inline]
    fn prepare_response(&mut self, response: Response) {
        self.tls
            .writer()
            .write_all(response.get_serialized().as_bytes())
            .unwrap();
    }

    fn is_closed(&self) -> bool {
        self.closed
    }

    #[inline]
    fn register(&mut self, registry: &Registry) -> Result<()> {
        let interest = self.event_set();
        registry.register(&mut self.stream, self.token, interest)
    }

    #[inline]
    fn reregister(&mut self, registry: &Registry) -> Result<()> {
        let interest = self.event_set();
        registry.reregister(&mut self.stream, self.token, interest)
    }

    #[inline]
    fn deregister(&mut self, registry: &Registry) -> Result<()> {
        registry.deregister(&mut self.stream)
    }

    fn token(&self) -> Token {
        self.token
    }
}
