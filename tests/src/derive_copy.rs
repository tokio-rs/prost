#![allow(dead_code)]

#[derive(Clone, PartialEq, Eq, Hash, ::prost::Message)]
pub struct StringBacked {
    #[prost(string, tag = "1")]
    pub value: ::prost::alloc::string::String,
}

pub mod external_prefix {
    #[derive(Clone, PartialEq, Eq, Hash, ::prost::Message)]
    pub struct Empty {
        #[prost(string, tag = "1")]
        pub value: ::prost::alloc::string::String,
    }
}

include!(concat!(env!("OUT_DIR"), "/derive_copy.rs"));

trait TestCopyIsImplemented: Copy {}

impl TestCopyIsImplemented for EmptyMsg {}

impl TestCopyIsImplemented for IntegerMsg {}

impl TestCopyIsImplemented for FloatMsg {}

impl TestCopyIsImplemented for BoolMsg {}

impl TestCopyIsImplemented for AnEnum {}

impl TestCopyIsImplemented for EnumMsg {}

impl TestCopyIsImplemented for OneOfMsg {}

impl TestCopyIsImplemented for ComposedMsg {}

impl TestCopyIsImplemented for WellKnownMsg {}

impl TestCopyIsImplemented for PrefixNeighborParent {}

#[test]
fn external_path_messages_encode_and_decode() {
    use prost::Message;

    let external = StringBacked {
        value: "external".into(),
    };
    let parent = ExternalParent {
        field: Some(external.clone()),
        payload: Some(external_parent::Payload::Selected(external)),
    };
    let grandparent = ExternalGrandparent {
        parent: Some(parent),
    };
    let decoded = ExternalGrandparent::decode(grandparent.encode_to_vec().as_slice()).unwrap();
    assert_eq!(decoded, grandparent);

    let prefix_parent = PrefixParent {
        field: Some(external_prefix::Empty {
            value: "prefix".into(),
        }),
    };
    let decoded = PrefixParent::decode(prefix_parent.encode_to_vec().as_slice()).unwrap();
    assert_eq!(decoded, prefix_parent);
}
