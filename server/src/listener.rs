// Copyright 2022 Ryan Seipp
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The main listener implementation

use std::{
    io::{self, Error, ErrorKind},
    sync::{
        mpsc::{Receiver, Sender},
        Arc,
    },
};

use mio::{event::Event, net::TcpListener, Events, Interest, Poll, Token};
use slab::Slab;

use crate::sessions::Session;

const LISTENER_TOKEN: Token = Token(usize::MAX);

/// `Listener` implements the core logic for accepting Tcp connections, creating HTTP sessions, and
/// driving all network socket reads
#[derive(Debug)]
pub struct Listener {
    inner: TcpListener,
    num_events: usize,
    poll: Poll,
    // all sessions currently open
    sessions: Slab<Arc<Session>>,
    // channels to send `Session`s with data to be processed by worker
    workers_tx: Vec<Sender<Arc<Session>>>,
    // channels to receive completed work from worker
    workers_rx: Vec<Receiver<Arc<Session>>>,
}

impl Listener {
    fn accept(&mut self) {
        loop {
            let session = match self.inner.accept().map(Session::from) {
                Ok(session) => Some(session),
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(_) => None,
            };

            if let Some(mut session) = session {
                let s = self.sessions.vacant_entry();
                // drop session if we can't register with poll
                if self
                    .poll
                    .registry()
                    .register(&mut session, Token(s.key()), Interest::READABLE)
                    .is_ok()
                {
                    self.sessions.insert(Arc::new(session));
                }
            }
        }
    }

    fn read(&mut self, token: Token) -> std::io::Result<()> {
        let session = self
            .sessions
            .get_mut(token.0)
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "Session does not exist"))?;

        match session.fill() {
            Ok(0) => Err(Error::new(ErrorKind::Other, "Session closed successfully")),
            Ok(_) => {
                // TODO: determine more fair method of spreading work between workers. Currently
                // dumps all work on first worker assuming server is operational
                for i in 0..self.workers_tx.len() {
                    if self.workers_tx[i].send(session.clone()).is_ok() {
                        return Ok(());
                    }
                }
                Err(Error::new(
                    ErrorKind::Other,
                    "Workers are stopped, server shutting down",
                ))
            }
            Err(e) => match e.kind() {
                ErrorKind::WouldBlock => Ok(()),
                _ => Err(e),
            },
        }
    }

    fn close(&mut self, token: Token) {
        if self.sessions.contains(token.0) {
            let session = self.sessions.remove(token.0);
            let _ = session.flush();
        }
    }

    fn session_event(&mut self, event: &Event) {
        let token = event.token();

        if event.is_error() {
            self.close(token);
            return;
        }

        if event.is_readable() && self.read(token).is_err() {
            self.close(token);
        }
    }

    /// Main event listener event loop. Entry point for all incoming packets. Will block until
    /// server shutdown
    pub fn run(&mut self) {
        let mut events = Events::with_capacity(self.num_events);

        loop {
            match self.poll.poll(&mut events, None) {
                Ok(_) => {
                    for event in events.iter() {
                        match event.token() {
                            LISTENER_TOKEN => {
                                self.accept();
                            }
                            _ => {
                                self.session_event(event);
                            }
                        }
                    }
                }
                Err(err) => {
                    println!("Failed to poll for events with Error: {err}");
                    return;
                }
            }
        }
    }
}
