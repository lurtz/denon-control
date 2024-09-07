use std::cmp::{Eq, PartialEq};
use std::fmt::{Display, Error, Formatter, Write};
use std::hash::Hash;
use std::slice::Iter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerState {
    On,
    Standby,
}

impl Display for PowerState {
    fn fmt(&self, format: &mut Formatter) -> Result<(), Error> {
        let mut buffer = String::new();
        write!(&mut buffer, "{:?}", self)?;
        write!(format, "{}", buffer.to_ascii_uppercase())
    }
}

impl PowerState {
    pub fn iterator() -> Iter<'static, PowerState> {
        static STATES: [PowerState; 2] = [PowerState::On, PowerState::Standby];
        STATES.iter()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceInputState {
    Cd,
    Tuner,
    Dvd,
    Bd,
    Tv,
    Satcbl,
    Game,
    Game2,
    Vaux,
    Dock,
    Ipod,
    Netusb,
    Rhapsody,
    Napster,
    Pandora,
    Lastfm,
    Flickr,
    Favorites,
    Iradio,
    Server,
    Usbipod,
    Usb,
    Ipd,
    Irp,
    Fvp,
    Unknown,
}

impl Display for SourceInputState {
    fn fmt(&self, format: &mut Formatter) -> Result<(), Error> {
        let mut buffer = String::new();
        match *self {
            SourceInputState::Netusb => write!(&mut buffer, "NET/USB")?,
            SourceInputState::Usbipod => write!(&mut buffer, "USB/IPOD")?,
            _ => write!(&mut buffer, "{:?}", self)?,
        }
        write!(format, "{}", buffer.to_ascii_uppercase())
    }
}

impl SourceInputState {
    pub fn iterator() -> Iter<'static, SourceInputState> {
        static STATES: [SourceInputState; 25] = [
            SourceInputState::Cd,
            SourceInputState::Tuner,
            SourceInputState::Dvd,
            SourceInputState::Bd,
            SourceInputState::Tv,
            SourceInputState::Satcbl,
            SourceInputState::Game,
            SourceInputState::Game2,
            SourceInputState::Vaux,
            SourceInputState::Dock,
            SourceInputState::Ipod,
            SourceInputState::Netusb,
            SourceInputState::Rhapsody,
            SourceInputState::Napster,
            SourceInputState::Pandora,
            SourceInputState::Lastfm,
            SourceInputState::Flickr,
            SourceInputState::Favorites,
            SourceInputState::Iradio,
            SourceInputState::Server,
            SourceInputState::Usbipod,
            SourceInputState::Usb,
            SourceInputState::Ipd,
            SourceInputState::Irp,
            SourceInputState::Fvp,
        ];
        STATES.iter()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum State {
    Power,
    SourceInput,
    MaxVolume,
    MainVolume,
}

impl Display for State {
    fn fmt(&self, format: &mut Formatter) -> Result<(), Error> {
        let val = match *self {
            State::Power => "PW",
            State::SourceInput => "SI",
            State::MaxVolume => "MVMAX",
            State::MainVolume => "MV",
        };
        write!(format, "{}", val)
    }
}

#[allow(dead_code)] // currently MaxVolume is not used, but supported
#[derive(Debug, PartialEq)]
pub enum SetState {
    Power(PowerState),
    SourceInput(SourceInputState),
    MaxVolume(u32),
    MainVolume(u32),
}

impl SetState {
    pub fn convert(&self) -> (State, StateValue) {
        match *self {
            SetState::MainVolume(i) => (State::MainVolume, StateValue::Integer(i)),
            SetState::MaxVolume(i) => (State::MaxVolume, StateValue::Integer(i)),
            SetState::Power(ps) => (State::Power, StateValue::Power(ps)),
            SetState::SourceInput(si) => (State::SourceInput, StateValue::SourceInput(si)),
        }
    }
}

impl Display for SetState {
    fn fmt(&self, format: &mut Formatter) -> Result<(), Error> {
        let (state, value) = self.convert();
        write!(format, "{}{}", state, value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateValue {
    Power(PowerState),
    SourceInput(SourceInputState),
    Integer(u32),
    Unknown,
}

impl Display for StateValue {
    fn fmt(&self, format: &mut Formatter) -> Result<(), Error> {
        match *self {
            StateValue::Power(ref p) => write!(format, "{}", p),
            StateValue::SourceInput(ref si) => write!(format, "{}", si),
            StateValue::Integer(i) => write!(format, "{}", i),
            StateValue::Unknown => Ok(()),
        }
    }
}

#[cfg(test)]
mod test {
    use super::StateValue;
    use crate::state::{PowerState, SetState, SourceInputState, State};
    use std::collections::HashMap;

    fn check_value(hs: &HashMap<State, StateValue>, key: &State, expected_value: &StateValue) {
        let value = hs.get(key).unwrap();
        assert!(matches!(value, value if value == expected_value));
    }

    #[test]
    fn state_works_in_map() {
        let mut hm = HashMap::new();
        let mv = State::MainVolume;
        let i100 = StateValue::Integer(100);
        hm.insert(mv, i100);
        assert_eq!(1, hm.len());
        check_value(&hm, &mv, &i100);

        let i129 = StateValue::Integer(129);
        hm.insert(mv, i129);
        assert_eq!(1, hm.len());
        check_value(&hm, &mv, &i129);

        let maxv = State::MaxVolume;
        hm.insert(maxv, i100);
        assert_eq!(2, hm.len());
        check_value(&hm, &mv, &i129);
        check_value(&hm, &maxv, &i100);

        let power = State::Power;
        let pon = StateValue::Power(PowerState::On);
        hm.insert(power, pon);
        assert_eq!(3, hm.len());
        check_value(&hm, &mv, &i129);
        check_value(&hm, &maxv, &i100);
        check_value(&hm, &power, &pon);

        let si = State::SourceInput;
        let sibd = StateValue::SourceInput(SourceInputState::Bd);
        hm.insert(si, sibd);
        assert_eq!(4, hm.len());
        check_value(&hm, &mv, &i129);
        check_value(&hm, &maxv, &i100);
        check_value(&hm, &power, &pon);
        check_value(&hm, &si, &sibd);
    }

    #[test]
    fn power_state_iterator() {
        let mut piter = PowerState::iterator();
        assert!(matches!(piter.next(), Some(PowerState::On)));
        assert!(matches!(piter.next(), Some(PowerState::Standby)));
        assert!(matches!(piter.next(), None));
    }

    #[test]
    fn power_state_display() {
        assert_eq!("ON", PowerState::On.to_string());
        assert_eq!("STANDBY", PowerState::Standby.to_string());
    }

    #[test]
    fn source_input_state_display() {
        assert_eq!("DVD", SourceInputState::Dvd.to_string());
        assert_eq!("FLICKR", SourceInputState::Flickr.to_string());
    }

    #[test]
    fn state_display() {
        assert_eq!("MV", State::MainVolume.to_string());
        assert_eq!("MVMAX", State::MaxVolume.to_string());
        assert_eq!("PW", State::Power.to_string());
        assert_eq!("SI", State::SourceInput.to_string());
    }

    #[test]
    fn state_statevalue_display() {
        let ts = |s, sv| format!("{}{}", s, sv);
        assert_eq!("MV230", ts(State::MainVolume, StateValue::Integer(230)));
        assert_eq!("MVMAX666", ts(State::MaxVolume, StateValue::Integer(666)));
        assert_eq!("PWON", ts(State::Power, StateValue::Power(PowerState::On)));
        assert_eq!(
            "SIDVD",
            ts(
                State::SourceInput,
                StateValue::SourceInput(SourceInputState::Dvd)
            )
        );
        assert_eq!("PW", ts(State::Power, StateValue::Unknown));
    }

    #[test]
    fn setstate_diplay() {
        assert_eq!("MV230", SetState::MainVolume(230).to_string());
        assert_eq!("MVMAX666", SetState::MaxVolume(666).to_string());
        assert_eq!("PWON", SetState::Power(PowerState::On).to_string());
        assert_eq!(
            "SIDVD",
            SetState::SourceInput(SourceInputState::Dvd).to_string()
        );
    }
}
