//! Serialize a Rust data structure into Protocol Buffers data.

use serde::{ser, Serialize};

use std::{io, str};

use super::error::{Error, Result};

use prost_types::field_descriptor_proto;

pub struct Serializer {
    output: String,
}

pub fn to_string<T>(value: &T) -> Result<String>
where
    T: Serialize,
{
    let mut serializer = Serializer {
        output: String::new(),
    };
    value.serialize(&mut serializer)?;
    Ok(serializer.output)
}

#[inline]
fn write_bool<W: ?Sized>(&mut self, writer: &mut W, value: bool) -> io::Result<()>
where
    W: io::Write,
{
    let s = if value {
        field_descriptor_proto::Type.
    } else {
        field_descriptor_proto::Type.
    };
    writer.write_all(s)
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = ();

    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<(), error::Error> {

    }

    fn serialize_i8(self, v: i8) -> Result<()> {
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        
    }

    fn serialize_i64(self, v: i64) -> Result<()> {

    }

    fn serialize_u8(self, v: u8) -> Result<()> {

    }

    fn serialize_u16(self, v: u16) -> Result<()> {
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
    }

    fn serialize_char(self, v: char) -> Result<()> {
    }

    fn serialize_str(self, v: &str) -> Result<()> {
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
    }

    fn serialize_none(self) -> Result<()> {
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
    }

    fn serialize_unit(self) -> Result<()> {
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
    }
    
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
    }
    
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
    }
    
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct> {
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
    }
}

impl<'a> ser::SerializeSeq for &'a mut Serializer {
    type Ok = ();
    
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
    }

    fn end(self) -> Result<()> {
    }
}

impl<'a> ser::SerializeTuple for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
    }

    fn end(self) -> Result<()> {
    }
}

impl<'a> ser::SerializeTupleStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
    }

    fn end(self) -> Result<()> {
    }
}

impl<'a> ser::SerializeTupleVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
    }

    fn end(self) -> Result<()> {
    }
}

impl<'a> ser::SerializeMap for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
    }

    fn end(self) -> Result<()> {
    }
}

impl<'a> ser::SerializeStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
    }

    fn end(self) -> Result<()> {
    }
}

impl<'a> ser::SerializeStructVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
    }

    fn end(self) -> Result<()> {
    }
}