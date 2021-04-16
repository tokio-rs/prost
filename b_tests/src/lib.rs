mod b_generated {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Shirt {
        #[prost(string, tag = "1")]
        pub a_color: ::prost::alloc::string::String,
        #[prost(uuid, tag = "12")]
        pub uuid: uuid::Uuid,
        #[prost(enumeration = "shirt::Inner", tag = "2")]
        #[prost(strict)]
        pub size: i32,
        #[prost(enumeration = "Outer", tag = "11")]
        #[prost(strict)]
        pub size_outer: i32,
        #[prost(message, optional, tag = "3")]
        #[prost(strict)]
        pub option: ::core::option::Option<Option>,
        #[prost(message, optional, tag = "8")]
        pub o_option: ::core::option::Option<Option>,
        #[prost(message, repeated, tag = "9")]
        pub options: ::prost::alloc::vec::Vec<Option>,
        #[prost(oneof = "shirt::SomethingElse", tags = "4, 5, 6, 7")]
        #[prost(strict)]
        pub something_else: ::core::option::Option<shirt::SomethingElse>,
        #[prost(oneof = "shirt::OSomethingElse", tags = "10")]
        pub o_something_else: ::core::option::Option<shirt::OSomethingElse>,
    }
    /// Nested message and enum types in `Shirt`.
    pub mod shirt {
        #[derive(
            Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration,
        )]
        #[repr(i32)]
        pub enum Inner {
            Small = 0,
            Medium = 1,
            Large = 2,
        }
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum SomethingElse {
            #[prost(message, tag = "4")]
            O(super::Option),
            #[prost(string, tag = "5")]
            S(::prost::alloc::string::String),
            #[prost(int32, tag = "6")]
            I(i32),
            #[prost(enumeration = "Inner", tag = "7")]
            E(i32),
        }
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum OSomethingElse {
            #[prost(string, tag = "10")]
            Jmr(::prost::alloc::string::String),
        }
    }
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Option {}
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Outer {
        Small = 0,
        Medium = 1,
        Large = 2,
    }
}

mod generated {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Shirt {
        #[prost(string, tag = "1")]
        pub a_color: ::prost::alloc::string::String,
        #[prost(string, tag = "12")]
        pub uuid: ::prost::alloc::string::String,
        #[prost(enumeration = "shirt::Inner", tag = "2")]
        pub size: i32,
        #[prost(enumeration = "Outer", tag = "11")]
        pub size_outer: i32,
        #[prost(message, optional, tag = "3")]
        pub option: ::core::option::Option<Option>,
        #[prost(message, optional, tag = "8")]
        pub o_option: ::core::option::Option<Option>,
        #[prost(message, repeated, tag = "9")]
        pub options: ::prost::alloc::vec::Vec<Option>,
        #[prost(oneof = "shirt::SomethingElse", tags = "4, 5, 6, 7")]
        pub something_else: ::core::option::Option<shirt::SomethingElse>,
        #[prost(oneof = "shirt::OSomethingElse", tags = "10")]
        pub o_something_else: ::core::option::Option<shirt::OSomethingElse>,
    }
    /// Nested message and enum types in `Shirt`.
    pub mod shirt {
        #[derive(
            Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration,
        )]
        #[repr(i32)]
        pub enum Inner {
            Small = 0,
            Medium = 1,
            Large = 2,
        }
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum SomethingElse {
            #[prost(message, tag = "4")]
            O(super::Option),
            #[prost(string, tag = "5")]
            S(::prost::alloc::string::String),
            #[prost(int32, tag = "6")]
            I(i32),
            #[prost(enumeration = "Inner", tag = "7")]
            E(i32),
        }
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum OSomethingElse {
            #[prost(string, tag = "10")]
            Jmr(::prost::alloc::string::String),
        }
    }
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Option {}
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Outer {
        Small = 0,
        Medium = 1,
        Large = 2,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use prost::Message;

    fn b_generated_shirt() -> b_generated::Shirt {
        b_generated::Shirt {
            a_color: "color".to_string(),
            uuid: uuid::Uuid::new_v4(),
            size: 1,
            size_outer: 1,
            option: Some(b_generated::Option {}),
            o_option: None,
            options: vec![b_generated::Option {}],
            something_else: Some(b_generated::shirt::SomethingElse::O(b_generated::Option {})),
            o_something_else: None,
        }
    }

    fn generated_shirt() -> generated::Shirt {
        let g = b_generated_shirt();

        generated::Shirt {
            a_color: g.a_color.clone(),
            uuid: g.uuid.to_string(),
            size: g.size,
            size_outer: g.size_outer,
            option: Some(generated::Option {}),
            o_option: None,
            options: vec![generated::Option {}],
            something_else: Some(generated::shirt::SomethingElse::O(generated::Option {})),
            o_something_else: None,
        }
    }

    #[test]
    fn equal() {
        let g = b_generated_shirt();
        let b = generated_shirt();

        let g = g.encode_buffer().unwrap();
        let b = b.encode_buffer().unwrap();

        assert_eq!(g, b);
        assert_eq!(g.encoded_len(), b.encoded_len());

        // Check if encoding works
        b_generated::Shirt::decode(b.as_slice());
        generated::Shirt::decode(g.as_slice());
    }

    fn check_shirt(shirt: generated::Shirt) {
        let b = shirt.encode_buffer().unwrap();
        let _ = b_generated::Shirt::decode(b.as_slice());
    }

    macro_rules! write_invalid_test {
        ($method_name: ident, $shirt: ident, $change: tt) => {
            #[test]
            #[should_panic]
            fn $method_name() {
                let mut $shirt = generated_shirt();

                $change;

                check_shirt($shirt);
            }
        };
    }

    write_invalid_test!(
        invalid_color,
        shirt,
        ({
            shirt.a_color = shirt.a_color[1..].to_string();
        })
    );
    write_invalid_test!(
        invalid_inner_size_0,
        shirt,
        ({
            shirt.size = 0;
        })
    );
    write_invalid_test!(
        over_max_inner_size,
        shirt,
        ({
            shirt.size = 999;
        })
    );
    write_invalid_test!(
        invalid_outer_size_0,
        shirt,
        ({
            shirt.size_outer = 0;
        })
    );
    write_invalid_test!(
        over_max_outer_size,
        shirt,
        ({
            shirt.size_outer = 999;
        })
    );
    write_invalid_test!(
        none_option,
        shirt,
        ({
            shirt.option = None;
        })
    );
    write_invalid_test!(
        none_oneof,
        shirt,
        ({
            shirt.something_else = None;
        })
    );
}
