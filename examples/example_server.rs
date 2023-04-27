use std::{io::Result, thread};

use crossbeam_channel::unbounded;
use mio::net::TcpListener;
use rask::{
    connection::PlainConnection,
    listener::{Listener, ListenerConfig},
    worker::Worker,
};

fn main() -> Result<()> {
    let config = ListenerConfig {
        tls: None,
        http_port: 8080,
        https_port: 8443,
    };

    let (tx, rx) = unbounded();
    let (c_tx, c_rx) = unbounded();

    let tcp_listener = TcpListener::bind("127.0.0.1:8080".parse().unwrap()).unwrap();
    let mut listener = Listener::<_, _, PlainConnection<_>>::new(tcp_listener, tx, c_rx, config);

    let mut workers = Vec::default();
    for _ in 0..3 {
        let waker = listener.waker();
        let rx = rx.clone();
        let c_tx = c_tx.clone();
        let jh = thread::spawn(move || Worker::new(rx, c_tx, waker).run());
        workers.push(jh);
    }

    listener.run();

    for worker in workers {
        worker.join().unwrap();
    }

    Ok(())
}
