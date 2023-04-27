//! Listener impl

use std::{
    io::{ErrorKind, Read, Result, Write},
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use crossbeam_channel::{Receiver, Sender, TryRecvError};
use mio::{event::Source, Events, Interest, Poll, Token, Waker};
use rustls::ServerConfig;
use slab::Slab;

use crate::{
    connection::{Connection, PlainConnection},
    Event,
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
pub struct Listener<T, S, C>
where
    T: TcpListener<S> + Source,
    S: TcpStream + Read + Write + Source,
    C: Connection,
{
    inner: T,
    num_events: usize,
    poll: Poll,
    connections: Slab<Arc<Mutex<C>>>,
    workers: Sender<Event<C>>,
    closed_connections: Receiver<Token>,
    configuration: ListenerConfig,
    waker: Arc<Waker>,
    _marker: PhantomData<S>,
}

impl<T, S> Listener<T, S, PlainConnection<S>>
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
                    self.connections.insert(Arc::new(Mutex::new(connection)));
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

                            WAKE_TOKEN => loop {
                                match self.closed_connections.try_recv() {
                                    Ok(token) => self.close_connection(token),
                                    Err(TryRecvError::Empty) => {
                                        break;
                                    }
                                    Err(TryRecvError::Disconnected) => {
                                        return;
                                    }
                                }
                            },

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

impl<T, S> Listener<T, S, TlsConnection<S>>
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

                    self.connections.insert(Arc::new(Mutex::new(connection)));
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

                            WAKE_TOKEN => loop {
                                match self.closed_connections.try_recv() {
                                    Ok(token) => self.close_connection(token),
                                    Err(TryRecvError::Empty) => {
                                        break;
                                    }
                                    Err(TryRecvError::Disconnected) => {
                                        return;
                                    }
                                }
                            },

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

impl<T, S, C> Listener<T, S, C>
where
    T: TcpListener<S> + Source,
    S: TcpStream + Read + Write + Source,
    C: Connection,
{
    /// TODO
    pub fn new(
        mut tcp_listener: T,
        workers: Sender<Event<C>>,
        closed_connections: Receiver<Token>,
        config: ListenerConfig,
    ) -> Self {
        let poll = Poll::new().unwrap();
        poll.registry()
            .register(&mut tcp_listener, LISTEN_TOKEN, Interest::READABLE)
            .unwrap();

        let waker = Arc::new(
            Waker::new(poll.registry(), WAKE_TOKEN).expect("Unable to create Waker for Listener"),
        );

        Self {
            inner: tcp_listener,
            num_events: 1024,
            poll,
            connections: Slab::default(),
            workers,
            closed_connections,
            configuration: config,
            waker,
            _marker: PhantomData::default(),
        }
    }

    /// Retrieve a waker for this Listener. This waker should be called any time a connection is
    /// intended to close, and placed on the `closed_connections` channel.
    #[inline]
    pub fn waker(&self) -> Arc<Waker> {
        self.waker.clone()
    }

    #[inline]
    fn event(&mut self, event: &mio::event::Event) {
        let token = event.token();

        if let Some(connection) = self.connections.get(token.0) {
            self.workers
                .send(Event {
                    connection: connection.clone(),
                    event: event.clone(),
                })
                .expect("All workers exited")
        }
    }

    #[inline]
    fn close_connection(&mut self, token: Token) {
        let mut closed = false;
        if let Some(connection) = self.connections.get(token.0) {
            let mut locked = connection.lock().unwrap();

            if locked.is_closed() {
                locked.deregister(self.poll.registry()).unwrap();
                closed = true;
            }
        }

        if closed {
            self.connections.try_remove(token.0);
        }
    }
}
