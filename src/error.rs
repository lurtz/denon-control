use crate::avahi_error;

#[derive(Debug)]
pub enum Error {
    ParseInt(std::num::ParseIntError),
    Avahi(avahi_error::Error),
    IO(std::io::Error),
    Input(String),
}

impl std::convert::From<std::num::ParseIntError> for Error {
    fn from(parse_error: std::num::ParseIntError) -> Self {
        Error::ParseInt(parse_error)
    }
}

impl std::convert::From<avahi_error::Error> for Error {
    fn from(avahi_error: avahi_error::Error) -> Self {
        Error::Avahi(avahi_error)
    }
}

impl std::convert::From<std::io::Error> for Error {
    fn from(io_error: std::io::Error) -> Self {
        Error::IO(io_error)
    }
}

impl std::convert::From<String> for Error {
    fn from(value: String) -> Self {
        Error::Input(value)
    }
}

#[cfg(test)]
mod test {
    use crate::avahi_error;
    use crate::error::Error;
    use std::io;

    macro_rules! check_error {
        ($error_value:expr, $expected:pat, $string:expr ) => {
            let error = Error::from($error_value);
            assert!(matches!(error, $expected));
            assert_eq!($string, format!("{:?}", error));
        };
    }

    #[test]
    fn error_test() {
        check_error!(
            i32::from_str_radix("a23", 10).unwrap_err(),
            Error::ParseInt(_),
            "ParseInt(ParseIntError { kind: InvalidDigit })"
        );
        check_error!(
            avahi_error::Error::NoHostsFound,
            Error::Avahi(_),
            "Avahi(NoHostsFound)"
        );
        check_error!(
            std::io::Error::from(io::ErrorKind::AddrInUse),
            Error::IO(_),
            "IO(Kind(AddrInUse))"
        );
        check_error!(String::from("blub"), Error::Input(_), "Input(\"blub\")");
    }
}
