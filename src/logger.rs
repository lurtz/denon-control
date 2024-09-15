use std::io::{stdout, Write};

#[cfg(test)]
use mockall::{automock, mock, predicate::*};
#[cfg(test)]
use std::io::{self};

#[cfg_attr(test, automock)]
pub trait Logger2 {
    fn log(&self, message: &str);
}

pub struct StdoutLogger {}

impl StdoutLogger {
    pub fn new() -> Self {
        StdoutLogger {}
    }
}

impl Logger2 for StdoutLogger {
    fn log(&self, message: &str) {
        let _ = stdout().write(message.as_bytes());
        let _ = stdout().write("\n".as_bytes());
    }
}

#[cfg(test)]
pub fn nothing(_message: &str) {}

#[cfg(test)]
mock! {pub Logger {} impl Write for Logger {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize>;
    fn flush(&mut self) -> io::Result<()>;
}}
