pub trait ToProtoCamelCase: ToOwned {
    fn to_proto_camel_case(&self) -> Self::Owned;
}

impl ToProtoCamelCase for str {
    fn to_proto_camel_case(&self) -> Self::Owned {
        // Reference: https://protobuf.com/docs/language-spec#default-json-names
        //
        // If no json_name pseudo-option is present, the JSON name of the field will be
        // the field's name converted to camelCase. To convert to camelCase:
        //
        // - Discard any trailing underscores (_)
        // - When a leading or interior underscore is encountered, discard the underscore and
        //   capitalize the next non-underscore character encountered.
        // - Any other non-underscore and non-capitalized character is retained as is.
        //
        let mut capitalize_next = false;
        let mut out = String::with_capacity(self.len());
        for chr in self.chars() {
            if chr == '_' {
                capitalize_next = true;
            } else if capitalize_next {
                out.push(chr.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                out.push(chr);
            }
        }
        out
    }
}

impl ToProtoCamelCase for String {
    fn to_proto_camel_case(&self) -> Self::Owned {
        self.as_str().to_proto_camel_case()
    }
}
