use crate::check_serialize_equivalent;

include!(concat!(env!("OUT_DIR"), "/url.rs"));

#[test]
fn test_url_serializes_equivalent_to_string() {
    let no_opts = RequestNoOpts {
        url: String::from("https://google.com/"),
    };

    let with_opts = Request {
        url: ::url::Url::parse("https://google.com/").unwrap(),
    };

    check_serialize_equivalent(&no_opts, &with_opts)
}
