use chrono::{DateTime, SecondsFormat};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct SystemTimestamp;

impl Timestamp for SystemTimestamp {
    fn now(&mut self) -> Duration {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("failed to get system time")
    }
}

pub trait Timestamp {
    fn now(&mut self) -> Duration;

    fn as_secs(&mut self) -> u64 {
        self.now().as_secs()
    }

    fn as_rfc3339(&mut self) -> String {
        let duration = self.now();
        let secs = duration.as_secs();
        let nanos = duration.subsec_nanos();
        let datetime = DateTime::from_timestamp(secs as i64, nanos).unwrap();

        datetime.to_rfc3339_opts(SecondsFormat::Millis, true)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub struct TestTimestamp;

    impl Timestamp for TestTimestamp {
        fn now(&mut self) -> Duration {
            Duration::from_secs(1_000_000_000)
        }
    }

    pub const EXPECTED_SECS: u64 = 1_000_000_000;
    pub const EXPECTED_RFC3339: &str = "2001-09-09T01:46:40.000Z";
}
