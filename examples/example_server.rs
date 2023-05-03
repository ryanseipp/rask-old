use std::{
    io::Result,
    thread::{self, available_parallelism},
};

use mio::net::TcpListener as MioTcpListener;
use rask::{
    connection::PlainConnection,
    multilistener::{ListenerConfig, MultiListener},
};
use std::net::TcpListener;

fn main() -> Result<()> {
    let tcp_listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    tcp_listener.set_nonblocking(true).unwrap();

    let mut listeners = Vec::default();
    for _ in 0..usize::from(available_parallelism().unwrap()) {
        let mio_listener = MioTcpListener::from_std(tcp_listener.try_clone().unwrap());
        let jh = thread::spawn(move || {
            let config = ListenerConfig {
                tls: None,
                http_port: 8080,
                https_port: 8443,
            };

            let mut listener = MultiListener::<_, _, PlainConnection<_>>::new(mio_listener, config);

            listener.run();
        });

        listeners.push(jh);
    }

    for listener in listeners {
        listener.join().unwrap();
    }

    Ok(())
}
