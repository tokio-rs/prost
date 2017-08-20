use prost_types::source_code_info::Location;
use prost_types;

/// Comments on a Protobuf item.
#[derive(Debug)]
pub struct Comments {
    /// Leading detached blocks of comments.
    pub leading_detached: Vec<Vec<String>>,

    /// Leading comments.
    pub leading: Vec<String>,

    /// Trailing comments.
    pub trailing: Vec<String>,
}

impl Comments {
    pub(crate) fn from_location(location: &Location) -> Comments {
        fn get_lines(comments: &String) -> Vec<String> {
            comments.lines().map(str::to_owned).collect()
        }

        let leading_detached = location.leading_detached_comments.iter().map(get_lines).collect();
        let leading = location.leading_comments.as_ref().map_or(Vec::new(), get_lines);
        let trailing = location.trailing_comments.as_ref().map_or(Vec::new(), get_lines);
        Comments {
            leading_detached: leading_detached,
            leading: leading,
            trailing: trailing,
        }
    }
}

/// A service descriptor.
#[derive(Debug)]
pub struct Service {
    /// The service name.
    pub name: String,
    /// The service comments.
    pub comments: Comments,
    /// The service methods.
    pub methods: Vec<Method>,
    /// The service options.
    pub options: prost_types::ServiceOptions,
}

/// A service method descriptor.
#[derive(Debug)]
pub struct Method {
    /// The name of the method.
    pub name: String,
    /// The method comments.
    pub comments: Comments,
    /// The input Rust type.
    pub input_type: String,
    /// The output Rust type.
    pub output_type: String,
    /// The input Protobuf type.
    pub input_proto_type: String,
    /// The output Protobuf type.
    pub output_proto_type: String,
    /// The method options.
    pub options: prost_types::MethodOptions,
    /// Identifies if client streams multiple client messages.
    pub client_streaming: bool,
    /// Identifies if server streams multiple server messages.
    pub server_streaming: bool,
}
