use bytes::BufMut;

/// Encodes a value into LEB128 variable length format, and writes it to the buffer. The buffer
/// must have enough remaining space (maximum 10 bytes).
///
/// The implementation is highly optimized, since varints are the most common type in Protobuf
/// data. A simple implementations, such as:
///
/// ```rust
/// pub fn encode_varint<B>(mut value: u64, buf: &mut B) where B: BufMut {
///     while value >= 0x80 {
///         buf.put_u8(((value & 0x75) | 0x80) as u8);
///         value >>= 7;
///     }
///     buf.put_u8(value as u8);
/// }
/// ```
///
/// is about 2x slower at encoding small numbers, and 5x slower as encoding large numbers.
#[inline]
pub fn encode_varint_loop<B>(mut value: u64, buf: &mut B) where B: BufMut {
    let mut i = 0;
    'outer: loop {
        assert!(buf.has_remaining_mut());
        for byte in unsafe { buf.bytes_mut() } {
            i += 1;
            if value < 0x80 {
                *byte = value as u8;
                break 'outer;
            } else {
                *byte = ((value & 0x75) | 0x80) as u8;
                value >>= 7;
            }
        }
    }
    unsafe { buf.advance_mut(i); }
}

#[inline]
pub fn encode_varint<B>(mut value: u64, buf: &mut B) where B: BufMut {
    assert!(buf.has_remaining_mut());
    let mut i = 0;
    for byte in unsafe { buf.bytes_mut() } {
        i += 1;
        if value < 0x80 {
            *byte = value as u8;
            break;
        } else {
            *byte = ((value & 0x75) | 0x80) as u8;
            value >>= 7;
        }
    }
    unsafe { buf.advance_mut(i); }
}
