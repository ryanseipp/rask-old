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

//! Worker to process HTTP requests

use std::sync::{
    mpsc::{Receiver, Sender},
    Arc,
};

use crate::sessions::Session;

// TODO: Need a data structure to manage owned sessions. HTTP requests may arrive in multiple reads
// into session, so must support incremental parsing. Hopefully we can parse everything currently
// held, then move on to next bit of work. Perhaps by letting session own the currently partially
// parsed request, and sending the session back to the listener when all work is done on currently
// available data?
/// Worker which lives on a separate thread, receives Sessions to process, and write HTTP responses
#[derive(Debug)]
pub struct Worker {
    session_rx: Receiver<Arc<Session>>,
    session_tx: Sender<Arc<Session>>,
}

impl Worker {
    /// Main event loop for worker
    pub fn run(&mut self) {
        // do we just block on receiving from `session_rx`? Or is there a better way to handle it?
        // TODO: just block for now. May be a better way to handle this when we can profile
        while let Ok(_session) = self.session_rx.recv() {
            // parse bytes in `session.read_buffer`
            todo!()
        }
    }
}
