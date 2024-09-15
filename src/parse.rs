use crate::state::get_state;
use crate::state::SetState;
use crate::state::{PowerState, SourceInputState, State};

macro_rules! parsehelper {
    ($trimmed:expr, $op:expr, $ss:expr, $func:path) => {
        if $trimmed.starts_with($op.to_string().as_str()) {
            let value = get_value($trimmed, &$op);
            return $func(value).and_then(|x| Some($ss(x)));
        }
    };
}

fn get_value<'a>(trimmed: &'a str, op: &State) -> &'a str {
    let to_skip = op.to_string().len();
    trimmed[to_skip..].trim()
}

fn parse_int(to_parse: &str) -> Option<u32> {
    let value = to_parse.parse::<u32>();
    value.ok().and_then(|mut v| {
        if v < 100 {
            v *= 10;
        }
        Some(v)
    })
}

fn parse_power(value: &str) -> Option<PowerState> {
    let ps = get_state(PowerState::states(), value);
    ps.ok()
}

fn parse_source_input(value: &str) -> Option<SourceInputState> {
    let sis = get_state(SourceInputState::states(), value);
    sis.ok()
}

pub fn parse(str: &str) -> Option<SetState> {
    let trimmed = str.trim().trim_matches('\r');
    parsehelper!(trimmed, State::MaxVolume, SetState::MaxVolume, parse_int);
    parsehelper!(trimmed, State::MainVolume, SetState::MainVolume, parse_int);
    parsehelper!(trimmed, State::Power, SetState::Power, parse_power);
    parsehelper!(
        trimmed,
        State::SourceInput,
        SetState::SourceInput,
        parse_source_input
    );
    None
}

#[cfg(test)]
mod test {
    use super::parse;
    use crate::{
        parse::{PowerState, SourceInputState},
        state::SetState,
    };

    #[test]
    fn parse_with_unknown_string() {
        assert_eq!(None, parse(""));
        assert_eq!(None, parse("blub"));
    }

    #[test]
    fn max_volume_without_value_returns_none() {
        assert_eq!(None, parse("MVMAX"));
        assert_eq!(None, parse("MVMAXfda"));
        assert_eq!(None, parse("MVMAXđðſæ"));
        assert_eq!(None, parse("MVMAX&%"));
        assert_eq!(None, parse("MVMAX!"));
    }

    #[test]
    fn max_volume() {
        let create = |i| Some(SetState::MaxVolume(i));

        assert_eq!(parse("MVMAX0"), create(0));
        assert_eq!(parse("MVMAX23"), create(230));
        assert_eq!(parse("MVMAX99"), create(990));
        assert_eq!(parse("MVMAX100"), create(100));
        assert_eq!(parse("MVMAX230"), create(230));
        assert_eq!(parse("MVMAX999"), create(999));
        assert_eq!(parse("MVMAX 999"), create(999));
    }

    #[test]
    fn main_volume_without_value_returns_none() {
        assert_eq!(None, parse("MV"));
        assert_eq!(None, parse("MVŧ¶ŋđ"));
        assert_eq!(None, parse("MV»«"));
        assert_eq!(None, parse("MV²!"));
    }

    #[test]
    fn main_volume() {
        let create = |i| Some(SetState::MainVolume(i));

        assert_eq!(parse("MV 0"), create(0));
        assert_eq!(parse("MV 23"), create(230));
        assert_eq!(parse("MV 99"), create(990));
        assert_eq!(parse("MV 100"), create(100));
        assert_eq!(parse("MV 230"), create(230));
        assert_eq!(parse("MV 999"), create(999));
        assert_eq!(parse("MV999"), create(999));
    }

    #[test]
    fn power() {
        let create = |ps| Some(SetState::Power(ps));

        assert_eq!(parse("PW"), None);
        assert_eq!(parse("PWđðæſ"), None);
        assert_eq!(parse("PWfdasfdas"), None);
        assert_eq!(parse("PWOFF"), None);
        assert_eq!(parse("PWSTANDBY"), create(PowerState::Standby));
        assert_eq!(parse("PWON"), create(PowerState::On));
    }

    #[test]
    fn source_input() {
        let create = |si| Some(SetState::SourceInput(si));

        assert_eq!(parse("SI"), None);
        assert_eq!(parse("SIblub"), None);
        assert_eq!(parse("SITV"), create(SourceInputState::Tv));
    }
}
