use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use prost::Message;

include!(concat!(env!("OUT_DIR"), "/nesting.rs"));

#[test]
fn test_nesting() {
    let _ = A {
        a: Some(Box::default()),
        repeated_a: Vec::<A>::new(),
        map_a: BTreeMap::<i32, A>::new(),
        b: Some(Box::default()),
        repeated_b: Vec::<B>::new(),
        map_b: BTreeMap::<i32, B>::new(),
    };
}

#[test]
fn test_deep_nesting() {
    fn build_and_roundtrip(depth: usize) -> Result<(), prost::DecodeError> {
        let mut a = Box::<A>::default();
        for _ in 0..depth {
            let mut next = Box::<A>::default();
            next.a = Some(a);
            a = next;
        }

        let mut buf = Vec::new();
        a.encode(&mut buf).unwrap();
        A::decode(buf.as_slice()).map(|_| ())
    }

    assert!(build_and_roundtrip(100).is_ok());
    assert!(build_and_roundtrip(101).is_err());
}

#[test]
fn test_deep_nesting_repeated() {
    fn build_and_roundtrip(depth: usize) -> Result<(), prost::DecodeError> {
        let mut c = C::default();
        for _ in 0..depth {
            let mut next = C::default();
            next.r.push(c);
            c = next;
        }

        let mut buf = Vec::new();
        c.encode(&mut buf).unwrap();
        C::decode(buf.as_slice()).map(|_| ())
    }

    assert!(build_and_roundtrip(100).is_ok());
    assert!(build_and_roundtrip(101).is_err());
}

#[test]
fn test_deep_nesting_map() {
    fn build_and_roundtrip(depth: usize) -> Result<(), prost::DecodeError> {
        let mut d = D::default();
        for _ in 0..depth {
            let mut next = D::default();
            next.m.insert("foo".to_owned(), d);
            d = next;
        }

        let mut buf = Vec::new();
        d.encode(&mut buf).unwrap();
        D::decode(buf.as_slice()).map(|_| ())
    }

    assert!(build_and_roundtrip(50).is_ok());
    assert!(build_and_roundtrip(51).is_err());
}

#[test]
fn test_deep_nesting_with_custom_recursion_limit() {
    fn build_and_roundtrip(depth: usize) -> Result<(), prost::DecodeError> {
        let mut e = Box::new(E::default());
        for _ in 0..depth {
            let mut next = Box::new(E::default());
            next.e = Some(e);
            e = next;
        }

        let mut buf = Vec::new();
        e.encode(&mut buf).unwrap();
        E::decode(&*buf).map(|_| ())
    }

    assert!(build_and_roundtrip(200).is_ok());
    assert!(build_and_roundtrip(201).is_err());
}
