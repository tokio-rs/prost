use super::*;

impl Duration {
    /// Normalizes the duration to a canonical format.
    ///
    /// Based on [`google::protobuf::util::CreateNormalized`][1].
    ///
    /// [1]: https://github.com/google/protobuf/blob/v3.3.2/src/google/protobuf/util/time_util.cc#L79-L100
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
                // Negative overflow! Set to the least normal value.
                self.seconds = i64::MIN;
                self.nanos = -NANOS_MAX;
            } else {
                // Positive overflow! Set to the greatest normal value.
                self.seconds = i64::MAX;
                self.nanos = NANOS_MAX;
            }
        }

        // nanos should have the same sign as seconds.
        if self.seconds < 0 && self.nanos > 0 {
            if let Some(seconds) = self.seconds.checked_add(1) {
                self.seconds = seconds;
                self.nanos -= NANOS_PER_SECOND;
            } else {
                // Positive overflow! Set to the greatest normal value.
                debug_assert_eq!(self.seconds, i64::MAX);
                self.nanos = NANOS_MAX;
            }
        } else if self.seconds > 0 && self.nanos < 0 {
            if let Some(seconds) = self.seconds.checked_sub(1) {
                self.seconds = seconds;
                self.nanos += NANOS_PER_SECOND;
            } else {
                // Negative overflow! Set to the least normal value.
                debug_assert_eq!(self.seconds, i64::MIN);
                self.nanos = -NANOS_MAX;
            }
        }
        // TODO: should this be checked?
        // debug_assert!(self.seconds >= -315_576_000_000 && self.seconds <= 315_576_000_000,
        //               "invalid duration: {:?}", self);
    }

    /// Returns a normalized copy of the duration to a canonical format.
    ///
    /// Based on [`google::protobuf::util::CreateNormalized`][1].
    ///
    /// [1]: https://github.com/google/protobuf/blob/v3.3.2/src/google/protobuf/util/time_util.cc#L79-L100
    pub fn normalized(&self) -> Self {
        let mut result = *self;
        result.normalize();
        result
    }
}

impl Name for Duration {
    const PACKAGE: &'static str = PACKAGE;
    const NAME: &'static str = "Duration";

    fn type_url() -> String {
        type_url_for::<Self>()
    }
}

impl TryFrom<time::Duration> for Duration {
    type Error = DurationError;

    /// Converts a `std::time::Duration` to a `Duration`, failing if the duration is too large.
    fn try_from(duration: time::Duration) -> Result<Duration, DurationError> {
        let seconds = i64::try_from(duration.as_secs()).map_err(|_| DurationError::OutOfRange)?;
        let nanos = duration.subsec_nanos() as i32;

        let duration = Duration { seconds, nanos };
        Ok(duration.normalized())
    }
}

impl TryFrom<Duration> for time::Duration {
    type Error = DurationError;

    /// Converts a `Duration` to a `std::time::Duration`, failing if the duration is negative.
    fn try_from(mut duration: Duration) -> Result<time::Duration, DurationError> {
        duration.normalize();
        if duration.seconds >= 0 && duration.nanos >= 0 {
            Ok(time::Duration::new(
                duration.seconds as u64,
                duration.nanos as u32,
            ))
        } else {
            Err(DurationError::NegativeDuration(time::Duration::new(
                (-duration.seconds) as u64,
                (-duration.nanos) as u32,
            )))
        }
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let d = self.normalized();
        if self.seconds < 0 || self.nanos < 0 {
            write!(f, "-")?;
        }
        write!(f, "{}", d.seconds.abs())?;

        // Format subseconds to either nothing, millis, micros, or nanos.
        let nanos = d.nanos.abs();
        if nanos == 0 {
            write!(f, "s")
        } else if nanos % 1_000_000 == 0 {
            write!(f, ".{:03}s", nanos / 1_000_000)
        } else if nanos % 1_000 == 0 {
            write!(f, ".{:06}s", nanos / 1_000)
        } else {
            write!(f, ".{:09}s", nanos)
        }
    }
}

/// A duration handling error.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum DurationError {
    /// Indicates failure to parse a [`Duration`] from a string.
    ///
    /// The [`Duration`] string format is specified in the [Protobuf JSON mapping specification][1].
    ///
    /// [1]: https://developers.google.com/protocol-buffers/docs/proto3#json
    ParseFailure,

    /// Indicates failure to convert a `prost_types::Duration` to a `std::time::Duration` because
    /// the duration is negative. The included `std::time::Duration` matches the magnitude of the
    /// original negative `prost_types::Duration`.
    NegativeDuration(time::Duration),

    /// Indicates failure to convert a `std::time::Duration` to a `prost_types::Duration`.
    ///
    /// Converting a `std::time::Duration` to a `prost_types::Duration` fails if the magnitude
    /// exceeds that representable by `prost_types::Duration`.
    OutOfRange,
}

impl fmt::Display for DurationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DurationError::ParseFailure => write!(f, "failed to parse duration"),
            DurationError::NegativeDuration(duration) => {
                write!(f, "failed to convert negative duration: {:?}", duration)
            }
            DurationError::OutOfRange => {
                write!(f, "failed to convert duration out of range")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DurationError {}

impl FromStr for Duration {
    type Err = DurationError;

    fn from_str(s: &str) -> Result<Duration, DurationError> {
        datetime::parse_duration(s).ok_or(DurationError::ParseFailure)
    }
}

#[cfg(kani)]
mod proofs {
    use super::*;

    #[cfg(feature = "std")]
    #[kani::proof]
    fn check_duration_roundtrip() {
        let seconds = kani::any();
        let nanos = kani::any();
        kani::assume(nanos < 1_000_000_000);
        let std_duration = std::time::Duration::new(seconds, nanos);
        let Ok(prost_duration) = Duration::try_from(std_duration) else {
            // Test case not valid: duration out of range
            return;
        };
        assert_eq!(
            time::Duration::try_from(prost_duration).unwrap(),
            std_duration
        );

        if std_duration != time::Duration::default() {
            let neg_prost_duration = Duration {
                seconds: -prost_duration.seconds,
                nanos: -prost_duration.nanos,
            };

            assert!(matches!(
                time::Duration::try_from(neg_prost_duration),
                Err(DurationError::NegativeDuration(d)) if d == std_duration,
            ))
        }
    }

    #[cfg(feature = "std")]
    #[kani::proof]
    fn check_duration_roundtrip_nanos() {
        let seconds = 0;
        let nanos = kani::any();
        let std_duration = std::time::Duration::new(seconds, nanos);
        let Ok(prost_duration) = Duration::try_from(std_duration) else {
            // Test case not valid: duration out of range
            return;
        };
        assert_eq!(
            time::Duration::try_from(prost_duration).unwrap(),
            std_duration
        );

        if std_duration != time::Duration::default() {
            let neg_prost_duration = Duration {
                seconds: -prost_duration.seconds,
                nanos: -prost_duration.nanos,
            };

            assert!(matches!(
                time::Duration::try_from(neg_prost_duration),
                Err(DurationError::NegativeDuration(d)) if d == std_duration,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "std")]
    #[test]
    fn test_duration_from_str() {
        assert_eq!(
            Duration::from_str("0s"),
            Ok(Duration {
                seconds: 0,
                nanos: 0
            })
        );
        assert_eq!(
            Duration::from_str("123s"),
            Ok(Duration {
                seconds: 123,
                nanos: 0
            })
        );
        assert_eq!(
            Duration::from_str("0.123s"),
            Ok(Duration {
                seconds: 0,
                nanos: 123_000_000
            })
        );
        assert_eq!(
            Duration::from_str("-123s"),
            Ok(Duration {
                seconds: -123,
                nanos: 0
            })
        );
        assert_eq!(
            Duration::from_str("-0.123s"),
            Ok(Duration {
                seconds: 0,
                nanos: -123_000_000
            })
        );
        assert_eq!(
            Duration::from_str("22041211.6666666666666s"),
            Ok(Duration {
                seconds: 22041211,
                nanos: 666_666_666
            })
        );
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_format_duration() {
        assert_eq!(
            "0s",
            Duration {
                seconds: 0,
                nanos: 0
            }
            .to_string()
        );
        assert_eq!(
            "123s",
            Duration {
                seconds: 123,
                nanos: 0
            }
            .to_string()
        );
        assert_eq!(
            "0.123s",
            Duration {
                seconds: 0,
                nanos: 123_000_000
            }
            .to_string()
        );
        assert_eq!(
            "-123s",
            Duration {
                seconds: -123,
                nanos: 0
            }
            .to_string()
        );
        assert_eq!(
            "-0.123s",
            Duration {
                seconds: 0,
                nanos: -123_000_000
            }
            .to_string()
        );
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
            let test_duration = Duration {
                seconds: case.1,
                nanos: case.2,
            };

            assert_eq!(
                test_duration.normalized(),
                Duration {
                    seconds: case.3,
                    nanos: case.4,
                },
                "test case on line {} doesn't match",
                case.0,
            );
        }
    }
}
