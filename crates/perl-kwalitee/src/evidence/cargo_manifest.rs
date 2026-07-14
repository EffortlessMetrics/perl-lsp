//! Manifest and license indicators — parsed from `Cargo.toml` files.
//!
//! These are the cheapest indicators: they read the workspace root manifest and
//! the `perl-kwalitee` crate manifest and answer three questions —
//!
//! - is the crate a declared workspace member?
//! - is its publish policy intentional (explicitly private, or allowlisted)?
//! - does it declare license metadata?

use std::path::Path;

use crate::evidence::Outcome;
use crate::indicator::EvidenceRef;

/// Relative path of the crate under the workspace root.
const CRATE_MEMBER_PATH: &str = "crates/perl-kwalitee";
const CRATE_NAME: &str = "perl-kwalitee";

/// Read + parse a TOML file, returning `None` on any error (missing/invalid).
fn read_toml(path: &Path) -> Option<toml::Value> {
    let text = std::fs::read_to_string(path).ok()?;
    toml::from_str::<toml::Value>(&text).ok()
}

/// Whether a `[workspace].members` entry `pattern` covers `target`.
///
/// Cargo allows glob members (e.g. `crates/*`). We handle exact matches and the
/// common trailing-`/*` glob (a directory's direct children); other glob shapes
/// fall back to exact match.
fn member_pattern_covers(pattern: &str, target: &str) -> bool {
    if pattern == target {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix("/*") {
        // `crates/*` covers `crates/perl-kwalitee` (a direct child, no deeper slash).
        if let Some(rest) = target.strip_prefix(prefix).and_then(|r| r.strip_prefix('/')) {
            return !rest.contains('/');
        }
    }
    false
}

/// `manifest.workspace_member_declared`.
pub(crate) fn workspace_member_declared(repo_root: &Path) -> Outcome {
    let root_manifest = repo_root.join("Cargo.toml");
    let evidence = vec![EvidenceRef::file("Cargo.toml [workspace].members")];

    let Some(value) = read_toml(&root_manifest) else {
        return Outcome::unverified(
            evidence,
            "Root Cargo.toml could not be read or parsed as TOML.",
        );
    };

    let is_member = value
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
        .map(|members| {
            members
                .iter()
                .filter_map(|v| v.as_str())
                .any(|m| member_pattern_covers(m, CRATE_MEMBER_PATH))
        })
        .unwrap_or(false);

    if is_member {
        Outcome::pass(evidence)
    } else {
        Outcome::fail(
            evidence,
            format!("Add \"{CRATE_MEMBER_PATH}\" to [workspace].members in the root Cargo.toml."),
        )
    }
}

/// `manifest.publish_policy_clean`.
///
/// The policy is "clean" when it is *intentional*: either the crate is
/// explicitly private (`publish = false`), or it is present in the workspace
/// publish allowlist. An unset publish field with no allowlist entry is
/// ambiguous and fails.
pub(crate) fn publish_policy_clean(repo_root: &Path) -> Outcome {
    let crate_manifest = repo_root.join(CRATE_MEMBER_PATH).join("Cargo.toml");
    let root_manifest = repo_root.join("Cargo.toml");
    let evidence = vec![
        EvidenceRef::file(format!("{CRATE_MEMBER_PATH}/Cargo.toml [package].publish")),
        EvidenceRef::file("Cargo.toml [workspace.metadata.publish].allow"),
    ];

    let Some(crate_value) = read_toml(&crate_manifest) else {
        return Outcome::unverified(
            evidence,
            format!("{CRATE_MEMBER_PATH}/Cargo.toml could not be read or parsed as TOML."),
        );
    };

    // `publish = false` → explicitly private and intentional.
    let publish_false = crate_value
        .get("package")
        .and_then(|p| p.get("publish"))
        .and_then(|p| p.as_bool())
        .map(|b| !b)
        .unwrap_or(false);

    if publish_false {
        return Outcome::pass(evidence);
    }

    // Otherwise it must be allowlisted to be an intentional public crate.
    // The crate manifest was readable but publish is not `false`; we must check
    // the allowlist. If the root manifest cannot be read we cannot decide, so
    // report unverified rather than a false "ambiguous" failure — consistent
    // with `workspace_member_declared`.
    let Some(root_value) = read_toml(&root_manifest) else {
        return Outcome::unverified(
            evidence,
            "Root Cargo.toml could not be read or parsed as TOML to check the publish allowlist.",
        );
    };

    let allowlisted = root_value
        .get("workspace")
        .and_then(|w| w.get("metadata"))
        .and_then(|m| m.get("publish"))
        .and_then(|p| p.get("allow"))
        .and_then(|a| a.as_array())
        .map(|allow| allow.iter().filter_map(|x| x.as_str()).any(|c| c == CRATE_NAME))
        .unwrap_or(false);

    if allowlisted {
        Outcome::pass(evidence)
    } else {
        Outcome::fail(
            evidence,
            format!(
                "Publish policy is ambiguous: set `publish = false` in {CRATE_MEMBER_PATH}/Cargo.toml \
                 while the schema stabilizes, or add \"{CRATE_NAME}\" to \
                 [workspace.metadata.publish].allow once it is publishable."
            ),
        )
    }
}

/// `license.declared`.
pub(crate) fn license_declared(repo_root: &Path) -> Outcome {
    let crate_manifest = repo_root.join(CRATE_MEMBER_PATH).join("Cargo.toml");
    let evidence =
        vec![EvidenceRef::file(format!("{CRATE_MEMBER_PATH}/Cargo.toml [package].license"))];

    let Some(value) = read_toml(&crate_manifest) else {
        return Outcome::unverified(
            evidence,
            format!("{CRATE_MEMBER_PATH}/Cargo.toml could not be read or parsed as TOML."),
        );
    };

    let package = value.get("package");

    // `license = "MIT OR Apache-2.0"` (explicit string, non-empty).
    let explicit_license = package
        .and_then(|p| p.get("license"))
        .and_then(|l| l.as_str())
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);

    // `license.workspace = true`.
    let workspace_license = package
        .and_then(|p| p.get("license"))
        .and_then(|l| l.get("workspace"))
        .and_then(|w| w.as_bool())
        .unwrap_or(false);

    // `license-file = "LICENSE"`.
    let license_file = package
        .and_then(|p| p.get("license-file"))
        .and_then(|l| l.as_str())
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);

    if explicit_license || workspace_license || license_file {
        Outcome::pass(evidence)
    } else {
        Outcome::fail(
            evidence,
            format!(
                "Add `license.workspace = true` (or an explicit SPDX license) to \
                 {CRATE_MEMBER_PATH}/Cargo.toml."
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicator::IndicatorStatus;
    use std::fs;

    fn write(root: &Path, rel: &str, contents: &str) {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
        fs::write(p, contents).expect("write");
    }

    #[test]
    fn member_declared_pass_and_fail() {
        let dir = tempfile::tempdir().expect("tmp");
        let root = dir.path();
        write(root, "Cargo.toml", "[workspace]\nmembers = [\"crates/other\"]\n");
        assert_eq!(workspace_member_declared(root).status, IndicatorStatus::Fail);

        write(
            root,
            "Cargo.toml",
            "[workspace]\nmembers = [\"crates/other\", \"crates/perl-kwalitee\"]\n",
        );
        assert_eq!(workspace_member_declared(root).status, IndicatorStatus::Pass);
    }

    #[test]
    fn member_declared_via_glob() {
        let dir = tempfile::tempdir().expect("tmp");
        let root = dir.path();
        write(root, "Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]\n");
        assert_eq!(workspace_member_declared(root).status, IndicatorStatus::Pass);
    }

    #[test]
    fn glob_does_not_match_deeper_path() {
        assert!(member_pattern_covers("crates/*", "crates/perl-kwalitee"));
        assert!(!member_pattern_covers("crates/*", "crates/sub/deeper"));
        assert!(!member_pattern_covers("other/*", "crates/perl-kwalitee"));
        assert!(member_pattern_covers("crates/perl-kwalitee", "crates/perl-kwalitee"));
    }

    #[test]
    fn publish_false_is_clean() {
        let dir = tempfile::tempdir().expect("tmp");
        let root = dir.path();
        write(root, "Cargo.toml", "[workspace]\n");
        write(
            root,
            "crates/perl-kwalitee/Cargo.toml",
            "[package]\nname = \"perl-kwalitee\"\npublish = false\n",
        );
        assert_eq!(publish_policy_clean(root).status, IndicatorStatus::Pass);
    }

    #[test]
    fn allowlisted_is_clean() {
        let dir = tempfile::tempdir().expect("tmp");
        let root = dir.path();
        write(root, "Cargo.toml", "[workspace.metadata.publish]\nallow = [\"perl-kwalitee\"]\n");
        write(root, "crates/perl-kwalitee/Cargo.toml", "[package]\nname = \"perl-kwalitee\"\n");
        assert_eq!(publish_policy_clean(root).status, IndicatorStatus::Pass);
    }

    #[test]
    fn unreadable_root_manifest_is_unverified() {
        // Crate manifest readable and not publish=false, but no root manifest to
        // check the allowlist against → unverified, not a false ambiguity fail.
        let dir = tempfile::tempdir().expect("tmp");
        let root = dir.path();
        write(root, "crates/perl-kwalitee/Cargo.toml", "[package]\nname = \"perl-kwalitee\"\n");
        assert_eq!(publish_policy_clean(root).status, IndicatorStatus::Unverified);
    }

    #[test]
    fn ambiguous_publish_fails() {
        let dir = tempfile::tempdir().expect("tmp");
        let root = dir.path();
        write(root, "Cargo.toml", "[workspace]\n");
        write(root, "crates/perl-kwalitee/Cargo.toml", "[package]\nname = \"perl-kwalitee\"\n");
        assert_eq!(publish_policy_clean(root).status, IndicatorStatus::Fail);
    }

    #[test]
    fn license_forms_all_pass() {
        for manifest in [
            "[package]\nname=\"perl-kwalitee\"\nlicense = \"MIT OR Apache-2.0\"\n",
            "[package]\nname=\"perl-kwalitee\"\nlicense.workspace = true\n",
            "[package]\nname=\"perl-kwalitee\"\nlicense-file = \"LICENSE\"\n",
        ] {
            let dir = tempfile::tempdir().expect("tmp");
            let root = dir.path();
            write(root, "crates/perl-kwalitee/Cargo.toml", manifest);
            assert_eq!(license_declared(root).status, IndicatorStatus::Pass, "{manifest}");
        }
    }

    #[test]
    fn missing_license_fails() {
        let dir = tempfile::tempdir().expect("tmp");
        let root = dir.path();
        write(root, "crates/perl-kwalitee/Cargo.toml", "[package]\nname=\"perl-kwalitee\"\n");
        assert_eq!(license_declared(root).status, IndicatorStatus::Fail);
    }
}
