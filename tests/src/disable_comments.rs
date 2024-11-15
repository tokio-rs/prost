//! MessageWithInvalidDoctest would generate a invalid doc test if
//! `Config::disable_comments` doesn't work correctly.
include!(concat!(env!("OUT_DIR"), "/disable_comments.rs"));
