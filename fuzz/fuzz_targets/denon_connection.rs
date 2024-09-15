#![no_main]

use denon_control::create_connected_connection;
use libfuzzer_sys::fuzz_target;
use std::io::Write;

fuzz_target!(|data: &[u8]| {
    let (mut to_receiver, _dc) = create_connected_connection().unwrap();
    let _ = to_receiver.write_all(data);
});
