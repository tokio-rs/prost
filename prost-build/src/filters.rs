use core::{
    convert::TryFrom,
    fmt,
    ops::{BitAnd, BitOr, Not},
};
use prost_types::field_descriptor_proto::{Label, Type};

macro_rules! implement_conversions {
    ($filter:ident, $selector:ident { $($var:ident),* $(,)? }) => {
        impl $filter {
            pub(crate) fn is_set(&self, en: $selector) -> bool {
                self.0 & (en as u32) != 0
            }
        }

        impl From<$selector> for $filter {
            fn from(en: $selector) -> Self {
                $filter(en as u32)
            }
        }

        impl Not for $selector {
            type Output = $filter;
            fn not(self) -> Self::Output {
                !$filter::from(self)
            }
        }

        impl Not for $filter {
            type Output = $filter;
            fn not(self) -> Self::Output {
                $filter(!self.0)
            }
        }

        impl<T: Into<$filter>> BitOr<T> for $selector {
            type Output = $filter;
            fn bitor(self, rhs: T) -> Self::Output {
                $filter::from(self) | <T as Into<$filter>>::into(rhs)
            }
        }

        impl<T: Into<$filter>> BitOr<T> for $filter {
            type Output = $filter;
            fn bitor(self, rhs: T) -> Self::Output {
                $filter(self.0 | rhs.into().0)
            }
        }

        impl<T: Into<$filter>> BitAnd<T> for $selector {
            type Output = $filter;
            fn bitand(self, rhs: T) -> Self::Output {
                $filter::from(self) & <T as Into<$filter>>::into(rhs)
            }
        }

        impl<T: Into<$filter>> BitAnd<T> for $filter {
            type Output = $filter;
            fn bitand(self, rhs: T) -> Self::Output {
                $filter(self.0 & rhs.into().0)
            }
        }

        impl fmt::Debug for $filter {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!(stringify!($filter), "( "))?;
                let mut written = false;
                for idx in 0..32 {
                    let mask = 1<<idx;
                    if mask & self.0 != 0 {
                        if written { write!(f, " | ")?; }
                        written = true;
                        match $selector::try_from(mask) {
                            Ok(ref x) => fmt::Debug::fmt(x, f)?,
                            Err(e) => write!(f, "Unknonwn({})", e)?,
                        }
                    }
                }
                write!(f, " )")
            }
        }

        // this would make automatic https://github.com/rust-lang/rust/pull/81642
        impl TryFrom<u32> for $selector {
            type Error = u32;

            fn try_from(value: u32) -> Result<Self, Self::Error> {
                match value {
                    $( n if n == Self::$var as u32   => Ok(Self::$var), )*
                    oth => Err(oth)
                }
            }
        }
    }
}

macro_rules! impl_from_another {
    ($another:ident, $selector:ident { $($var:ident),* $(,)? }) => {
        impl From<$another> for $selector {
            fn from(another: $another) -> $selector {
                match another {
                    $( $another::$var => $selector::$var ),*

                }
            }
        }
    }
}

/// A collection of `TypeSelector`s. The filter matches if ANY of the
/// inner `TypeSelector`s match. Can be created by `BitOr`ing together
/// `TypeSelector`s or calling `Into::into()` on a `TypeSelector`.
#[derive(Default, Clone, Copy)]
pub struct TypeFilter(u32);

/// Selects a output object (rust struct or enum) during code
/// generation based on either the output rust type or the
/// protobuf type (message, enum oneof) that it represents.
#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum TypeSelector {
    ProtobufMessage = 1 << 0,
    ProtobufEnum = 1 << 1,
    ProtobufOneof = 1 << 2,
    RustStruct = 1 << 3,
    RustEnum = 1 << 4,
    RustEnumCLike = 1 << 5,
    RustEnumWithData = 1 << 6,
    Everything = u32::MAX,
}

implement_conversions!(
    TypeFilter,
    TypeSelector {
        ProtobufMessage,
        ProtobufEnum,
        ProtobufOneof,
        RustStruct,
        RustEnum,
        RustEnumCLike,
        RustEnumWithData,
    }
);

#[derive(Default, Clone, Copy)]
pub struct LabelFilter(u32);

#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum LabelSelector {
    // Protobuf types
    Optional = 1 << 0,
    Required = 1 << 1,
    Repeated = 1 << 2,
    Everything = u32::MAX,
}

implement_conversions!(
    LabelFilter,
    LabelSelector {
        Optional,
        Required,
        Repeated
    }
);

impl_from_another!(
    Label,
    LabelSelector {
        Optional,
        Required,
        Repeated
    }
);

#[derive(Default, Clone, Copy)]
pub struct FieldFilter(u32);

#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum FieldSelector {
    // Protobuf types
    Double = 1 << 0,
    Float = 1 << 1,
    Int64 = 1 << 2,
    Uint64 = 1 << 3,

    Int32 = 1 << 4,
    Fixed64 = 1 << 5,
    Fixed32 = 1 << 6,
    Bool = 1 << 7,

    String = 1 << 8,
    Group = 1 << 9,
    Message = 1 << 10,
    Bytes = 1 << 11,

    Uint32 = 1 << 12,
    Enum = 1 << 13,
    Sfixed32 = 1 << 14,
    Sfixed64 = 1 << 15,

    Sint32 = 1 << 16,
    Sint64 = 1 << 17,

    /// All protobuf types except Group, Message, Enum
    ProtobufScalar = 0b11_1101_1001_1111_1111,

    /// Enum variant with no data
    NoDataEnumVariant = 1 << 19,

    /// oneof field
    OneofField = 1 << 20,

    /// map field
    MapField = 1 << 21,
    Everything = u32::MAX,
}

implement_conversions!(
    FieldFilter,
    FieldSelector {
        Double,
        Float,
        Int64,
        Uint64,
        Int32,
        Fixed64,
        Fixed32,
        Bool,
        String,
        Group,
        Message,
        Bytes,
        Uint32,
        Enum,
        Sfixed32,
        Sfixed64,
        Sint32,
        Sint64,

        ProtobufScalar,
        NoDataEnumVariant,
        OneofField,
        MapField,
    }
);

impl_from_another!(
    Type,
    FieldSelector {
        Double,
        Float,
        Int64,
        Uint64,
        Int32,
        Fixed64,
        Fixed32,
        Bool,
        String,
        Group,
        Message,
        Bytes,
        Uint32,
        Enum,
        Sfixed32,
        Sfixed64,
        Sint32,
        Sint64,
    }
);
