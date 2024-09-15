use std::io::{stdout, Write};

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
pub trait Logger {
    fn log(&self, message: &str);
}

#[derive(Default)]
pub struct StdoutLogger {}

impl Logger for StdoutLogger {
    fn log(&self, message: &str) {
        let _ = stdout().write(message.as_bytes());
        let _ = stdout().write("\n".as_bytes());
    }
}

#[cfg(test)]
pub fn nothing(_message: &str) {}
