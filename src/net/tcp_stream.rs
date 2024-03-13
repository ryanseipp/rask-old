use std::io::{Error, Result};
use std::net::{Shutdown, SocketAddr};

pub trait TcpStream {
    fn connect(addr: SocketAddr) -> Result<Self>
    where
        Self: Sized;

    fn peer_addr(&self) -> Result<SocketAddr>;

    fn local_addr(&self) -> Result<SocketAddr>;

    fn shutdown(&self, how: Shutdown) -> Result<()>;

    fn set_nodelay(&self, nodelay: bool) -> Result<()>;

    fn nodelay(&self) -> Result<bool>;

    fn set_ttl(&self, ttl: u32) -> Result<()>;

    fn ttl(&self) -> Result<u32>;

    fn take_error(&self) -> Result<Option<Error>>;

    fn peek(&self, buf: &mut [u8]) -> Result<usize>;
}

impl TcpStream for mio::net::TcpStream {
    #[inline]
    fn connect(addr: SocketAddr) -> Result<Self>
    where
        Self: Sized,
    {
        Self::connect(addr)
    }

    #[inline]
    fn peer_addr(&self) -> Result<SocketAddr> {
        Self::peer_addr(self)
    }

    #[inline]
    fn local_addr(&self) -> Result<SocketAddr> {
        Self::local_addr(self)
    }

    #[inline]
    fn shutdown(&self, how: Shutdown) -> Result<()> {
        Self::shutdown(self, how)
    }

    #[inline]
    fn set_nodelay(&self, nodelay: bool) -> Result<()> {
        Self::set_nodelay(self, nodelay)
    }

    #[inline]
    fn nodelay(&self) -> Result<bool> {
        Self::nodelay(self)
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

    #[inline]
    fn peek(&self, buf: &mut [u8]) -> Result<usize> {
        Self::peek(self, buf)
    }
}

impl TcpStream for std::net::TcpStream {
    #[inline]
    fn connect(addr: SocketAddr) -> Result<Self>
    where
        Self: Sized,
    {
        Self::connect(addr)
    }

    #[inline]
    fn peer_addr(&self) -> Result<SocketAddr> {
        Self::peer_addr(self)
    }

    #[inline]
    fn local_addr(&self) -> Result<SocketAddr> {
        Self::local_addr(self)
    }

    #[inline]
    fn shutdown(&self, how: Shutdown) -> Result<()> {
        Self::shutdown(self, how)
    }

    #[inline]
    fn set_nodelay(&self, nodelay: bool) -> Result<()> {
        Self::set_nodelay(self, nodelay)
    }

    #[inline]
    fn nodelay(&self) -> Result<bool> {
        Self::nodelay(self)
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

    #[inline]
    fn peek(&self, buf: &mut [u8]) -> Result<usize> {
        Self::peek(self, buf)
    }
}
