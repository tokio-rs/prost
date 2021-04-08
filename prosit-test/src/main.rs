//! A serialization/deserialization test for all types extended by prosit.

include!(concat!(env!("OUT_DIR"), "/prosit_test.rs"));

use ::prost::Message;

fn main() {
    let uuid = uuid::Uuid::new_v4();
    let meta = Meta {
        id: uuid::Uuid::new_v4(),
    };

    let with_opts = Request {
        id: uuid.clone(),
        meta: meta.clone(),
        foo: request::Foo::Bax(1),
        ..Default::default()
    };

    let no_opts = RequestNoOpts {
        id: String::from("bar"),
        meta: None,
        foo: None,
        ..Default::default()
    };

    let mut with_opts_buf = Vec::new();
    let mut no_opts_buf = Vec::new();

    with_opts.encode(&mut with_opts_buf).unwrap();
    no_opts.encode(&mut no_opts_buf).unwrap();
    // assert_eq!(with_opts_buf, no_opts_buf);

    // check that we can actually decode their inverses
    let with_opts_decoded = Request::decode(&no_opts_buf[..]).unwrap();
    assert_eq!(with_opts, with_opts_decoded);

    let no_opts_decoded = RequestNoOpts::decode(&with_opts_buf[..]).unwrap();
    assert_eq!(no_opts, no_opts_decoded)
}
