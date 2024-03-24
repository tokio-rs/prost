mod proto {
    include!(concat!(env!("OUT_DIR"), "/no_package/_includes.rs"));
}

#[test]
fn it_works() {
    assert_eq!(
        proto::NoPackageWithMessageExampleMsg::default(),
        proto::NoPackageWithMessageExampleMsg::default()
    );
}
