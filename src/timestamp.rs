use chrono::{DateTime, SecondsFormat};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy)]
pub struct SystemTimestamp;

impl Timestamp for SystemTimestamp {
    fn now(&mut self) -> Duration {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("failed to get system time")
    }
}

const MONOTONIC_GAP: Duration = Duration::from_millis(1);

// This works around an issue with the logging feature, where timestamps only
// have millisecond precision. This can cause issues when multiple logs are
// written in the same millisecond, as they will have the same timestamp and
// they will be displayed out of order in the UI.

// The monotonic timestamp prevents this issue by ensuring that the
// timestamps returned between two successive calls are at least one
// millisecond apart. This means, however, that the timestamps may not
// accurately reflect the times at which the logs were written.
pub struct MonotonicTimestamp<T: Timestamp> {
    last: Option<Duration>,
    source: T,
}

impl<T: Timestamp> MonotonicTimestamp<T> {
    pub fn new(source: T) -> Self {
        Self { last: None, source }
    }

    #[cfg(test)]
    pub fn swap(&mut self, source: T) {
        self.source = source;
    }
}

impl<T: Timestamp> Timestamp for MonotonicTimestamp<T> {
    fn now(&mut self) -> Duration {
        let now = self.source.now();

        self.last = Some(match self.last {
            Some(last) => match now.checked_sub(last) {
                Some(diff) if diff > MONOTONIC_GAP => now,
                _ => last + MONOTONIC_GAP,
            },
            None => now,
        });

        self.last.unwrap()
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

    pub struct TestTimestamp(Duration);

    impl Timestamp for TestTimestamp {
        fn now(&mut self) -> Duration {
            self.0
        }
    }

    pub fn timestamp() -> TestTimestamp {
        TestTimestamp(Duration::from_secs(1_000_000_000))
    }

    pub const EXPECTED_SECS: u64 = 1_000_000_000;
    pub const EXPECTED_RFC3339: &str = "2001-09-09T01:46:40.000Z";

    #[test]
    fn monotonic_timestamp() {
        // If the source time stays the same between calls,
        // it should increase monotonically from the last time by the gap.
        let mut monotonic = MonotonicTimestamp::new(TestTimestamp(Duration::from_millis(1_000)));

        assert_eq!(monotonic.now().as_millis(), 1_000);
        assert_eq!(monotonic.now().as_millis(), 1_001);
        assert_eq!(monotonic.now().as_millis(), 1_002);

        // If the source time is greater than the last time,
        // it should increase to the new source time.
        monotonic.swap(TestTimestamp(Duration::from_millis(1_500)));

        assert_eq!(monotonic.now().as_millis(), 1_500);
        assert_eq!(monotonic.now().as_millis(), 1_501);
        assert_eq!(monotonic.now().as_millis(), 1_502);

        // If the source time is smaller than the last time,
        // it should increase monotonically from the last time by the gap.
        monotonic.swap(TestTimestamp(Duration::from_millis(1_000)));

        assert_eq!(monotonic.now().as_millis(), 1_503);
        assert_eq!(monotonic.now().as_millis(), 1_504);
        assert_eq!(monotonic.now().as_millis(), 1_505);

        // Source is one gap below the last time (two gaps below the next time)
        monotonic.swap(TestTimestamp(Duration::from_millis(1_504)));
        assert_eq!(monotonic.now().as_millis(), 1_506);

        // Source is at the last time (one gap below the next time)
        monotonic.swap(TestTimestamp(Duration::from_millis(1_506)));
        assert_eq!(monotonic.now().as_millis(), 1_507);

        // Source is one gap above the last time (at the next time)
        monotonic.swap(TestTimestamp(Duration::from_millis(1_508)));
        assert_eq!(monotonic.now().as_millis(), 1_508);

        // Source is two gaps above the last time (one gap above the next time)
        monotonic.swap(TestTimestamp(Duration::from_millis(1_510)));
        assert_eq!(monotonic.now().as_millis(), 1_510);
    }
}
