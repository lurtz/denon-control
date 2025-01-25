use crate::parse::parse;
use crate::state::{SetState, State, StateValue};
use crate::stream::{ConnectionStream, ReadStream};
use std::collections::HashMap;
use std::io::{self, ErrorKind, Write};
use std::thread;
use std::time::Duration;

pub fn write_string(stream: &mut dyn Write, input: &str) -> Result<(), std::io::Error> {
    stream.write_all(input.as_bytes())
}

fn write_state(stream: &mut dyn Write, state: SetState) -> Result<(), io::Error> {
    write_string(stream, format!("{}\r", state).as_str())
}

fn write_query(stream: &mut dyn Write, state: State) -> Result<(), io::Error> {
    write_string(stream, format!("{}?\r", state).as_str())
}

pub fn read(stream: &dyn ReadStream, lines: u8) -> Result<Vec<String>, std::io::Error> {
    let mut result = Vec::<String>::new();

    // guarantee to read a full line. check that read content ends with \r
    while (lines as usize) != result.len() {
        let mut buffer = [0; 100];
        let read_bytes;
        match stream.peekly(&mut buffer) {
            Ok(rb) => read_bytes = rb,
            Err(e) => {
                if result.is_empty() {
                    return Err(e);
                } else {
                    break;
                }
            }
        }

        if 0 == read_bytes {
            break;
        }

        // search for first \r in buffer
        let first_cariage_return = buffer[0..read_bytes]
            .iter()
            .position(|&c| '\r' == (c as char));

        if first_cariage_return.is_none() {
            break;
        }

        // include cariage return in read_exact()
        let bytes_to_extract = first_cariage_return.unwrap() + 1;

        // do not add \r to string
        if let Ok(tmp) = std::str::from_utf8(&buffer[0..first_cariage_return.unwrap()]) {
            result.push(tmp.trim().to_owned());
        }

        stream.read_exactly(&mut buffer[0..bytes_to_extract])?;
    }

    Ok(result)
}

fn process_receiver_updates(
    stream: &dyn ReadStream,
    hstate: &mut HashMap<State, StateValue>,
) -> Result<(), std::io::Error> {
    loop {
        match read(stream, 1) {
            Ok(status_update) => {
                let parsed_response = parse_response(&status_update);
                if status_update.is_empty() {
                    return Ok(());
                }
                for sstate in parsed_response {
                    let (state, value) = sstate.convert();
                    hstate.insert(state, value);
                }
            }
            // check for timeout error -> return success on timeout error, else error
            Err(e) => {
                if ErrorKind::TimedOut != e.kind() && ErrorKind::WouldBlock != e.kind() {
                    return Err(e);
                }
                return Ok(());
            }
        }
    }
}

fn parse_response(response: &[String]) -> Vec<SetState> {
    response.iter().filter_map(|x| parse(x.as_str())).collect()
}

pub struct DenonConnection {
    state: HashMap<State, StateValue>,
    to_receiver: Box<dyn ConnectionStream>,
}

impl DenonConnection {
    pub fn new(to_receiver: Box<dyn ConnectionStream>) -> Result<DenonConnection, io::Error> {
        let state = HashMap::new();

        Ok(DenonConnection { state, to_receiver })
    }

    pub fn get(&mut self, op: State) -> Result<StateValue, io::Error> {
        process_receiver_updates(self.to_receiver.get_readstream()?.as_ref(), &mut self.state)?;
        // should first check if the requested op is present in state
        // if it is not present it should send the request to the thread and wait until completion
        {
            if let Some(received_state) = self.state.get(&op) {
                return Ok(*received_state);
            }
        }
        write_query(&mut self.to_receiver, op)?;
        for _ in 0..50 {
            thread::sleep(Duration::from_millis(10));
            process_receiver_updates(self.to_receiver.get_readstream()?.as_ref(), &mut self.state)?;
            if let Some(state) = self.state.get(&op) {
                return Ok(*state);
            }
        }
        Ok(StateValue::Unknown)
    }

    pub fn set(&mut self, sstate: SetState) -> Result<(), io::Error> {
        write_state(&mut self.to_receiver, sstate)
    }
}

#[cfg(test)]
pub mod test {
    use mockall::Sequence;

    use super::{process_receiver_updates, DenonConnection};
    use crate::denon_connection::{read, write_string};
    use crate::state::{PowerState, SetState, SourceInputState, State, StateValue};
    use crate::stream::{create_tcp_stream, MockReadStream};
    use std::cmp::min;
    use std::collections::HashMap;
    use std::io::{self, Error};
    use std::net::{TcpListener, TcpStream};
    use std::thread::yield_now;

    pub fn create_connected_connection() -> Result<(TcpStream, DenonConnection), io::Error> {
        let listen_socket = TcpListener::bind("localhost:0")?;
        let addr = listen_socket.local_addr()?;
        let s = create_tcp_stream(addr.ip().to_string().as_str(), addr.port())?;
        let dc = DenonConnection::new(s)?;
        let (to_denon_client, _) = listen_socket.accept()?;
        Ok((to_denon_client, dc))
    }

    fn copy_string_into_slice(src: &str, dst: &mut [u8]) -> usize {
        let length = min(src.len(), dst.len());
        dst[0..length].copy_from_slice(&src.as_bytes()[0..length]);
        length
    }

    macro_rules! wait_for_value_in_database {
        ($denon_connection:ident, $sstate:expr) => {
            let (state, value) = $sstate.convert();
            for _ in 0..100000 {
                if $denon_connection.get(state)? == value {
                    break;
                }
                yield_now();
            }
        };
    }

    macro_rules! assert_db_value {
        ($denon_connection:ident, $sstate:expr) => {
            let (state, value) = $sstate.convert();
            assert_eq!($denon_connection.get(state)?, value);
        };
    }

    #[test]
    fn connection_gets_no_reply_and_returns_unknown() -> Result<(), io::Error> {
        let (mut to_denon_client, mut dc) = create_connected_connection()?;
        let rc = dc.get(State::MainVolume)?;
        let query = read(&mut to_denon_client, 1)?;
        assert_eq!(rc, StateValue::Unknown);
        assert_eq!(query, vec!["MV?"]);
        Ok(())
    }

    #[test]
    fn connection_sends_main_volume_to_receiver() -> Result<(), io::Error> {
        let (mut to_denon_client, mut dc) = create_connected_connection()?;
        dc.set(SetState::MainVolume(666))?;
        let received = read(&mut to_denon_client, 1)?;
        assert_eq!("MV666", received[0]);
        Ok(())
    }

    #[test]
    fn connection_sends_max_volume_to_receiver() -> Result<(), io::Error> {
        let (mut to_denon_client, mut dc) = create_connected_connection()?;
        dc.set(SetState::MaxVolume(666))?;
        let received = read(&mut to_denon_client, 1)?;
        assert_eq!("MVMAX666", received[0]);
        Ok(())
    }

    #[test]
    fn connection_sends_source_input_to_receiver() -> Result<(), io::Error> {
        let (mut to_denon_client, mut dc) = create_connected_connection()?;
        dc.set(SetState::SourceInput(SourceInputState::Fvp))?;
        let received = read(&mut to_denon_client, 1)?;
        assert_eq!("SIFVP", received[0]);
        Ok(())
    }

    #[test]
    fn connection_sends_power_to_receiver() -> Result<(), io::Error> {
        let (mut to_denon_client, mut dc) = create_connected_connection()?;
        dc.set(SetState::Power(PowerState::On))?;
        let received = read(&mut to_denon_client, 1)?;
        assert_eq!("PWON", received[0]);
        Ok(())
    }

    #[test]
    fn connection_receives_volume_from_receiver() -> Result<(), io::Error> {
        let (mut to_denon_client, mut dc) = create_connected_connection()?;
        write_string(&mut to_denon_client, "MV234\r")?;
        assert_db_value!(dc, SetState::MainVolume(234));
        Ok(())
    }

    #[test]
    fn connection_receives_multiple_values_volume_from_receiver() -> Result<(), io::Error> {
        let (mut to_denon_client, mut dc) = create_connected_connection()?;
        assert_eq!(StateValue::Unknown, dc.get(State::MainVolume)?);
        assert_eq!(StateValue::Unknown, dc.get(State::SourceInput)?);
        assert_eq!(StateValue::Unknown, dc.get(State::Power)?);
        write_string(&mut to_denon_client, "MV234\rSICD\rPWON\r")?;
        assert_db_value!(dc, SetState::MainVolume(234));
        assert_db_value!(dc, SetState::SourceInput(SourceInputState::Cd));
        assert_db_value!(dc, SetState::Power(PowerState::On));
        Ok(())
    }

    #[test]
    fn connection_updates_values_with_newly_received_data() -> Result<(), io::Error> {
        let (mut to_denon_client, mut dc) = create_connected_connection()?;
        write_string(&mut to_denon_client, "MV234\r")?;
        assert_db_value!(dc, SetState::MainVolume(234));
        write_string(&mut to_denon_client, "MV320\r")?;
        wait_for_value_in_database!(dc, SetState::MainVolume(320));
        assert_db_value!(dc, SetState::MainVolume(320));

        Ok(())
    }

    #[test]
    fn read_without_valid_content_returns_empty_vec() -> Result<(), io::Error> {
        let listen_socket = TcpListener::bind("localhost:0")?;
        let addr = listen_socket.local_addr()?;
        let mut client = TcpStream::connect(addr)?;
        let (mut to_client, _) = listen_socket.accept()?;

        // as \r is missing, read() does not read or extract anything
        write_string(&mut to_client, "blub")?;
        let lines = read(&mut client, 1)?;
        assert_eq!(lines, Vec::<String>::new());

        // read() reads until \r and leaves other data in the stream
        write_string(&mut to_client, "bla\rfoo")?;
        let lines = read(&mut client, 2)?;
        assert_eq!(lines, vec!["blubbla".to_owned()]);

        Ok(())
    }

    #[test]
    fn read_reads_content_gets_error_and_returns_content() -> Result<(), io::Error> {
        let mut sequence = Sequence::new();
        let mut mstream = MockReadStream::new();
        // peek works
        mstream
            .expect_peekly()
            .once()
            .in_sequence(&mut sequence)
            .returning(|buf| Ok(copy_string_into_slice("some_data\r", buf)));
        // read works
        mstream
            .expect_read_exactly()
            .once()
            .in_sequence(&mut sequence)
            .returning(|_| Ok(()));
        // peek with error
        mstream
            .expect_peekly()
            .once()
            .in_sequence(&mut sequence)
            .returning(|_| Err(Error::from(io::ErrorKind::ConnectionAborted)));
        let lines = read(&mut mstream, 2)?;
        assert_eq!(vec!(String::from("some_data")), lines);
        Ok(())
    }

    #[test]
    fn thread_func_impl_gets_error_and_returns() {
        let mut mstream = MockReadStream::new();
        mstream
            .expect_peekly()
            .returning(|_| Err(Error::from(io::ErrorKind::ConnectionAborted)));
        let mut state = HashMap::default();
        let thread_err = process_receiver_updates(&mstream, &mut state);
        assert!(thread_err.is_err());
        assert_eq!(
            io::ErrorKind::ConnectionAborted,
            thread_err.unwrap_err().kind()
        );
    }

    #[test]
    fn thread_func_impl_gets_timeout_and_returns() {
        let mut sequence = Sequence::new();
        let mut mstream = MockReadStream::new();
        mstream
            .expect_peekly()
            .once()
            .in_sequence(&mut sequence)
            .returning(|_| Err(Error::from(io::ErrorKind::TimedOut)));
        let mut state = HashMap::default();
        let thread_err = process_receiver_updates(&mstream, &mut state);
        assert!(thread_err.is_ok());
    }
}
