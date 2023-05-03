//! Multi-Listener impl

use std::{
    io::{ErrorKind, Read, Result, Write},
    marker::PhantomData,
    sync::Arc,
};

use mio::{event::Source, Events, Interest, Poll, Token};
use rustls::ServerConfig;
use slab::Slab;

use crate::{
    connection::{Connection, PlainConnection},
    parser::{h1::response::Response, status::Status, Version},
};
use crate::{
    connection::{ConnectionBuilder, TlsConnection},
    net::{tcp_listener::TcpListener, tcp_stream::TcpStream},
};

const LISTEN_TOKEN: Token = Token(usize::MAX);
const WAKE_TOKEN: Token = Token(usize::MAX - 1);

/// Configuration for the listener
#[derive(Debug)]
pub struct ListenerConfig {
    /// TODO
    pub tls: Option<Arc<ServerConfig>>,
    /// TODO
    pub http_port: u16,
    /// TODO
    pub https_port: u16,
}

/// Socket listener for the server.
#[derive(Debug)]
pub struct MultiListener<T, S, C>
where
    T: TcpListener<S> + Source,
    S: TcpStream + Read + Write + Source,
    C: Connection,
{
    inner: T,
    num_events: usize,
    poll: Poll,
    connections: Slab<C>,
    configuration: ListenerConfig,
    _marker: PhantomData<S>,
}

impl<T, S> MultiListener<T, S, PlainConnection<S>>
where
    T: TcpListener<S> + Source,
    S: TcpStream + Read + Write + Source,
{
    #[inline]
    fn accept(&mut self) -> Result<()> {
        loop {
            match self.inner.accept() {
                Ok((stream, _)) => {
                    let entry = self.connections.vacant_entry();
                    let token = Token(entry.key());

                    let mut connection = ConnectionBuilder::new(stream, token)
                        .with_plaintext()
                        .build();
                    connection.register(self.poll.registry())?;
                    self.connections.insert(connection);
                }
                Err(ref err) if err.kind() == ErrorKind::WouldBlock => return Ok(()),
                Err(err) => {
                    return Err(err);
                }
            }
        }
    }

    /// Runs the listener's main event loop, accepting connections and notifying sessions of
    /// connection events.
    pub fn run(&mut self) {
        let mut events = Events::with_capacity(self.num_events);

        loop {
            match self.poll.poll(&mut events, None) {
                Ok(_) => {
                    for event in events.iter() {
                        match event.token() {
                            LISTEN_TOKEN => {
                                self.accept()
                                    .expect("Could not accept connections from socket");
                            }

                            _ => {
                                self.event(event);
                            }
                        }
                    }
                }
                Err(err) => {
                    println!("Failed to poll for events: {}", err);
                    return;
                }
            }
        }
    }
}

impl<T, S> MultiListener<T, S, TlsConnection<S>>
where
    T: TcpListener<S> + Source,
    S: TcpStream + Read + Write + Source,
{
    #[inline]
    fn accept(&mut self) -> Result<()> {
        loop {
            match self.inner.accept() {
                Ok((stream, _)) => {
                    let entry = self.connections.vacant_entry();
                    let token = Token(entry.key());

                    let connection = ConnectionBuilder::new(stream, token)
                        .with_tls(
                            self.configuration
                                .tls
                                .as_ref()
                                .expect("Tls configuration is required")
                                .clone(),
                        )
                        .build()
                        .expect("Invalid TLS Configuration");

                    self.connections.insert(connection);
                }
                Err(ref err) if err.kind() == ErrorKind::WouldBlock => return Ok(()),
                Err(err) => {
                    return Err(err);
                }
            }
        }
    }

    /// Runs the listener's main event loop, accepting connections and notifying sessions of
    /// connection events.
    pub fn run(&mut self) {
        let mut events = Events::with_capacity(self.num_events);

        loop {
            match self.poll.poll(&mut events, None) {
                Ok(_) => {
                    for event in events.iter() {
                        match event.token() {
                            LISTEN_TOKEN => {
                                self.accept()
                                    .expect("Could not accept connections from socket");
                            }

                            _ => {
                                self.event(event);
                            }
                        }
                    }
                }
                Err(err) => {
                    println!("Failed to poll for events: {}", err);
                    return;
                }
            }
        }
    }
}

impl<T, S, C> MultiListener<T, S, C>
where
    T: TcpListener<S> + Source,
    S: TcpStream + Read + Write + Source,
    C: Connection,
{
    /// TODO
    pub fn new(mut tcp_listener: T, config: ListenerConfig) -> Self {
        let poll = Poll::new().unwrap();
        poll.registry()
            .register(&mut tcp_listener, LISTEN_TOKEN, Interest::READABLE)
            .unwrap();

        Self {
            inner: tcp_listener,
            num_events: 1024,
            poll,
            connections: Slab::default(),
            configuration: config,
            _marker: PhantomData::default(),
        }
    }

    #[inline]
    fn event(&mut self, event: &mio::event::Event) {
        let token = event.token();

        let Some(ref mut connection) = self.connections.get_mut(token.0) else { return };

        if event.is_readable() {
            let read_result = connection.read();

            if read_result.is_err() || connection.is_closed() {
                return self.close_connection(token);
            }

            if let Ok(_request) = connection.parse() {
                // TODO: handle routing for request handlers here

                let response = Response::new_with_status_line(Version::H1_1, Status::NoContent);
                connection.prepare_response(response);
            }
        }

        if (event.is_writable() && connection.write().is_err()) || connection.is_closed() {
            self.close_connection(event.token())
        }
    }

    #[inline]
    fn close_connection(&mut self, token: Token) {
        let mut closed = false;
        if let Some(ref mut connection) = self.connections.get_mut(token.0) {
            if connection.is_closed() {
                connection.deregister(self.poll.registry()).unwrap();
                closed = true;
            }
        }

        if closed {
            self.connections.try_remove(token.0);
        }
    }
}
