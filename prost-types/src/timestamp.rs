use super::*;
use core::ops::{Add, Sub};

impl Timestamp {
    /// Normalizes the timestamp to a canonical format.
    ///
    /// Based on [`google::protobuf::util::CreateNormalized`][1].
    ///
    /// [1]: https://github.com/google/protobuf/blob/v3.3.2/src/google/protobuf/util/time_util.cc#L59-L77
    pub fn normalize(&mut self) {
        // Make sure nanos is in the range.
        if self.nanos <= -NANOS_PER_SECOND || self.nanos >= NANOS_PER_SECOND {
            if let Some(seconds) = self
                .seconds
                .checked_add((self.nanos / NANOS_PER_SECOND) as i64)
            {
                self.seconds = seconds;
                self.nanos %= NANOS_PER_SECOND;
            } else if self.nanos < 0 {
                // Negative overflow! Set to the earliest normal value.
                self.seconds = i64::MIN;
                self.nanos = 0;
            } else {
                // Positive overflow! Set to the latest normal value.
                self.seconds = i64::MAX;
                self.nanos = 999_999_999;
            }
        }

        // For Timestamp nanos should be in the range [0, 999999999].
        if self.nanos < 0 {
            if let Some(seconds) = self.seconds.checked_sub(1) {
                self.seconds = seconds;
                self.nanos += NANOS_PER_SECOND;
            } else {
                // Negative overflow! Set to the earliest normal value.
                debug_assert_eq!(self.seconds, i64::MIN);
                self.nanos = 0;
            }
        }

        // TODO: should this be checked?
        // debug_assert!(self.seconds >= -62_135_596_800 && self.seconds <= 253_402_300_799,
        //               "invalid timestamp: {:?}", self);
    }

    /// Normalizes the timestamp to a canonical format, returning the original value if it cannot be
    /// normalized.
    ///
    /// Normalization is based on [`google::protobuf::util::CreateNormalized`][1].
    ///
    /// [1]: https://github.com/google/protobuf/blob/v3.3.2/src/google/protobuf/util/time_util.cc#L59-L77
    pub fn try_normalize(mut self) -> Result<Timestamp, Timestamp> {
        let before = self;
        self.normalize();
        // If the seconds value has changed, and is either i64::MIN or i64::MAX, then the timestamp
        // normalization overflowed.
        if (self.seconds == i64::MAX || self.seconds == i64::MIN) && self.seconds != before.seconds
        {
            Err(before)
        } else {
            Ok(self)
        }
    }

    /// Return a normalized copy of the timestamp to a canonical format.
    ///
    /// Based on [`google::protobuf::util::CreateNormalized`][1].
    ///
    /// [1]: https://github.com/google/protobuf/blob/v3.3.2/src/google/protobuf/util/time_util.cc#L59-L77
    pub fn normalized(&self) -> Self {
        let mut result = *self;
        result.normalize();
        result
    }

    /// Creates a new `Timestamp` at the start of the provided UTC date.
    pub fn date(year: i64, month: u8, day: u8) -> Result<Timestamp, TimestampError> {
        Timestamp::date_time_nanos(year, month, day, 0, 0, 0, 0)
    }

    /// Creates a new `Timestamp` instance with the provided UTC date and time.
    pub fn date_time(
        year: i64,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
    ) -> Result<Timestamp, TimestampError> {
        Timestamp::date_time_nanos(year, month, day, hour, minute, second, 0)
    }

    /// Creates a new `Timestamp` instance with the provided UTC date and time.
    pub fn date_time_nanos(
        year: i64,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
        nanos: u32,
    ) -> Result<Timestamp, TimestampError> {
        let date_time = datetime::DateTime {
            year,
            month,
            day,
            hour,
            minute,
            second,
            nanos,
        };

        Timestamp::try_from(date_time)
    }

    /// The latest timestamp in the documented range: `9999-12-31T23:59:59.999999999Z`.
    pub const MAX: Timestamp = Timestamp {
        seconds: 253_402_300_799,
        nanos: NANOS_PER_SECOND - 1,
    };

    /// The earliest timestamp in the documented range: `0001-01-01T00:00:00Z`.
    pub const MIN: Timestamp = Timestamp {
        seconds: -62_135_596_800,
        nanos: 0,
    };

    /// Returns `Some(t)` where `t` is the normalized timestamp `self + duration`, or `None` if
    /// the result can't be represented.
    ///
    /// Neither `self` nor `duration` needs to be normalized.
    pub fn checked_add(&self, duration: Duration) -> core::option::Option<Timestamp> {
        Self::checked_from_parts(
            i128::from(self.seconds) + i128::from(duration.seconds),
            i64::from(self.nanos) + i64::from(duration.nanos),
        )
    }

    /// Returns `Some(t)` where `t` is the normalized timestamp `self - duration`, or `None` if
    /// the result can't be represented.
    ///
    /// Neither `self` nor `duration` needs to be normalized.
    pub fn checked_sub(&self, duration: Duration) -> core::option::Option<Timestamp> {
        Self::checked_from_parts(
            i128::from(self.seconds) - i128::from(duration.seconds),
            i64::from(self.nanos) - i64::from(duration.nanos),
        )
    }

    /// Builds a normalized timestamp from a sum of seconds and a sum of nanos.
    ///
    /// The wider types make the sums exact: they can't overflow, so the only failure is a
    /// result whose seconds don't fit in `i64`.
    fn checked_from_parts(seconds: i128, nanos: i64) -> core::option::Option<Timestamp> {
        let nanos_per_second = i64::from(NANOS_PER_SECOND);
        let seconds = seconds + i128::from(nanos.div_euclid(nanos_per_second));
        Some(Timestamp {
            seconds: i64::try_from(seconds).ok()?,
            nanos: nanos.rem_euclid(nanos_per_second) as i32,
        })
    }
}

impl Add<Duration> for Timestamp {
    type Output = Timestamp;

    /// # Panics
    ///
    /// This function may panic if the resulting point in time cannot be represented by the
    /// underlying data structure. See [`Timestamp::checked_add`] for a version without panic.
    fn add(self, duration: Duration) -> Timestamp {
        self.checked_add(duration)
            .expect("overflow when adding duration to timestamp")
    }
}

impl Sub<Duration> for Timestamp {
    type Output = Timestamp;

    /// # Panics
    ///
    /// This function may panic if the resulting point in time cannot be represented by the
    /// underlying data structure. See [`Timestamp::checked_sub`] for a version without panic.
    fn sub(self, duration: Duration) -> Timestamp {
        self.checked_sub(duration)
            .expect("overflow when subtracting duration from timestamp")
    }
}

/// Asserts that `result` is the normalized sum of `seconds` and `nanos`, or `None` when that sum
/// can't be represented by a [`Timestamp`].
///
/// Unlike the implementation, this uses no division: `nanos` is a sum or difference of two
/// `i32`, so it lies in `[-2^32, 2^32]` and the carry into seconds is one of `-5..=4`. Exactly
/// one carry leaves the nanos in `0..NANOS_PER_SECOND`. Avoiding `i128` multiplication and
/// division keeps the Kani proofs fast enough to finish.
#[cfg(any(test, kani))]
fn assert_carry_search(result: core::option::Option<Timestamp>, seconds: i128, nanos: i64) {
    let nanos_per_second = i64::from(NANOS_PER_SECOND);
    for carry in -5..=4 {
        let normalized_nanos = nanos - carry * nanos_per_second;
        if (0..nanos_per_second).contains(&normalized_nanos) {
            let expected = i64::try_from(seconds + i128::from(carry))
                .ok()
                .map(|seconds| Timestamp {
                    seconds,
                    nanos: normalized_nanos as i32,
                });
            assert_eq!(result, expected);
        }
    }
}

/// Asserts that `result` is the normalized form of `exact` total nanoseconds, or `None` when
/// `exact` can't be represented by a [`Timestamp`].
#[cfg(test)]
fn assert_exact_result(result: core::option::Option<Timestamp>, exact: i128) {
    let nanos_per_second = i128::from(NANOS_PER_SECOND);
    let representable = exact >= i128::from(i64::MIN) * nanos_per_second
        && exact < (i128::from(i64::MAX) + 1) * nanos_per_second;
    match result {
        Some(timestamp) => {
            assert!(representable);
            assert!((0..NANOS_PER_SECOND).contains(&timestamp.nanos));
            assert_eq!(
                i128::from(timestamp.seconds) * nanos_per_second + i128::from(timestamp.nanos),
                exact
            );
        }
        None => assert!(!representable),
    }
}

#[cfg(test)]
mod tests_ops {
    use super::*;
    use prost::alloc::string::ToString;

    const NANOS: i128 = NANOS_PER_SECOND as i128;

    const EDGE_SECONDS: [i64; 7] = [i64::MIN, i64::MIN + 1, -1, 0, 1, i64::MAX - 1, i64::MAX];

    const EDGE_NANOS: [i32; 9] = [
        i32::MIN,
        -NANOS_PER_SECOND,
        -NANOS_PER_SECOND + 1,
        -1,
        0,
        1,
        NANOS_PER_SECOND - 1,
        NANOS_PER_SECOND,
        i32::MAX,
    ];

    fn edge_values() -> impl Iterator<Item = (i64, i32)> {
        EDGE_SECONDS
            .into_iter()
            .flat_map(|seconds| EDGE_NANOS.into_iter().map(move |nanos| (seconds, nanos)))
    }

    #[test]
    fn test_min_max_match_documented_range() {
        assert_eq!(Timestamp::MIN.to_string(), "0001-01-01T00:00:00Z");
        assert_eq!(Timestamp::MAX.to_string(), "9999-12-31T23:59:59.999999999Z");
    }

    #[test]
    fn test_add_simple() {
        let ts = Timestamp {
            seconds: 10,
            nanos: 100,
        };
        let dur = Duration {
            seconds: 5,
            nanos: 200,
        };
        assert_eq!(
            ts + dur,
            Timestamp {
                seconds: 15,
                nanos: 300
            }
        );
    }

    #[test]
    fn test_add_nanos_overflow() {
        let ts = Timestamp {
            seconds: 10,
            nanos: 800_000_000,
        };
        let dur = Duration {
            seconds: 1,
            nanos: 300_000_000,
        };
        assert_eq!(
            ts + dur,
            Timestamp {
                seconds: 12,
                nanos: 100_000_000
            }
        );
    }

    #[test]
    fn test_add_nanos_overflow_i32_min() {
        let ts = Timestamp {
            seconds: 0,
            nanos: i32::MIN,
        };
        let dur = Duration {
            seconds: 0,
            nanos: i32::MIN,
        };
        assert_eq!(
            ts + dur,
            Timestamp {
                seconds: -5,
                nanos: 705_032_704
            }
        );
    }

    #[test]
    fn test_add_nanos_overflow_i32_max() {
        let ts = Timestamp {
            seconds: 0,
            nanos: i32::MAX,
        };
        let dur = Duration {
            seconds: 0,
            nanos: i32::MAX,
        };
        assert_eq!(
            ts + dur,
            Timestamp {
                seconds: 4,
                nanos: 294_967_294
            }
        );
    }

    #[test]
    fn test_add_nanos_overflow_i32_min_minus_one() {
        let ts = Timestamp {
            seconds: 0,
            nanos: i32::MIN,
        };
        let dur = Duration {
            seconds: 0,
            nanos: -1,
        };
        assert_eq!(
            ts + dur,
            Timestamp {
                seconds: -3,
                nanos: 852_516_351
            }
        );
    }

    #[test]
    fn test_add_negative_duration() {
        let ts = Timestamp {
            seconds: 10,
            nanos: 100_000_000,
        };
        let dur = Duration {
            seconds: -2,
            nanos: -200_000_000,
        };
        assert_eq!(
            ts + dur,
            Timestamp {
                seconds: 7,
                nanos: 900_000_000
            }
        );
    }

    #[test]
    fn test_add_seconds_overflow_cancelled_by_nanos() {
        // `i64::MAX + 1` seconds only exists in the intermediate sum; the nanos borrow it back.
        let ts = Timestamp {
            seconds: i64::MAX,
            nanos: -1,
        };
        let dur = Duration {
            seconds: 1,
            nanos: 0,
        };
        assert_eq!(
            ts + dur,
            Timestamp {
                seconds: i64::MAX,
                nanos: 999_999_999
            }
        );
    }

    #[test]
    #[should_panic(expected = "overflow when adding duration to timestamp")]
    fn test_add_overflow_panics() {
        let ts = Timestamp {
            seconds: i64::MAX - 1,
            nanos: 500_000_000,
        };
        let dur = Duration {
            seconds: 10,
            nanos: 0,
        };
        let _ = ts + dur;
    }

    #[test]
    fn test_checked_add_overflow() {
        let ts = Timestamp {
            seconds: i64::MAX,
            nanos: 999_999_999,
        };
        let dur = Duration {
            seconds: 0,
            nanos: 1,
        };
        assert_eq!(ts.checked_add(dur), None);
    }

    #[test]
    fn test_sub_simple() {
        let ts = Timestamp {
            seconds: 15,
            nanos: 300,
        };
        let dur = Duration {
            seconds: 5,
            nanos: 200,
        };
        assert_eq!(
            ts - dur,
            Timestamp {
                seconds: 10,
                nanos: 100
            }
        );
    }

    #[test]
    fn test_sub_nanos_underflow() {
        let ts = Timestamp {
            seconds: 12,
            nanos: 100_000_000,
        };
        let dur = Duration {
            seconds: 1,
            nanos: 300_000_000,
        };
        assert_eq!(
            ts - dur,
            Timestamp {
                seconds: 10,
                nanos: 800_000_000
            }
        );
    }

    #[test]
    fn test_sub_i64_min_duration() {
        // `-i64::MIN` doesn't fit in `i64`, so subtraction can't be implemented by negating
        // the duration.
        let ts = Timestamp {
            seconds: -1,
            nanos: 0,
        };
        let dur = Duration {
            seconds: i64::MIN,
            nanos: 0,
        };
        assert_eq!(
            ts - dur,
            Timestamp {
                seconds: i64::MAX,
                nanos: 0
            }
        );
    }

    #[test]
    #[should_panic(expected = "overflow when subtracting duration from timestamp")]
    fn test_sub_overflow_panics() {
        let ts = Timestamp {
            seconds: i64::MIN,
            nanos: 0,
        };
        let dur = Duration {
            seconds: 0,
            nanos: 1,
        };
        let _ = ts - dur;
    }

    #[test]
    fn test_checked_add_sub_edge_values() {
        for (ts_seconds, ts_nanos) in edge_values() {
            for (dur_seconds, dur_nanos) in edge_values() {
                let ts = Timestamp {
                    seconds: ts_seconds,
                    nanos: ts_nanos,
                };
                let dur = Duration {
                    seconds: dur_seconds,
                    nanos: dur_nanos,
                };
                let ts_total = i128::from(ts_seconds) * NANOS + i128::from(ts_nanos);
                let dur_total = i128::from(dur_seconds) * NANOS + i128::from(dur_nanos);

                assert_exact_result(ts.checked_add(dur), ts_total + dur_total);
                assert_exact_result(ts.checked_sub(dur), ts_total - dur_total);

                // Cross-check the division-free spec used by the Kani proofs.
                let seconds_sum = i128::from(ts_seconds) + i128::from(dur_seconds);
                let seconds_diff = i128::from(ts_seconds) - i128::from(dur_seconds);
                let nanos_sum = i64::from(ts_nanos) + i64::from(dur_nanos);
                let nanos_diff = i64::from(ts_nanos) - i64::from(dur_nanos);
                assert_carry_search(ts.checked_add(dur), seconds_sum, nanos_sum);
                assert_carry_search(ts.checked_sub(dur), seconds_diff, nanos_diff);
            }
        }
    }
}

#[cfg(kani)]
mod proofs_ops {
    use super::*;

    #[kani::proof]
    #[kani::unwind(11)]
    fn check_checked_add() {
        let ts = Timestamp {
            seconds: kani::any(),
            nanos: kani::any(),
        };
        let dur = Duration {
            seconds: kani::any(),
            nanos: kani::any(),
        };

        assert_carry_search(
            ts.checked_add(dur),
            i128::from(ts.seconds) + i128::from(dur.seconds),
            i64::from(ts.nanos) + i64::from(dur.nanos),
        );
    }

    #[kani::proof]
    #[kani::unwind(11)]
    fn check_checked_sub() {
        let ts = Timestamp {
            seconds: kani::any(),
            nanos: kani::any(),
        };
        let dur = Duration {
            seconds: kani::any(),
            nanos: kani::any(),
        };

        assert_carry_search(
            ts.checked_sub(dur),
            i128::from(ts.seconds) - i128::from(dur.seconds),
            i64::from(ts.nanos) - i64::from(dur.nanos),
        );
    }
}

impl Name for Timestamp {
    const PACKAGE: &'static str = PACKAGE;
    const NAME: &'static str = "Timestamp";

    fn type_url() -> String {
        type_url_for::<Self>()
    }
}

#[cfg(feature = "std")]
impl From<std::time::SystemTime> for Timestamp {
    fn from(system_time: std::time::SystemTime) -> Timestamp {
        let (seconds, nanos) = match system_time.duration_since(std::time::UNIX_EPOCH) {
            Ok(duration) => {
                let seconds = i64::try_from(duration.as_secs()).unwrap();
                (seconds, duration.subsec_nanos() as i32)
            }
            Err(error) => {
                let duration = error.duration();
                let seconds = i64::try_from(duration.as_secs()).unwrap();
                let nanos = duration.subsec_nanos() as i32;
                if nanos == 0 {
                    (-seconds, 0)
                } else {
                    (-seconds - 1, 1_000_000_000 - nanos)
                }
            }
        };
        Timestamp { seconds, nanos }
    }
}

/// A timestamp handling error.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum TimestampError {
    /// Indicates that a [`Timestamp`] could not be converted to
    /// [`SystemTime`][std::time::SystemTime] because it is out of range.
    ///
    /// The range of times that can be represented by `SystemTime` depends on the platform. All
    /// `Timestamp`s are likely representable on 64-bit Unix-like platforms, but other platforms,
    /// such as Windows and 32-bit Linux, may not be able to represent the full range of
    /// `Timestamp`s.
    OutOfSystemRange(Timestamp),

    /// An error indicating failure to parse a timestamp in RFC-3339 format.
    ParseFailure,

    /// Indicates an error when constructing a timestamp due to invalid date or time data.
    InvalidDateTime,
}

impl fmt::Display for TimestampError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimestampError::OutOfSystemRange(timestamp) => {
                write!(
                    f,
                    "{timestamp} is not representable as a `SystemTime` because it is out of range",
                )
            }
            TimestampError::ParseFailure => {
                write!(f, "failed to parse RFC-3339 formatted timestamp")
            }
            TimestampError::InvalidDateTime => {
                write!(f, "invalid date or time")
            }
        }
    }
}

impl core::error::Error for TimestampError {}

#[cfg(feature = "std")]
impl TryFrom<Timestamp> for std::time::SystemTime {
    type Error = TimestampError;

    fn try_from(mut timestamp: Timestamp) -> Result<std::time::SystemTime, Self::Error> {
        let orig_timestamp = timestamp;
        timestamp.normalize();

        let system_time = if timestamp.seconds >= 0 {
            std::time::UNIX_EPOCH.checked_add(time::Duration::from_secs(timestamp.seconds as u64))
        } else {
            std::time::UNIX_EPOCH.checked_sub(time::Duration::from_secs(
                timestamp
                    .seconds
                    .checked_neg()
                    .ok_or(TimestampError::OutOfSystemRange(timestamp))? as u64,
            ))
        };

        let system_time = system_time.and_then(|system_time| {
            system_time.checked_add(time::Duration::from_nanos(timestamp.nanos as u64))
        });

        system_time.ok_or(TimestampError::OutOfSystemRange(orig_timestamp))
    }
}

impl FromStr for Timestamp {
    type Err = TimestampError;

    fn from_str(s: &str) -> Result<Timestamp, TimestampError> {
        datetime::parse_timestamp(s).ok_or(TimestampError::ParseFailure)
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        datetime::DateTime::from(*self).fmt(f)
    }
}

#[cfg(kani)]
mod proofs {
    use super::*;

    #[cfg(feature = "std")]
    #[kani::proof]
    #[kani::unwind(3)]
    fn check_timestamp_roundtrip_via_system_time() {
        let seconds = kani::any();
        let nanos = kani::any();

        let mut timestamp = Timestamp { seconds, nanos };
        timestamp.normalize();

        if let Ok(system_time) = std::time::SystemTime::try_from(timestamp) {
            assert_eq!(Timestamp::from(system_time), timestamp);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "std")]
    use proptest::prelude::*;
    #[cfg(feature = "std")]
    use std::time::{self, SystemTime, UNIX_EPOCH};

    #[cfg(feature = "std")]
    proptest! {
        #[test]
        fn check_system_time_roundtrip(
            system_time in SystemTime::arbitrary(),
        ) {
            prop_assert_eq!(SystemTime::try_from(Timestamp::from(system_time)).unwrap(), system_time);
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn check_timestamp_negative_seconds() {
        // Representative tests for the case of timestamps before the UTC Epoch time:
        // validate the expected behaviour that "negative second values with fractions
        // must still have non-negative nanos values that count forward in time"
        // https://protobuf.dev/reference/protobuf/google.protobuf/#timestamp
        //
        // To ensure cross-platform compatibility, all nanosecond values in these
        // tests are in minimum 100 ns increments.  This does not affect the general
        // character of the behaviour being tested, but ensures that the tests are
        // valid for both POSIX (1 ns precision) and Windows (100 ns precision).
        assert_eq!(
            Timestamp::from(UNIX_EPOCH - time::Duration::new(1_001, 0)),
            Timestamp {
                seconds: -1_001,
                nanos: 0
            }
        );
        assert_eq!(
            Timestamp::from(UNIX_EPOCH - time::Duration::new(0, 999_999_900)),
            Timestamp {
                seconds: -1,
                nanos: 100
            }
        );
        assert_eq!(
            Timestamp::from(UNIX_EPOCH - time::Duration::new(2_001_234, 12_300)),
            Timestamp {
                seconds: -2_001_235,
                nanos: 999_987_700
            }
        );
        assert_eq!(
            Timestamp::from(UNIX_EPOCH - time::Duration::new(768, 65_432_100)),
            Timestamp {
                seconds: -769,
                nanos: 934_567_900
            }
        );
    }

    #[cfg(all(unix, feature = "std"))]
    #[test]
    fn check_timestamp_negative_seconds_1ns() {
        // UNIX-only test cases with 1 ns precision
        assert_eq!(
            Timestamp::from(UNIX_EPOCH - time::Duration::new(0, 999_999_999)),
            Timestamp {
                seconds: -1,
                nanos: 1
            }
        );
        assert_eq!(
            Timestamp::from(UNIX_EPOCH - time::Duration::new(1_234_567, 123)),
            Timestamp {
                seconds: -1_234_568,
                nanos: 999_999_877
            }
        );
        assert_eq!(
            Timestamp::from(UNIX_EPOCH - time::Duration::new(890, 987_654_321)),
            Timestamp {
                seconds: -891,
                nanos: 12_345_679
            }
        );
    }

    #[cfg(feature = "std")]
    #[test]
    fn check_timestamp_normalize() {
        // Make sure that `Timestamp::normalize` behaves correctly on and near overflow.
        #[rustfmt::skip] // Don't mangle the table formatting.
        let cases = [
            // --- Table of test cases ---
            //        test seconds      test nanos  expected seconds  expected nanos
            (line!(),            0,              0,                0,              0),
            (line!(),            1,              1,                1,              1),
            (line!(),           -1,             -1,               -2,    999_999_999),
            (line!(),            0,    999_999_999,                0,    999_999_999),
            (line!(),            0,   -999_999_999,               -1,              1),
            (line!(),            0,  1_000_000_000,                1,              0),
            (line!(),            0, -1_000_000_000,               -1,              0),
            (line!(),            0,  1_000_000_001,                1,              1),
            (line!(),            0, -1_000_000_001,               -2,    999_999_999),
            (line!(),           -1,              1,               -1,              1),
            (line!(),            1,             -1,                0,    999_999_999),
            (line!(),           -1,  1_000_000_000,                0,              0),
            (line!(),            1, -1_000_000_000,                0,              0),
            (line!(), i64::MIN    ,              0,     i64::MIN    ,              0),
            (line!(), i64::MIN + 1,              0,     i64::MIN + 1,              0),
            (line!(), i64::MIN    ,              1,     i64::MIN    ,              1),
            (line!(), i64::MIN    ,  1_000_000_000,     i64::MIN + 1,              0),
            (line!(), i64::MIN    , -1_000_000_000,     i64::MIN    ,              0),
            (line!(), i64::MIN + 1, -1_000_000_000,     i64::MIN    ,              0),
            (line!(), i64::MIN + 2, -1_000_000_000,     i64::MIN + 1,              0),
            (line!(), i64::MIN    , -1_999_999_998,     i64::MIN    ,              0),
            (line!(), i64::MIN + 1, -1_999_999_998,     i64::MIN    ,              0),
            (line!(), i64::MIN + 2, -1_999_999_998,     i64::MIN    ,              2),
            (line!(), i64::MIN    , -1_999_999_999,     i64::MIN    ,              0),
            (line!(), i64::MIN + 1, -1_999_999_999,     i64::MIN    ,              0),
            (line!(), i64::MIN + 2, -1_999_999_999,     i64::MIN    ,              1),
            (line!(), i64::MIN    , -2_000_000_000,     i64::MIN    ,              0),
            (line!(), i64::MIN + 1, -2_000_000_000,     i64::MIN    ,              0),
            (line!(), i64::MIN + 2, -2_000_000_000,     i64::MIN    ,              0),
            (line!(), i64::MIN    ,   -999_999_998,     i64::MIN    ,              0),
            (line!(), i64::MIN + 1,   -999_999_998,     i64::MIN    ,              2),
            (line!(), i64::MAX    ,              0,     i64::MAX    ,              0),
            (line!(), i64::MAX - 1,              0,     i64::MAX - 1,              0),
            (line!(), i64::MAX    ,             -1,     i64::MAX - 1,    999_999_999),
            (line!(), i64::MAX    ,  1_000_000_000,     i64::MAX    ,    999_999_999),
            (line!(), i64::MAX - 1,  1_000_000_000,     i64::MAX    ,              0),
            (line!(), i64::MAX - 2,  1_000_000_000,     i64::MAX - 1,              0),
            (line!(), i64::MAX    ,  1_999_999_998,     i64::MAX    ,    999_999_999),
            (line!(), i64::MAX - 1,  1_999_999_998,     i64::MAX    ,    999_999_998),
            (line!(), i64::MAX - 2,  1_999_999_998,     i64::MAX - 1,    999_999_998),
            (line!(), i64::MAX    ,  1_999_999_999,     i64::MAX    ,    999_999_999),
            (line!(), i64::MAX - 1,  1_999_999_999,     i64::MAX    ,    999_999_999),
            (line!(), i64::MAX - 2,  1_999_999_999,     i64::MAX - 1,    999_999_999),
            (line!(), i64::MAX    ,  2_000_000_000,     i64::MAX    ,    999_999_999),
            (line!(), i64::MAX - 1,  2_000_000_000,     i64::MAX    ,    999_999_999),
            (line!(), i64::MAX - 2,  2_000_000_000,     i64::MAX    ,              0),
            (line!(), i64::MAX    ,    999_999_998,     i64::MAX    ,    999_999_998),
            (line!(), i64::MAX - 1,    999_999_998,     i64::MAX - 1,    999_999_998),
        ];

        for case in cases.iter() {
            let test_timestamp = crate::Timestamp {
                seconds: case.1,
                nanos: case.2,
            };

            assert_eq!(
                test_timestamp.normalized(),
                crate::Timestamp {
                    seconds: case.3,
                    nanos: case.4,
                },
                "test case on line {} doesn't match",
                case.0,
            );
        }
    }

    #[cfg(feature = "arbitrary")]
    #[test]
    fn check_timestamp_implements_arbitrary() {
        use arbitrary::{Arbitrary, Unstructured};

        let mut unstructured = Unstructured::new(&[]);

        assert_eq!(
            Timestamp::arbitrary(&mut unstructured),
            Ok(Timestamp {
                seconds: 0,
                nanos: 0
            })
        );
    }
}
