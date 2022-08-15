pub mod hello_world {
    include!(concat!(env!("OUT_DIR"), "/hello.rs"));
}

#[cfg(test)]
mod test {
    use prost::Message;

    #[test]
    fn test_hello() {
        let msg = super::hello_world::HelloWorld {
            foo: 32,
            unknown_fields: ::prost::unknown::UnknownFields::default(),
        };
        let mut buf = Vec::new();
        msg.encode_raw(&mut buf);

        let decoded = super::hello_world::HelloWorld::decode(buf.as_slice()).unwrap();
        assert_eq!(msg, decoded);
    }
}
