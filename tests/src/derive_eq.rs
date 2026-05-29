#![allow(dead_code)]

include!(concat!(env!("OUT_DIR"), "/derive_eq.rs"));

trait TestEqIsImplemented: Eq {}

impl TestEqIsImplemented for EmptyMsg {}

impl TestEqIsImplemented for IntegerMsg {}

impl TestEqIsImplemented for BoolMsg {}

impl TestEqIsImplemented for AnEnum {}

impl TestEqIsImplemented for EnumMsg {}

impl TestEqIsImplemented for OneOfMsg {}

impl TestEqIsImplemented for ComposedMsg {}

impl TestEqIsImplemented for WellKnownMsg {}
