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
    use crate::{avahi_error::Error as Avahi_error, check_error};
    use std::{error::Error, io};

    #[test]
    fn no_hosts_found() {
        assert_eq!("NoHostsFound", format!("{}", Avahi_error::NoHostsFound));
        assert!(Avahi_error::NoHostsFound.source().is_none());
    }

    #[test]
    fn from_io_error() {
        check_error!(
            Avahi_error,
            io::Error::from(io::ErrorKind::AddrInUse),
            Avahi_error::IO(_),
            "IO(",
            true
        );
    }

    #[test]
    fn from_zeroconf_error() {
        check_error!(
            Avahi_error,
            zeroconf::error::Error::new(String::from("blub")),
            Avahi_error::Zeroconf(_),
            "Zeroconf(",
            true
        );
    }
}
