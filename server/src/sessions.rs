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

//! Session data

use std::{
    io::{Error, ErrorKind, Read, Result, Write},
    net::SocketAddr,
    ops::Deref,
    sync::Mutex,
};

use mio::{event::Source, net::TcpStream, Interest, Registry, Token};

use crate::buffer::Buffer;

const KB: usize = 1024;
const BUFFER_CAPACITY: usize = 16 * KB;

/// Contains the connection's `TcpStream` and associated read and write buffers
#[derive(Debug)]
pub struct Session {
    stream: Mutex<TcpStream>,
    read_buffer: Mutex<Buffer>,
    write_buffer: Mutex<Buffer>,
}

impl Session {
    /// Creates session
    pub fn new(
        stream: TcpStream,
        read_buffer_capacity: usize,
        write_buffer_capacity: usize,
    ) -> Self {
        Self {
            stream: Mutex::new(stream),
            read_buffer: Mutex::new(Buffer::new(read_buffer_capacity)),
            write_buffer: Mutex::new(Buffer::new(write_buffer_capacity)),
        }
    }

    /// fills buffer with data from TcpStream
    pub fn fill(&self) -> Result<usize> {
        let mut read = 0;

        if let (Ok(stream), Ok(mut read_buffer)) = (self.stream.lock(), self.read_buffer.lock()) {
            loop {
                // Read 4KB-16KB at a time
                if read_buffer.remaining_mut() - read_buffer.len() < 4096 {
                    read_buffer.reserve(16384);
                }

                match stream.deref().read(&mut read_buffer) {
                    // Stream has closed
                    Ok(0) => return Ok(0),
                    Ok(n) => {
                        read_buffer.mark_written(n);
                        read += n;
                    }
                    Err(e) => match e.kind() {
                        // no more bytes to be read
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
        } else {
            Err(Error::new(
                ErrorKind::Other,
                "Could not take a lock on mutex",
            ))
        }
    }

    /// Flushes any pending write data to the TcpStream
    pub fn flush(&self) -> Result<usize> {
        let mut flushed = 0;
        if let (Ok(mut stream), Ok(mut write_buffer)) =
            (self.stream.lock(), self.read_buffer.lock())
        {
            while write_buffer.remaining() > 0 {
                match stream.write(&write_buffer) {
                    Ok(amt) => {
                        write_buffer.mark_read(amt);
                        flushed += amt;
                    }
                    Err(e) => match e.kind() {
                        ErrorKind::WouldBlock => {
                            if flushed == 0 {
                                return Err(e);
                            }
                            break;
                        }
                        ErrorKind::Interrupted => {}
                        _ => {
                            return Err(e);
                        }
                    },
                }
            }
        } else {
            return Err(Error::new(
                ErrorKind::Other,
                "Could not take a lock on mutex",
            ));
        }

        Ok(flushed)
    }
}

impl From<(TcpStream, SocketAddr)> for Session {
    fn from((value, _): (TcpStream, SocketAddr)) -> Self {
        Self::new(value, BUFFER_CAPACITY, BUFFER_CAPACITY)
    }
}

impl Source for Session {
    fn register(&mut self, registry: &Registry, token: Token, interests: Interest) -> Result<()> {
        self.stream
            .lock()
            .map_err(|_| Error::new(ErrorKind::Other, "Mutex could not be locked"))?
            .register(registry, token, interests)
    }

    fn reregister(&mut self, registry: &Registry, token: Token, interests: Interest) -> Result<()> {
        self.stream
            .lock()
            .map_err(|_| Error::new(ErrorKind::Other, "Mutex could not be locked"))?
            .reregister(registry, token, interests)
    }

    fn deregister(&mut self, registry: &Registry) -> Result<()> {
        self.stream
            .lock()
            .map_err(|_| Error::new(ErrorKind::Other, "Mutex could not be locked"))?
            .deregister(registry)
    }
}
