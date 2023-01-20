use std::{sync::mpsc::channel, thread};

use rask::{listener::Listener, worker::Worker};

fn main() {
    let (tx, rx) = channel();
    let (r_tx, r_rx) = channel();
    let mut listener = Listener::new(vec![tx], vec![r_rx]);

    let t1 = thread::spawn(move || listener.run());
    let t2 = Worker::spawn_new_and_run(rx, r_tx);

    let _ = t1.join();
    let _ = t2.join();
}
