use std::convert::From;
use std::fmt;

use crate::avahi_error;

#[derive(Debug)]
pub enum Error {
    ParseInt(std::num::ParseIntError),
    Avahi(avahi_error::Error),
    IO(std::io::Error),
    Input(String),
}

impl fmt::Display for Error {
    fn fmt(&self, format: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(format, "{self:?}")
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::IO(error) => Some(error),
            Error::ParseInt(parse_int_error) => Some(parse_int_error),
            Error::Avahi(error) => Some(error),
            Error::Input(_) => None,
        }
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(parse_error: std::num::ParseIntError) -> Self {
        Error::ParseInt(parse_error)
    }
}

impl From<avahi_error::Error> for Error {
    fn from(avahi_error: avahi_error::Error) -> Self {
        Error::Avahi(avahi_error)
    }
}

impl From<std::io::Error> for Error {
    fn from(io_error: std::io::Error) -> Self {
        Error::IO(io_error)
    }
}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Error::Input(value)
    }
}

#[cfg(test)]
mod test {
    use crate::avahi_error;
    use crate::error::Error as Le_error;
    use std::error::Error;
    use std::io;

    #[macro_export]
    macro_rules! check_error {
        ($error_value:expr, $expected:pat, $string:expr, $source_result:expr ) => {
            let error = Le_error::from($error_value);
            assert!(matches!(error, $expected));
            assert_eq!($source_result, error.source().is_some());
            let starts_with_string = {
                use predicates::Predicate;
                let matcher = predicates::str::starts_with($string);
                matcher.eval(&format!("{error}"))
            };
            assert!(starts_with_string);
        };
    }

    #[test]
    fn error_test() {
        check_error!(
            i32::from_str_radix("a23", 10).unwrap_err(),
            Le_error::ParseInt(_),
            "ParseInt(",
            true
        );
        check_error!(
            avahi_error::Error::NoHostsFound,
            Le_error::Avahi(_),
            "Avahi(NoHostsFound)",
            true
        );
        check_error!(
            io::Error::from(io::ErrorKind::AddrInUse),
            Le_error::IO(_),
            "IO(",
            true
        );
        check_error!(
            String::from("blub"),
            Le_error::Input(_),
            "Input(\"blub\")",
            false
        );
    }
}
