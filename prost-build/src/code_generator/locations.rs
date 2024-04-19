pub(super) struct FileDescriptorProtoLocations;

impl FileDescriptorProtoLocations {
    pub const MESSAGE_TYPE: i32 = 4;
    pub const ENUM_TYPE: i32 = 5;
    pub const SERVICE: i32 = 6;
}

pub(super) struct DescriptorLocations;

impl DescriptorLocations {
    pub const FIELD: i32 = 2;
    pub const NESTED_TYPE: i32 = 3;
    pub const ENUM_TYPE: i32 = 4;
    pub const ONEOF_DECL: i32 = 8;
}

pub(super) struct EnumDescriptorLocations;

impl EnumDescriptorLocations {
    pub const VALUE: i32 = 2;
}

pub(super) struct ServiceDescriptorProtoLocations;

impl ServiceDescriptorProtoLocations {
    pub const METHOD: i32 = 2;
}
