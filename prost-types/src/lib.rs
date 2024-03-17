#![doc(html_root_url = "https://docs.rs/prost-types/0.12.2")]

//! Protocol Buffers well-known types.
//!
//! Note that the documentation for the types defined in this crate are generated from the Protobuf
//! definitions, so code examples are not in Rust.
//!
//! See the [Protobuf reference][1] for more information about well-known types.
//!
//! [1]: https://developers.google.com/protocol-buffers/docs/reference/google.protobuf

#![cfg_attr(not(feature = "std"), no_std)]

#[rustfmt::skip]
pub mod compiler;
mod datetime;
#[rustfmt::skip]
mod protobuf;

use core::convert::TryFrom;
use core::fmt;
use core::i32;
use core::i64;
use core::str::FromStr;
use core::time;

use prost::alloc::format;
use prost::alloc::string::String;
use prost::alloc::vec::Vec;
use prost::{DecodeError, EncodeError, Message, Name};

pub use protobuf::*;

// The Protobuf `Duration` and `Timestamp` types can't delegate to the standard library equivalents
// because the Protobuf versions are signed. To make them easier to work with, `From` conversions
// are defined in both directions.

const NANOS_PER_SECOND: i32 = 1_000_000_000;
const NANOS_MAX: i32 = NANOS_PER_SECOND - 1;

const PACKAGE: &str = "google.protobuf";

mod any;

mod duration;
pub use duration::DurationError;

mod timestamp;
pub use timestamp::TimestampError;

mod type_url;
pub(crate) use type_url::{type_url_for, TypeUrl};

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

        #[test]
        fn check_timestamp_roundtrip_via_system_time(
            seconds in i64::arbitrary(),
            nanos in i32::arbitrary(),
        ) {
            let mut timestamp = Timestamp { seconds, nanos };
            timestamp.normalize();
            if let Ok(system_time) = SystemTime::try_from(timestamp.clone()) {
                prop_assert_eq!(Timestamp::from(system_time), timestamp);
            }
        }

        #[test]
        fn check_duration_roundtrip(
            seconds in u64::arbitrary(),
            nanos in 0u32..1_000_000_000u32,
        ) {
            let std_duration = time::Duration::new(seconds, nanos);
            let prost_duration = match Duration::try_from(std_duration) {
                Ok(duration) => duration,
                Err(_) => return Err(TestCaseError::reject("duration out of range")),
            };
            prop_assert_eq!(time::Duration::try_from(prost_duration.clone()).unwrap(), std_duration);

            if std_duration != time::Duration::default() {
                let neg_prost_duration = Duration {
                    seconds: -prost_duration.seconds,
                    nanos: -prost_duration.nanos,
                };

                prop_assert!(
                    matches!(
                        time::Duration::try_from(neg_prost_duration),
                        Err(DurationError::NegativeDuration(d)) if d == std_duration,
                    )
                )
            }
        }

        #[test]
        fn check_duration_roundtrip_nanos(
            nanos in u32::arbitrary(),
        ) {
            let seconds = 0;
            let std_duration = std::time::Duration::new(seconds, nanos);
            let prost_duration = match Duration::try_from(std_duration) {
                Ok(duration) => duration,
                Err(_) => return Err(TestCaseError::reject("duration out of range")),
            };
            prop_assert_eq!(time::Duration::try_from(prost_duration.clone()).unwrap(), std_duration);

            if std_duration != time::Duration::default() {
                let neg_prost_duration = Duration {
                    seconds: -prost_duration.seconds,
                    nanos: -prost_duration.nanos,
                };

                prop_assert!(
                    matches!(
                        time::Duration::try_from(neg_prost_duration),
                        Err(DurationError::NegativeDuration(d)) if d == std_duration,
                    )
                )
            }
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn check_duration_try_from_negative_nanos() {
        let seconds: u64 = 0;
        let nanos: u32 = 1;
        let std_duration = std::time::Duration::new(seconds, nanos);

        let neg_prost_duration = Duration {
            seconds: 0,
            nanos: -1,
        };

        assert!(matches!(
           time::Duration::try_from(neg_prost_duration),
           Err(DurationError::NegativeDuration(d)) if d == std_duration,
        ))
    }

    #[cfg(feature = "std")]
    #[test]
    fn check_timestamp_negative_seconds() {
        // Representative tests for the case of timestamps before the UTC Epoch time:
        // validate the expected behaviour that "negative second values with fractions
        // must still have non-negative nanos values that count forward in time"
        // https://developers.google.com/protocol-buffers/docs/reference/google.protobuf#google.protobuf.Timestamp
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

    #[test]
    fn check_duration_normalize() {
        #[rustfmt::skip] // Don't mangle the table formatting.
        let cases = [
            // --- Table of test cases ---
            //        test seconds      test nanos  expected seconds  expected nanos
            (line!(),            0,              0,                0,              0),
            (line!(),            1,              1,                1,              1),
            (line!(),           -1,             -1,               -1,             -1),
            (line!(),            0,    999_999_999,                0,    999_999_999),
            (line!(),            0,   -999_999_999,                0,   -999_999_999),
            (line!(),            0,  1_000_000_000,                1,              0),
            (line!(),            0, -1_000_000_000,               -1,              0),
            (line!(),            0,  1_000_000_001,                1,              1),
            (line!(),            0, -1_000_000_001,               -1,             -1),
            (line!(),           -1,              1,                0,   -999_999_999),
            (line!(),            1,             -1,                0,    999_999_999),
            (line!(),           -1,  1_000_000_000,                0,              0),
            (line!(),            1, -1_000_000_000,                0,              0),
            (line!(), i64::MIN    ,              0,     i64::MIN    ,              0),
            (line!(), i64::MIN + 1,              0,     i64::MIN + 1,              0),
            (line!(), i64::MIN    ,              1,     i64::MIN + 1,   -999_999_999),
            (line!(), i64::MIN    ,  1_000_000_000,     i64::MIN + 1,              0),
            (line!(), i64::MIN    , -1_000_000_000,     i64::MIN    ,   -999_999_999),
            (line!(), i64::MIN + 1, -1_000_000_000,     i64::MIN    ,              0),
            (line!(), i64::MIN + 2, -1_000_000_000,     i64::MIN + 1,              0),
            (line!(), i64::MIN    , -1_999_999_998,     i64::MIN    ,   -999_999_999),
            (line!(), i64::MIN + 1, -1_999_999_998,     i64::MIN    ,   -999_999_998),
            (line!(), i64::MIN + 2, -1_999_999_998,     i64::MIN + 1,   -999_999_998),
            (line!(), i64::MIN    , -1_999_999_999,     i64::MIN    ,   -999_999_999),
            (line!(), i64::MIN + 1, -1_999_999_999,     i64::MIN    ,   -999_999_999),
            (line!(), i64::MIN + 2, -1_999_999_999,     i64::MIN + 1,   -999_999_999),
            (line!(), i64::MIN    , -2_000_000_000,     i64::MIN    ,   -999_999_999),
            (line!(), i64::MIN + 1, -2_000_000_000,     i64::MIN    ,   -999_999_999),
            (line!(), i64::MIN + 2, -2_000_000_000,     i64::MIN    ,              0),
            (line!(), i64::MIN    ,   -999_999_998,     i64::MIN    ,   -999_999_998),
            (line!(), i64::MIN + 1,   -999_999_998,     i64::MIN + 1,   -999_999_998),
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
            let mut test_duration = Duration {
                seconds: case.1,
                nanos: case.2,
            };
            test_duration.normalize();

            assert_eq!(
                test_duration,
                Duration {
                    seconds: case.3,
                    nanos: case.4,
                },
                "test case on line {} doesn't match",
                case.0,
            );
        }
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
            let mut test_timestamp = crate::Timestamp {
                seconds: case.1,
                nanos: case.2,
            };
            test_timestamp.normalize();

            assert_eq!(
                test_timestamp,
                crate::Timestamp {
                    seconds: case.3,
                    nanos: case.4,
                },
                "test case on line {} doesn't match",
                case.0,
            );
        }
    }

    #[test]
    fn check_any_serialization() {
        let message = Timestamp::date(2000, 01, 01).unwrap();
        let any = Any::from_msg(&message).unwrap();
        assert_eq!(
            &any.type_url,
            "type.googleapis.com/google.protobuf.Timestamp"
        );

        let message2 = any.to_msg::<Timestamp>().unwrap();
        assert_eq!(message, message2);

        // Wrong type URL
        assert!(any.to_msg::<Duration>().is_err());
    }

    #[test]
    fn check_type_url_parsing() {
        let example_type_name = "google.protobuf.Duration";

        let url = TypeUrl::new("type.googleapis.com/google.protobuf.Duration").unwrap();
        assert_eq!(url.full_name, example_type_name);

        let full_url =
            TypeUrl::new("https://type.googleapis.com/google.protobuf.Duration").unwrap();
        assert_eq!(full_url.full_name, example_type_name);

        let relative_url = TypeUrl::new("/google.protobuf.Duration").unwrap();
        assert_eq!(relative_url.full_name, example_type_name);

        // The name should be in a canonical form (e.g., leading "." is not accepted).
        assert_eq!(TypeUrl::new("/.google.protobuf.Duration"), None);

        // Must contain at least one "/" character.
        assert_eq!(TypeUrl::new("google.protobuf.Duration"), None);
    }
}
