use std::io::{self, Write};

use mockall::mock;

mock! {pub Logger {} impl Write for Logger {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize>;
    fn flush(&mut self) -> io::Result<()>;
}}
