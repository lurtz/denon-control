#![no_main]

use denon_control::{get_receiver_and_port, parse_args, Error, Logger};
use libfuzzer_sys::fuzz_target;

struct NoLogger {}

impl Logger for NoLogger {
    fn log(&self, _message: &str) {}
}

fn wrap_error(mut cmd_input: Vec<String>) -> Result<(), Error> {
    cmd_input.insert(0, "denon-control-fuzz-test".to_string());
    let logger = Box::new(NoLogger {});
    let args = parse_args(cmd_input, &*logger);
    let get_rec = |_logger: &dyn Logger| Ok(String::new());

    get_receiver_and_port(&args, &*logger, get_rec)?;

    Ok(())
}

fuzz_target!(|data: Vec<String>| {
    let _ = wrap_error(data);
});
