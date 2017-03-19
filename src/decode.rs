use std::io::{
    Result,
    Error,
    ErrorKind,
};
use std::cmp::min;

use bytes::Buf;

/// Decodes a LEB128-encoded variable length integer from the buffer.
#[inline]
pub fn decode_varint<B>(buf: &mut B) -> Result<u64> where B: Buf {
    let mut value = 0;
    let mut i = 0;
    'outer: loop {
        let bytes = buf.bytes();
        let len = bytes.len();

        for &byte in &bytes[..min(len, 10 - i)] {
            value |= ((byte & 0x7F) as u64) << (i * 7);
            i += 1;
            if byte <= 0x7F {
                break 'outer;
            }
        }

        if i == 10 {
            return Err(Error::new(ErrorKind::InvalidData,
                                  "failed to decode varint: integer overflow"));
        }
        if !buf.has_remaining() {
            return Err(Error::new(ErrorKind::InvalidData,
                                  "failed to decode varint: buffer underflow"));
        }
    }
    buf.advance(i);
    return Ok(value);
}

#[cfg(test)]
mod tests {

    use bytes::Bytes;
    use bytes::IntoBuf;

    use super::*;
    use encode::encode_varint;


    #[test]
    fn varint() {
        fn check(value: u64, encoded: &[u8]) {
            let mut buf = Vec::new();

            encode_varint(value, &mut buf);

            assert_eq!(buf, encoded);

            let roundtrip_value = decode_varint(&mut Bytes::from(encoded).into_buf()).expect("decoding failed");
            assert_eq!(value, roundtrip_value);
        }

        //check(0, &[0b0000_0000]);
        //check(1, &[0b0000_0001]);

        //check(127, &[0b0111_1111]);
        //check(128, &[0b1000_0000, 0b0000_0001]);

        check(300, &[0b1010_1100, 0b0000_0010]);

        //check(16_383, &[0b1111_1111, 0b0111_1111]);
        //check(16_384, &[0b1000_0000, 0b1000_0000, 0b0000_0001]);
    }
}
