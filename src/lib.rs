// $ printf "MV53\r" | nc -i 1 0005cd221b08.lan 23 | stdbuf -o 0 tr "\r" "\n"
// MV53
// MVMAX 86

mod avahi;
mod avahi3;
mod avahi_error;
mod denon_connection;
mod error;
mod logger;
mod parse;
mod state;
mod stream;

pub use denon_connection::read;
use denon_connection::DenonConnection;
pub use error::Error;
use getopts::Options;
use logger::Logger;
pub use logger::StdoutLogger;
use state::{get_state, PowerState, SetState, SourceInputState, State};
use std::rc::Rc;
pub use stream::create_tcp_stream;
use stream::ConnectionStream;

type GetReceiverFn = fn(&dyn Logger) -> Result<String, avahi_error::Error>;

// status object shall get the current status of the avr 1912
// easiest way would be a map<Key, Value> where Value is an enum of u32 and String
// Key is derived of a mapping from the protocol strings to constants -> define each string once
// the status object can be shared or the communication thread can be asked about a
// status which queries the receiver if it is not set

pub fn parse_args(args: Vec<String>, logger: &dyn Logger) -> getopts::Matches {
    let mut ops = Options::new();
    ops.optopt(
        "a",
        "address",
        "Address of Denon AVR with optional port (default: 23)",
        "HOSTNAME[:port]",
    );
    ops.optopt("p", "power", "Power ON, STANDBY or OFF", "POWER_MODE");
    ops.optopt("v", "volume", "set volume in range 30..50", "VOLUME");
    ops.optopt("i", "input", "set source input: DVD, GAME2", "SOURCE_INPUT");
    ops.optflag(
        "e",
        "extern-avahi",
        "use avahi-browser to find receiver instead of library",
    );
    ops.optflag("s", "status", "print status of receiver");
    ops.optflag("h", "help", "print help");

    let arguments = match ops.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            panic!("{}", f.to_string())
        }
    };

    if arguments.opt_present("h") {
        let brief = format!("Usage: {} [options]", args[0]);
        logger.log(&brief);
        let exit_success: i32 = 0;
        std::process::exit(exit_success);
    }

    arguments
}

fn print_status(dc: &mut DenonConnection) -> Result<String, std::io::Error> {
    Ok(format!(
        "Current status of receiver:\n\tPower({})\n\tSourceInput({})\n\tMainVolume({})\n\tMaxVolume({})\n",
        dc.get(State::Power)?,
        dc.get(State::SourceInput)?,
        dc.get(State::MainVolume)?,
        dc.get(State::MaxVolume)?
    ))
}

pub fn get_avahi_impl(args: &getopts::Matches) -> GetReceiverFn {
    if args.opt_present("e") {
        avahi::get_receiver
    } else {
        avahi3::get_receiver
    }
}

pub fn get_receiver_and_port(
    args: &getopts::Matches,
    logger: &dyn Logger,
    get_rec: GetReceiverFn,
) -> Result<(String, u16), avahi_error::Error> {
    let default_port = 23;
    let (denon_name, port) = match args.opt_str("a") {
        Some(name) => match name.find(':') {
            Some(pos) => (
                String::from(&name[0..pos]),
                name[(pos + 1)..].parse().unwrap_or(default_port),
            ),
            None => (name, default_port),
        },
        None => (get_rec(logger)?, default_port),
    };
    let _ = logger.log(&format!("using receiver: {}:{}", denon_name, port));
    Ok((denon_name, port))
}

pub fn main2(
    args: getopts::Matches,
    stream: Box<dyn ConnectionStream>,
    logger: Box<dyn Logger>,
) -> Result<(), Error> {
    let rclogger: Rc<dyn Logger> = logger.into();
    let mut dc = DenonConnection::new(stream, rclogger.clone())?;

    if args.opt_present("s") {
        rclogger.log(&print_status(&mut dc)?);
    }
    if let Some(p) = args.opt_str("p") {
        let state = get_state(PowerState::states(), p.as_str())?;
        dc.set(SetState::Power(state))?;
    }
    if let Some(i) = args.opt_str("i") {
        let state = get_state(SourceInputState::states(), i.as_str())?;
        dc.set(SetState::SourceInput(state))?;
    }
    if let Some(mut vi) = args.opt_get::<u32>("v")? {
        // do not accidentally kill the ears
        if vi > 50 {
            vi = 50;
        }
        dc.set(SetState::MainVolume(vi))?;
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use predicates::ord::eq;

    use crate::denon_connection::{read, test::create_connected_connection, write_string};
    use crate::error::Error;
    use crate::logger::{nothing, MockLogger};
    use crate::state::{PowerState, SetState, SourceInputState, State};
    use crate::stream::{create_tcp_stream, MockReadStream, MockShutdownStream};
    use crate::{avahi, avahi3, avahi_error, GetReceiverFn};
    use crate::{get_avahi_impl, get_receiver_and_port, main2, parse_args, print_status};
    use std::io;
    use std::net::{TcpListener, TcpStream};
    use std::thread;

    #[test]
    #[should_panic]
    fn parse_args_parnics_with_empty_vec() {
        let logger = MockLogger::new();
        parse_args(vec![], &logger);
    }

    #[test]
    #[should_panic]
    fn parse_args_parnics_with_unknown_option() {
        let logger = MockLogger::new();
        let string_args = vec!["blub", "-w"];
        parse_args(
            string_args.into_iter().map(|a| a.to_string()).collect(),
            &logger,
        );
    }

    #[test]
    fn parse_args_works_with_empty_strings() {
        let logger = MockLogger::new();
        parse_args(vec!["".to_string()], &logger);
        parse_args(vec!["blub".to_string()], &logger);
    }

    #[test]
    fn parse_args_short_options() {
        let logger = MockLogger::new();
        let string_args = vec![
            "blub",
            "-a",
            "some_host",
            "-p",
            "OFF",
            "-v",
            "20",
            "-i",
            "DVD",
            "-e",
            "-s",
        ];
        let args = parse_args(
            string_args.into_iter().map(|a| a.to_string()).collect(),
            &logger,
        );
        assert!(matches!(args.opt_str("a"), Some(x) if x == "some_host"));
        assert!(matches!(args.opt_str("p"), Some(x) if x == "OFF"));
        assert!(matches!(args.opt_str("v"), Some(x) if x == "20"));
        assert!(matches!(args.opt_get::<u32>("v"), Ok(Some(x)) if x == 20));
        assert!(matches!(args.opt_str("i"), Some(x) if x == "DVD"));
        assert!(args.opt_present("e"));
        assert!(args.opt_present("s"));
    }

    #[test]
    fn parse_args_long_options() {
        let logger = MockLogger::new();
        let string_args = vec![
            "blub",
            "--address",
            "some_host",
            "--power",
            "OFF",
            "--volume",
            "20",
            "--input",
            "DVD",
            "--extern-avahi",
            "--status",
        ];
        let args = parse_args(
            string_args.into_iter().map(|a| a.to_string()).collect(),
            &logger,
        );
        assert!(matches!(args.opt_str("a"), Some(x) if x == "some_host"));
        assert!(matches!(args.opt_str("p"), Some(x) if x == "OFF"));
        assert!(matches!(args.opt_str("v"), Some(x) if x == "20"));
        assert!(matches!(args.opt_get::<u32>("v"), Ok(Some(x)) if x == 20));
        assert!(matches!(args.opt_str("i"), Some(x) if x == "DVD"));
        assert!(args.opt_present("e"));
        assert!(args.opt_present("s"));
    }

    #[test]
    fn print_status_test() -> Result<(), io::Error> {
        let (mut to_receiver, mut dc) = create_connected_connection()?;
        write_string(&mut to_receiver, "PWON\rSICD\rMV230\rMVMAX666\r")?;

        let expected = "Current status of receiver:\n\tPower(ON)\n\tSourceInput(CD)\n\tMainVolume(230)\n\tMaxVolume(666)\n";
        assert_eq!(expected, print_status(&mut dc).unwrap());
        Ok(())
    }

    #[test]
    fn get_avahi_impl_extern_test() {
        let logger = MockLogger::new();
        let string_args = vec!["blub", "--extern-avahi"];
        let args = parse_args(
            string_args.into_iter().map(|a| a.to_string()).collect(),
            &logger,
        );

        assert_eq!(avahi::get_receiver as GetReceiverFn, get_avahi_impl(&args));
    }

    #[test]
    fn get_avahi_impl_intern_test() {
        let logger = MockLogger::new();
        let string_args = vec!["blub"];
        let args = parse_args(
            string_args.into_iter().map(|a| a.to_string()).collect(),
            &logger,
        );

        assert_eq!(avahi3::get_receiver as GetReceiverFn, get_avahi_impl(&args));
    }

    #[test]
    fn get_receiver_and_port_using_avahi_test() -> Result<(), Error> {
        let mut logger = MockLogger::new();
        let string_args = vec!["blub"];
        let args = parse_args(
            string_args.into_iter().map(|a| a.to_string()).collect(),
            &logger,
        );
        let receiver_address = String::from("some_receiver");
        logger
            .expect_log()
            .once()
            .with(eq("using receiver: some_receiver:23"))
            .returning(nothing);
        assert_eq!(
            (receiver_address, 23),
            get_receiver_and_port(&args, &mut logger, |_| Ok(String::from("some_receiver")))?
        );
        Ok(())
    }

    #[test]
    fn get_receiver_and_port_using_avahi_fails_test() -> Result<(), Error> {
        let logger = MockLogger::new();
        let string_args = vec!["blub"];
        let args = parse_args(
            string_args.into_iter().map(|a| a.to_string()).collect(),
            &logger,
        );
        assert!(matches!(
            get_receiver_and_port(&args, &logger, |_| Err(avahi_error::Error::NoHostsFound)),
            Err(avahi_error::Error::NoHostsFound)
        ));
        Ok(())
    }

    #[test]
    fn get_receiver_and_port_using_args_test() -> Result<(), Error> {
        let mut logger = MockLogger::new();
        let string_args = vec!["blub", "-a", "blub_receiver"];
        let args = parse_args(
            string_args.into_iter().map(|a| a.to_string()).collect(),
            &logger,
        );
        let receiver_address = String::from("blub_receiver");
        logger
            .expect_log()
            .once()
            .with(eq("using receiver: blub_receiver:23"))
            .returning(nothing);
        assert_eq!(
            (receiver_address, 23),
            get_receiver_and_port(&args, &logger, |_| Ok(String::from("some_receiver")))?
        );
        Ok(())
    }

    #[test]
    fn get_receiver_and_port_using_args_with_port_test() -> Result<(), Error> {
        let mut logger = MockLogger::new();
        let string_args = vec!["blub", "-a", "blub_receiver:666"];
        let args = parse_args(
            string_args.into_iter().map(|a| a.to_string()).collect(),
            &logger,
        );
        let receiver_address = String::from("blub_receiver");
        logger
            .expect_log()
            .once()
            .with(eq("using receiver: blub_receiver:666"))
            .returning(nothing);
        assert_eq!(
            (receiver_address, 666),
            get_receiver_and_port(&args, &logger, |_| panic!())?
        );
        Ok(())
    }

    #[test]
    fn main2_test() -> Result<(), io::Error> {
        // TODO use mocks
        let mut mlogger = Box::new(MockLogger::new());
        let listen_socket = TcpListener::bind("localhost:0")?;
        let local_port = listen_socket.local_addr()?.port();
        let string_args = vec![
            "blub",
            "-a",
            "localhost",
            "-s",
            "-p",
            "STANDBY",
            "-i",
            "CD",
            "-v",
            "127",
        ];
        let args = parse_args(
            string_args.into_iter().map(|a| a.to_string()).collect(),
            &*mlogger,
        );

        let acceptor = thread::spawn(move || -> Result<(TcpStream, Vec<String>), io::Error> {
            let mut to_receiver = listen_socket.accept()?.0;

            let mut received_data = read(&mut to_receiver, 1)?;
            write_string(&mut to_receiver, "PWON\r")?;
            received_data.append(&mut read(&mut to_receiver, 1)?);
            write_string(&mut to_receiver, "SIDVD\r")?;
            received_data.append(&mut read(&mut to_receiver, 1)?);
            write_string(&mut to_receiver, "MV230\r")?;
            received_data.append(&mut read(&mut to_receiver, 1)?);
            write_string(&mut to_receiver, "MVMAX666\r")?;
            Ok((to_receiver, received_data))
        });

        let s = create_tcp_stream("localhost", local_port)?;
        mlogger
            .expect_log()
            .once()
            .with(eq(
        "Current status of receiver:\n\tPower(ON)\n\tSourceInput(DVD)\n\tMainVolume(230)\n\tMaxVolume(666)\n"
        )).returning(nothing);
        assert!(main2(args, s, mlogger).is_ok());

        let (to_receiver, query_data) = acceptor.join().unwrap()?;
        assert!(query_data.contains(&format!("{}?", State::Power)));
        assert!(query_data.contains(&format!("{}?", State::SourceInput)));
        assert!(query_data.contains(&format!("{}?", State::MainVolume)));
        assert!(query_data.contains(&format!("{}?", State::MaxVolume)));

        let set_data = read(&to_receiver, 3)?;
        assert!(set_data.contains(&format!("{}", SetState::SourceInput(SourceInputState::Cd))));
        assert!(set_data.contains(&format!("{}", SetState::MainVolume(50))));
        assert!(set_data.contains(&format!("{}", SetState::Power(PowerState::Standby))));
        Ok(())
    }

    #[test]
    fn main2_less_args_test() -> Result<(), io::Error> {
        let mut mlogger = Box::new(MockLogger::new());
        let string_args = vec!["blub", "-a", "localhost"];
        let args = parse_args(
            string_args.into_iter().map(|a| a.to_string()).collect(),
            &*mlogger,
        );

        let mut msdstream = Box::new(MockShutdownStream::new());

        msdstream.expect_get_readstream().once().returning(|| {
            let mut blub = MockReadStream::new();
            blub.expect_peekly()
                .once()
                .returning(|_| Err(io::Error::new(io::ErrorKind::ConnectionAborted, "ha")));
            Ok(Box::new(blub))
        });

        msdstream.expect_shutdownly().once().returning(|| Ok(()));

        mlogger
            .expect_log()
            .once()
            .with(eq("got error: ha"))
            .returning(nothing);

        main2(args, msdstream, mlogger).unwrap();

        Ok(())
    }
}
