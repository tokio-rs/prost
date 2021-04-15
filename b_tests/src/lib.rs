mod b_generated {
    /// A snazzy new shirt!
    #[derive(Clone, PartialEq, prost::Message)]
    pub struct Shirt {
        #[prost(uuid, tag = "1")]
        pub color: uuid::Uuid,
        #[prost(enumeration = "shirt::Size", tag = "2")]
        pub size: i32,
        #[prost(message, optional, tag = "3")]
        pub o_option: std::option::Option<Option>,
        #[prost(message, optional, tag = "4")]
        pub option: std::option::Option<Option>,
    }
    /// Nested message and enum types in `Shirt`.
    pub mod shirt {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, prost::Enumeration)]
        #[repr(i32)]
        pub enum Size {
            Small = 0,
            Medium = 1,
            Large = 2,
        }
    }
    #[derive(Clone, PartialEq, prost::Message)]
    pub struct Option {}
}

mod generated {
    /// A snazzy new shirt!
    #[derive(Clone, PartialEq, prost::Message)]
    pub struct Shirt {
        #[prost(string, tag = "1")]
        pub a_color: String,
        #[prost(enumeration = "shirt::Size", tag = "2")]
        pub size: i32,
        #[prost(message, optional, tag = "3")]
        pub o_option: std::option::Option<Option>,
        #[prost(message, optional, tag = "4")]
        pub option: std::option::Option<Option>,
    }
    /// Nested message and enum types in `Shirt`.
    pub mod shirt {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, prost::Enumeration)]
        #[repr(i32)]
        pub enum Size {
            Small = 0,
            Medium = 1,
            Large = 2,
        }
    }
    #[derive(Clone, PartialEq, prost::Message)]
    pub struct Option {}
}

#[cfg(test)]
mod test {
    use super::*;
    use prost::Message;

    #[test]
    fn equal() {
        let uuid = uuid::Uuid::new_v4();
        let size = 1;
        let g = b_generated::Shirt {
            color: uuid,
            size,
            o_option: Some(b_generated::Option {}),
            option: Some(b_generated::Option {}),
        };
        let b = generated::Shirt {
            a_color: uuid.to_string(),
            size,
            o_option: Some(generated::Option {}),
            option: Some(generated::Option {}),
        };

        let g = g.encode_buffer().unwrap();
        let b = b.encode_buffer().unwrap();

        assert_eq!(g, b);
    }

    fn check_shirt(shirt: generated::Shirt) {
        let mut b = shirt.encode_buffer().unwrap();

        let _ = b_generated::Shirt::decode(b.as_slice());
    }

    #[test]
    #[should_panic]
    fn invalid_color() {
        check_shirt(generated::Shirt {
            a_color: uuid::Uuid::new_v4().to_string()[1..].to_string(),
            size: 1,
            o_option: Some(generated::Option {}),
            option: Some(generated::Option {})
        })
    }

    #[test]
    #[should_panic]
    fn invalid_size_0() {
        check_shirt(generated::Shirt {
            a_color: uuid::Uuid::new_v4().to_string(),
            size: 0,
            o_option: Some(generated::Option {}),
            option: Some(generated::Option {})
        })
    }

    #[test]
    #[should_panic]
    fn invalid_size_over_max() {
        check_shirt(generated::Shirt {
            a_color: uuid::Uuid::new_v4().to_string(),
            size: 999,
            o_option: Some(generated::Option {}),
            option: Some(generated::Option {})
        })
    }

    #[test]
    #[should_panic]
    fn invalid_option() {
        check_shirt(generated::Shirt {
            a_color: uuid::Uuid::new_v4().to_string(),
            size: 1,
            o_option: Some(generated::Option {}),
            option: None
        })
    }

    #[test]
    fn valid_option() {
        check_shirt(generated::Shirt {
            a_color: uuid::Uuid::new_v4().to_string(),
            size: 1,
            o_option: None,
            option: Some(generated::Option {})
        })
    }
}
