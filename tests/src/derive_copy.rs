include!(concat!(env!("OUT_DIR"), "/derive_copy.rs"));

#[allow(dead_code)]
trait TestCopyIsImplemented: Copy {}

impl TestCopyIsImplemented for EmptyMsg {}

impl TestCopyIsImplemented for IntegerMsg {}

impl TestCopyIsImplemented for FloatMsg {}

impl TestCopyIsImplemented for BoolMsg {}

impl TestCopyIsImplemented for AnEnum {}

impl TestCopyIsImplemented for EnumMsg {}

impl TestCopyIsImplemented for OneOfMsg {}

impl TestCopyIsImplemented for ComposedMsg {}

impl TestCopyIsImplemented for WellKnownMsg {}
