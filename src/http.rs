use std::io::{self, ErrorKind, Read, Write};
use std::net::{SocketAddr, Shutdown};
use std::fmt;

use mio::tcp::{self, TcpStream};

use hyper::net::NetworkStream;

pub struct HttpStream(pub TcpStream);

impl Clone for HttpStream {
    #[inline]
    fn clone(&self) -> HttpStream {
        HttpStream(self.0.try_clone().unwrap())
    }
}

impl fmt::Debug for HttpStream {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("HttpStream(_)")
    }
}

impl Read for HttpStream {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

impl Write for HttpStream {
    #[inline]
    fn write(&mut self, msg: &[u8]) -> io::Result<usize> {
        self.0.write(msg)
    }
    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

#[cfg(windows)]
impl ::std::os::windows::io::AsRawSocket for HttpStream {
    fn as_raw_socket(&self) -> ::std::os::windows::io::RawSocket {
        self.0.as_raw_socket()
    }
}

#[cfg(unix)]
impl ::std::os::unix::io::AsRawFd for HttpStream {
    fn as_raw_fd(&self) -> i32 {
        self.0.as_raw_fd()
    }
}

impl NetworkStream for HttpStream {
    #[inline]
    fn peer_addr(&mut self) -> io::Result<SocketAddr> {
            self.0.peer_addr()
    }

    #[inline]
    fn close(&mut self, how: Shutdown) -> io::Result<()> {
        let how: tcp::Shutdown = match how {
            Shutdown::Read => tcp::Shutdown::Read,
            Shutdown::Write => tcp::Shutdown::Write,
            Shutdown::Both => tcp::Shutdown::Both
        };
        match self.0.shutdown(how) {
            Ok(_) => Ok(()),
            // see https://github.com/hyperium/hyper/issues/508
            Err(ref e) if e.kind() == ErrorKind::NotConnected => Ok(()),
            err => err
        }
    }
}
