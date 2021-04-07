use prost::Message;

include!(concat!(env!("OUT_DIR"), "/must.rs"));

#[test]
fn test_missing_must_errors_during_decode() {
    let uuid = uuid::Uuid::new_v4();

    let no_opts = RequestNoOpts {
        id: uuid.to_string(),
        meta: None,
    };

    let mut no_opts_buf = Vec::new();

    no_opts.encode(&mut no_opts_buf).unwrap();
    Request::decode(&no_opts_buf[..]).unwrap_err();
}
