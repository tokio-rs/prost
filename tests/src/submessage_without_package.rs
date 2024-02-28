include!(concat!(env!("OUT_DIR"), "/_.rs"));

#[test]
fn test_submessage_without_package() {
    let _msg = M::default();
}
