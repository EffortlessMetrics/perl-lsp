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
