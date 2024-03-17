use super::*;

/// URL/resource name that uniquely identifies the type of the serialized protocol buffer message,
/// e.g. `type.googleapis.com/google.protobuf.Duration`.
///
/// This string must contain at least one "/" character.
///
/// The last segment of the URL's path must represent the fully qualified name of the type (as in
/// `path/google.protobuf.Duration`). The name should be in a canonical form (e.g., leading "." is
/// not accepted).
///
/// If no scheme is provided, `https` is assumed.
///
/// Schemes other than `http`, `https` (or the empty scheme) might be used with implementation
/// specific semantics.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct TypeUrl<'a> {
    /// Fully qualified name of the type, e.g. `google.protobuf.Duration`
    pub(crate) full_name: &'a str,
}

impl<'a> TypeUrl<'a> {
    pub(crate) fn new(s: &'a str) -> core::option::Option<Self> {
        // Must contain at least one "/" character.
        let slash_pos = s.rfind('/')?;

        // The last segment of the URL's path must represent the fully qualified name
        // of the type (as in `path/google.protobuf.Duration`)
        let full_name = s.get((slash_pos + 1)..)?;

        // The name should be in a canonical form (e.g., leading "." is not accepted).
        if full_name.starts_with('.') {
            return None;
        }

        Some(Self { full_name })
    }
}

/// Compute the type URL for the given `google.protobuf` type, using `type.googleapis.com` as the
/// authority for the URL.
pub(crate) fn type_url_for<T: Name>() -> String {
    format!("type.googleapis.com/{}.{}", T::PACKAGE, T::NAME)
}
