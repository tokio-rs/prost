//! Translation of
//! [plugin.proto v3.2.0](https://github.com/google/protobuf/blob/v3.2.0/src/google/protobuf/descriptor.proto)
//! to Rust.

/// The protocol compiler can output a FileDescriptorSet containing the .proto
/// files it parses.
#[derive(Debug, Message)]
struct FileDescriptorSet {
    #[proto(tag="1")]
    file: Vec<FileDescriptorProto>,
}

/// Describes a complete .proto file.
#[derive(Debug, Message)]
struct FileDescriptorProto {
    /// file name, relative to root of source tree
    #[proto(tag="1")]
    name: String,
    /// e.g. "foo", "foo.bar", etc.
    #[proto(tag="2")]
    package: String,

    /// Names of files imported by this file.
    #[proto(tag="3")]
    dependency: Vec<String>,
    /// Indexes of the public imported files in the dependency list above.
    #[proto(tag="10")]
    public_dependency: Vec<i32>,
    /// Indexes of the weak imported files in the dependency list.
    /// For Google-internal migration only. Do not use.
    #[proto(tag="11")]
    weak_dependency: Vec<i32>,

    /// All top-level definitions in this file.
    #[proto(tag="4")]
    message_type: Vec<DescriptorProto>,
    #[proto(tag="5")]
    enum_type: Vec<EnumDescriptorProto>,
    #[proto(tag="6")]
    service: Vec<ServiceDescriptorProto>,
    #[proto(tag="7")]
    extension: Vec<FieldDescriptorProto>,

    #[proto(tag="8")]
    options: Option<FileOptions>,

    /// This field contains optional information about the original source code.
    /// You may safely remove this entire field without harming runtime
    /// functionality of the descriptors -- the information is needed only by
    /// development tools.
    #[proto(tag="9")]
    source_code_info: Option<SourceCodeInfo>,

    /// The syntax of the proto file.
    /// The supported values are "proto2" and "proto3".
    #[proto(tag="12")]
    syntax: String,
}

pub mod descriptor_proto {
    #[derive(Debug, Message)]
    pub struct ExtensionRange {
        #[proto(tag="1")]
        start: i32,
        #[proto(tag="2")]
        end: i32,
    }

    /// Range of reserved tag numbers. Reserved tag numbers may not be used by
    /// fields or extension ranges in the same message. Reserved ranges may
    /// not overlap.
    #[derive(Debug, Message)]
    pub struct ReservedRange {
        /// Inclusive.
        #[proto(tag="1")]
        start: i32,
        /// Exclusive.
        #[proto(tag="2")]
        end: i32,
    }
}

/// Describes a message type.
#[derive(Debug, Message)]
struct DescriptorProto {
    #[proto(tag="1")]
    name: String,

    #[proto(tag="2")]
    field: Vec<FieldDescriptorProto>,
    #[proto(tag="6")]
    extension: Vec<FieldDescriptorProto>,

    #[proto(tag="3")]
    nested_type: Vec<DescriptorProto>,
    #[proto(tag="4")]
    enum_type: Vec<EnumDescriptorProto>,

    #[proto(tag="5")]
    extension_range: Vec<descriptor_proto::ExtensionRange>,

    #[proto(tag="8")]
    oneof_decl: Vec<OneofDescriptorProto>,

    #[proto(tag="7")]
    options: Option<MessageOptions>,

    #[proto(tag="9")]
    reserved_range: Vec<descriptor_proto::ReservedRange>,
    /// Reserved field names, which may not be used by fields in the same message.
    /// A given name may only be reserved once.
    #[proto(tag="10")]
    reserved_name: Vec<String>,
}

pub mod field_descriptor_proto {
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, Enumeration)]
    pub enum Type {
        /// 0 is reserved for errors.
        /// Order is weird for historical reasons.
        TYPE_DOUBLE         = 1,
        TYPE_FLOAT          = 2,
        /// Not ZigZag encoded.  Negative numbers take 10 bytes.  Use TYPE_SINT64 if
        /// negative values are likely.
        TYPE_INT64          = 3,
        TYPE_UINT64         = 4,
        /// Not ZigZag encoded.  Negative numbers take 10 bytes.  Use TYPE_SINT32 if
        /// negative values are likely.
        TYPE_INT32          = 5,
        TYPE_FIXED64        = 6,
        TYPE_FIXED32        = 7,
        TYPE_BOOL           = 8,
        TYPE_STRING         = 9,
        /// Tag-delimited aggregate.
        /// Group type is deprecated and not supported in proto3. However, Proto3
        /// implementations should still be able to parse the group wire format and
        /// treat group fields as unknown fields.
        TYPE_GROUP          = 10,
        /// Length-delimited aggregate.
        TYPE_MESSAGE        = 11,

        /// New in version 2.
        TYPE_BYTES          = 12,
        TYPE_UINT32         = 13,
        TYPE_ENUM           = 14,
        TYPE_SFIXED32       = 15,
        TYPE_SFIXED64       = 16,
        /// Uses ZigZag encoding.
        TYPE_SINT32         = 17,
        /// Uses ZigZag encoding.
        TYPE_SINT64         = 18,
    }

    #[repr(u8)]
    #[derive(Clone, Copy, Debug, Enumeration)]
    pub enum Label {
        /// 0 is reserved for errors
        LABEL_OPTIONAL      = 1,
        LABEL_REQUIRED      = 2,
        LABEL_REPEATED      = 3,
    }
}

/// Describes a field within a message.
#[derive(Debug, Message)]
struct FieldDescriptorProto {
#[proto(tag="1")]
    name: String,
    #[proto(tag="3")]
    number: i32,
    #[proto(tag="4")]
    label: field_descriptor_proto::Label,

    /// If type_name is set, this need not be set.  If both this and type_name
    /// are set, this must be one of TYPE_ENUM, TYPE_MESSAGE or TYPE_GROUP.
    #[proto(tag="5")]
    _type: field_descriptor_proto::Type,

    /// For message and enum types, this is the name of the type.  If the name
    /// starts with a '.', it is fully-qualified.  Otherwise, C++-like scoping
    /// rules are used to find the type (i.e. first the nested types within this
    /// message are searched, then within the parent, on up to the root
    /// namespace).
    #[proto(tag="6")]
    type_name: String,

    /// For extensions, this is the name of the type being extended.  It is
    /// resolved in the same manner as type_name.
    #[proto(tag="2")]
    extendee: String,

    /// For numeric types, contains the original text representation of the value.
    /// For booleans, "true" or "false".
    /// For strings, contains the default text contents (not escaped in any way).
    /// For bytes, contains the C escaped value.  All bytes >= 128 are escaped.
    /// TODO(kenton):  Base-64 encode?
    #[proto(tag="7")]
    default_value: String,

    /// If set, gives the index of a oneof in the containing type's oneof_decl
    /// list.  This field is a member of that oneof.
    #[proto(tag="9")]
    oneof_index: i32,

    /// JSON name of this field. The value is set by protocol compiler. If the
    /// user has set a "json_name" option on this field, that option's value
    /// will be used. Otherwise, it's deduced from the field's name by converting
    /// it to camelCase.
    #[proto(tag="10")]
    json_name: String,

    #[proto(tag="8")]
    options: Option<FieldOptions>,
}

/// Describes a oneof.
#[derive(Debug, Message)]
struct OneofDescriptorProto {
    #[proto(tag="1")]
    name: String,
    #[proto(tag="2")]
    options: Option<OneofOptions>,
}

/// Describes an enum type.
#[derive(Debug, Message)]
struct EnumDescriptorProto {
    #[proto(tag="1")]
    name: String,

    #[proto(tag="2")]
    value: Vec<EnumValueDescriptorProto>,

    #[proto(tag="3")]
    options: Option<EnumOptions>,
}

/// Describes a value within an enum.
#[derive(Debug, Message)]
struct EnumValueDescriptorProto {
    #[proto(tag="1")]
    name: String,

    #[proto(tag="2")]
    number: i32,

    #[proto(tag="3")]
    options: Option<EnumValueOptions>,
}

/// Describes a service.
#[derive(Debug, Message)]
struct ServiceDescriptorProto {
    #[proto(tag="1")]
    name: String,
    #[proto(tag="2")]
    method: Vec<MethodDescriptorProto>,

    #[proto(tag="3")]
    options: Option<ServiceOptions>,
}

/// Describes a method of a service.
#[derive(Debug, Message)]
struct MethodDescriptorProto {
    #[proto(tag="1")]
    name: String,

    /// Input and output type names.  These are resolved in the same way as
    /// FieldDescriptorProto.type_name, but must refer to a message type.
    #[proto(tag="2")]
    input_type: String,
    #[proto(tag="3")]
    output_type: String,

    #[proto(tag="4")]
    options: Option<MethodOptions>,

    /// Identifies if client streams multiple client messages
    #[proto(tag="5", default="false")]
    client_streaming: bool,
    /// Identifies if server streams multiple server messages
    #[proto(tag="6", default="false")]
    server_streaming: bool,
}

// ===================================================================
// Options

// Each of the definitions above may have "options" attached.  These are
// just annotations which may cause code to be generated slightly differently
// or may contain hints for code that manipulates protocol messages.
//
// Clients may define custom options as extensions of the *Options messages.
// These extensions may not yet be known at parsing time, so the parser cannot
// store the values in them.  Instead it stores them in a field in the *Options
// message called uninterpreted_option. This field must have the same name
// across all *Options messages. We then use this field to populate the
// extensions when we build a descriptor, at which point all protos have been
// parsed and so all extensions are known.
//
// Extension numbers for custom options may be chosen as follows:
// * For options which will only be used within a single application or
//   organization, or for experimental options, use field numbers 50000
//   through 99999.  It is up to you to ensure that you do not use the
//   same number for multiple options.
// * For options which will be published and used publicly by multiple
//   independent entities, e-mail protobuf-global-extension-registry@google.com
//   to reserve extension numbers. Simply provide your project name (e.g.
//   Objective-C plugin) and your project website (if available) -- there's no
//   need to explain how you intend to use them. Usually you only need one
//   extension number. You can declare multiple options with only one extension
//   number by putting them in a sub-message. See the Custom Options section of
//   the docs for examples:
//   https://developers.google.com/protocol-buffers/docs/proto#options
//   If this turns out to be popular, a web service will be set up
//   to automatically assign option numbers.

pub mod file_options {
    /// Generated classes can be optimized for speed or code size.
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, Enumeration)]
    pub enum OptimizeMode {
        /// Generate complete code for parsing, serialization, etc.
        SPEED = 1,
        /// Use ReflectionOps to implement these methods.
        CODE_SIZE = 2,
        /// Generate code using MessageLite and the lite runtime.
        LITE_RUNTIME = 3,
    }
}

#[derive(Debug, Message)]
struct FileOptions {

    /// Sets the Java package where classes generated from this .proto will be
    /// placed.  By default, the proto package is used, but this is often
    /// inappropriate because proto packages do not normally start with backwards
    /// domain names.
    #[proto(tag="1")]
    java_package: String,

    /// If set, all the classes from the .proto file are wrapped in a single
    /// outer class with the given name.  This applies to both Proto1
    /// (equivalent to the old "--one_java_file" option) and Proto2 (where
    /// a .proto always translates to a single class, but you may want to
    /// explicitly choose the class name).
    #[proto(tag="8")]
    java_outer_classname: String,

    /// If set true, then the Java code generator will generate a separate .java
    /// file for each top-level message, enum, and service defined in the .proto
    /// file.  Thus, these types will *not* be nested inside the outer class
    /// named by java_outer_classname.  However, the outer class will still be
    /// generated to contain the file's getDescriptor() method as well as any
    /// top-level extensions defined in the file.
    #[proto(tag="10", default="false")]
    java_multiple_files: bool,

    /// This option does nothing.
    #[proto(tag="20")]
    #[deprecated]
    java_generate_equals_and_hash: bool,

    /// If set true, then the Java2 code generator will generate code that
    /// throws an exception whenever an attempt is made to assign a non-UTF-8
    /// byte sequence to a string field.
    /// Message reflection will do the same.
    /// However, an extension field still accepts non-UTF-8 byte sequences.
    /// This option has no effect on when used with the lite runtime.
    #[proto(tag="27", default="SPEED")]
    optimize_for: file_options::OptimizeMode,

    /// Sets the Go package where structs generated from this .proto will be
    /// placed. If omitted, the Go package will be derived from the following:
    ///   - The basename of the package import path, if provided.
    ///   - Otherwise, the package statement in the .proto file, if present.
    ///   - Otherwise, the basename of the .proto file, without extension.
    #[proto(tag="11")]
    go_package: String,

    /// Should generic services be generated in each language?  "Generic" services
    /// are not specific to any particular RPC system.  They are generated by the
    /// main code generators in each language (without additional plugins).
    /// Generic services were the only kind of service generation supported by
    /// early versions of google.protobuf.
    ///
    /// Generic services are now considered deprecated in favor of using plugins
    /// that generate code specific to your particular RPC system.  Therefore,
    /// these default to false.  Old code which depends on generic services should
    /// explicitly set them to true.
    #[proto(tag="16", default="false")]
    cc_generic_services: bool,
    #[proto(tag="17", default="false")]
    java_generic_services: bool,
    #[proto(tag="18", default="false")]
    py_generic_services: bool,

    /// Is this file deprecated?
    /// Depending on the target platform, this can emit Deprecated annotations
    /// for everything in the file, or it will be completely ignored; in the very
    /// least, this is a formalization for deprecating files.
    #[proto(tag="23", default="false")]
    deprecated: bool,

    /// Enables the use of arenas for the proto messages in this file. This applies
    /// only to generated classes for C++.
    #[proto(tag="31", default="false")]
    cc_enable_arenas: bool,

    /// Sets the objective c class prefix which is prepended to all objective c
    /// generated classes from this .proto. There is no default.
    #[proto(tag="36")]
    objc_class_prefix: String,

    /// Namespace for generated classes; defaults to the package.
    #[proto(tag="37")]
    csharp_namespace: String,

    /// By default Swift generators will take the proto package and CamelCase it
    /// replacing '.' with underscore and use that to prefix the types/symbols
    /// defined. When this options is provided, they will use this value instead
    /// to prefix the types/symbols defined.
    #[proto(tag="39")]
    swift_prefix: String,

    /// The parser stores options it doesn't recognize here. See above.
    #[proto(tag="999")]
    uninterpreted_option: Vec<UninterpretedOption>,
}

#[derive(Debug, Message)]
struct MessageOptions {
    /// Set true to use the old proto1 MessageSet wire format for extensions.
    /// This is provided for backwards-compatibility with the MessageSet wire
    /// format.  You should not use this for any other reason:  It's less
    /// efficient, has fewer features, and is more complicated.
    ///
    /// The message must be defined exactly as follows:
    ///   message Foo {
    ///     option message_set_wire_format = true;
    ///     extensions 4 to max;
    ///   }
    /// Note that the message cannot have any defined fields; MessageSets only
    /// have extensions.
    ///
    /// All extensions of your type must be singular messages; e.g. they cannot
    /// be int32s, enums, or repeated messages.
    ///
    /// Because this is an option, the above two restrictions are not enforced by
    /// the protocol compiler.
    #[proto(tag="1", default="false")]
    message_set_wire_format: bool,

    /// Disables the generation of the standard "descriptor()" accessor, which can
    /// conflict with a field of the same name.  This is meant to make migration
    /// from proto1 easier; new code should avoid fields named "descriptor".
    #[proto(tag="2", default="false")]
    no_standard_descriptor_accessor: bool,

    /// Is this message deprecated?
    /// Depending on the target platform, this can emit Deprecated annotations
    /// for the message, or it will be completely ignored; in the very least,
    /// this is a formalization for deprecating messages.
    #[proto(tag="3", default="false")]
    deprecated: bool,

    /// Whether the message is an automatically generated map entry type for the
    /// maps field.
    ///
    /// For maps fields:
    ///     map<KeyType, ValueType> map_field = 1;
    /// The parsed descriptor looks like:
    ///     message MapFieldEntry {
    ///         option map_entry = true;
    ///         optional KeyType key = 1;
    ///         optional ValueType value = 2;
    ///     }
    ///     repeated MapFieldEntry map_field = 1;
    ///
    /// Implementations may choose not to generate the map_entry=true message, but
    /// use a native map in the target language to hold the keys and values.
    /// The reflection APIs in such implementions still need to work as
    /// if the field is a repeated message field.
    ///
    /// NOTE: Do not set the option in .proto files. Always use the maps syntax
    /// instead. The option should only be implicitly set by the proto compiler
    /// parser.
    #[proto(tag="7")]
    map_entry: bool,

    /// The parser stores options it doesn't recognize here. See above.
    #[proto(tag="999")]
    uninterpreted_option: Vec<UninterpretedOption>,
}

pub mod field_options {
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, Enumeration)]
    pub enum CType {
        /// Default mode.
        STRING = 0,

        CORD = 1,

        STRING_PIECE = 2,
    }

    #[repr(u8)]
    #[derive(Clone, Copy, Debug, Enumeration)]
    pub enum JSType {
        /// Use the default type.
        JS_NORMAL = 0,

        /// Use JavaScript strings.
        JS_STRING = 1,

        /// Use JavaScript numbers.
        JS_NUMBER = 2,
    }
}

#[derive(Debug, Message)]
struct FieldOptions {
    /// The ctype option instructs the C++ code generator to use a different
    /// representation of the field than it normally would.  See the specific
    /// options below.  This option is not yet implemented in the open source
    /// release -- sorry, we'll try to include it in a future version!
    #[proto(tag="1", default="STRING")]
    ctype: field_options::CType,

    /// The packed option can be enabled for repeated primitive fields to enable
    /// a more efficient representation on the wire. Rather than repeatedly
    /// writing the tag and type for each element, the entire array is encoded as
    /// a single length-delimited blob. In proto3, only explicit setting it to
    /// false will avoid using packed encoding.
    #[proto(tag="2")]
    packed: bool,

    /// The jstype option determines the JavaScript type used for values of the
    /// field.  The option is permitted only for 64 bit integral and fixed types
    /// (int64, uint64, sint64, fixed64, sfixed64).  By default these types are
    /// represented as JavaScript strings.  This avoids loss of precision that can
    /// happen when a large value is converted to a floating point JavaScript
    /// numbers.  Specifying JS_NUMBER for the jstype causes the generated
    /// JavaScript code to use the JavaScript "number" type instead of strings.
    /// This option is an enum to permit additional types to be added,
    /// e.g. goog.math.Integer.
    #[proto(tag="6", default="JS_NORMAL")]
    jstype: field_options::JSType,

    /// Should this field be parsed lazily?  Lazy applies only to message-type
    /// fields.  It means that when the outer message is initially parsed, the
    /// inner message's contents will not be parsed but instead stored in encoded
    /// form.  The inner message will actually be parsed when it is first accessed.
    ///
    /// This is only a hint.  Implementations are free to choose whether to use
    /// eager or lazy parsing regardless of the value of this option.  However,
    /// setting this option true suggests that the protocol author believes that
    /// using lazy parsing on this field is worth the additional bookkeeping
    /// overhead typically needed to implement it.
    ///
    /// This option does not affect the public interface of any generated code;
    /// all method signatures remain the same.  Furthermore, thread-safety of the
    /// interface is not affected by this option; const methods remain safe to
    /// call from multiple threads concurrently, while non-const methods continue
    /// to require exclusive access.
    ///
    ///
    /// Note that implementations may choose not to check required fields within
    /// a lazy sub-message.  That is, calling IsInitialized() on the outer message
    /// may return true even if the inner message has missing required fields.
    /// This is necessary because otherwise the inner message would have to be
    /// parsed in order to perform the check, defeating the purpose of lazy
    /// parsing.  An implementation which chooses not to check required fields
    /// must be consistent about it.  That is, for any particular sub-message, the
    /// implementation must either *always* check its required fields, or *never*
    /// check its required fields, regardless of whether or not the message has
    /// been parsed.
    #[proto(tag="5", default="false")]
    lazy: bool,

    /// Is this field deprecated?
    /// Depending on the target platform, this can emit Deprecated annotations
    /// for accessors, or it will be completely ignored; in the very least, this
    /// is a formalization for deprecating fields.
    #[proto(tag="3", default="false")]
    deprecated: bool,

    /// For Google-internal migration only. Do not use.
    #[proto(tag="10", default="false")]
    weak: bool,


    /// The parser stores options it doesn't recognize here. See above.
    #[proto(tag="999")]
    uninterpreted_option: Vec<UninterpretedOption>,
}

#[derive(Debug, Message)]
struct OneofOptions {
    /// The parser stores options it doesn't recognize here. See above.
#[proto(tag="999")]
    uninterpreted_option: Vec<UninterpretedOption>,
}

#[derive(Debug, Message)]
struct EnumOptions {

    /// Set this option to true to allow mapping different tag names to the same
    /// value.
    #[proto(tag="2")]
    allow_alias: bool,

    /// Is this enum deprecated?
    /// Depending on the target platform, this can emit Deprecated annotations
    /// for the enum, or it will be completely ignored; in the very least, this
    /// is a formalization for deprecating enums.
    #[proto(tag="3", default="false")]
    deprecated: bool,

    /// The parser stores options it doesn't recognize here. See above.
    #[proto(tag="999")]
    uninterpreted_option: Vec<UninterpretedOption>,
}

#[derive(Debug, Message)]
struct EnumValueOptions {
    /// Is this enum value deprecated?
    /// Depending on the target platform, this can emit Deprecated annotations
    /// for the enum value, or it will be completely ignored; in the very least,
    /// this is a formalization for deprecating enum values.
    #[proto(tag="1", default="false")]
    deprecated: bool,

    /// The parser stores options it doesn't recognize here. See above.
    #[proto(tag="999")]
    uninterpreted_option: Vec<UninterpretedOption>,
}

#[derive(Debug, Message)]
struct ServiceOptions {

    /// Note:  Field numbers 1 through 32 are reserved for Google's internal RPC
    ///   framework.  We apologize for hoarding these numbers to ourselves, but
    ///   we were already using them long before we decided to release Protocol
    ///   Buffers.

    /// Is this service deprecated?
    /// Depending on the target platform, this can emit Deprecated annotations
    /// for the service, or it will be completely ignored; in the very least,
    /// this is a formalization for deprecating services.
    #[proto(tag="33", default="false")]
    deprecated: bool,

    /// The parser stores options it doesn't recognize here. See above.
    #[proto(tag="999")]
    uninterpreted_option: Vec<UninterpretedOption>,
}

pub mod method_options {
    /// Is this method side-effect-free (or safe in HTTP parlance), or idempotent,
    /// or neither? HTTP based RPC implementation may choose GET verb for safe
    /// methods, and PUT verb for idempotent methods instead of the default POST.
    #[repr(u8)]
    #[derive(Clone, Copy, Debug, Enumeration)]
    pub enum IdempotencyLevel {
        IDEMPOTENCY_UNKNOWN = 0,
        /// implies idempotent
        NO_SIDE_EFFECTS     = 1,
        /// idempotent, but may have side effects
        IDEMPOTENT          = 2,
    }
}

#[derive(Debug, Message)]
struct MethodOptions {

    /// Note: Field numbers 1 through 32 are reserved for Google's internal RPC
    ///   framework.  We apologize for hoarding these numbers to ourselves, but
    ///   we were already using them long before we decided to release Protocol
    ///   Buffers.

    /// Is this method deprecated?
    /// Depending on the target platform, this can emit Deprecated annotations
    /// for the method, or it will be completely ignored; in the very least,
    /// this is a formalization for deprecating methods.
    #[proto(tag="33", default="false")]
    deprecated: bool,

    #[proto(tag="34", default="IDEMPOTENCY_UNKNOWN")]
    idempotency_level: method_options::IdempotencyLevel,

    /// The parser stores options it doesn't recognize here. See above.
    #[proto(tag="999")]
    uninterpreted_option: Vec<UninterpretedOption>,
}

pub mod uninterpreted_option {
    #[derive(Debug, Message)]
    pub struct NamePart {
        #[proto(tag="1")]
        name_part: String,
        #[proto(tag="2")]
        is_extension: bool,
    }
}

/// A message representing a option the parser does not recognize. This only
/// appears in options protos created by the compiler::Parser class.
/// DescriptorPool resolves these when building Descriptor objects. Therefore,
/// options protos in descriptor objects (e.g. returned by Descriptor::options(),
/// or produced by Descriptor::CopyTo()) will never have UninterpretedOptions
/// in them.
#[derive(Debug, Message)]
struct UninterpretedOption {
    /// The name of the uninterpreted option.  Each string represents a segment in
    /// a dot-separated name.  is_extension is true iff a segment represents an
    /// extension (denoted with parentheses in options specs in .proto files).
    /// E.g.,{ ["foo", false], ["bar.baz", true], ["qux", false] } represents
    /// "foo.(bar.baz).qux".
    #[proto(tag="2")]
    name: Vec<uninterpreted_option::NamePart>,

    /// The value of the uninterpreted option, in whatever type the tokenizer
    /// identified it as during parsing. Exactly one of these should be set.
    #[proto(tag="3")]
    identifier_value: String,
    #[proto(tag="4")]
    positive_int_value: u64,
    #[proto(tag="5")]
    negative_int_value: i64,
    #[proto(tag="6")]
    double_value: f64,
    #[proto(tag="7")]
    string_value: Vec<u8>,
    #[proto(tag="8")]
    aggregate_value: String,
}

// ===================================================================
// Optional source code info

pub mod source_code_info {
    #[derive(Debug, Message)]
    pub struct Location {
        /// Identifies which part of the FileDescriptorProto was defined at this
        /// location.
        ///
        /// Each element is a field number or an index.  They form a path from
        /// the root FileDescriptorProto to the place where the definition.  For
        /// example, this path:
        ///   [ 4, 3, 2, 7, 1 ]
        /// refers to:
        ///   file.message_type(3)  // 4, 3
        ///       .field(7)         // 2, 7
        ///       .name()           // 1
        /// This is because FileDescriptorProto.message_type has field number 4:
        ///   repeated DescriptorProto message_type = 4;
        /// and DescriptorProto.field has field number 2:
        ///   repeated FieldDescriptorProto field = 2;
        /// and FieldDescriptorProto.name has field number 1:
        ///   optional string name = 1;
        ///
        /// Thus, the above path gives the location of a field name.  If we removed
        /// the last element:
        ///   [ 4, 3, 2, 7 ]
        /// this path refers to the whole field declaration (from the beginning
        /// of the label to the terminating semicolon).
        #[proto(tag="1")]
        path: Vec<i32>,

        /// Always has exactly three or four elements: start line, start column,
        /// end line (optional, otherwise assumed same as start line), end column.
        /// These are packed into a single field for efficiency.  Note that line
        /// and column numbers are zero-based -- typically you will want to add
        /// 1 to each before displaying to a user.
        #[proto(tag="2")]
        span: Vec<i32>,

        /// If this SourceCodeInfo represents a complete declaration, these are any
        /// comments appearing before and after the declaration which appear to be
        /// attached to the declaration.
        ///
        /// A series of line comments appearing on consecutive lines, with no other
        /// tokens appearing on those lines, will be treated as a single comment.
        ///
        /// leading_detached_comments will keep paragraphs of comments that appear
        /// before (but not connected to) the current element. Each paragraph,
        /// separated by empty lines, will be one comment element in the repeated
        /// field.
        ///
        /// Only the comment content is provided; comment markers (e.g. //) are
        /// stripped out.  For block comments, leading whitespace and an asterisk
        /// will be stripped from the beginning of each line other than the first.
        /// Newlines are included in the output.
        ///
        /// Examples:
        ///
        ///   optional int32 foo = 1;  // Comment attached to foo.
        ///   // Comment attached to bar.
        ///   optional int32 bar = 2;
        ///
        ///   optional string baz = 3;
        ///   // Comment attached to baz.
        ///   // Another line attached to baz.
        ///
        ///   // Comment attached to qux.
        ///   //
        ///   // Another line attached to qux.
        ///   optional double qux = 4;
        ///
        ///   // Detached comment for corge. This is not leading or trailing comments
        ///   // to qux or corge because there are blank lines separating it from
        ///   // both.
        ///
        ///   // Detached comment for corge paragraph 2.
        ///
        ///   optional string corge = 5;
        ///   /* Block comment attached
        ///    * to corge.  Leading asterisks
        ///    * will be removed. */
        ///   /* Block comment attached to
        ///    * grault. */
        ///   optional int32 grault = 6;
        ///
        ///   // ignored detached comments.
        #[proto(tag="3")]
        leading_comments: String,
        #[proto(tag="4")]
        trailing_comments: String,
        #[proto(tag="6")]
        leading_detached_comments: Vec<String>,
    }
}

/// Encapsulates information about the original source file from which a
/// FileDescriptorProto was generated.
#[derive(Debug, Message)]
struct SourceCodeInfo {
    /// A Location identifies a piece of source code in a .proto file which
    /// corresponds to a particular definition.  This information is intended
    /// to be useful to IDEs, code indexers, documentation generators, and similar
    /// tools.
    ///
    /// For example, say we have a file like:
    ///   message Foo {
    ///     optional string foo = 1;
    ///   }
    /// Let's look at just the field definition:
    ///   optional string foo = 1;
    ///   ^       ^^     ^^  ^  ^^^
    ///   a       bc     de  f  ghi
    /// We have the following locations:
    ///   span   path               represents
    ///   [a,i)  [ 4, 0, 2, 0 ]     The whole field definition.
    ///   [a,b)  [ 4, 0, 2, 0, 4 ]  The label (optional).
    ///   [c,d)  [ 4, 0, 2, 0, 5 ]  The type (string).
    ///   [e,f)  [ 4, 0, 2, 0, 1 ]  The name (foo).
    ///   [g,h)  [ 4, 0, 2, 0, 3 ]  The number (1).
    ///
    /// Notes:
    /// - A location may refer to a repeated field itself (i.e. not to any
    ///   particular index within it).  This is used whenever a set of elements are
    ///   logically enclosed in a single code segment.  For example, an entire
    ///   extend block (possibly containing multiple extension definitions) will
    ///   have an outer location whose path refers to the "extensions" repeated
    ///   field without an index.
    /// - Multiple locations may have the same path.  This happens when a single
    ///   logical declaration is spread out across multiple places.  The most
    ///   obvious example is the "extend" block again -- there may be multiple
    ///   extend blocks in the same scope, each of which will have the same path.
    /// - A location's span is not always a subset of its parent's span.  For
    ///   example, the "extendee" of an extension declaration appears at the
    ///   beginning of the "extend" block and is shared by all extensions within
    ///   the block.
    /// - Just because a location's span is a subset of some other location's span
    ///   does not mean that it is a descendent.  For example, a "group" defines
    ///   both a type and a field in a single declaration.  Thus, the locations
    ///   corresponding to the type and field and their components will overlap.
    /// - Code which tries to interpret locations should probably be designed to
    ///   ignore those that it doesn't understand, as more types of locations could
    ///   be recorded in the future.
    #[proto(tag="1")]
    location: Vec<source_code_info::Location>,
}

pub mod generated_code_info {
    #[derive(Debug, Message)]
    pub struct Annotation {
        /// Identifies the element in the original source .proto file. This field
        /// is formatted the same as SourceCodeInfo.Location.path.
        #[proto(tag="1")]
        path: Vec<i32>,

        /// Identifies the filesystem path to the original source .proto.
        #[proto(tag="2")]
        source_file: String,

        /// Identifies the starting offset in bytes in the generated code
        /// that relates to the identified object.
        #[proto(tag="3")]
        begin: i32,

        /// Identifies the ending offset in bytes in the generated code that
        /// relates to the identified offset. The end offset should be one past
        /// the last relevant byte (so the length of the text = end - begin).
        #[proto(tag="4")]
        end: i32,
    }
}

/// Describes the relationship between generated code and its original source
/// file. A GeneratedCodeInfo message is associated with only one generated
/// source file, but may contain references to different source .proto files.
#[derive(Debug, Message)]
struct GeneratedCodeInfo {
    /// An Annotation connects some span of text in generated code to an element
    /// of its generating .proto file.
    #[proto(tag="1")]
    annotation: Vec<generated_code_info::Annotation>,
}
