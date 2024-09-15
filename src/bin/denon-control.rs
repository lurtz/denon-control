use denon_control::{
    create_tcp_stream, get_avahi_impl, get_receiver_and_port, main2, parse_args, Error,
    StdoutLogger,
};
use std::env;

fn main() -> Result<(), Error> {
    let logger = Box::new(StdoutLogger::new());
    let args = parse_args(env::args().collect(), &*logger);
    let (denon_name, denon_port) = get_receiver_and_port(&args, &*logger, get_avahi_impl(&args))?;
    let s = create_tcp_stream(denon_name.as_str(), denon_port)?;
    main2(args, s, logger)?;
    Ok(())
}
