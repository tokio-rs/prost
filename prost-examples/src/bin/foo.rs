use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
pub mod hello_world {
    pub struct HelloWorld {
        pub foo: u32,
        pub unknown_fields: ::prost::unknown::UnknownFields,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for HelloWorld {
        #[inline]
        fn clone(&self) -> HelloWorld {
            HelloWorld {
                foo: ::core::clone::Clone::clone(&self.foo),
                unknown_fields: ::core::clone::Clone::clone(&self.unknown_fields),
            }
        }
    }
    // impl ::core::marker::StructuralPartialEq for HelloWorld {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for HelloWorld {
        #[inline]
        fn eq(&self, other: &HelloWorld) -> bool {
            self.foo == other.foo && self.unknown_fields == other.unknown_fields
        }
        #[inline]
        fn ne(&self, other: &HelloWorld) -> bool {
            self.foo != other.foo || self.unknown_fields != other.unknown_fields
        }
    }
    impl ::prost::Message for HelloWorld {
        #[allow(unused_variables)]
        fn encode_raw<B>(&self, buf: &mut B)
        where
            B: ::prost::bytes::BufMut,
        {
            if self.foo != 0u32 {
                ::prost::encoding::uint32::encode(1u32, &self.foo, buf);
            }
            let y = 0;
        }
        #[allow(unused_variables)]
        fn merge_field<B>(
            &mut self,
            tag: u32,
            wire_type: ::prost::encoding::WireType,
            buf: &mut B,
            ctx: ::prost::encoding::DecodeContext,
        ) -> ::core::result::Result<(), ::prost::DecodeError>
        where
            B: ::prost::bytes::Buf,
        {
            const STRUCT_NAME: &'static str = "HelloWorld";
            match tag {
                1u32 => {
                    let mut value = &mut self.foo;
                    ::prost::encoding::uint32::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "foo");
                            error
                        },
                    )
                }
                1000u32 => {
                    let mut value = &mut self.unknown_fields;
                    let rs: Result<(), ::prost::DecodeError> = Ok(());
                    rs.map_err(|mut error| {
                        error.push(STRUCT_NAME, "unknown_fields");
                        error
                    })
                }
                _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
            }
        }
        #[inline]
        fn encoded_len(&self) -> usize {
            0 + if self.foo != 0u32 {
                ::prost::encoding::uint32::encoded_len(1u32, &self.foo)
            } else {
                0
            } + 1000
        }
        fn clear(&mut self) {
            self.foo = 0u32;
            self.unknown_fields = ::prost::unknown::UnknownFields::default();
        }
    }
    impl ::core::default::Default for HelloWorld {
        fn default() -> Self {
            HelloWorld {
                foo: 0u32,
                unknown_fields: ::core::default::Default::default(),
            }
        }
    }
    impl ::core::fmt::Debug for HelloWorld {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let mut builder = f.debug_struct("HelloWorld");
            let builder = {
                let wrapper = {
                    fn ScalarWrapper<T>(v: T) -> T {
                        v
                    }
                    ScalarWrapper(&self.foo)
                };
                builder.field("foo", &wrapper)
            };
            let builder = {
                let wrapper = &self.unknown_fields;
                builder.field("unknown_fields", &wrapper)
            };
            builder.finish()
        }
    }
}

fn main() {}
