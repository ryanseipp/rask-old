use std::io::{Error, Result};
use std::net::SocketAddr;

use super::tcp_stream::TcpStream;

pub trait TcpListener<S: TcpStream> {
    fn bind(addr: SocketAddr) -> Result<Self>
    where
        Self: Sized;

    fn accept(&self) -> Result<(S, SocketAddr)>;

    fn local_addr(&self) -> Result<SocketAddr>;

    fn set_ttl(&self, ttl: u32) -> Result<()>;

    fn ttl(&self) -> Result<u32>;

    fn take_error(&self) -> Result<Option<Error>>;
}

type MTcpListener = mio::net::TcpListener;
type MTcpStream = mio::net::TcpStream;

impl TcpListener<MTcpStream> for MTcpListener {
    #[inline]
    fn bind(addr: SocketAddr) -> Result<Self> {
        Self::bind(addr)
    }

    #[inline]
    fn accept(&self) -> Result<(MTcpStream, SocketAddr)> {
        Self::accept(self)
    }

    #[inline]
    fn local_addr(&self) -> Result<SocketAddr> {
        Self::local_addr(self)
    }

    #[inline]
    fn set_ttl(&self, ttl: u32) -> Result<()> {
        Self::set_ttl(self, ttl)
    }

    #[inline]
    fn ttl(&self) -> Result<u32> {
        Self::ttl(self)
    }

    #[inline]
    fn take_error(&self) -> Result<Option<Error>> {
        Self::take_error(self)
    }
}

type STcpListener = std::net::TcpListener;
type STcpStream = std::net::TcpStream;

impl TcpListener<STcpStream> for STcpListener {
    #[inline]
    fn bind(addr: SocketAddr) -> Result<Self> {
        Self::bind(addr)
    }

    #[inline]
    fn accept(&self) -> Result<(STcpStream, SocketAddr)> {
        Self::accept(self)
    }

    #[inline]
    fn local_addr(&self) -> Result<SocketAddr> {
        Self::local_addr(self)
    }

    #[inline]
    fn set_ttl(&self, ttl: u32) -> Result<()> {
        Self::set_ttl(self, ttl)
    }

    #[inline]
    fn ttl(&self) -> Result<u32> {
        Self::ttl(self)
    }

    #[inline]
    fn take_error(&self) -> Result<Option<Error>> {
        Self::take_error(self)
    }
}
