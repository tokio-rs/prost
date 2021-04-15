mod b_generated {
    /// A snazzy new shirt!
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Shirt {
        #[prost(uuid, tag="1")]
        pub color: uuid::Uuid,
        #[prost(enumeration="shirt::Size", tag="2")]
        pub size: i32,
        #[prost(message, optional, tag="3")]
        pub o_option: std::option::Option<Option>,
    }
    /// Nested message and enum types in `Shirt`.
    pub mod shirt {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
        #[repr(i32)]
        pub enum Size {
            Small = 0,
            Medium = 1,
            Large = 2,
        }
    }
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Option {
    }
}

mod generated {
    /// A snazzy new shirt!
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Shirt {
        #[prost(string, tag="1")]
        pub a_color: crate::alloc::prelude::v1::String,
        #[prost(enumeration="shirt::Size", tag="2")]
        pub size: i32,
        #[prost(message, optional, tag="3")]
        pub option: std::option::Option<Option>,
    }
    /// Nested message and enum types in `Shirt`.
    pub mod shirt {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
        #[repr(i32)]
        pub enum Size {
            Small = 0,
            Medium = 1,
            Large = 2,
        }
    }
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Option {
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn equal() {
        let uuid = uuid::Uuid::new_v4();
        let size = 1;
        let g = b_generated::Shirt {
            color: uuid,
            size,
            o_option: Some(b_generated::Option {})
        };
        let b = generated::Shirt {
            a_color: uuid.to_string(),
            size,
            option: Some(generated::Option {})
        };

        let g = g.encode_buffer().uwnrap();
        let b = b.encode_buffer().uwnrap();

        assert_eq!(g, b);
    }

    #[test]
    fn fails() {
        // Invalid color
        let mut shirt = generated::Shirt {
            a_color: uuid::Uuid::new_v4().to_string()[1..].to_string(),
            size: 1,
            option: Some(generated::Option {})
        };

        macro_rules! check_invalid {
            () => {
                let mut b = shirt.encode_buffer().unwrap();

                assert!(b_generated::Shirt::decode(b.as_slice()).is_err());
            };
        }

        // The uuid is invalid
        check_invalid!();

        // Test invalid size
        shirt.a_color = uuid::Uuid::new_v4();
        shirt.size = 0;

        check_invalid!();

        // Check invalid option
        shirt.size = 1;
        shirt.option = None;

        // Make sure it passes when using the correct configuration
        let mut b = shirt.encode_buffer().unwrap();

        assert!(b_generated::Shirt::decode(b.as_slice()).is_ok());

        // Above is encoding, now try decoding
        // Check
        let mut shirt = b_generated::Shirt {
            color: uuid::Uuid::new_v4(),
            size: 0,
            o_option: Some(b_generated::Option {})
        };

        // Invalid size
        assert!(shirt.encode_buffer().is_err());

        shirt.size = 1;
        shirt.o_option = None;

        // Invalid o_option
        assert!(shirt.encode_buffer().is_err());

        shirt.o_option = Some(b_generated::Option {});
        // Everything valid

        assert!(shirt.encode_buffer().is_ok());
    }
}