use super::*;

#[cfg(feature = "std")]
impl std::hash::Hash for Duration {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.seconds.hash(state);
        self.nanos.hash(state);
    }
}

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

        let mut duration = Duration { seconds, nanos };
        duration.normalize();
        Ok(duration)
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
        let mut d = self.clone();
        d.normalize();
        if self.seconds < 0 && self.nanos < 0 {
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
#[allow(clippy::derive_partial_eq_without_eq)]
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
