use std::io::{self, Read, Write};
use std::net::TcpStream;

#[cfg(test)]
use mockall::{automock, mock, predicate::*};

#[cfg_attr(test, automock)]
pub trait ReadStream: Send {
    fn peekly(&self, buf: &mut [u8]) -> io::Result<usize>;
    fn read_exactly(&self, buf: &mut [u8]) -> io::Result<()>;
}

impl ReadStream for TcpStream {
    fn peekly(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.peek(buf)
    }

    fn read_exactly(&self, buf: &mut [u8]) -> io::Result<()> {
        // why does this work?
        let mut mself = self;
        mself.read_exact(buf)
    }
}

pub trait ConnectionStream: Write {
    fn shutdownly(&self) -> io::Result<()>;
    fn get_readstream(&self) -> io::Result<Box<dyn ReadStream>>;
}

impl ConnectionStream for TcpStream {
    fn shutdownly(&self) -> io::Result<()> {
        self.shutdown(std::net::Shutdown::Both)
    }

    fn get_readstream(&self) -> io::Result<Box<dyn ReadStream>> {
        Ok(Box::new(self.try_clone()?))
    }
}

#[cfg(test)]
mock! {
    pub ShutdownStream {}
    impl Write for ShutdownStream {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize>;
        fn flush(&mut self) -> io::Result<()>;
    }
    impl ConnectionStream for ShutdownStream {
        fn shutdownly(&self) -> io::Result<()>;
        fn get_readstream(&self) -> io::Result<Box<dyn ReadStream>>;
    }
}

pub fn create_tcp_stream(
    denon_name: &str,
    denon_port: u16,
) -> Result<Box<dyn ConnectionStream>, io::Error> {
    let s = TcpStream::connect((denon_name, denon_port))?;
    s.set_read_timeout(None)?;
    s.set_nonblocking(false)?;
    Ok(Box::new(s))
}

#[cfg(test)]
mod test {
    use std::{io, net::TcpListener};

    use crate::stream::create_tcp_stream;

    #[test]
    fn connects_to_server() -> Result<(), io::Error> {
        let listener = TcpListener::bind("localhost:0")?;
        let addr = listener.local_addr()?;
        assert!(create_tcp_stream(addr.ip().to_string().as_str(), addr.port()).is_ok());
        Ok(())
    }

    #[test]
    fn fails_to_connect_and_returns_unknown() {
        let dc = create_tcp_stream("value", 0);
        assert!(matches!(dc, Err(_)));
    }
}
