use std::convert::From;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::io;

#[derive(Debug)]
pub enum Error {
    NoHostsFound,
    IO(io::Error),
    Zeroconf(zeroconf::error::Error),
}

impl Display for Error {
    fn fmt(&self, format: &mut Formatter) -> Result<(), fmt::Error> {
        write!(format, "{self:?}")
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::NoHostsFound => None,
            Error::IO(error) => Some(error),
            Error::Zeroconf(error) => Some(error),
        }
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
    use crate::avahi_error::Error as Avahi_error;
    use std::{error::Error, io};

    #[test]
    fn format() {
        assert_eq!("NoHostsFound", format!("{}", Avahi_error::NoHostsFound));
    }

    #[test]
    fn from_io_error() {
        let eio = io::Error::from(io::ErrorKind::Other);
        let e = Avahi_error::from(eio);
        assert!(matches!(e, Avahi_error::IO(_)));
    }

    #[test]
    fn from_zeroconf_error() {
        let ezc = zeroconf::error::Error::new(String::from(""));
        let e = Avahi_error::from(ezc);
        assert!(matches!(e, Avahi_error::Zeroconf(_)));
    }

    #[test]
    fn source() {
        assert!(Avahi_error::NoHostsFound.source().is_none());
        assert!(
            Avahi_error::IO(io::Error::from(io::ErrorKind::AddrInUse))
                .source()
                .is_some()
        );
        assert!(
            Avahi_error::Zeroconf(zeroconf::error::Error::new("".to_string()))
                .source()
                .is_some()
        );
    }
}
