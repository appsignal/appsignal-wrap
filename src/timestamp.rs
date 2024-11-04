use chrono::{DateTime, SecondsFormat};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn duration() -> Duration {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap()
}

pub fn as_secs() -> u64 {
    duration().as_secs()
}

pub fn as_rfc3339() -> String {
    let duration = duration();
    let secs = duration.as_secs();
    let nanos = duration.subsec_nanos();
    let datetime = DateTime::from_timestamp(secs as i64, nanos).unwrap();

    datetime.to_rfc3339_opts(SecondsFormat::Millis, true)
}
