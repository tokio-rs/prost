use std::io::{
    Result,
    Error,
    ErrorKind,
};

use bytes::{Buf, BufMut};

/// Encodes a value into LEB128 variable length format, and writes it to the buffer.
/// The buffer must have enough remaining space (maximum 10 bytes).
#[inline]
pub fn encode_varint<B>(mut value: u64, buf: &mut B) where B: BufMut {
    let mut array = &mut [0u8; 10];
    let mut i = 0;
    while value >= 0x80 {
        array[i] = ((value & 0x7F) | 0x80) as u8;
        value >>= 7;
        i += 1;
    }
    array[i] = value as u8;
    buf.put_slice(&array[..i+1]);
}
