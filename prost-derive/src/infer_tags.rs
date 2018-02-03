use failure::Error;

use syn::{
    Lit,
    Meta,
    MetaNameValue,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InferTagsMode {
    Sequential,
}

impl InferTagsMode {
    pub fn from_attr(attr: &Meta) -> Result<Option<InferTagsMode>, Error> {
        match *attr {
            Meta::NameValue(MetaNameValue { ref ident, lit: Lit::Str(ref l), .. }) if ident == "tags" => {
                match l.value().as_str() {
                    "sequential" => Ok(Some(InferTagsMode::Sequential)),
                    other => bail!("invalid value for tags attribute: {}", other),
                }
            },
            _ => Ok(None),
        }
    }
}
