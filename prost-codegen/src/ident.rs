//! Utility functions for working with identifiers.

/// Converts a camelCase or SCREAMING_SNAKE_CASE identifier to lower_snake case
/// Rust field identifier.
pub fn camel_to_snake(camel: &str) -> String {
    // protoc does not allow non-ascii identifiers.
    let len = camel.as_bytes().iter().skip(1).filter(|&&c| is_uppercase(c)).count() + camel.len();
    let mut snake = Vec::with_capacity(len);

    let mut break_on_cap = false;
    for &c in camel.as_bytes().iter() {
        if is_uppercase(c) {
            if break_on_cap {
                snake.push(b'_');
            }
            snake.push(to_lowercase(c));
            break_on_cap = false;
        } else if c == b'_' {
            snake.push(b'_');
            break_on_cap = false;
        } else {
            snake.push(c);
            break_on_cap = true;
        }
    }

    let mut ident = String::from_utf8(snake).expect(&format!("non-utf8 identifier: {}", camel));

    // Add a trailing underscore if the identifier matches a Rust keyword
    // (https://doc.rust-lang.org/grammar.html#keywords).
    match &ident[..] {
        "abstract" | "alignof" | "as"     | "become"  | "box"   | "break"   | "const"    |
        "continue" | "crate"   | "do"     | "else"    | "enum"  | "extern"  | "false"    |
        "final"    | "fn"      | "for"    | "if"      | "impl"  | "in"      | "let"      |
        "loop"     | "macro"   | "match"  | "mod"     | "move"  | "mut"     | "offsetof" |
        "override" | "priv"    | "proc"   | "pub"     | "pure"  | "ref"     | "return"   |
        "self"     | "sizeof"  | "static" | "struct"  | "super" | "trait"   | "true"     |
        "type"     | "typeof"  | "unsafe" | "unsized" | "use"   | "virtual" | "where"    |
        "while"    | "yield" => {
            ident.push('_');
        }
        _ => (),
    }
    ident
}

/// Converts a snake_case identifier to UpperCamel case Rust type identifier.
pub fn snake_to_upper_camel(snake: &str) -> String {
    let mut ident = String::with_capacity(snake.len());

    if snake.is_empty() {
        return ident;
    }

    for fragment in snake.split('_') {
        if fragment.is_empty() {
            ident.push('_');
        } else {
            let (first, rest) = fragment.split_at(1);
            ident.push_str(&first.to_uppercase());
            ident.push_str(&rest.to_lowercase());
        }
    }

    // Add a trailing underscore if the identifier matches a Rust keyword
    // (https://doc.rust-lang.org/grammar.html#keywords).
    if ident == "Self" {
        ident.push('_');
    }
    ident
}

/// Returns true if the character is an upper-case ASCII character.
#[inline]
fn is_uppercase(c: u8) -> bool {
    c >= b'A' && c <= b'Z'
}

/// Converts an upper-case ASCII character to lower-case.
#[inline]
fn to_lowercase(c: u8) -> u8 {
    debug_assert!(is_uppercase(c));
    c + 32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camel_to_snake() {
        assert_eq!("foo_bar", &camel_to_snake("FooBar"));
        assert_eq!("foo_bar_baz", &camel_to_snake("FooBarBAZ"));
        assert_eq!("foo_bar_baz", &camel_to_snake("FooBArBAZ"));
        assert_eq!("foo_bar_bazle_e", &camel_to_snake("FooBArBAZleE"));
        assert_eq!("while_", &camel_to_snake("While"));
        assert_eq!("fuzz_buster", &camel_to_snake("FUZZ_BUSTER"));
        assert_eq!("foo_bar_baz", &camel_to_snake("foo_bar_baz"));
        assert_eq!("fuzz_buster", &camel_to_snake("FUZZ_buster"));
        assert_eq!("_fuzz", &camel_to_snake("_FUZZ"));
        assert_eq!("_fuzz", &camel_to_snake("_fuzz"));
        assert_eq!("_fuzz", &camel_to_snake("_Fuzz"));
        assert_eq!("fuzz_", &camel_to_snake("FUZZ_"));
        assert_eq!("fuzz_", &camel_to_snake("fuzz_"));
        assert_eq!("fuzz_", &camel_to_snake("Fuzz_"));
        assert_eq!("fuz_z_", &camel_to_snake("FuzZ_"));

        // From test_messages_proto3.proto.
        assert_eq!("fieldname1", &camel_to_snake("fieldname1"));
        assert_eq!("field_name2", &camel_to_snake("field_name2"));
        assert_eq!("_field_name3", &camel_to_snake("_field_name3"));
        assert_eq!("field__name4_", &camel_to_snake("field__name4_"));
        assert_eq!("field0name5", &camel_to_snake("field0name5"));
        assert_eq!("field_0_name6", &camel_to_snake("field_0_name6"));
        assert_eq!("field_name7", &camel_to_snake("fieldName7"));
        assert_eq!("field_name8", &camel_to_snake("FieldName8"));
        assert_eq!("field_name9", &camel_to_snake("field_Name9"));
        assert_eq!("field_name10", &camel_to_snake("Field_Name10"));
        assert_eq!("field_name11", &camel_to_snake("FIELD_NAME11"));
        assert_eq!("field_name12", &camel_to_snake("FIELD_name12"));
        assert_eq!("__field_name13", &camel_to_snake("__field_name13"));
        assert_eq!("__field_name14", &camel_to_snake("__Field_name14"));
        assert_eq!("field__name15", &camel_to_snake("field__name15"));
        assert_eq!("field__name16", &camel_to_snake("field__Name16"));
        assert_eq!("field_name17__", &camel_to_snake("field_name17__"));
        assert_eq!("field_name18__", &camel_to_snake("Field_name18__"));
    }

    #[test]
    fn test_snake_to_upper_camel() {
        assert_eq!("", &snake_to_upper_camel(""));
        assert_eq!("F", &snake_to_upper_camel("F"));
        assert_eq!("Foo", &snake_to_upper_camel("FOO"));
        assert_eq!("FooBar", &snake_to_upper_camel("FOO_BAR"));
        assert_eq!("_FooBar", &snake_to_upper_camel("_FOO_BAR"));
        assert_eq!("FooBar_", &snake_to_upper_camel("FOO_BAR_"));
        assert_eq!("_FooBar_", &snake_to_upper_camel("_FOO_BAR_"));
        assert_eq!("Fuzzbuster", &snake_to_upper_camel("fuzzBuster"));
        assert_eq!("Self_", &snake_to_upper_camel("self"));
    }
}
