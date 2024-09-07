use denon_control::{
    create_tcp_stream, get_avahi_impl, get_receiver_and_port, main2, parse_args, Error,
};
use std::env;

fn main() -> Result<(), Error> {
    let args = parse_args(env::args().collect());
    let (denon_name, denon_port) = get_receiver_and_port(&args, get_avahi_impl(&args))?;
    let s = create_tcp_stream(denon_name.as_str(), denon_port)?;
    main2(args, s, Box::new(std::io::stdout()))?;
    Ok(())
}
