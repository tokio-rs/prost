use prost::alloc::vec;

#[cfg(target_pointer_width = "64")]
fn verify_overflowing_encoded_len(actual: usize, expected: u64) -> bool {
    actual as u64 == expected
}

#[cfg(target_pointer_width = "32")]
fn verify_overflowing_encoded_len(actual: usize, _expected: u64) -> bool {
    // Tests calling this function are expected to panic on 32-bit platforms
    // before this check is called. Returning true here allows the
    // #[should_panic] tests to fail.
    eprintln!("expected panic, but the function returned {actual}");
    true
}

mod bool {
    use super::*;
    use prost::encoding::{bool, MAX_TAG};

    #[test]
    #[ignore = "allocates and fills about 666 MiB"]
    #[cfg_attr(target_pointer_width = "32", should_panic)]
    fn encoded_len_repeated_can_overflow_u32() {
        let filler = false;
        let filler_len = bool::encoded_len(MAX_TAG, &filler);
        let bomb32 = vec![filler; u32::MAX as usize / filler_len + 1];
        let encoded_len = bool::encoded_len_repeated(MAX_TAG, &bomb32);
        let expected_len = bomb32.len() as u64 * filler_len as u64;
        assert!(verify_overflowing_encoded_len(encoded_len, expected_len));
    }
}

/// The tests in this module each allocate and fill more than 1 GB in memory,
/// so they should better be run in isolation and in a sufficiently
/// memory-budgeted environment.
mod int32 {
    use super::*;
    use prost::encoding::{encoded_len_varint, int32, key_len, MAX_TAG};

    #[test]
    #[ignore = "allocates and fills more than 1 GiB"]
    #[cfg_attr(target_pointer_width = "32", should_panic)]
    fn encoded_len_repeated_can_overflow_u32() {
        let filler = -1i32;
        let filler_len = int32::encoded_len(MAX_TAG, &filler);
        let bomb32 = vec![filler; u32::MAX as usize / filler_len + 1];
        let encoded_len = int32::encoded_len_repeated(MAX_TAG, &bomb32);
        let expected_len = bomb32.len() as u64 * filler_len as u64;
        assert!(verify_overflowing_encoded_len(encoded_len, expected_len));
    }

    #[test]
    #[ignore = "allocates and fills about 1.6 GiB"]
    #[cfg_attr(target_pointer_width = "32", should_panic)]
    fn encoded_len_packed_can_overflow_u32() {
        let filler = -1i32;
        let filler_len = encoded_len_varint(filler as u64);
        let bomb_len = (u32::MAX as usize - key_len(MAX_TAG) - 5) / filler_len + 1;
        let bomb32 = vec![filler; bomb_len];
        let encoded_len = int32::encoded_len_packed(MAX_TAG, &bomb32);
        let expected_data_len = bomb_len as u64 * filler_len as u64;
        let expected_len = key_len(MAX_TAG) as u64
            + encoded_len_varint(expected_data_len) as u64
            + expected_data_len;
        assert!(verify_overflowing_encoded_len(encoded_len, expected_len));
    }
}

mod message {
    use super::*;
    use crate::encoded_len::proto;
    use prost::encoding::{encoded_len_varint, key_len, message, MAX_TAG};

    #[test]
    #[cfg_attr(target_pointer_width = "32", should_panic)]
    fn encoded_len_can_overflow_u32() {
        let filler = proto::Empty {};
        let filler_len = message::encoded_len(MAX_TAG, &filler);
        let subcritical = vec![filler; u32::MAX as usize / filler_len];
        let payload_len = subcritical.len() * filler_len;
        assert_eq!(encoded_len_varint(payload_len as u64), 5);
        assert!(key_len(MAX_TAG) + 5 >= filler_len);
        let bomb32 = proto::Testbed {
            repeated_empty: subcritical,
            ..Default::default()
        };
        let encoded_len = message::encoded_len(MAX_TAG, &bomb32);
        let expected_len = (key_len(MAX_TAG) + 5) as u64 + payload_len as u64;
        assert!(verify_overflowing_encoded_len(encoded_len, expected_len));
    }

    #[test]
    #[cfg_attr(target_pointer_width = "32", should_panic)]
    fn encoded_len_repeated_can_overflow_u32() {
        let filler = proto::Empty {};
        let filler_len = message::encoded_len(MAX_TAG, &filler);
        let bomb32 = vec![filler; u32::MAX as usize / filler_len + 1];
        let encoded_len = message::encoded_len_repeated(MAX_TAG, &bomb32);
        let expected_len = bomb32.len() as u64 * filler_len as u64;
        assert!(verify_overflowing_encoded_len(encoded_len, expected_len));
    }
}
