use std::{
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

#[derive(Debug)]
pub enum ConnectionType<S>
where
    S: TcpStream + Read + Write + Source,
{
    Plain(PlainConnection<S>),
    Tls(TlsConnection<S>),
}

#[derive(Debug)]
pub enum ConnectionVersion {
    Http11(H1Request),
    H2,
    H3,
}

#[derive(Debug)]
pub struct Connection<S>
where
    S: TcpStream + Read + Write + Source,
{
    inner: ConnectionType<S>,
}

impl<S> Connection<S>
where
    S: TcpStream + Read + Write + Source,
{
    pub fn new(
        token: Token,
        stream: S,
        config: Option<Arc<ServerConfig>>,
        registry: &Registry,
    ) -> Result<Self> {
        if let Some(config) = config {
            let tls = ServerConnection::new(config).expect("Invalid TLS configuration");
            let mut c = TlsConnection::new(token, stream, tls);
            c.register(registry)?;

            Ok(Connection {
                inner: ConnectionType::Tls(c),
            })
        } else {
            let mut c = PlainConnection::new(token, stream);
            c.register(registry)?;
            Ok(Connection {
                inner: ConnectionType::Plain(c),
            })
        }
    }

    #[inline]
    pub fn read(&mut self) -> Result<()> {
        match &mut self.inner {
            ConnectionType::Tls(c) => c.read(),
            ConnectionType::Plain(c) => c.read(),
        }
    }

    #[inline]
    pub fn prepare_response(&mut self, response: Response) {
        match &mut self.inner {
            ConnectionType::Tls(c) => c.prepare_response(response),
            ConnectionType::Plain(c) => c.prepare_response(response),
        }
    }

    #[inline]
    pub fn write(&mut self) -> Result<usize> {
        match &mut self.inner {
            ConnectionType::Tls(c) => c.write(),
            ConnectionType::Plain(c) => c.write(),
        }
    }

    #[inline]
    pub fn parse(&mut self) -> ParseResult<usize> {
        let state = match &mut self.inner {
            ConnectionType::Tls(ref mut c) => &mut c.state,
            ConnectionType::Plain(ref mut c) => &mut c.state,
        };

        if let Some(ref mut state) = state {
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
    pub fn request(&self) -> Option<&H1Request> {
        let state = match &self.inner {
            ConnectionType::Tls(ref c) => &c.state,
            ConnectionType::Plain(ref c) => &c.state,
        };

        if let Some(ConnectionVersion::Http11(h1_request)) = state {
            return Some(h1_request);
        }

        None
    }

    #[inline]
    pub fn is_closed(&self) -> bool {
        match &self.inner {
            ConnectionType::Tls(c) => c.closed,
            ConnectionType::Plain(c) => c.closed,
        }
    }

    #[inline]
    #[must_use]
    pub fn token(&self) -> Token {
        match &self.inner {
            ConnectionType::Tls(c) => c.token,
            ConnectionType::Plain(c) => c.token,
        }
    }

    #[inline]
    pub fn register(&mut self, registry: &Registry) -> Result<()> {
        match &mut self.inner {
            ConnectionType::Tls(ref mut c) => c.register(registry),
            ConnectionType::Plain(ref mut c) => c.register(registry),
        }
    }

    #[inline]
    pub fn reregister(&mut self, registry: &Registry) -> Result<()> {
        match &mut self.inner {
            ConnectionType::Tls(ref mut c) => c.reregister(registry),
            ConnectionType::Plain(ref mut c) => c.reregister(registry),
        }
    }

    #[inline]
    pub fn deregister(&mut self, registry: &Registry) -> Result<()> {
        match &mut self.inner {
            ConnectionType::Tls(ref mut c) => c.deregister(registry),
            ConnectionType::Plain(ref mut c) => c.deregister(registry),
        }
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
    pub state: Option<ConnectionVersion>,
}

impl<S> PlainConnection<S>
where
    S: TcpStream + Read + Write + Source,
{
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
    pub fn read(&mut self) -> Result<()> {
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
    pub fn prepare_response(&mut self, response: Response) {
        self.responses.push(response);
        self.state = None;
    }

    #[inline]
    pub fn write(&mut self) -> io::Result<usize> {
        let mut total = 0;
        for response in self.responses.drain(..) {
            let write_buf = response.get_serialized();
            total += write_buf.as_bytes().len();
            self.stream.write_all(write_buf.as_bytes())?;
            self.stream.flush()?;
        }

        Ok(total)
    }

    #[inline]
    pub fn register(&mut self, registry: &Registry) -> Result<()> {
        let interest = self.event_set();
        registry.register(&mut self.stream, self.token, interest)
    }

    #[inline]
    pub fn reregister(&mut self, registry: &Registry) -> Result<()> {
        let interest = self.event_set();
        registry.reregister(&mut self.stream, self.token, interest)
    }

    #[inline]
    pub fn deregister(&mut self, registry: &Registry) -> Result<()> {
        registry.deregister(&mut self.stream)
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
    pub state: Option<ConnectionVersion>,
}

impl<S> TlsConnection<S>
where
    S: TcpStream + Read + Write + Source,
{
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
    pub fn read(&mut self) -> Result<()> {
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
    pub fn prepare_response(&mut self, response: Response) {
        self.tls
            .writer()
            .write_all(response.get_serialized().as_bytes())
            .unwrap();
    }

    #[inline]
    pub fn write(&mut self) -> io::Result<usize> {
        // TODO: this may be supressing errors
        self.tls.write_tls(&mut self.stream)
    }

    #[inline]
    pub fn register(&mut self, registry: &Registry) -> Result<()> {
        let interest = self.event_set();
        registry.register(&mut self.stream, self.token, interest)
    }

    #[inline]
    pub fn reregister(&mut self, registry: &Registry) -> Result<()> {
        let interest = self.event_set();
        registry.reregister(&mut self.stream, self.token, interest)
    }

    #[inline]
    pub fn deregister(&mut self, registry: &Registry) -> Result<()> {
        registry.deregister(&mut self.stream)
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
