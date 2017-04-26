pub trait Enumeration : default::Default {
    /// Encodes the enumeration to the buffer, without a key.
    /// The buffer must have enough remaining space to hold the encoded key and field.
    fn encode<B>(self, buf: &mut B) where B: BufMut;

    /// Decodes an instance of the field from the buffer.
    fn decode<B>(buf: &mut B) -> Result<Self> where B: Buf;

    /// Returns the encoded length of the field, without a key.
    fn encoded_len(self) -> usize;

    /// Returns the wire type of the numeric scalar field.
    fn wire_type() -> WireType;
}
