// Test that the limits returned by field::scalar::Ty::encoded_len_limit()
// in prost-derive can be reached with actual values.

macro_rules! test_limit_is_reachable {
    ($name:ident, $value:expr, $limit:expr) => {
        mod $name {
            use crate::encoded_len::proto;
            use prost::Message;

            #[test]
            fn encoded_len_limit_is_reachable() {
                let msg = proto::Numerics {
                    $name: $value,
                    ..Default::default()
                };
                assert_eq!(msg.encoded_len() - 1, $limit);
            }
        }
    };
}

test_limit_is_reachable!(int32, -1, 10);
test_limit_is_reachable!(int64, -1, 10);
test_limit_is_reachable!(uint32, u32::MAX, 5);
test_limit_is_reachable!(uint64, u64::MAX, 10);
test_limit_is_reachable!(sint32, i32::MIN, 5);
test_limit_is_reachable!(sint64, i64::MIN, 10);
test_limit_is_reachable!(fixed32, 1, 4);
test_limit_is_reachable!(fixed64, 1, 8);
test_limit_is_reachable!(sfixed32, -1, 4);
test_limit_is_reachable!(sfixed64, -1, 8);
test_limit_is_reachable!(float, 1.0, 4);
test_limit_is_reachable!(double, 1.0, 8);
test_limit_is_reachable!(bool, true, 1);
test_limit_is_reachable!(enumeration, proto::BadEnum::Long as i32, 10);
