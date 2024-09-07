use std::convert::From;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::io;

#[derive(Debug)]
pub enum Error {
    NoHostsFound,
    #[allow(dead_code)] // used in Display implementation
    IO(io::Error),
    #[allow(dead_code)] // used in Display implementation
    Zeroconf(zeroconf::error::Error),
}

impl Display for Error {
    fn fmt(&self, format: &mut Formatter) -> Result<(), fmt::Error> {
        write!(format, "{:?}", self)
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::IO(error)
    }
}

impl From<zeroconf::error::Error> for Error {
    fn from(error: zeroconf::error::Error) -> Self {
        Error::Zeroconf(error)
    }
}

#[cfg(test)]
mod test {
    use crate::avahi_error::Error;
    use std::io;

    #[test]
    fn format() {
        assert_eq!("NoHostsFound", format!("{}", Error::NoHostsFound));
    }

    #[test]
    fn from_io_error() {
        let eio = io::Error::new(io::ErrorKind::Other, "");
        let e = Error::from(eio);
        assert!(matches!(e, Error::IO(_)));
    }

    #[test]
    fn from_zeroconf_error() {
        let ezc = zeroconf::error::Error::new(String::from(""));
        let e = Error::from(ezc);
        assert!(matches!(e, Error::Zeroconf(_)));
    }
}
