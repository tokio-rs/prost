use comrak::{
    format_commonmark,
    nodes::{AstNode, NodeValue},
    parse_document, Arena, ComrakOptions,
};
use prost_types;
use prost_types::source_code_info::Location;

/// Comments on a Protobuf item.
#[derive(Debug)]
pub struct Comments {
    /// Leading detached blocks of comments.
    pub leading_detached: Vec<String>,

    /// Leading comments.
    pub leading: String,

    /// Trailing comments.
    pub trailing: String,
}

impl Comments {
    pub(crate) fn from_location(location: &Location) -> Comments {
        let leading_detached = location.leading_detached_comments.clone();
        let leading = location
            .leading_comments
            .as_ref()
            .map_or(String::new(), String::clone);
        let trailing = location
            .trailing_comments
            .as_ref()
            .map_or(String::new(), String::clone);
        Comments {
            leading_detached,
            leading,
            trailing,
        }
    }

    /// Appends the comments to a buffer with indentation.
    ///
    /// Each level of indentation corresponds to four space (' ') characters.
    pub fn append_with_indent(&self, indent_level: u8, buf: &mut String) {
        // Append blocks of detached comments.
        for detached_block in &self.leading_detached {
            let detached_block = add_text_to_code_blocks(&detached_block);
            for line in detached_block.lines() {
                for _ in 0..indent_level {
                    buf.push_str("    ");
                }
                buf.push_str("//");
                if !line.trim().is_empty() && !line.starts_with(' ') {
                    buf.push(' ');
                }
                buf.push_str(line);
                buf.push_str("\n");
            }
            buf.push_str("\n");
        }

        let leading = add_text_to_code_blocks(&self.leading);

        // Append leading comments.
        for line in leading.lines() {
            for _ in 0..indent_level {
                buf.push_str("    ");
            }
            buf.push_str("///");
            if !line.trim().is_empty() && !line.starts_with(' ') {
                buf.push(' ');
            }
            buf.push_str(line);
            buf.push_str("\n");
        }

        // Append an empty comment line if there are leading and trailing comments.
        if !self.leading.is_empty() && !self.trailing.is_empty() {
            for _ in 0..indent_level {
                buf.push_str("    ");
            }
            buf.push_str("///\n");
        }

        let trailing = add_text_to_code_blocks(&self.trailing);

        // Append trailing comments.
        for line in trailing.lines() {
            for _ in 0..indent_level {
                buf.push_str("    ");
            }
            buf.push_str("///");
            if !line.trim().is_empty() && !line.starts_with(' ') {
                buf.push(' ');
            }
            buf.push_str(line);
            buf.push_str("\n");
        }
    }
}

fn add_text_to_code_blocks(markdown: &str) -> String {
    let arena = Arena::new();
    let options = ComrakOptions::default();
    let root = parse_document(&arena, markdown, &options);

    fn iter_nodes<'a, F>(node: &'a AstNode<'a>, f: &F)
    where
        F: Fn(&'a AstNode<'a>),
    {
        f(node);
        for c in node.children() {
            iter_nodes(c, f);
        }
    }

    iter_nodes(root, &|node| match node.data.borrow_mut().value {
        NodeValue::CodeBlock(ref mut block) => {
            block.fenced = true;
            block.fence_char = b'`';
            block.fence_length = 3;
            block.info = Vec::from("text".as_bytes());
        }
        _ => (),
    });
    let mut ret = Vec::new();
    format_commonmark(&root, &options, &mut ret)
        .expect("Failed to render markdown in proto comment");
    String::from_utf8(ret).unwrap()
}

/// A service descriptor.
#[derive(Debug)]
pub struct Service {
    /// The service name in Rust style.
    pub name: String,
    /// The service name as it appears in the .proto file.
    pub proto_name: String,
    /// The package name as it appears in the .proto file.
    pub package: String,
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
    /// The name of the method in Rust style.
    pub name: String,
    /// The name of the method as it appears in the .proto file.
    pub proto_name: String,
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
