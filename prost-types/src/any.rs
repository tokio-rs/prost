use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeAnyError {
    Decode(DecodeError),
    /// Unexpected type URL
    UnexpectedTypeUrl {
        actual: String,
        expected: String,
    },
}

impl fmt::Display for DecodeAnyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeAnyError::Decode(err) => write!(f, "{err}"),
            DecodeAnyError::UnexpectedTypeUrl { actual, expected } => {
                write!(f, "failed to decode Protobuf message: unexpected type URL.type_url: expected type URL: \"{expected}\" (got: \"{actual}\")")
            }
        }
    }
}

impl From<DecodeError> for DecodeAnyError {
    fn from(err: DecodeError) -> Self {
        Self::Decode(err)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DecodeAnyError {}

impl Any {
    /// Serialize the given message type `M` as [`Any`].
    pub fn from_msg<M>(msg: &M) -> Result<Self, EncodeError>
    where
        M: Name,
    {
        let type_url = M::type_url();
        let mut value = Vec::new();
        Message::encode(msg, &mut value)?;
        Ok(Any { type_url, value })
    }

    /// Decode the given message type `M` from [`Any`], validating that it has
    /// the expected type URL.
    pub fn to_msg<M>(&self) -> Result<M, DecodeAnyError>
    where
        M: Default + Name + Sized,
    {
        let expected_type_url = M::type_url();

        if let (Some(expected), Some(actual)) = (
            TypeUrl::new(&expected_type_url),
            TypeUrl::new(&self.type_url),
        ) {
            if expected == actual {
                return M::decode(self.value.as_slice()).map_err(DecodeAnyError::from);
            }
        }

        Err(DecodeAnyError::UnexpectedTypeUrl {
            actual: self.type_url.clone(),
            expected: expected_type_url,
        })
    }
}

impl Name for Any {
    const PACKAGE: &'static str = PACKAGE;
    const NAME: &'static str = "Any";

    fn type_url() -> String {
        type_url_for::<Self>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_any_serialization() {
        let message = Timestamp::date(2000, 1, 1).unwrap();
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
}
