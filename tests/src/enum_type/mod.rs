mod enum_type {
    pub mod closed {
        include!(concat!(env!("OUT_DIR"), "/enum_type.closed.rs"));
    }

    pub mod open {
        include!(concat!(env!("OUT_DIR"), "/enum_type.open.rs"));
    }

    pub mod use_in_fields {
        include!(concat!(env!("OUT_DIR"), "/enum_type.use_in_fields.rs"));
    }
}

use self::enum_type::closed::Closed;
use self::enum_type::open::Open;
use self::enum_type::use_in_fields::Msg;

use prost::OpenEnum;

#[test]
fn test_field_types() {
    let _msg = Msg {
        closed: Some(Closed::A),
        open: Some(Open::A.into()),
        closed_required: Closed::B,
        open_required: Open::B.into(),
    };
}

#[test]
fn test_open_enum_is_word_sized() {
    use core::mem::{align_of, size_of};

    // An open enum field costs exactly what the `i32` it replaces costs.
    assert_eq!(size_of::<OpenEnum<Open>>(), size_of::<i32>());
    assert_eq!(align_of::<OpenEnum<Open>>(), align_of::<i32>());

    // And so a repeated field occupies the same buffer as the `[i32; n]` it
    // replaces. Measured on slices: `size_of::<Vec<_>>()` is three words
    // whatever the element type.
    let raw = [0i32; 8];
    let values = OpenEnum::<Open>::from_raw_slice(&raw);
    assert_eq!(
        core::mem::size_of_val(values),
        core::mem::size_of_val(&raw),
        "repeated open enum fields must not be larger than repeated i32 fields",
    );
}

#[test]
fn test_raw_slice_conversion_is_lossless() {
    let raw = [0, 1, 777, -3];
    let values = OpenEnum::<Open>::from_raw_slice(&raw);

    assert_eq!(values[0].known(), Some(Open::Unset));
    assert_eq!(values[1].known(), Some(Open::A));
    assert_eq!(values[2].known(), None, "777 is not a known variant");
    assert_eq!(values[3].known(), None, "-3 is not a known variant");
    assert_eq!(values[2].to_raw(), 777, "unknown values are preserved");
    assert_eq!(OpenEnum::as_raw_slice(values), &raw);
}

#[test]
fn test_compares_directly_against_variants() {
    let value: OpenEnum<Open> = Open::A.into();

    assert!(value == Open::A);
    assert!(value != Open::B);

    let described = match value.known() {
        Some(Open::A) => "a",
        Some(Open::B) => "b",
        Some(Open::Unset) => "unset",
        None => "unknown",
    };
    assert_eq!(described, "a");
    assert_eq!(OpenEnum::<Open>::from_raw(777).known(), None);
}
