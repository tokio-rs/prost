pub mod compiler;
/// The protocol compiler can output a FileDescriptorSet containing the .proto
/// files it parses.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct FileDescriptorSet {
    #[proto(message, repeated, tag="1")]
    pub file: Vec<FileDescriptorProto>,
}
/// Describes a complete .proto file.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct FileDescriptorProto {
    /// file name, relative to root of source tree
    #[proto(string, optional, tag="1")]
    pub name: Option<String>,
    /// e.g. "foo", "foo.bar", etc.
    #[proto(string, optional, tag="2")]
    pub package: Option<String>,
    /// Names of files imported by this file.
    #[proto(string, repeated, tag="3")]
    pub dependency: Vec<String>,
    /// Indexes of the public imported files in the dependency list above.
    #[proto(int32, repeated, packed="false", tag="10")]
    pub public_dependency: Vec<i32>,
    /// Indexes of the weak imported files in the dependency list.
    /// For Google-internal migration only. Do not use.
    #[proto(int32, repeated, packed="false", tag="11")]
    pub weak_dependency: Vec<i32>,
    /// All top-level definitions in this file.
    #[proto(message, repeated, tag="4")]
    pub message_type: Vec<DescriptorProto>,
    #[proto(message, repeated, tag="5")]
    pub enum_type: Vec<EnumDescriptorProto>,
    #[proto(message, repeated, tag="6")]
    pub service: Vec<ServiceDescriptorProto>,
    #[proto(message, repeated, tag="7")]
    pub extension: Vec<FieldDescriptorProto>,
    #[proto(message, optional, tag="8")]
    pub options: Option<FileOptions>,
    /// This field contains optional information about the original source code.
    /// You may safely remove this entire field without harming runtime
    /// functionality of the descriptors -- the information is needed only by
    /// development tools.
    #[proto(message, optional, tag="9")]
    pub source_code_info: Option<SourceCodeInfo>,
    /// The syntax of the proto file.
    /// The supported values are "proto2" and "proto3".
    #[proto(string, optional, tag="12")]
    pub syntax: Option<String>,
}
/// Describes a message type.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct DescriptorProto {
    #[proto(string, optional, tag="1")]
    pub name: Option<String>,
    #[proto(message, repeated, tag="2")]
    pub field: Vec<FieldDescriptorProto>,
    #[proto(message, repeated, tag="6")]
    pub extension: Vec<FieldDescriptorProto>,
    #[proto(message, repeated, tag="3")]
    pub nested_type: Vec<DescriptorProto>,
    #[proto(message, repeated, tag="4")]
    pub enum_type: Vec<EnumDescriptorProto>,
    #[proto(message, repeated, tag="5")]
    pub extension_range: Vec<descriptor_proto::ExtensionRange>,
    #[proto(message, repeated, tag="8")]
    pub oneof_decl: Vec<OneofDescriptorProto>,
    #[proto(message, optional, tag="7")]
    pub options: Option<MessageOptions>,
    #[proto(message, repeated, tag="9")]
    pub reserved_range: Vec<descriptor_proto::ReservedRange>,
    /// Reserved field names, which may not be used by fields in the same message.
    /// A given name may only be reserved once.
    #[proto(string, repeated, tag="10")]
    pub reserved_name: Vec<String>,
}
pub mod descriptor_proto {
    #[derive(Clone, Debug, PartialEq, Message)]
    pub struct ExtensionRange {
        #[proto(int32, optional, tag="1")]
        pub start: Option<i32>,
        #[proto(int32, optional, tag="2")]
        pub end: Option<i32>,
    }
    /// Range of reserved tag numbers. Reserved tag numbers may not be used by
    /// fields or extension ranges in the same message. Reserved ranges may
    /// not overlap.
    #[derive(Clone, Debug, PartialEq, Message)]
    pub struct ReservedRange {
        /// Inclusive.
        #[proto(int32, optional, tag="1")]
        pub start: Option<i32>,
        /// Exclusive.
        #[proto(int32, optional, tag="2")]
        pub end: Option<i32>,
    }
}
/// Describes a field within a message.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct FieldDescriptorProto {
    #[proto(string, optional, tag="1")]
    pub name: Option<String>,
    #[proto(int32, optional, tag="3")]
    pub number: Option<i32>,
    #[proto(enumeration="field_descriptor_proto::Label", optional, tag="4")]
    pub label: Option<i32>,
    /// If type_name is set, this need not be set.  If both this and type_name
    /// are set, this must be one of TYPE_ENUM, TYPE_MESSAGE or TYPE_GROUP.
    #[proto(enumeration="field_descriptor_proto::Type", optional, tag="5")]
    pub type_: Option<i32>,
    /// For message and enum types, this is the name of the type.  If the name
    /// starts with a '.', it is fully-qualified.  Otherwise, C++-like scoping
    /// rules are used to find the type (i.e. first the nested types within this
    /// message are searched, then within the parent, on up to the root
    /// namespace).
    #[proto(string, optional, tag="6")]
    pub type_name: Option<String>,
    /// For extensions, this is the name of the type being extended.  It is
    /// resolved in the same manner as type_name.
    #[proto(string, optional, tag="2")]
    pub extendee: Option<String>,
    /// For numeric types, contains the original text representation of the value.
    /// For booleans, "true" or "false".
    /// For strings, contains the default text contents (not escaped in any way).
    /// For bytes, contains the C escaped value.  All bytes >= 128 are escaped.
    /// TODO(kenton):  Base-64 encode?
    #[proto(string, optional, tag="7")]
    pub default_value: Option<String>,
    /// If set, gives the index of a oneof in the containing type's oneof_decl
    /// list.  This field is a member of that oneof.
    #[proto(int32, optional, tag="9")]
    pub oneof_index: Option<i32>,
    /// JSON name of this field. The value is set by protocol compiler. If the
    /// user has set a "json_name" option on this field, that option's value
    /// will be used. Otherwise, it's deduced from the field's name by converting
    /// it to camelCase.
    #[proto(string, optional, tag="10")]
    pub json_name: Option<String>,
    #[proto(message, optional, tag="8")]
    pub options: Option<FieldOptions>,
}
pub mod field_descriptor_proto {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Enumeration)]
    pub enum Type {
        /// 0 is reserved for errors.
        /// Order is weird for historical reasons.
        TypeDouble = 1,
        TypeFloat = 2,
        /// Not ZigZag encoded.  Negative numbers take 10 bytes.  Use TYPE_SINT64 if
        /// negative values are likely.
        TypeInt64 = 3,
        TypeUint64 = 4,
        /// Not ZigZag encoded.  Negative numbers take 10 bytes.  Use TYPE_SINT32 if
        /// negative values are likely.
        TypeInt32 = 5,
        TypeFixed64 = 6,
        TypeFixed32 = 7,
        TypeBool = 8,
        TypeString = 9,
        /// Tag-delimited aggregate.
        /// Group type is deprecated and not supported in proto3. However, Proto3
        /// implementations should still be able to parse the group wire format and
        /// treat group fields as unknown fields.
        TypeGroup = 10,
        /// Length-delimited aggregate.
        TypeMessage = 11,
        /// New in version 2.
        TypeBytes = 12,
        TypeUint32 = 13,
        TypeEnum = 14,
        TypeSfixed32 = 15,
        TypeSfixed64 = 16,
        /// Uses ZigZag encoding.
        TypeSint32 = 17,
        /// Uses ZigZag encoding.
        TypeSint64 = 18,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Enumeration)]
    pub enum Label {
        /// 0 is reserved for errors
        LabelOptional = 1,
        LabelRequired = 2,
        LabelRepeated = 3,
    }
}
/// Describes a oneof.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct OneofDescriptorProto {
    #[proto(string, optional, tag="1")]
    pub name: Option<String>,
    #[proto(message, optional, tag="2")]
    pub options: Option<OneofOptions>,
}
/// Describes an enum type.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct EnumDescriptorProto {
    #[proto(string, optional, tag="1")]
    pub name: Option<String>,
    #[proto(message, repeated, tag="2")]
    pub value: Vec<EnumValueDescriptorProto>,
    #[proto(message, optional, tag="3")]
    pub options: Option<EnumOptions>,
}
/// Describes a value within an enum.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct EnumValueDescriptorProto {
    #[proto(string, optional, tag="1")]
    pub name: Option<String>,
    #[proto(int32, optional, tag="2")]
    pub number: Option<i32>,
    #[proto(message, optional, tag="3")]
    pub options: Option<EnumValueOptions>,
}
/// Describes a service.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct ServiceDescriptorProto {
    #[proto(string, optional, tag="1")]
    pub name: Option<String>,
    #[proto(message, repeated, tag="2")]
    pub method: Vec<MethodDescriptorProto>,
    #[proto(message, optional, tag="3")]
    pub options: Option<ServiceOptions>,
}
/// Describes a method of a service.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct MethodDescriptorProto {
    #[proto(string, optional, tag="1")]
    pub name: Option<String>,
    /// Input and output type names.  These are resolved in the same way as
    /// FieldDescriptorProto.type_name, but must refer to a message type.
    #[proto(string, optional, tag="2")]
    pub input_type: Option<String>,
    #[proto(string, optional, tag="3")]
    pub output_type: Option<String>,
    #[proto(message, optional, tag="4")]
    pub options: Option<MethodOptions>,
    /// Identifies if client streams multiple client messages
    #[proto(bool, optional, tag="5")]
    pub client_streaming: Option<bool>,
    /// Identifies if server streams multiple server messages
    #[proto(bool, optional, tag="6")]
    pub server_streaming: Option<bool>,
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

#[derive(Clone, Debug, PartialEq, Message)]
pub struct FileOptions {
    /// Sets the Java package where classes generated from this .proto will be
    /// placed.  By default, the proto package is used, but this is often
    /// inappropriate because proto packages do not normally start with backwards
    /// domain names.
    #[proto(string, optional, tag="1")]
    pub java_package: Option<String>,
    /// If set, all the classes from the .proto file are wrapped in a single
    /// outer class with the given name.  This applies to both Proto1
    /// (equivalent to the old "--one_java_file" option) and Proto2 (where
    /// a .proto always translates to a single class, but you may want to
    /// explicitly choose the class name).
    #[proto(string, optional, tag="8")]
    pub java_outer_classname: Option<String>,
    /// If set true, then the Java code generator will generate a separate .java
    /// file for each top-level message, enum, and service defined in the .proto
    /// file.  Thus, these types will *not* be nested inside the outer class
    /// named by java_outer_classname.  However, the outer class will still be
    /// generated to contain the file's getDescriptor() method as well as any
    /// top-level extensions defined in the file.
    #[proto(bool, optional, tag="10")]
    pub java_multiple_files: Option<bool>,
    /// This option does nothing.
    #[proto(bool, optional, tag="20")]
    pub java_generate_equals_and_hash: Option<bool>,
    /// If set true, then the Java2 code generator will generate code that
    /// throws an exception whenever an attempt is made to assign a non-UTF-8
    /// byte sequence to a string field.
    /// Message reflection will do the same.
    /// However, an extension field still accepts non-UTF-8 byte sequences.
    /// This option has no effect on when used with the lite runtime.
    #[proto(bool, optional, tag="27")]
    pub java_string_check_utf8: Option<bool>,
    #[proto(enumeration="file_options::OptimizeMode", optional, tag="9")]
    pub optimize_for: Option<i32>,
    /// Sets the Go package where structs generated from this .proto will be
    /// placed. If omitted, the Go package will be derived from the following:
    ///   - The basename of the package import path, if provided.
    ///   - Otherwise, the package statement in the .proto file, if present.
    ///   - Otherwise, the basename of the .proto file, without extension.
    #[proto(string, optional, tag="11")]
    pub go_package: Option<String>,
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
    #[proto(bool, optional, tag="16")]
    pub cc_generic_services: Option<bool>,
    #[proto(bool, optional, tag="17")]
    pub java_generic_services: Option<bool>,
    #[proto(bool, optional, tag="18")]
    pub py_generic_services: Option<bool>,
    /// Is this file deprecated?
    /// Depending on the target platform, this can emit Deprecated annotations
    /// for everything in the file, or it will be completely ignored; in the very
    /// least, this is a formalization for deprecating files.
    #[proto(bool, optional, tag="23")]
    pub deprecated: Option<bool>,
    /// Enables the use of arenas for the proto messages in this file. This applies
    /// only to generated classes for C++.
    #[proto(bool, optional, tag="31")]
    pub cc_enable_arenas: Option<bool>,
    /// Sets the objective c class prefix which is prepended to all objective c
    /// generated classes from this .proto. There is no default.
    #[proto(string, optional, tag="36")]
    pub objc_class_prefix: Option<String>,
    /// Namespace for generated classes; defaults to the package.
    #[proto(string, optional, tag="37")]
    pub csharp_namespace: Option<String>,
    /// By default Swift generators will take the proto package and CamelCase it
    /// replacing '.' with underscore and use that to prefix the types/symbols
    /// defined. When this options is provided, they will use this value instead
    /// to prefix the types/symbols defined.
    #[proto(string, optional, tag="39")]
    pub swift_prefix: Option<String>,
    /// Sets the php class prefix which is prepended to all php generated classes
    /// from this .proto. Default is empty.
    #[proto(string, optional, tag="40")]
    pub php_class_prefix: Option<String>,
    /// The parser stores options it doesn't recognize here. See above.
    #[proto(message, repeated, tag="999")]
    pub uninterpreted_option: Vec<UninterpretedOption>,
}
pub mod file_options {
    /// Generated classes can be optimized for speed or code size.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Enumeration)]
    pub enum OptimizeMode {
        /// Generate complete code for parsing, serialization,
        Speed = 1,
        /// etc.
        /// Use ReflectionOps to implement these methods.
        CodeSize = 2,
        /// Generate code using MessageLite and the lite runtime.
        LiteRuntime = 3,
    }
}
#[derive(Clone, Debug, PartialEq, Message)]
pub struct MessageOptions {
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
    #[proto(bool, optional, tag="1")]
    pub message_set_wire_format: Option<bool>,
    /// Disables the generation of the standard "descriptor()" accessor, which can
    /// conflict with a field of the same name.  This is meant to make migration
    /// from proto1 easier; new code should avoid fields named "descriptor".
    #[proto(bool, optional, tag="2")]
    pub no_standard_descriptor_accessor: Option<bool>,
    /// Is this message deprecated?
    /// Depending on the target platform, this can emit Deprecated annotations
    /// for the message, or it will be completely ignored; in the very least,
    /// this is a formalization for deprecating messages.
    #[proto(bool, optional, tag="3")]
    pub deprecated: Option<bool>,
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
    #[proto(bool, optional, tag="7")]
    pub map_entry: Option<bool>,
    /// The parser stores options it doesn't recognize here. See above.
    #[proto(message, repeated, tag="999")]
    pub uninterpreted_option: Vec<UninterpretedOption>,
}
#[derive(Clone, Debug, PartialEq, Message)]
pub struct FieldOptions {
    /// The ctype option instructs the C++ code generator to use a different
    /// representation of the field than it normally would.  See the specific
    /// options below.  This option is not yet implemented in the open source
    /// release -- sorry, we'll try to include it in a future version!
    #[proto(enumeration="field_options::CType", optional, tag="1")]
    pub ctype: Option<i32>,
    /// The packed option can be enabled for repeated primitive fields to enable
    /// a more efficient representation on the wire. Rather than repeatedly
    /// writing the tag and type for each element, the entire array is encoded as
    /// a single length-delimited blob. In proto3, only explicit setting it to
    /// false will avoid using packed encoding.
    #[proto(bool, optional, tag="2")]
    pub packed: Option<bool>,
    /// The jstype option determines the JavaScript type used for values of the
    /// field.  The option is permitted only for 64 bit integral and fixed types
    /// (int64, uint64, sint64, fixed64, sfixed64).  By default these types are
    /// represented as JavaScript strings.  This avoids loss of precision that can
    /// happen when a large value is converted to a floating point JavaScript
    /// numbers.  Specifying JS_NUMBER for the jstype causes the generated
    /// JavaScript code to use the JavaScript "number" type instead of strings.
    /// This option is an enum to permit additional types to be added,
    /// e.g. goog.math.Integer.
    #[proto(enumeration="field_options::JSType", optional, tag="6")]
    pub jstype: Option<i32>,
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
    #[proto(bool, optional, tag="5")]
    pub lazy: Option<bool>,
    /// Is this field deprecated?
    /// Depending on the target platform, this can emit Deprecated annotations
    /// for accessors, or it will be completely ignored; in the very least, this
    /// is a formalization for deprecating fields.
    #[proto(bool, optional, tag="3")]
    pub deprecated: Option<bool>,
    /// For Google-internal migration only. Do not use.
    #[proto(bool, optional, tag="10")]
    pub weak: Option<bool>,
    /// The parser stores options it doesn't recognize here. See above.
    #[proto(message, repeated, tag="999")]
    pub uninterpreted_option: Vec<UninterpretedOption>,
}
pub mod field_options {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Enumeration)]
    pub enum CType {
        /// Default mode.
        String = 0,
        Cord = 1,
        StringPiece = 2,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Enumeration)]
    pub enum JSType {
        /// Use the default type.
        JsNormal = 0,
        /// Use JavaScript strings.
        JsString = 1,
        /// Use JavaScript numbers.
        JsNumber = 2,
    }
}
#[derive(Clone, Debug, PartialEq, Message)]
pub struct OneofOptions {
    /// The parser stores options it doesn't recognize here. See above.
    #[proto(message, repeated, tag="999")]
    pub uninterpreted_option: Vec<UninterpretedOption>,
}
#[derive(Clone, Debug, PartialEq, Message)]
pub struct EnumOptions {
    /// Set this option to true to allow mapping different tag names to the same
    /// value.
    #[proto(bool, optional, tag="2")]
    pub allow_alias: Option<bool>,
    /// Is this enum deprecated?
    /// Depending on the target platform, this can emit Deprecated annotations
    /// for the enum, or it will be completely ignored; in the very least, this
    /// is a formalization for deprecating enums.
    #[proto(bool, optional, tag="3")]
    pub deprecated: Option<bool>,
    /// The parser stores options it doesn't recognize here. See above.
    #[proto(message, repeated, tag="999")]
    pub uninterpreted_option: Vec<UninterpretedOption>,
}
#[derive(Clone, Debug, PartialEq, Message)]
pub struct EnumValueOptions {
    /// Is this enum value deprecated?
    /// Depending on the target platform, this can emit Deprecated annotations
    /// for the enum value, or it will be completely ignored; in the very least,
    /// this is a formalization for deprecating enum values.
    #[proto(bool, optional, tag="1")]
    pub deprecated: Option<bool>,
    /// The parser stores options it doesn't recognize here. See above.
    #[proto(message, repeated, tag="999")]
    pub uninterpreted_option: Vec<UninterpretedOption>,
}
#[derive(Clone, Debug, PartialEq, Message)]
pub struct ServiceOptions {
    // Note:  Field numbers 1 through 32 are reserved for Google's internal RPC
    //   framework.  We apologize for hoarding these numbers to ourselves, but
    //   we were already using them long before we decided to release Protocol
    //   Buffers.

    /// Is this service deprecated?
    /// Depending on the target platform, this can emit Deprecated annotations
    /// for the service, or it will be completely ignored; in the very least,
    /// this is a formalization for deprecating services.
    #[proto(bool, optional, tag="33")]
    pub deprecated: Option<bool>,
    /// The parser stores options it doesn't recognize here. See above.
    #[proto(message, repeated, tag="999")]
    pub uninterpreted_option: Vec<UninterpretedOption>,
}
#[derive(Clone, Debug, PartialEq, Message)]
pub struct MethodOptions {
    // Note:  Field numbers 1 through 32 are reserved for Google's internal RPC
    //   framework.  We apologize for hoarding these numbers to ourselves, but
    //   we were already using them long before we decided to release Protocol
    //   Buffers.

    /// Is this method deprecated?
    /// Depending on the target platform, this can emit Deprecated annotations
    /// for the method, or it will be completely ignored; in the very least,
    /// this is a formalization for deprecating methods.
    #[proto(bool, optional, tag="33")]
    pub deprecated: Option<bool>,
    #[proto(enumeration="method_options::IdempotencyLevel", optional, tag="34")]
    pub idempotency_level: Option<i32>,
    /// The parser stores options it doesn't recognize here. See above.
    #[proto(message, repeated, tag="999")]
    pub uninterpreted_option: Vec<UninterpretedOption>,
}
pub mod method_options {
    /// Is this method side-effect-free (or safe in HTTP parlance), or idempotent,
    /// or neither? HTTP based RPC implementation may choose GET verb for safe
    /// methods, and PUT verb for idempotent methods instead of the default POST.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Enumeration)]
    pub enum IdempotencyLevel {
        IdempotencyUnknown = 0,
        /// implies idempotent
        NoSideEffects = 1,
        /// idempotent, but may have side effects
        Idempotent = 2,
    }
}
/// A message representing a option the parser does not recognize. This only
/// appears in options protos created by the compiler::Parser class.
/// DescriptorPool resolves these when building Descriptor objects. Therefore,
/// options protos in descriptor objects (e.g. returned by Descriptor::options(),
/// or produced by Descriptor::CopyTo()) will never have UninterpretedOptions
/// in them.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct UninterpretedOption {
    #[proto(message, repeated, tag="2")]
    pub name: Vec<uninterpreted_option::NamePart>,
    /// The value of the uninterpreted option, in whatever type the tokenizer
    /// identified it as during parsing. Exactly one of these should be set.
    #[proto(string, optional, tag="3")]
    pub identifier_value: Option<String>,
    #[proto(uint64, optional, tag="4")]
    pub positive_int_value: Option<u64>,
    #[proto(int64, optional, tag="5")]
    pub negative_int_value: Option<i64>,
    #[proto(double, optional, tag="6")]
    pub double_value: Option<f64>,
    #[proto(bytes, optional, tag="7")]
    pub string_value: Option<Vec<u8>>,
    #[proto(string, optional, tag="8")]
    pub aggregate_value: Option<String>,
}
pub mod uninterpreted_option {
    /// The name of the uninterpreted option.  Each string represents a segment in
    /// a dot-separated name.  is_extension is true iff a segment represents an
    /// extension (denoted with parentheses in options specs in .proto files).
    /// E.g.,{ ["foo", false], ["bar.baz", true], ["qux", false] } represents
    /// "foo.(bar.baz).qux".
    #[derive(Clone, Debug, PartialEq, Message)]
    pub struct NamePart {
        #[proto(string, required, tag="1")]
        pub name_part: String,
        #[proto(bool, required, tag="2")]
        pub is_extension: bool,
    }
}
// ===================================================================
// Optional source code info

/// Encapsulates information about the original source file from which a
/// FileDescriptorProto was generated.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct SourceCodeInfo {
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
    #[proto(message, repeated, tag="1")]
    pub location: Vec<source_code_info::Location>,
}
pub mod source_code_info {
    #[derive(Clone, Debug, PartialEq, Message)]
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
        #[proto(int32, repeated, tag="1")]
        pub path: Vec<i32>,
        /// Always has exactly three or four elements: start line, start column,
        /// end line (optional, otherwise assumed same as start line), end column.
        /// These are packed into a single field for efficiency.  Note that line
        /// and column numbers are zero-based -- typically you will want to add
        /// 1 to each before displaying to a user.
        #[proto(int32, repeated, tag="2")]
        pub span: Vec<i32>,
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
        #[proto(string, optional, tag="3")]
        pub leading_comments: Option<String>,
        #[proto(string, optional, tag="4")]
        pub trailing_comments: Option<String>,
        #[proto(string, repeated, tag="6")]
        pub leading_detached_comments: Vec<String>,
    }
}
/// Describes the relationship between generated code and its original source
/// file. A GeneratedCodeInfo message is associated with only one generated
/// source file, but may contain references to different source .proto files.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct GeneratedCodeInfo {
    /// An Annotation connects some span of text in generated code to an element
    /// of its generating .proto file.
    #[proto(message, repeated, tag="1")]
    pub annotation: Vec<generated_code_info::Annotation>,
}
pub mod generated_code_info {
    #[derive(Clone, Debug, PartialEq, Message)]
    pub struct Annotation {
        /// Identifies the element in the original source .proto file. This field
        /// is formatted the same as SourceCodeInfo.Location.path.
        #[proto(int32, repeated, tag="1")]
        pub path: Vec<i32>,
        /// Identifies the filesystem path to the original source .proto.
        #[proto(string, optional, tag="2")]
        pub source_file: Option<String>,
        /// Identifies the starting offset in bytes in the generated code
        /// that relates to the identified object.
        #[proto(int32, optional, tag="3")]
        pub begin: Option<i32>,
        /// Identifies the ending offset in bytes in the generated code that
        /// relates to the identified offset. The end offset should be one past
        /// the last relevant byte (so the length of the text = end - begin).
        #[proto(int32, optional, tag="4")]
        pub end: Option<i32>,
    }
}
