//! This tests the custom attributes support by abusing docs.
//!
//! Docs really are full-blown attributes. So we use them to ensure we can place them on everything
//! we need. If they aren't put onto something or allowed not to be there (by the generator),
//! compilation fails.
#![deny(missing_docs)]

include!(concat!(env!("OUT_DIR"), "/custom_attributes.rs"));
