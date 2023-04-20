//! Workers process events occurring on connections, including TLS handshakes, request parsing,
//! routing, and writing of responses. They are the driver behind the entire HTTP request pipeline,
//! besides accepting or closing the TCP connection.
//!
//! Workers are designed to run alongside other workers, taking connections waiting to be processed
//! from a channel, delivered by the listener. In essence, you can run as many workers as you have
//! threads (minus N for the listener threads).

// --------------------------------------------- TODO ---------------------------------------------
// The mutex wrapping each connection has the capability to kill our performance. It can
// essentially take a worker offline while another worker is executing a potentially lengthy
// request handler. It may be worthwhile getting to the point where we can write responses to test
// the throughput of the current configuration... Just in order to not keep spinning on this
// implementation, based on a hunch of how this will perform. Though that test won't include
// lengthy requests like hitting a DB under load...
//
// How would we get around the mutex? Sync only implies immutability when taking a reference. As we
// require mulitple threads to potentially take a mutable reference, we instead need thread-safe
// interior mutability. So, we'd need interior mutability on a connection, as otherwise we cannot
// read, write, etc. from a TcpStream, ServerConnection, registry, etc.
//
// The trick is whether or not we can ensure thread-safe interior mutability on a Connection. It's
// likely impossible, as we'd be implementing a Mutex ourselves. Let's leave that to the pros.
// RwLock may be interesting if we plan on doing many more reads than writes. This may be the case,
// as the main read is around getting incoming network data. Parsing can still be done without
// copying as we only need an immutable reference to read through the data. Request handlers should
// not be modifying requests, but rather mutating the response, which doesn't need access to the
// connection state (potentially) until we go to write the response to the wire.
//
// Instead, the need may be for me to limit the likelihood that contention ever occurs on a
// connection. Though, this can all be fine-tuned when I have a working benchmark that I can
// profile. Some ideas though: Swiching crossbeam_channel for crossbeam_queue would allow the
// listener to do more introspection on the queues for fairer scheduling, as well as scheduling
// consecutive events for a single token onto a single worker. This could also enable work
// stealing, rather than the current work-sharing model. If workers have completed their queue,
// they could look at other workers' queues to determine if there is work available to "steal".
// This would of course need knowledge of whether or not a connection in the worker's queue is
// currently being processed, though that could be handled via atomics.
//
// Alternatively, instead of queueing all requests for a single connection onto a single worker,
// which could be problematic with multiplexing in the case of H2 or H3, a worker could
// de-prioritize the connection event onto the back of its queue, making it available for work
// stealing if another worker is otherwise busy.
//
// Is the Connection the right point to pass work off to workers? It's very nice that the listener
// is so incredibly lightweight, only accepting new connections and passing everything else off.
// This leaves room for slightly more computation for scheduling the workers. Though I could also
// see the case for H2/H3 streams being the correct delineation for passing work off to workers.
// This would require the listener to read from the underlying stream, and place the read data onto
// an interiorly-mutable buffer. Though streams are a step higher, and require more processing than
// may be desirable. Maybe it's simply necessary to make Connections capable of interior mutability
// by making the locks more fine-grained. One for anything TCP/TLS related, and one per stream?
// This could allow workers to work-steal at the stream level, though lock contention may still
// hurt at the stream level.
//
// Path forward: Continue down this implementation path until we can benchmark and profile. I'll
// have a better understanding of the problem space once that occurs. From there, I can try these:
//  * crossbeam_queue for finer-grained control over scheduling
//  * fine-grained mutexes if overhead is low and work-stealing streams is feasible
// ------------------------------------------------------------------------------------------------

use std::{
    io::{Read, Write},
    sync::Arc,
};

use crossbeam_channel::{Receiver, Sender};
use mio::{event::Source, Token, Waker};

use crate::{
    net::tcp_stream::TcpStream,
    parser::{h1::response::Response, status::Status, Version},
    Event,
};

/// Worker that recieves connections on a channel and drives the request towards completion.
#[derive(Debug)]
pub struct Worker<S>
where
    S: TcpStream + Read + Write + Source,
{
    connections: Receiver<Event<S>>,
    inform_listener: Sender<Token>,
    listener_waker: Arc<Waker>,
}

impl<S> Worker<S>
where
    S: TcpStream + Read + Write + Source,
{
    /// TODO
    pub fn new(
        receiver: Receiver<Event<S>>,
        sender: Sender<Token>,
        listener_waker: Arc<Waker>,
    ) -> Self {
        Self {
            connections: receiver,
            inform_listener: sender,
            listener_waker,
        }
    }

    #[inline]
    fn inform_listener(&mut self, token: Token) -> Result<(), ()> {
        self.inform_listener.send(token).map_err(|_| ())?;
        self.listener_waker.wake().map_err(|_| ())
    }

    /// Main loop of the worker. Will block the thread until a signal to shutdown has been
    /// received.
    pub fn run(&mut self) {
        // if recv returns error, sender has disconnected and the server is shutting down.
        while let Ok(event) = self.connections.recv() {
            let mut locked_connection = match event.connection.lock() {
                // consider potentially reading here and returning result of read
                // problem is that'd require buffer allocation outside of the connection
                // otherwise we're forced to sustain the lock while we parse + route
                // hmmmmmmmmmmm
                Ok(c) => c,
                Err(c) => {
                    match self.inform_listener(c.get_ref().token()) {
                        Ok(()) => continue,
                        Err(()) => return,
                    };
                }
            };

            if event.event.is_readable() {
                let read_result = locked_connection.read();

                if read_result.is_err() || locked_connection.is_closed() {
                    match self.inform_listener(locked_connection.token()) {
                        Ok(()) => continue,
                        Err(()) => return, // server is shutting down
                    }
                }

                if locked_connection.parse().is_ok() {
                    let response = Response::new_with_status_line(Version::H1_1, Status::NoContent);
                    locked_connection.prepare_response(response);
                }
            }

            if event.event.is_writable() {
                // TODO: fix this unwrap
                locked_connection.write().unwrap();
            }

            drop(locked_connection);
            match self.inform_listener(event.event.token()) {
                Ok(()) => continue,
                Err(()) => return,
            }
        }
    }
}
