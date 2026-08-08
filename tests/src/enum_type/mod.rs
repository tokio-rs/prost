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
        open: Some(OpenEnum::Known(Open::A)),
        closed_required: Closed::B,
        open_required: OpenEnum::Known(Open::B),
    };
}
