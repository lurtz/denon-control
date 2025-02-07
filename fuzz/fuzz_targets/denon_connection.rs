#![no_main]

use denon_control::create_connected_connection;
use denon_control::State;
use libfuzzer_sys::fuzz_target;
use std::io::Write;

fuzz_target!(|data: &[u8]| {
    let (mut to_receiver, mut _dc) = create_connected_connection().unwrap();
    if data.len() <= 1 {
        let _ = to_receiver.write_all(data);
        return;
    }

    let action = data[0] % 4;

    let _ = to_receiver.write_all(&data[1..]);
    let _get_result = match action {
        0 => _dc.get(State::MainVolume),
        1 => _dc.get(State::MaxVolume),
        2 => _dc.get(State::Power),
        _ => _dc.get(State::SourceInput),
    };
    println!("result == {:?}", _get_result);
});
