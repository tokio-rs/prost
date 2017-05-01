use std::default;
use std::io::Result;
use std::str;

use bytes::{
    Buf,
    BufMut,
};

use encoding::*;
use field::Field;

/// Marker trait for scalar Protobuf types.
pub trait Scalar<E=Plain> : Field + default::Default {}

/// Provides a Field implementation for length delimited scalar fields (bytes and string).
/// Has to be provided as a macro instead of a blanket impl due to coherence.
macro_rules! repeated_length_delimited_scalar {
    ($ty: ty) => {
        impl Field for Vec<$ty> {
            #[inline]
            fn encode<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
                for value in self {
                    <$ty as Field>::encode(value, tag, buf);
                }
            }
            #[inline]
            fn merge<B>(&mut self, tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
                check_wire_type(WireType::LengthDelimited, wire_type)?;
                let mut value = default::Default::default();
                <$ty as Field>::merge(&mut value, tag, WireType::LengthDelimited, buf)?;
                self.push(value);
                Ok(())
            }
            #[inline]
            fn encoded_len(&self, tag: u32) -> usize {
                self.iter().map(|f| f.encoded_len(tag)).sum()
            }
        }
    };
}

impl Field for Vec<u8> {
    #[inline]
    fn encode<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
        encode_key(tag, WireType::LengthDelimited, buf);
        encode_varint(self.len() as u64, buf);
        buf.put_slice(self);
    }
    #[inline]
    fn merge<B>(&mut self, _tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
        check_wire_type(WireType::LengthDelimited, wire_type)?;
        let len = decode_varint(buf)?;
        if (buf.remaining() as u64) < len {
            return Err(invalid_input("failed to decode bytes: buffer underflow"));
        }
        let len = len as usize;
        self.clear();
        self.extend_from_slice(&buf.bytes()[..len]);
        buf.advance(len);
        Ok(())
    }
    #[inline]
    fn encoded_len(&self, tag: u32) -> usize {
        let len = self.len();
        key_len(tag) + encoded_len_varint(len as u64) + len
    }
}
repeated_length_delimited_scalar!(Vec<u8>);

impl Field for String {
    #[inline]
    fn encode<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
        encode_key(tag, WireType::LengthDelimited, buf);
        encode_varint(self.len() as u64, buf);
        buf.put_slice(self.as_bytes());
    }
    #[inline]
    fn merge<B>(&mut self, _tag: u32, wire_type: WireType, buf: &mut B) -> Result<()> where B: Buf {
        check_wire_type(WireType::LengthDelimited, wire_type)?;
        let len = decode_varint(buf)?;
        if (buf.remaining() as u64) < len {
            return Err(invalid_input("failed to decode string: buffer underflow"));
        }
        let len = len as usize;
        self.clear();
        self.push_str(str::from_utf8(&buf.bytes()[..len]).map_err(|_| {
            invalid_data("failed to decode string: data is not UTF-8 encoded")
        })?);
        buf.advance(len);
        Ok(())
    }
    #[inline]
    fn encoded_len(&self, tag: u32) -> usize {
        let len = self.len();
        key_len(tag) + encoded_len_varint(len as u64) + len
    }
}
repeated_length_delimited_scalar!(String);
