use super::*;

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
}

impl Name for Timestamp {
    const PACKAGE: &'static str = PACKAGE;
    const NAME: &'static str = "Timestamp";

    fn type_url() -> String {
        type_url_for::<Self>()
    }
}

#[cfg(feature = "chrono")]
mod chrono {
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

    impl From<NaiveTime> for Timestamp {
        fn from(naive_time: NaiveTime) -> Self {
            NaiveDate::default().and_time(naive_time).and_utc().into()
        }
    }

    impl TryFrom<Timestamp> for NaiveTime {
        type Error = TimestampError;

        fn try_from(timestamp: Timestamp) -> Result<Self, Self::Error> {
            DateTime::try_from(timestamp).map(|date_time| date_time.time())
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
    OutOfSystemRange(Timestamp),

    /// An error indicating failure to parse a timestamp in RFC-3339 format.
    ParseFailure,

    /// Indicates an error when constructing a timestamp due to invalid date or time data.
    InvalidDateTime,

    #[cfg(feature = "chrono")]
    /// Indicates that a [`Timestamp`] could not bet converted to
    /// [`chrono::{DateTime, NaiveDateTime, NaiveDate, NaiveTime`] out of range
    OutOfChronoDateTimeRanges(Timestamp),
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

#[cfg(feature = "std")]
impl std::error::Error for TimestampError {}

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
    mod kani_chrono {
        use super::*;
        use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Utc};
        use std::convert::{TryFrom, TryInto};

        #[kani::proof]
        fn verify_from_datetime_utc() {
            let date_time: chrono::DateTime<Utc> = kani::any();
            let timestamp = Timestamp::from(date_time);
            assert_eq!(timestamp.seconds, date_time.timestamp());
            assert_eq!(timestamp.nanos, date_time.timestamp_subsec_nanos() as i32);
        }

        #[kani::proof]
        fn verify_from_naive_datetime() {
            let naive_dt: NaiveDateTime = kani::any();
            let timestamp = Timestamp::from(naive_dt);
            let expected_dt_utc = naive_dt.and_utc();
            assert_eq!(timestamp.seconds, expected_dt_utc.timestamp());
            assert_eq!(
                timestamp.nanos,
                expected_dt_utc.timestamp_subsec_nanos() as i32
            );
        }

        #[kani::proof]
        fn verify_from_naive_date() {
            let naive_date: NaiveDate = kani::any();
            let timestamp = Timestamp::from(naive_date);
            let naive_dt = naive_date.and_time(NaiveTime::default());
            let expected_dt_utc = naive_dt.and_utc();
            assert_eq!(timestamp.seconds, expected_dt_utc.timestamp());
            assert_eq!(
                timestamp.nanos,
                expected_dt_utc.timestamp_subsec_nanos() as i32
            );
        }

        #[kani::proof]
        fn verify_from_naive_time() {
            let naive_time: NaiveTime = kani::any();
            let timestamp = Timestamp::from(naive_time);
            let naive_dt = NaiveDate::default().and_time(naive_time);
            let expected_dt_utc = naive_dt.and_utc();
            assert_eq!(timestamp.seconds, expected_dt_utc.timestamp());
            assert_eq!(
                timestamp.nanos,
                expected_dt_utc.timestamp_subsec_nanos() as i32
            );
        }

        #[kani::proof]
        fn verify_roundtrip_from_timestamp_to_datetime() {
            let timestamp: Timestamp = kani::any();
            // Precondition: The timestamp must be valid according to its spec.
            kani::assume((0..1_000_000_000).contains(&timestamp.nanos));

            if let Ok(dt_utc) = ::chrono::DateTime::<Utc>::try_from(timestamp.clone()) {
                // If conversion succeeds, the reverse must also succeed and be identical.
                let roundtrip_timestamp = Timestamp::from(dt_utc);
                assert_eq!(timestamp, roundtrip_timestamp);
            }
        }

        #[kani::proof]
        fn verify_roundtrip_from_timestamp_to_naive_datetime() {
            let timestamp: Timestamp = kani::any();
            kani::assume((0..1_000_000_000).contains(&timestamp.nanos));

            if let Ok(naive_dt) = ::chrono::NaiveDateTime::try_from(timestamp.clone()) {
                let roundtrip_timestamp = Timestamp::from(naive_dt);
                assert_eq!(timestamp, roundtrip_timestamp);
            }
        }

        #[kani::proof]
        fn verify_roundtrip_from_timestamp_to_naive_date() {
            let timestamp: Timestamp = kani::any();
            kani::assume((0..1_000_000_000).contains(&timestamp.nanos));

            if let Ok(naive_date) = ::chrono::NaiveDate::try_from(timestamp.clone()) {
                let roundtrip_timestamp = Timestamp::from(naive_date);

                // The original timestamp, when converted, should match the round-tripped date.
                let original_dt = ::chrono::DateTime::<Utc>::try_from(timestamp).unwrap();
                assert_eq!(original_dt.date_naive(), naive_date);

                // The round-tripped timestamp should correspond to midnight of that day.
                let expected_dt = naive_date.and_time(NaiveTime::default()).and_utc();
                assert_eq!(roundtrip_timestamp.seconds, expected_dt.timestamp());
                assert_eq!(roundtrip_timestamp.nanos, 0);
            }
        }

        #[kani::proof]
        fn verify_roundtrip_from_timestamp_to_naive_time() {
            let timestamp: Timestamp = kani::any();
            kani::assume((0..1_000_000_000).contains(&timestamp.nanos));

            if let Ok(naive_time) = ::chrono::NaiveTime::try_from(timestamp.clone()) {
                let roundtrip_timestamp = Timestamp::from(naive_time);

                // The original timestamp's time part should match the converted naive_time.
                let original_dt = ::chrono::DateTime::<Utc>::try_from(timestamp).unwrap();
                assert_eq!(original_dt.time(), naive_time);

                // The round-tripped timestamp should correspond to the naive_time on the epoch date.
                let expected_dt = NaiveDate::default().and_time(naive_time).and_utc();
                assert_eq!(roundtrip_timestamp.seconds, expected_dt.timestamp());
                assert_eq!(
                    roundtrip_timestamp.nanos,
                    expected_dt.timestamp_subsec_nanos() as i32
                );
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
