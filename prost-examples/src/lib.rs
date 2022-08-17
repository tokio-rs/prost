pub mod hello_world {
    include!(concat!(env!("OUT_DIR"), "/hello.rs"));
}

#[cfg(test)]
mod test {
    use prost::Message;

    #[test]
    fn test_hello() {
        let mut msg = super::hello_world::NewHelloWorld::default();
        msg.foo = 32;
        msg.new_field = 45;
        let mut buf = Vec::new();
        msg.encode_raw(&mut buf);

        let decoded = super::hello_world::OldHelloWorld::decode(buf.as_slice()).unwrap();
        println!("{:?}", decoded);
        let mut buf = Vec::new();
        decoded.encode_raw(&mut buf);

        let decoded = super::hello_world::NewHelloWorld::decode(buf.as_slice()).unwrap();
        assert_eq!(msg, decoded);
    }
}
