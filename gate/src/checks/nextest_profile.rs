//! TST-004: the one nextest profile the organization runs its tests with,
//! `retries = 0`, so a flaky test fails the run instead of passing on its
//! second try. The gate's own run and the `.config/nextest.toml` every
//! repository holds are rendered from it.

/// The settings every profile holds.
const SETTINGS: &str = "retries = 0\n";

/// The profile `name`: the organization's settings, then `extra`.
pub(crate) fn nextest_profile(name: &str, extra: &str) -> String {
    format!("[profile.{name}]\n{SETTINGS}{extra}")
}

#[cfg(test)]
mod tests {
    use super::nextest_profile;

    #[test]
    fn every_profile_retries_nothing() {
        assert_eq!(
            nextest_profile("default", ""),
            "[profile.default]\nretries = 0\n"
        );
        assert_eq!(
            nextest_profile("gate", "junit = { path = 'x' }\n"),
            "[profile.gate]\nretries = 0\njunit = { path = 'x' }\n"
        );
    }
}
