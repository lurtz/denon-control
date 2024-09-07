use crate::avahi_error::Error;
use std::process::Command;

pub fn get_receiver() -> Result<String, Error> {
    let output = Command::new("/usr/bin/avahi-browse")
        .arg("-p")
        .arg("-t")
        .arg("-r")
        .arg("_raop._tcp")
        .output()?;

    let output_stdout = String::from_utf8_lossy(&output.stdout);
    let lines = output_stdout.lines();
    let denon_names: Vec<&str> = lines
        .filter(|&line| line.starts_with('='))
        .filter(|&line| line.contains("DENON"))
        .map(|line| line.split(';'))
        .map(|mut iter| iter.nth(6).unwrap())
        .collect();

    if denon_names.len() > 1 {
        println!(
            "multiple receivers found: {:?}, taking: {}",
            denon_names, denon_names[0]
        );
        println!("use -a option if you want to use another receiver");
    }

    if denon_names.is_empty() {
        Err(Error::NoHostsFound)
    } else {
        Ok(String::from(denon_names[0]))
    }
}

#[cfg(test)]
mod test {
    use super::get_receiver;
    use crate::avahi_error::Error;
    use std::net::TcpStream;

    #[test]
    fn get_receiver_may_return() {
        match get_receiver() {
            Ok(address) => assert!(TcpStream::connect((address, 23)).is_ok()),
            Err(e) => {
                println!("{}", e.to_string());
                assert!(matches!(e, Error::NoHostsFound))
            }
        }
    }
}
