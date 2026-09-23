//! Checked arithmetic for the library-only consumer fixture.

#![forbid(unsafe_code)]

/// Adds two bounded integers, returning `None` on overflow.
///
/// # Examples
///
/// ```
/// assert_eq!(bounded_arithmetic::checked_sum(20, 22), Some(42));
/// assert_eq!(bounded_arithmetic::checked_sum(u32::MAX, 1), None);
/// ```
#[must_use]
pub const fn checked_sum(left: u32, right: u32) -> Option<u32> {
    left.checked_add(right)
}

#[cfg(test)]
mod tests {
    use super::checked_sum;

    #[test]
    fn normal_and_overflow() {
        assert_eq!(checked_sum(20, 22), Some(42));
        assert_eq!(checked_sum(u32::MAX, 1), None);
        assert_eq!(checked_sum(0, 0), Some(0));
    }
}
