use assert_cmd::prelude::*; // Add methods on commands
use denon_control::{read, write_string};
use parameterized::parameterized;
use predicates::prelude::*; // Used for writing assertions
use predicates::str::contains;
use std::{
    io::{self, Read},
    net::{TcpListener, TcpStream},
    process::Command,
    thread::{self, JoinHandle},
}; // Run programs

fn create_acceptor_thread() -> Result<(JoinHandle<Result<TcpStream, io::Error>>, u16), io::Error> {
    let listen_socket = TcpListener::bind("localhost:0")?;
    let local_port = listen_socket.local_addr()?.port();

    let acceptor = thread::spawn(move || -> Result<TcpStream, io::Error> {
        let to_receiver = listen_socket.accept()?.0;
        Ok(to_receiver)
    });

    Ok((acceptor, local_port))
}

#[test]
fn denon_control_prints_help() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("denon-control")?;
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Usage"));

    Ok(())
}

#[test]
fn denon_control_fails_to_connect() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("denon-control")?;
    cmd.arg("--address").arg("localhost");
    cmd.assert()
        .failure()
        .stdout(predicate::str::contains("using receiver: localhost:"))
        .stderr(predicate::str::contains("Connection refused"));

    Ok(())
}

#[test]
fn denon_control_connects_to_test_receiver() -> Result<(), Box<dyn std::error::Error>> {
    let listen_socket = TcpListener::bind("localhost:0")?;
    let local_port = listen_socket.local_addr()?.port();

    let mut cmd = Command::cargo_bin("denon-control")?;
    cmd.arg("--address")
        .arg(format!("localhost:{}", local_port));
    cmd.assert()
        .success()
        .stdout(contains("using receiver: localhost:"));

    Ok(())
}

#[test]
fn denon_control_loses_connection() -> Result<(), Box<dyn std::error::Error>> {
    let listen_socket = TcpListener::bind("localhost:0")?;
    let local_port = listen_socket.local_addr()?.port();
    let mut cmd = Command::cargo_bin("denon-control")?;

    let acceptor = thread::spawn(move || -> Result<(), io::Error> {
        let mut to_receiver = listen_socket.accept()?.0;
        let mut buf = [0; 100];
        to_receiver.read(&mut buf)?;
        Ok(())
    });

    cmd.arg("--address")
        .arg(format!("localhost:{}", local_port))
        .arg("--status");
    cmd.assert().failure().stderr(contains("Error: IO"));

    let _ = acceptor.join().unwrap()?;

    Ok(())
}

#[parameterized(power = {"STANDBY", "ON", "ON", "STANDBY"},
                input = {"TUNER", "NET/USB", "BD", "DVD"},
                volume = {200, 300, 0, 100},
                max_volume = {333, 230, 666, 110}
            )]
fn denon_control_queries_receiver_state_and_gets_state_one_by_one(
    power: &str,
    input: &str,
    volume: u16,
    max_volume: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let listen_socket = TcpListener::bind("localhost:0")?;
    let local_port = listen_socket.local_addr()?.port();
    let mut cmd = Command::cargo_bin("denon-control")?;

    let acceptor = thread::spawn(move || -> Result<(TcpStream, Vec<String>), io::Error> {
        let mut to_receiver = listen_socket.accept()?.0;
        let mut received_data = read(&mut to_receiver, 1)?;
        write_string(&mut to_receiver, format!("PW{}\r", power))?;
        received_data.append(&mut read(&mut to_receiver, 1)?);
        write_string(&mut to_receiver, format!("SI{}\r", input))?;
        received_data.append(&mut read(&mut to_receiver, 1)?);
        write_string(&mut to_receiver, format!("MV{}\r", volume))?;
        received_data.append(&mut read(&mut to_receiver, 1)?);
        write_string(&mut to_receiver, format!("MVMAX{}\r", max_volume))?;
        Ok((to_receiver, received_data))
    });

    let expected = format!("Current status of receiver:\n\tPower({})\n\tSourceInput({})\n\tMainVolume({})\n\tMaxVolume({})\n", power, input, volume, max_volume);

    cmd.arg("--address")
        .arg(format!("localhost:{}", local_port))
        .arg("--status");
    cmd.assert().success().stdout(contains(expected));

    let (_, received_data) = acceptor.join().unwrap()?;

    assert!(received_data.contains(&String::from("PW?")));
    assert!(received_data.contains(&String::from("SI?")));
    assert!(received_data.contains(&String::from("MV?")));
    assert!(received_data.contains(&String::from("MVMAX?")));

    Ok(())
}

#[parameterized(power = {"ON", "ON", "STANDBY", "STANDBY"},
                input = {"BD", "DVD", "TUNER", "NET/USB"},
                volume = {0, 100, 200, 300},
                max_volume = {230, 110, 666, 333}
            )]
fn denon_control_queries_receiver_state_and_gets_all_states_at_once(
    power: &str,
    input: &str,
    volume: u16,
    max_volume: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let listen_socket = TcpListener::bind("localhost:0")?;
    let local_port = listen_socket.local_addr()?.port();
    let mut cmd = Command::cargo_bin("denon-control")?;

    let acceptor = thread::spawn(move || -> Result<(TcpStream, Vec<String>), io::Error> {
        let mut to_receiver = listen_socket.accept()?.0;
        let received_data = read(&mut to_receiver, 1)?;
        let response = format!(
            "PW{}\rSI{}\rMV{}\rMVMAX{}\r",
            power, input, volume, max_volume
        );
        write_string(&mut to_receiver, response)?;

        Ok((to_receiver, received_data))
    });

    let expected = format!("Current status of receiver:\n\tPower({})\n\tSourceInput({})\n\tMainVolume({})\n\tMaxVolume({})\n", power, input, volume, max_volume);

    cmd.arg("--address")
        .arg(format!("localhost:{}", local_port))
        .arg("--status");
    cmd.assert().success().stdout(contains(expected));

    let (_, received_data) = acceptor.join().unwrap()?;

    assert!(received_data.contains(&String::from("PW?")));

    Ok(())
}

#[test]
fn denon_control_sets_receiver_state() -> Result<(), Box<dyn std::error::Error>> {
    let (acceptor, local_port) = create_acceptor_thread()?;
    let mut cmd = Command::cargo_bin("denon-control")?;

    cmd.arg("--address")
        .arg(format!("localhost:{}", local_port))
        .arg("--power")
        .arg("STANDBY")
        .arg("--input")
        .arg("CD")
        .arg("--volume")
        .arg("127");
    cmd.assert().success();

    let mut to_receiver = acceptor.join().unwrap()?;
    let received_data = read(&mut to_receiver, 10)?;

    assert!(received_data.contains(&String::from("SICD")));
    assert!(received_data.contains(&String::from("MV50")));
    assert!(received_data.contains(&String::from("PWSTANDBY")));

    Ok(())
}

#[parameterized(power = {"ON", "STANDBY"})]
fn denon_control_sets_power(power: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (acceptor, local_port) = create_acceptor_thread()?;
    let mut cmd = Command::cargo_bin("denon-control")?;

    cmd.arg("--address")
        .arg(format!("localhost:{}", local_port))
        .arg("--power")
        .arg(power);
    cmd.assert().success();

    let mut to_receiver = acceptor.join().unwrap()?;
    let received_data = read(&mut to_receiver, 10)?;

    assert!(received_data.contains(&format!("PW{}", power)));

    Ok(())
}

#[parameterized(source_input = {"CD", "DVD", "BD", "NET/USB"})]
fn denon_control_sets_source_input(source_input: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (acceptor, local_port) = create_acceptor_thread()?;
    let mut cmd = Command::cargo_bin("denon-control")?;

    cmd.arg("--address")
        .arg(format!("localhost:{}", local_port))
        .arg("--input")
        .arg(source_input);
    cmd.assert().success();

    let mut to_receiver = acceptor.join().unwrap()?;
    let received_data = read(&mut to_receiver, 10)?;

    assert!(received_data.contains(&format!("SI{}", source_input)));

    Ok(())
}

#[parameterized(volume = {0, 1, 50})]
fn denon_control_sets_volume(volume: u16) -> Result<(), Box<dyn std::error::Error>> {
    let (acceptor, local_port) = create_acceptor_thread()?;
    let mut cmd = Command::cargo_bin("denon-control")?;

    cmd.arg("--address")
        .arg(format!("localhost:{}", local_port))
        .arg("--volume")
        .arg(volume.to_string());
    cmd.assert().success();

    let mut to_receiver = acceptor.join().unwrap()?;
    let received_data = read(&mut to_receiver, 10)?;

    assert!(received_data.contains(&format!("MV{}", volume)));

    Ok(())
}

#[parameterized(volume = {50, 51, 100, 999})]
fn denon_control_caps_higher_volumes_to_50(volume: u16) -> Result<(), Box<dyn std::error::Error>> {
    let (acceptor, local_port) = create_acceptor_thread()?;
    let mut cmd = Command::cargo_bin("denon-control")?;

    cmd.arg("--address")
        .arg(format!("localhost:{}", local_port))
        .arg("--volume")
        .arg(volume.to_string());
    cmd.assert().success();

    let mut to_receiver = acceptor.join().unwrap()?;
    let received_data = read(&mut to_receiver, 10)?;

    assert!(received_data.contains(&String::from("MV50")));

    Ok(())
}
