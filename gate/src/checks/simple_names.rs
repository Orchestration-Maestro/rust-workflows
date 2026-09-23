//! Names as the inputs spell them: one validator for every "simple name"
//! rule, and hexadecimal strings of a fixed width.

/// A simple name: the first character is alphanumeric or one of `first`, and
/// every character is alphanumeric or one of `rest`. Empty is never simple.
pub(crate) fn simple(value: &str, first: &str, rest: &str) -> bool {
    value
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphanumeric() || first.contains(c))
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || rest.contains(c))
}

/// A lowercase hexadecimal string of exactly `length` digits: a commit SHA at
/// 40, a sha256 at 64.
pub(crate) fn is_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::simple;

    #[test]
    fn a_simple_name_is_bounded_by_its_two_character_sets() {
        assert!(simple("cargo-vet", "", "._-") && simple("_x", "_", "_-"));
        for bad in ["", "-x", ".x", "a b", "a/b"] {
            assert!(!simple(bad, "_", "_-"), "{bad}");
        }
    }
}
