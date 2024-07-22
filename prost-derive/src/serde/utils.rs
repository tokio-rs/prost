pub trait ToProtoCamelCase: ToOwned {
    fn to_proto_camel_case(&self) -> Self::Owned;
}

impl ToProtoCamelCase for str {
    fn to_proto_camel_case(&self) -> Self::Owned {
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
