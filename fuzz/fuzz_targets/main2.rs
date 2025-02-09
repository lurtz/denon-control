#![no_main]

use std::{io::Write, net::TcpListener};

use denon_control::{create_tcp_stream, main2, parse_args, Error, Logger};
use libfuzzer_sys::fuzz_target;

struct NoLogger {}

impl Logger for NoLogger {
    fn log(&self, _message: &str) {}
}

fn wrap_error(data: (&[u8], Vec<String>)) -> Result<(), Error> {
    let listen_socket = TcpListener::bind("localhost:0")?;
    let addr = listen_socket.local_addr()?;
    let s = create_tcp_stream(addr.ip().to_string().as_str(), addr.port())?;
    let (mut to_denon_client, _) = listen_socket.accept()?;
    let (network_input, mut cmd_input) = data;
    cmd_input.insert(0, "denon-control-fuzz-test".to_string());
    to_denon_client.write_all(network_input)?;
    let logger = Box::new(NoLogger {});
    let args = parse_args(cmd_input, &*logger);
    main2(args, s, logger)?;

    Ok(())
}

fuzz_target!(|data: (&[u8], Vec<String>)| {
    let _ = wrap_error(data);
});
