use google::protobuf::source_code_info::Location;

#[derive(Debug, Default)]
pub struct Comments {
    /// Leading detached blocks of comments.
    pub leading_detached: Vec<Vec<String>>,

    /// Leading comments.
    pub leading: Vec<String>,

    /// Trailing comments.
    pub trailing: Vec<String>,
}

impl Comments {
    pub fn from_location(location: &Location) -> Comments {
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

pub struct Service {
    pub name: String,
    pub comments: Comments,
    pub methods: Vec<Method>,
}

pub struct Method {
    pub name: String,
    pub comments: Comments,
    pub input_type: String,
    pub input_proto_type: String,
    pub output_type: String,
    pub output_proto_type: String,
}
