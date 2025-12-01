//! Protobuf encoding and decoding errors.

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;
#[cfg(not(feature = "std"))]
use alloc::string::String;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use core::error::Error;
use core::fmt;

pub use decode_error_kind::DecodeErrorKind;

/// A Protobuf message decoding error.
///
/// `DecodeError` indicates that the input buffer does not contain a valid
/// Protobuf message. The error details should be considered 'best effort': in
/// general it is not possible to exactly pinpoint why data is malformed.
#[derive(Clone, PartialEq, Eq)]
pub struct DecodeError {
    inner: Box<Inner>,
}

#[derive(Clone, PartialEq, Eq)]
struct Inner {
    /// A 'best effort' root cause description.
    kind: DecodeErrorKind,
    /// Logical path to where the error occurred.
    ///
    /// Internally, this is a stack with an entry per level of nesting.
    path: ErrorPath,
}

impl DecodeError {
    /// Creates a new `DecodeError` with a DecodeErrorKind::UnexpectedTypeUrl.
    ///
    /// Meant to be used only by `prost_types::Any` implementation.
    #[doc(hidden)]
    #[cold]
    pub fn new_unexpected_type_url(actual: impl Into<String>, expected: impl Into<String>) -> Self {
        decode_error_kind::UnexpectedTypeUrl::new(actual.into(), expected.into()).into()
    }

    /// Get details about the decode error
    pub fn kind(&self) -> &DecodeErrorKind {
        &self.inner.kind
    }

    /// Get the location where the error occurred as a logical path.
    ///
    /// The error path represents the stack of message fields being
    /// decoded as the error occurred.
    pub fn path(&self) -> &ErrorPath {
        &self.inner.path
    }

    /// Get a mutable reference to the error path
    ///
    /// This API is hidden to prevent accidental misuse. It is still public to
    /// enable advanced use cases such as creating decode errors from manual
    /// implementations in third-party crates.
    #[doc(hidden)]
    pub fn path_mut(&mut self) -> &mut ErrorPath {
        &mut self.inner.path
    }
}

/// Logical path to the location of an error using Protobuf fields.
///
/// This struct provides context for errors such as [`DecodeError`]. It tracks
/// the logical call-stack of where an error occurred. It stores a path from
/// the root where decoding started down to some nested field where the error
/// actually occurred.
///
/// Each level is represented by an [`ErrorPathSegment`] value. You can retrieve
/// segments using the [`iter`] method.
///
/// An empty path represents an error that happened "at the root".
#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ErrorPath {
    segments: Vec<ErrorPathSegment>,
}

impl ErrorPath {
    /// Create a new empty error path.
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    /// Get an iterator for all the segments in this error path.
    ///
    /// The segments are iterated in the direction from the root down to the
    /// nested field.
    pub fn iter(&self) -> PathSegmentIter<'_> {
        PathSegmentIter {
            inner: self.segments.iter(),
        }
    }

    pub fn push_segment(&mut self, segment: ErrorPathSegment) {
        self.segments.push(segment);
    }
}

impl<'path> IntoIterator for &'path ErrorPath {
    type Item = &'path ErrorPathSegment;
    type IntoIter = PathSegmentIter<'path>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Iterator over all the segments in an [`ErrorPath`] value.
///
/// Segments are iterated starting from the root down to the nested field.
///
/// This iterator is double-ended. Use the [`rev`](Iterator::rev) method to
/// iterate from the nested field up to the root.
pub struct PathSegmentIter<'path> {
    inner: core::slice::Iter<'path, ErrorPathSegment>,
}

impl<'path> Iterator for PathSegmentIter<'path> {
    type Item = &'path ErrorPathSegment;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'path> DoubleEndedIterator for PathSegmentIter<'path> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}

/// A segment identifying a specific Protobuf message field by name.
///
/// This type is usually retrieved from an [`ErrorPath`] value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct ErrorPathSegment {
    message: &'static str,
    field: &'static str,
}

impl ErrorPathSegment {
    /// Create a new error path segment.
    ///
    /// The `message` and `field` parameters may be any string, but it is
    /// recommended to follow the following format:
    /// - `message`: dot-separated absolute Protobuf message name (no leading dot); e.g. `com.example.GitRepository`
    /// - `field`: field name, as found in the Protobuf definition; e.g. `default_branch`
    ///
    /// This API is hidden to prevent accidental misuse using invalid message or
    /// field names. It is still public to enable advanced use cases such as
    /// creating decode errors from manual implementations in third-party crates.
    #[doc(hidden)]
    pub fn new(message: &'static str, field: &'static str) -> Self {
        Self { message, field }
    }

    /// Get the protobuf message name
    pub fn message(&self) -> &'static str {
        self.message
    }

    /// Get the protobuf message name
    pub fn field(&self) -> &'static str {
        self.field
    }
}

impl fmt::Debug for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DecodeError")
            .field("kind", &self.inner.kind)
            .field("path", &self.path())
            .finish()
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("failed to decode Protobuf message")?;
        for segment in self.path().iter() {
            write!(
                f,
                ": {message}.{field}",
                message = segment.message(),
                field = segment.field()
            )?;
        }
        Ok(())
    }
}

impl From<DecodeErrorKind> for DecodeError {
    fn from(kind: DecodeErrorKind) -> Self {
        DecodeError {
            inner: Box::new(Inner {
                kind,
                path: ErrorPath::new(),
            }),
        }
    }
}

pub mod decode_error_kind {
    use super::*;
    use crate::encoding::WireType;

    macro_rules! impl_decode_error_kind {
        {
            $(
                $(#[doc = $doc:literal])?
                $(#[cfg($($cfg_value:tt)+)])*
                #[description($description:literal)]
                pub struct $name:ident {
                    $(
                        #[get($field_get:ty $(, $get_method:ident)?)]
                        $(#[$field_meta:meta])*
                        $field:ident: $field_type:ty
                    ),*$(,)?
                }
            )*
        } => {
            #[derive(Clone, Debug, PartialEq, Eq)]
            #[non_exhaustive]
            pub enum DecodeErrorKind {
                $(
                    $(#[doc = $doc])?
                    $name($name),
                )*
            }

            /// Retrieve the inner error value
            impl DecodeErrorKind {
                pub fn inner(&self) -> &(dyn Error + 'static) {
                    match self {
                        $(Self::$name(inner) => inner,)*
                    }
                }
            }

            impl fmt::Display for DecodeErrorKind {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    match self {
                        $(Self::$name(inner) => inner.fmt(f),)*
                    }
                }
            }

            $(
                $(#[doc = $doc])?
                #[derive(Debug, Clone, PartialEq, Eq)]
                #[non_exhaustive]
                pub struct $name {
                    $(
                        $(#[$field_meta])*
                        $field: $field_type,
                    )*
                }

                impl $name {
                    #[doc(hidden)]
                    pub fn new($($field: $field_type,)*) -> Self {
                        Self {
                            $($field: $field,)*
                        }
                    }

                    pub fn into_decode_error_kind(self) -> super::DecodeErrorKind {
                        super::DecodeErrorKind::$name(self)
                    }

                    pub fn into_decode_error(self) -> super::DecodeError {
                        super::DecodeError::from(self.into_decode_error_kind())
                    }

                    $(
                        pub fn $field(&self) -> $field_get {
                            self.$field $(.$get_method())?
                        }
                    )*
                }

                impl fmt::Display for $name {
                    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        write!(f, $description, $($field = self.$field,)*)
                    }
                }

                impl Error for $name {}

                impl From<$name> for DecodeErrorKind {
                    fn from(value: $name) -> Self {
                        value.into_decode_error_kind()
                    }
                }

                impl From<$name> for DecodeError {
                    fn from(value: $name) -> Self {
                        value.into_decode_error()
                    }
                }
            )*
        };
    }

    impl_decode_error_kind! {
        /// Length delimiter exceeds maximum usize value
        #[description("length delimiter exceeds maximum usize value")]
        pub struct LengthDelimiterTooLarge {}

        /// Invalid varint
        #[description("invalid varint")]
        pub struct InvalidVarint {}

        /// Recursion limit reached
        #[cfg(not(feature = "no-recursion-limit"))]
        #[description("recursion limit reached")]
        pub struct RecursionLimitReached {}

        /// Invalid wire type value
        #[description("invalid wire type value: {value}")]
        pub struct InvalidWireType {
          #[get(u64)]
          value: u64,
        }

        /// Invalid key value
        #[description("invalid key value: {key}")]
        pub struct InvalidKey { #[get(u64)]
            key: u64,
        }

        /// Invalid tag value: 0
        #[description("invalid tag value: 0")]
        pub struct InvalidTag {}

        /// Invalid wire type
        #[description("invalid wire type: {actual:?} (expected {expected:?})")]
        pub struct UnexpectedWireType {
            #[get(WireType)]
            actual: WireType,
            #[get(WireType)]
            expected: WireType,
        }

        /// Buffer underflow
        #[description("buffer underflow")]
        pub struct BufferUnderflow {}

        /// Delimited length exceeded
        #[description("delimited length exceeded")]
        pub struct DelimitedLengthExceeded{}

        /// Unexpected end group tag
        #[description("unexpected end group tag")]
        pub struct UnexpectedEndGroupTag{}

        /// Invalid string value: data is not UTF-8 encoded
        #[description("invalid string value: data is not UTF-8 encoded")]
        pub struct InvalidString{}

        /// Unexpected type URL
        #[description("unexpected type URL.type_url: expected type URL: \"{expected}\" (got: \"{actual}\")")]
        pub struct UnexpectedTypeUrl {
            #[get(&str, as_str)]
            actual: String,
            #[get(&str, as_str)]
            expected: String
        }
    }
}

impl Error for DecodeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.inner.kind.inner())
    }
}

#[cfg(feature = "std")]
impl From<DecodeError> for std::io::Error {
    fn from(error: DecodeError) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::InvalidData, error)
    }
}

/// A Protobuf message encoding error.
///
/// `EncodeError` always indicates that a message failed to encode because the
/// provided buffer had insufficient capacity. Message encoding is otherwise
/// infallible.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct EncodeError {
    required: usize,
    remaining: usize,
}

impl EncodeError {
    /// Creates a new `EncodeError`.
    ///
    /// This assumes that `required > remaining`, without checking it.
    pub(crate) fn new_unchecked(required: usize, remaining: usize) -> EncodeError {
        EncodeError {
            required,
            remaining,
        }
    }

    /// Creates a new `EncodeError`.
    ///
    /// The input must verify `required > remaining`. If it is not verified,
    /// `None` is returned.
    pub fn new(required: usize, remaining: usize) -> Option<EncodeError> {
        if required > remaining {
          Some(Self::new_unchecked(required, remaining))
        } else {
          None
        }
    }

    /// Returns the required buffer capacity to encode the message.
    pub fn required_capacity(&self) -> usize {
        self.required
    }

    /// Returns the remaining length in the provided buffer at the time of encoding.
    pub fn remaining(&self) -> usize {
        self.remaining
    }
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "failed to encode Protobuf message; insufficient buffer capacity (required: {}, remaining: {})",
            self.required, self.remaining
        )
    }
}

impl core::error::Error for EncodeError {}

#[cfg(feature = "std")]
impl From<EncodeError> for std::io::Error {
    fn from(error: EncodeError) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, error)
    }
}

/// An error indicating that an unknown enumeration value was encountered.
///
/// The Protobuf spec mandates that enumeration value sets are ‘open’, so this
/// error's value represents an integer value unrecognized by the
/// presently used enum definition.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct UnknownEnumValue(pub i32);

impl fmt::Display for UnknownEnumValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown enumeration value {}", self.0)
    }
}

impl core::error::Error for UnknownEnumValue {}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_push() {
        let mut decode_error = decode_error_kind::InvalidVarint::new().into_decode_error();
        decode_error
            .path_mut()
            .push_segment(ErrorPathSegment::new("Foo bad", "bar.foo"));
        decode_error
            .path_mut()
            .push_segment(ErrorPathSegment::new("Baz bad", "bar.baz"));

        assert_eq!(
            decode_error.to_string(),
            "failed to decode Protobuf message: Foo bad.bar.foo: Baz bad.bar.baz"
        );
        assert_eq!(decode_error.source().unwrap().to_string(), "invalid varint");
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_into_std_io_error() {
        let decode_error = decode_error_kind::InvalidVarint::new().into_decode_error();
        let std_io_error = std::io::Error::from(decode_error);

        assert_eq!(std_io_error.kind(), std::io::ErrorKind::InvalidData);
        assert_eq!(
            std_io_error.to_string(),
            "failed to decode Protobuf message"
        );
        assert_eq!(std_io_error.source().unwrap().to_string(), "invalid varint");
    }
}
