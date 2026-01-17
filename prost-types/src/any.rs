use super::*;

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
    pub fn to_msg<M>(&self) -> Result<M, DecodeError>
    where
        M: Default + Name + Sized,
    {
        let expected_type_url = M::type_url();
        let actual_type_url = &self.type_url;

        if let (Some(expected), Some(actual)) = (
            TypeUrl::new(&expected_type_url),
            TypeUrl::new(actual_type_url),
        ) {
            if expected == actual {
                return M::decode(self.value.as_slice());
            }
        }

        Err(DecodeError::new_unexpected_type_url(
            actual_type_url,
            expected_type_url,
        ))
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
    use prost::{bytes, encoding, Message};

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
    #[derive(Clone, PartialEq, Debug, Default)]
    struct Test {
        value: i32,
    }

    impl Message for Test {
        fn encode_raw(&self, buf: &mut impl bytes::BufMut) {
            encoding::int32::encode(1, &self.value, buf);
        }

        fn merge_field(
            &mut self,
            tag: u32,
            wire_type: encoding::WireType,
            buf: &mut impl bytes::Buf,
            ctx: encoding::DecodeContext,
        ) -> Result<(), crate::DecodeError> {
            if tag == 1 {
                encoding::int32::merge(wire_type, &mut self.value, buf, ctx)
            } else {
                encoding::skip_field(wire_type, tag, buf, ctx)
            }
        }

        fn encoded_len(&self) -> usize {
            encoding::int32::encoded_len(1, &self.value)
        }

        fn clear(&mut self) {
            self.value = 0;
        }
    }

    impl crate::Name for Test {
        const PACKAGE: &'static str = ""; // Empty package
        const NAME: &'static str = "Test";
    }

    #[test]
    fn dynamic_cast_round_trip() {
        let msg = Test::default();
        let any = Any::from_msg(&msg).unwrap();
        let result: Result<Test, _> = any.to_msg();
        result.expect("This should parse!");
    }

    #[test]
    fn default_type_url_should_parse() {
        let type_url = Test::type_url(); //any.type_url;
        TypeUrl::new(&type_url)
            .expect("The URL created by the default implementation should parse");
    }
}
