//! Support for associating a type URL with a [`Message`].

use crate::Message;

/// Associate a type URL with the given [`Message]`.
pub trait TypeUrl: Message {
    /// Type URL for this [`Message`]. They take the form:
    ///
    /// ```text
    /// /<package>.<TypeName>
    /// ```
    ///
    /// For example:
    ///
    /// ```text
    /// /foo.bar.baz.MyTypeName
    /// ```
    const TYPE_URL: &'static str;
}
