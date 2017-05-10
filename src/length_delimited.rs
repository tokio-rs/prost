use std::io::Result;
use std::str;

use bytes::{
    Buf,
    BufMut,
    Take,
};

use encoding::*;
use field::{
    Field,
    Type,
};

// Provides Field, Type, and repeated Field impls for length-delimited types.
macro_rules! length_delimited_field {
    ($ty: ty) => {
        impl Field for $ty {
            #[inline]
            fn encode<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
                encode_key(tag, WireType::LengthDelimited, buf);
                encode_varint(LengthDelimited::encoded_len(self) as u64, buf);
                LengthDelimited::encode(self, buf);
            }
            #[inline]
            fn merge<B>(&mut self, _tag: u32, wire_type: WireType, buf: &mut Take<B>) -> Result<()> where B: Buf {
                check_wire_type(WireType::LengthDelimited, wire_type)?;
                let len = decode_varint(buf)?;
                if (buf.remaining() as u64) < len {
                    return Err(invalid_input("failed to decode length-delimited field: buffer underflow"));
                }
                let limit = buf.limit();
                buf.set_limit(len as usize);
                LengthDelimited::merge(self, buf)?;
                buf.set_limit(limit - len as usize);
                Ok(())
            }
            #[inline]
            fn encoded_len(&self, tag: u32) -> usize {
                let len = LengthDelimited::encoded_len(self);
                key_len(tag) + encoded_len_varint(len as u64) + len
            }
        }
        impl Type for $ty {}

        impl Field for Vec<$ty> {
            #[inline]
            fn encode<B>(&self, tag: u32, buf: &mut B) where B: BufMut {
                for value in self {
                    Field::encode(value, tag, buf);
                }
            }
            #[inline]
            fn merge<B>(&mut self, tag: u32, wire_type: WireType, buf: &mut Take<B>) -> Result<()> where B: Buf {
                check_wire_type(WireType::LengthDelimited, wire_type)?;
                let mut value = Default::default();
                Field::merge(&mut value, tag, WireType::LengthDelimited, buf)?;
                self.push(value);
                Ok(())
            }
            #[inline]
            fn encoded_len(&self, tag: u32) -> usize {
                self.iter().map(|value| Field::encoded_len(value, tag)).sum()
            }
        }
    };
}
