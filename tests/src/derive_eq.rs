#![allow(dead_code)]

include!(concat!(env!("OUT_DIR"), "/derive_eq.rs"));

// Compile-time assertion: the listed types must implement `Eq`.
trait TestEqIsImplemented: Eq {}

impl TestEqIsImplemented for AllIntMsg {}
impl TestEqIsImplemented for ComposedSafeMsg {}

// Float-bearing messages do not implement `Eq`. We prove this negatively by
// checking that `PartialEq` is still derived for them at runtime (a `PartialEq`
// comparison compiles regardless), and by leaving them out of the `Eq` list
// above. Adding `impl TestEqIsImplemented for FloatMsg {}` would fail to
// compile, which is the desired behavior.
#[test]
fn float_messages_still_partial_eq() {
    let a = FloatMsg::default();
    let b = FloatMsg::default();
    assert_eq!(a, b);

    let c = DoubleMsg::default();
    let d = DoubleMsg::default();
    assert_eq!(c, d);

    let e = ComposedUnsafeMsg::default();
    let f = ComposedUnsafeMsg::default();
    assert_eq!(e, f);
}

#[test]
fn safe_messages_are_eq_and_hash() {
    use std::collections::HashSet;

    let mut set: HashSet<AllIntMsg> = HashSet::new();
    set.insert(AllIntMsg::default());
    assert!(set.contains(&AllIntMsg::default()));

    let mut set2: HashSet<ComposedSafeMsg> = HashSet::new();
    set2.insert(ComposedSafeMsg::default());
    assert!(set2.contains(&ComposedSafeMsg::default()));
}
