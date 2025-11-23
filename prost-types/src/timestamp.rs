use super::*;
use core::ops::{Add, Div, Sub};

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

    pub const MAX: Timestamp = Timestamp {
        seconds: i64::MAX,
        nanos: NANOS_PER_SECOND - 1,
    };

    pub const MIN: Timestamp = Timestamp {
        seconds: i64::MIN,
        nanos: 0,
    };
}

impl Add<Duration> for Timestamp {
    type Output = Timestamp;

    //Add Timestamp with Duration normalized
    fn add(self, rhs: Duration) -> Self::Output {
        let (mut nanos, overflowed) = match self.nanos.checked_add(rhs.nanos) {
            Some(nanos) => (nanos, 0),
            None => (
                // it's overflowed operation, then force 2 complements and goes out the direction
                // The complements of 2 carry rest of sum
                (!(self.nanos.wrapping_add(rhs.nanos))).wrapping_add(1),
                self.nanos.saturating_add(rhs.nanos),
            ),
        };

        // divided by NANOS_PER_SECOND it's impossible to overflow
        // Multiplay by 2 because 2^(n+1) == 2^n*2 for use 'i33' type
        let mut seconds_from_nanos = (overflowed / NANOS_PER_SECOND) * 2;
        seconds_from_nanos += nanos / NANOS_PER_SECOND;
        nanos %= NANOS_PER_SECOND;
        nanos += (overflowed % NANOS_PER_SECOND) * 2;
        seconds_from_nanos += nanos / NANOS_PER_SECOND;
        nanos %= NANOS_PER_SECOND;

        if nanos.is_negative() {
            nanos += NANOS_PER_SECOND;
            seconds_from_nanos -= 1;
        }

        if cfg!(debug_assertions) {
            // If in debug_assertions mode cause default overflow panic
            let seconds = self.seconds + rhs.seconds + (seconds_from_nanos as i64);
            Self { seconds, nanos }
        } else {
            let seconds = self
                .seconds
                .saturating_add(rhs.seconds)
                .saturating_add(seconds_from_nanos as i64);
            Self {
                seconds,
                nanos: match seconds {
                    i64::MAX => Self::MAX.nanos,
                    i64::MIN => Self::MIN.nanos,
                    _ => nanos,
                },
            }
        }
    }
}

impl Sub<Duration> for Timestamp {
    type Output = Timestamp;

    fn sub(self, rhs: Duration) -> Self::Output {
        let negated_duration = Duration {
            seconds: -rhs.seconds,
            nanos: -rhs.nanos,
        };
        self.add(negated_duration)
    }
}

macro_rules! impl_div_for_integer {
    ($($t:ty),*) => {
        $(
            impl Div<$t> for Timestamp {
                type Output = Duration;

                fn div(self, denominator: $t) -> Self::Output {
                    let mut total_nanos = self.seconds as i128 * NANOS_PER_SECOND as i128 + self.nanos as i128;

                    total_nanos /= denominator as i128;

                    let mut seconds = (total_nanos / NANOS_PER_SECOND as i128) as i64;
                    let mut nanos = (total_nanos % NANOS_PER_SECOND as i128) as i32;

                    if nanos < 0 {
                        seconds -= 1;
                        nanos += NANOS_PER_SECOND;
                    }

                    Duration { seconds, nanos }
                }
            }
        )*
    };
}

impl_div_for_integer!(i8, u8, i16, u16, i32, u32, i64, u64, i128, u128);

macro_rules! impl_div_for_float {
    ($($t:ty),*) => {
        $(
            impl Div<$t> for Timestamp {
                type Output = Duration;

                fn div(self, denominator: $t) -> Self::Output {
                    let mut total_seconds_float = (self.seconds as f64 + self.nanos as f64 / NANOS_PER_SECOND as f64);
                    total_seconds_float /= denominator as f64;

                    //Not necessary to create special treatment for overflow, if denominator is
                    //extreame low the value can be f64::INFINITY and then converted for i64 is i64::MAX
                    // assert_eq!((f64::MAX/f64::MIN_POSITIVE) as i64, i64::MAX)
                    // assert_eq!((f64::MIN/f64::MIN_POSITIVE) as i64, i64::MIN)
                    let mut seconds = total_seconds_float as i64;
                    if total_seconds_float < 0. && total_seconds_float != seconds as f64 {
                        seconds -= 1;
                    }

                    let nanos_float = (total_seconds_float - seconds as f64) * NANOS_PER_SECOND as f64;

                    let nanos = (nanos_float + 0.5) as i32;

                    if nanos == NANOS_PER_SECOND {
                        Duration { seconds: seconds + 1, nanos: 0 }
                    } else {
                        Duration { seconds, nanos }
                    }
                }
            }
        )*
    };
}

impl_div_for_float!(f32, f64);

#[cfg(test)]
mod tests_ops {
    use super::*;

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
                nanos: 294967296
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
            ts.add(dur),
            Timestamp {
                seconds: 7,
                nanos: 900_000_000
            }
        );
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic]
    fn test_add_saturating_seconds() {
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

    //This test needs to run --release argument
    //In production enviroments don't cause panic, only returns Timestamp::(MAX or MIN)
    #[test]
    #[cfg(not(debug_assertions))]
    fn test_add_saturating_seconds() {
        let ts = Timestamp {
            seconds: i64::MAX - 1,
            nanos: 500_000_000,
        };
        let dur = Duration {
            seconds: 10,
            nanos: 0,
        };

        assert_eq!((ts + dur), Timestamp::MAX);
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
    fn test_div_by_positive_integer() {
        let ts = Timestamp {
            seconds: 10,
            nanos: 500_000_000,
        };
        let duration = ts / 2;
        assert_eq!(
            duration,
            Duration {
                seconds: 5,
                nanos: 250_000_000
            }
        );
    }

    #[test]
    fn test_div_by_positive_integer_resulting_in_fractional_seconds() {
        let ts = Timestamp {
            seconds: 1,
            nanos: 0,
        };
        let duration = ts / 2;
        assert_eq!(
            duration,
            Duration {
                seconds: 0,
                nanos: 500_000_000
            }
        );
    }

    #[test]
    fn test_div_by_positive_integer_imperfect_division() {
        let ts = Timestamp {
            seconds: 10,
            nanos: 0,
        };
        let duration = ts / 3;
        assert_eq!(
            duration,
            Duration {
                seconds: 3,
                nanos: 333_333_333
            }
        );
    }

    #[test]
    fn test_div_by_positive_float() {
        let ts = Timestamp {
            seconds: 5,
            nanos: 0,
        };
        let duration = ts / 2.5;
        assert_eq!(
            duration,
            Duration {
                seconds: 2,
                nanos: 0
            }
        );
    }

    #[test]
    fn test_div_by_negative_integer() {
        let ts = Timestamp {
            seconds: 10,
            nanos: 500_000_000,
        };
        let duration = ts / -2;

        assert_eq!(
            duration,
            Duration {
                seconds: -6,
                nanos: 750_000_000
            }
        );
    }

    #[test]
    fn test_div_by_negative_float() {
        let ts = Timestamp {
            seconds: 5,
            nanos: 0,
        };
        let duration = ts / -2.0;
        assert_eq!(
            duration,
            Duration {
                seconds: -3,
                nanos: 500_000_000
            }
        );
    }

    #[test]
    fn test_div_negative_timestamp_by_positive_integer() {
        let ts = Timestamp {
            seconds: -10,
            nanos: 0,
        };
        let duration = ts / 4;
        assert_eq!(
            duration,
            Duration {
                seconds: -3,
                nanos: 500_000_000
            }
        );
    }

    #[test]
    fn test_div_negative_timestamp_by_negative_integer() {
        let ts = Timestamp {
            seconds: -10,
            nanos: 0,
        };
        let duration = ts / -2;
        assert_eq!(
            duration,
            Duration {
                seconds: 5,
                nanos: 0
            }
        );
    }

    #[test]
    fn test_div_zero_timestamp() {
        let ts = Timestamp {
            seconds: 0,
            nanos: 0,
        };
        let duration = ts / 100;
        assert_eq!(
            duration,
            Duration {
                seconds: 0,
                nanos: 0
            }
        );
    }

    #[test]
    #[should_panic]
    fn test_div_by_zero() {
        let ts = Timestamp {
            seconds: 0,
            nanos: 0,
        };
        let _duration = ts / 0;
    }
}

#[cfg(kani)]
mod proofs_ops {
    use super::*;

    #[kani::proof]
    fn verify_add() {
        let ts = Timestamp {
            seconds: kani::any(),
            nanos: kani::any(),
        };
        let dur = Duration {
            seconds: kani::any(),
            nanos: kani::any(),
        };

        kani::assume(i64::MAX / 3 > ts.seconds);
        kani::assume(i64::MIN / 3 < ts.seconds);
        kani::assume(i64::MAX / 3 > dur.seconds);
        kani::assume(i64::MIN / 3 < dur.seconds);

        kani::assume(i32::MAX != ts.nanos);
        kani::assume(i32::MAX != dur.nanos);
        kani::assume(i32::MIN != ts.nanos);
        kani::assume(i32::MIN != dur.nanos);

        let result = ts + dur;

        assert!((Timestamp::MIN.nanos..=Timestamp::MAX.nanos).contains(&result.nanos));
    }

    #[kani::proof]
    fn verify_sub() {
        let ts = Timestamp {
            seconds: kani::any(),
            nanos: kani::any(),
        };
        let dur = Duration {
            seconds: kani::any(),
            nanos: kani::any(),
        };

        kani::assume(i64::MAX / 3 > ts.seconds);
        kani::assume(i64::MIN / 3 < ts.seconds);
        kani::assume(i64::MAX / 3 > dur.seconds);
        kani::assume(i64::MIN / 3 < dur.seconds);

        kani::assume(i32::MAX != ts.nanos);
        kani::assume(i32::MAX != dur.nanos);
        kani::assume(i32::MIN != ts.nanos);
        kani::assume(i32::MIN != dur.nanos);

        let result = ts - dur;

        assert!((Timestamp::MIN.nanos..=Timestamp::MAX.nanos).contains(&result.nanos));
    }

    #[kani::proof]
    fn verify_div_by_int() {
        let ts = Timestamp {
            seconds: kani::any(),
            nanos: kani::any(),
        };
        let divisor: i32 = kani::any();

        kani::assume(divisor != 0);

        kani::assume(i64::MAX / 3 > ts.seconds);
        kani::assume(i64::MIN / 3 < ts.seconds);

        let result = ts / divisor;

        assert!((Timestamp::MIN.nanos..=Timestamp::MAX.nanos).contains(&result.nanos));
    }

    #[kani::proof]
    fn verify_div_by_float() {
        let ts = Timestamp {
            seconds: kani::any(),
            nanos: kani::any(),
        };
        let divisor: f32 = kani::any();
        kani::assume(divisor.is_finite() && divisor.abs() > 1e-9);

        let result = ts / divisor;

        assert!((Timestamp::MIN.nanos..=Timestamp::MAX.nanos).contains(&result.nanos));
    }
}

impl Name for Timestamp {
    const PACKAGE: &'static str = PACKAGE;
    const NAME: &'static str = "Timestamp";

    fn type_url() -> String {
        type_url_for::<Self>()
    }
}

#[cfg(feature = "chrono")]
mod timestamp_chrono {
    use super::*;
    use ::chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};

    impl<Tz: TimeZone> From<DateTime<Tz>> for Timestamp {
        fn from(date_time: DateTime<Tz>) -> Self {
            Self {
                seconds: date_time.timestamp(),
                nanos: date_time.timestamp_subsec_nanos() as i32,
            }
        }
    }

    impl TryFrom<Timestamp> for DateTime<Utc> {
        type Error = TimestampError;

        fn try_from(timestamp: Timestamp) -> Result<Self, Self::Error> {
            let timestamp = timestamp.normalized();
            DateTime::from_timestamp(timestamp.seconds, timestamp.nanos as u32)
                .ok_or(TimestampError::OutOfChronoDateTimeRanges(timestamp))
        }
    }

    impl From<NaiveDateTime> for Timestamp {
        fn from(naive_date_time: NaiveDateTime) -> Self {
            naive_date_time.and_utc().into()
        }
    }

    impl TryFrom<Timestamp> for NaiveDateTime {
        type Error = TimestampError;

        fn try_from(timestamp: Timestamp) -> Result<Self, Self::Error> {
            let timestamp = timestamp.normalized();
            DateTime::try_from(timestamp).map(|date_time| date_time.naive_utc())
        }
    }

    impl From<NaiveDate> for Timestamp {
        fn from(naive_date: NaiveDate) -> Self {
            naive_date.and_time(NaiveTime::default()).and_utc().into()
        }
    }

    impl TryFrom<Timestamp> for NaiveDate {
        type Error = TimestampError;

        fn try_from(timestamp: Timestamp) -> Result<Self, Self::Error> {
            DateTime::try_from(timestamp).map(|date_time| date_time.date_naive())
        }
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
    #[cfg(feature = "std")]
    OutOfSystemRange(Timestamp),

    /// An error indicating failure to parse a timestamp in RFC-3339 format.
    ParseFailure,

    /// Indicates an error when constructing a timestamp due to invalid date or time data.
    InvalidDateTime,

    /// Indicates that a [`Timestamp`] could not bet converted to
    /// [`chrono::{DateTime, NaiveDateTime, NaiveDate, NaiveTime`] out of range
    #[cfg(feature = "chrono")]
    OutOfChronoDateTimeRanges(Timestamp),
}

impl fmt::Display for TimestampError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(feature = "std")]
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
            #[cfg(feature = "chrono")]
            TimestampError::OutOfChronoDateTimeRanges(timestamp) => {
                write!(
                    f,
                    "{timestamp} is not representable in `DateTime, NaiveDateTime, NaiveDate, NaiveTime` because it is out of range",
                )
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

    #[cfg(feature = "chrono")]
    mod p_chrono {
        use super::*;
        use ::chrono::{DateTime, NaiveDate, NaiveDateTime};
        //Why does it limit? In testing, it was left for more than 2 hours and not completed.

        #[kani::proof]
        fn check_timestamp_roundtrip_via_date_time() {
            let seconds = kani::any();
            let nanos = kani::any();

            kani::assume(i64::MAX / 3 < seconds);
            kani::assume(i64::MIN / 3 > seconds);

            let mut timestamp = Timestamp { seconds, nanos };
            timestamp.normalize();

            if let Ok(date_time) = DateTime::try_from(timestamp) {
                assert_eq!(Timestamp::from(date_time), timestamp);
            }
        }

        #[kani::proof]
        fn check_timestamp_roundtrip_via_naive_date_time() {
            let seconds = kani::any();
            let nanos = kani::any();

            kani::assume(i64::MAX / 3 < seconds);
            kani::assume(i64::MIN / 3 > seconds);

            let mut timestamp = Timestamp { seconds, nanos };
            timestamp.normalize();

            if let Ok(naive_date_time) = NaiveDateTime::try_from(timestamp) {
                assert_eq!(Timestamp::from(naive_date_time), timestamp);
            }
        }

        #[kani::proof]
        fn check_timestamp_roundtrip_via_naive_date() {
            let seconds = kani::any();
            let nanos = kani::any();

            kani::assume(i64::MAX / 3 < seconds);
            kani::assume(i64::MIN / 3 > seconds);

            let mut timestamp = Timestamp { seconds, nanos };
            timestamp.normalize();

            if let Ok(naive_date) = NaiveDate::try_from(timestamp) {
                assert_eq!(Timestamp::from(naive_date), timestamp);
            }
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

    #[cfg(feature = "chrono")]
    mod chrono_test {
        use super::*;
        use ::chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};

        #[test]
        fn test_datetime_roundtrip() {
            let original_ndt = NaiveDate::from_ymd_opt(2025, 7, 26)
                .unwrap()
                .and_hms_nano_opt(10, 0, 0, 123_456_789)
                .unwrap();
            let original_dt = original_ndt.and_utc();
            let timestamp: Timestamp = original_dt.into();
            let converted_dt: DateTime<Utc> = timestamp.try_into().unwrap();
            assert_eq!(original_dt, converted_dt);
        }

        #[test]
        fn test_naivedatetime_roundtrip() {
            let original_ndt = NaiveDate::from_ymd_opt(2025, 7, 26)
                .unwrap()
                .and_hms_nano_opt(10, 0, 0, 123_456_789)
                .unwrap();
            let timestamp: Timestamp = original_ndt.into();
            let converted_ndt: NaiveDateTime = timestamp.try_into().unwrap();
            assert_eq!(original_ndt, converted_ndt);
        }

        #[test]
        fn test_naivedate_roundtrip() {
            let original_nd = NaiveDate::from_ymd_opt(1995, 12, 17).unwrap();
            // From<NaiveDate> converts to a timestamp at midnight.
            let timestamp: Timestamp = original_nd.into();
            let converted_nd: NaiveDate = timestamp.try_into().unwrap();
            assert_eq!(original_nd, converted_nd);
        }

        #[test]
        fn test_epoch_conversion() {
            let epoch_dt = DateTime::from_timestamp(0, 0).unwrap();
            let timestamp: Timestamp = epoch_dt.into();
            assert_eq!(
                timestamp,
                Timestamp {
                    seconds: 0,
                    nanos: 0
                }
            );

            let converted_dt: DateTime<Utc> = timestamp.try_into().unwrap();
            assert_eq!(epoch_dt, converted_dt);
        }

        #[test]
        fn test_timestamp_out_of_range() {
            // This timestamp is far beyond what chrono can represent.
            let far_future = Timestamp {
                seconds: i64::MAX,
                nanos: 0,
            };
            let result = DateTime::<Utc>::try_from(far_future);
            assert_eq!(
                result,
                Err(TimestampError::OutOfChronoDateTimeRanges(far_future))
            );
        }

        #[test]
        fn test_timestamp_normalization() {
            // A timestamp with negative nanos that should be normalized.
            // 10 seconds - 100 nanos should be 9 seconds + 999,999,900 nanos.
            let unnormalized = Timestamp {
                seconds: 10,
                nanos: -100,
            };
            let expected_dt = DateTime::from_timestamp(9, 999_999_900).unwrap();
            let converted_dt: DateTime<Utc> = unnormalized.try_into().unwrap();
            assert_eq!(converted_dt, expected_dt);

            // A timestamp with > 1B nanos.
            // 5s + 1.5B nanos should be 6s + 0.5B nanos.
            let overflow_nanos = Timestamp {
                seconds: 5,
                nanos: 1_500_000_000,
            };
            let expected_dt_2 = DateTime::from_timestamp(6, 500_000_000).unwrap();
            let converted_dt_2: DateTime<Utc> = overflow_nanos.try_into().unwrap();
            assert_eq!(converted_dt_2, expected_dt_2);
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
