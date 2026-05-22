use color_eyre::eyre::{Result, bail, eyre};

/// Semantic version X.Y.Z[-pre] validation. Accepts stable versions (`X.Y.Z`)
/// and pre-release versions (`X.Y.Z-alpha`, `X.Y.Z-rc1`, `X.Y.Z-beta.2`, etc.).
/// The pre-release suffix must consist of alphanumeric segments separated by dots or
/// dashes. Keep in sync with bump's CLI validation — they must accept the same shape.
pub fn validate_version_format(version: &str) -> Result<()> {
    let (base, pre_release) =
        version.split_once('-').map(|(b, p)| (b, Some(p))).unwrap_or((version, None));

    let mut parts = base.split('.');

    let major = parts.next().ok_or_else(|| {
        eyre!("invalid version format: {version:?} (expected X.Y.Z or X.Y.Z-pre)")
    })?;
    let minor = parts.next().ok_or_else(|| {
        eyre!("invalid version format: {version:?} (expected X.Y.Z or X.Y.Z-pre)")
    })?;
    let patch = parts.next().ok_or_else(|| {
        eyre!("invalid version format: {version:?} (expected X.Y.Z or X.Y.Z-pre)")
    })?;

    if parts.next().is_some()
        || major.is_empty()
        || minor.is_empty()
        || patch.is_empty()
        || !major.chars().all(|ch| ch.is_ascii_digit())
        || !minor.chars().all(|ch| ch.is_ascii_digit())
        || !patch.chars().all(|ch| ch.is_ascii_digit())
    {
        bail!("invalid version format: {version:?} (expected X.Y.Z or X.Y.Z-pre)");
    }

    if let Some(pre) = pre_release {
        let invalid = pre.is_empty()
            || !pre.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '-');
        if invalid {
            bail!(
                "invalid pre-release suffix in {version:?}: {pre:?} (expected alphanumeric segments)"
            );
        }
    }

    Ok(())
}

/// Returns `true` when `version` is a pre-release version (contains a `-` suffix,
/// e.g. `0.13.0-rc1`, `1.2.3-alpha`).
pub fn is_pre_release(version: &str) -> bool {
    version.contains('-')
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // validate_version_format — valid inputs
    // -----------------------------------------------------------------------

    #[test]
    fn valid_stable_version() {
        assert!(validate_version_format("1.2.3").is_ok());
    }

    #[test]
    fn valid_stable_version_large_numbers() {
        assert!(validate_version_format("12.345.6789").is_ok());
    }

    #[test]
    fn valid_stable_version_zeros() {
        assert!(validate_version_format("0.0.0").is_ok());
    }

    #[test]
    fn valid_pre_release_alpha() {
        assert!(validate_version_format("1.2.3-alpha").is_ok());
    }

    #[test]
    fn valid_pre_release_rc1() {
        assert!(validate_version_format("1.2.3-rc1").is_ok());
    }

    #[test]
    fn valid_pre_release_beta_dot() {
        assert!(validate_version_format("1.2.3-beta.2").is_ok());
    }

    #[test]
    fn valid_pre_release_with_dash_separator() {
        assert!(validate_version_format("1.0.0-pre-1").is_ok());
    }

    // -----------------------------------------------------------------------
    // validate_version_format — invalid inputs
    // -----------------------------------------------------------------------

    #[test]
    fn invalid_missing_patch() {
        assert!(validate_version_format("1.2").is_err());
    }

    #[test]
    fn invalid_only_major() {
        assert!(validate_version_format("1").is_err());
    }

    #[test]
    fn invalid_empty_string() {
        assert!(validate_version_format("").is_err());
    }

    #[test]
    fn invalid_extra_dot_segment() {
        assert!(validate_version_format("1.2.3.4").is_err());
    }

    #[test]
    fn invalid_non_numeric_all_parts() {
        assert!(validate_version_format("a.b.c").is_err());
    }

    #[test]
    fn invalid_non_numeric_minor() {
        assert!(validate_version_format("1.x.3").is_err());
    }

    #[test]
    fn invalid_empty_minor_part() {
        assert!(validate_version_format("1..3").is_err());
    }

    #[test]
    fn invalid_leading_dot() {
        assert!(validate_version_format(".2.3").is_err());
    }

    #[test]
    fn invalid_trailing_dot() {
        assert!(validate_version_format("1.2.").is_err());
    }

    #[test]
    fn invalid_empty_pre_release() {
        assert!(validate_version_format("1.2.3-").is_err());
    }

    #[test]
    fn invalid_pre_release_with_space() {
        assert!(validate_version_format("1.2.3-rc 1").is_err());
    }

    #[test]
    fn invalid_pre_release_with_plus() {
        assert!(validate_version_format("1.2.3-rc+1").is_err());
    }

    // -----------------------------------------------------------------------
    // is_pre_release
    // -----------------------------------------------------------------------

    #[test]
    fn is_pre_release_stable_returns_false() {
        assert!(!is_pre_release("1.2.3"));
    }

    #[test]
    fn is_pre_release_rc_returns_true() {
        assert!(is_pre_release("1.2.3-rc1"));
    }

    #[test]
    fn is_pre_release_alpha_returns_true() {
        assert!(is_pre_release("1.0.0-alpha"));
    }

    #[test]
    fn is_pre_release_zero_stable_returns_false() {
        assert!(!is_pre_release("0.0.0"));
    }
}
