//! `product_surface.native_only` — first-mile surface scan.
//!
//! Mirrors `cargo xtask check-native-product-surface`: the first-mile product
//! surfaces users read must not frame the product as *requiring* external Perl
//! tooling or a legacy `Perl::LanguageServer` bridge. Under the release profile
//! an additional, stricter list of raw external-tool names is also banned.
//!
//! The authoritative CI gate remains the xtask command; this mirror keeps the
//! crate self-contained so `perl_kwalitee::evaluate` produces a real
//! product-surface verdict from `repo_root` alone. The two lists are kept in
//! sync deliberately (see the sync test in the xtask suite and here).

use std::path::Path;

use crate::evidence::Outcome;
use crate::indicator::EvidenceRef;
use crate::profile::KwaliteeProfile;

/// First-mile product surfaces scanned for disallowed native-stack leaks.
///
/// Kept in sync with `xtask/src/tasks/native_product_surface.rs::SURFACES`.
const SURFACES: &[&str] = &[
    "vscode-extension/package.json",
    "vscode-extension/README.md",
    "crates/perl-dap/README.md",
    "docs/project/status/dap.md",
    "docs/tutorials/DAP_USER_GUIDE.md",
];

/// Misleading "external tooling / legacy bridge is the product" phrasings.
///
/// Kept in sync with `xtask/src/tasks/native_product_surface.rs::DISALLOWED`.
const DISALLOWED: &[&str] = &[
    "BridgeAdapter",
    "cpanm Perl::LanguageServer",
    "cpan Perl::LanguageServer",
    "Perl::LanguageServer requirement",
    "Bridge path documents",
    "requires perltidy",
    "requires perlcritic",
    "external Perl::Critic diagnostics",
    "Use bridge mode",
    "--bridge",
];

/// Additional raw external-tool markers banned on first-mile surfaces under the
/// stricter *release* profile ("if it is not native, we do not ship it").
///
/// `BridgeAdapter` and `--bridge` are already in [`DISALLOWED`]; this list adds
/// the raw external-tool names that are tolerated on `pr` surfaces (in
/// native-first negations) but must not appear on a release surface at all.
const RELEASE_STRICT_DISALLOWED: &[&str] = &[
    "Perl::LanguageServer",
    "perltidy",
    "perlcritic",
    "Perl::Critic",
    "Devel::TSPerlDAP",
    "TSPerlDAP.pm",
];

/// One violation: `surface:line` plus the offending marker.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Violation {
    surface: String,
    line: usize,
    marker: String,
}

impl Violation {
    fn describe(&self) -> String {
        format!("{}:{}: `{}`", self.surface, self.line, self.marker)
    }
}

/// `product_surface.native_only`.
pub(crate) fn native_only(repo_root: &Path, profile: KwaliteeProfile) -> Outcome {
    let strict = profile.requires_release_artifacts();
    let (violations, scanned) = scan(repo_root, strict);

    let mut evidence = vec![EvidenceRef::command("cargo xtask check-native-product-surface")];

    // If none of the first-mile surfaces could be read (e.g. they were all
    // renamed away), we cannot assert native-only cleanliness — report
    // Unverified rather than a false Pass, matching `dap.cli_native_only`.
    if !scanned {
        return Outcome::unverified(
            evidence,
            "Could not read any first-mile product surface to verify native-only status.",
        );
    }

    if violations.is_empty() {
        Outcome::pass(evidence)
    } else {
        for v in violations.iter().take(10) {
            evidence.push(EvidenceRef::file(v.describe()));
        }
        Outcome::fail(
            evidence,
            "Move external-tool/legacy-bridge product framing off first-mile surfaces \
             into docs/reference/ (e.g. DAP_LEGACY_BRIDGE_COMPAT.md).",
        )
    }
}

/// Scan every surface under `root`. Missing surfaces are skipped (not
/// violations) so the check is robust to individual file moves. Returns the
/// violations plus whether *any* surface was actually read, so the caller can
/// distinguish "all surfaces clean" from "all surfaces missing".
fn scan(root: &Path, strict: bool) -> (Vec<Violation>, bool) {
    let mut violations = Vec::new();
    let mut scanned = false;
    for surface in SURFACES {
        let path = root.join(surface);
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        scanned = true;
        collect(surface, &text, strict, &mut violations);
    }
    (violations, scanned)
}

/// Pure per-surface scan. Each line is normalized (backticks stripped,
/// lowercased) before matching so a banned phrasing cannot slip past wrapped in
/// backticks or recapitalized.
fn collect(surface: &str, text: &str, strict: bool, out: &mut Vec<Violation>) {
    // Pre-lowercase each marker once, paired with its original casing for the
    // report, rather than re-lowercasing per line.
    let markers: Vec<(String, &'static str)> = DISALLOWED
        .iter()
        .chain(if strict { RELEASE_STRICT_DISALLOWED.iter() } else { [].iter() })
        .map(|&m| (m.to_ascii_lowercase(), m))
        .collect();

    for (idx, line) in text.lines().enumerate() {
        let normalized = line.replace('`', "").to_ascii_lowercase();
        // Collect the matching markers on this line, then drop any that are a
        // substring of another match on the same line (e.g. "perltidy" ⊂
        // "requires perltidy", or "Perl::LanguageServer" ⊂ its "requirement"
        // phrase) so one offending line yields one violation, not duplicates.
        let matched: Vec<&(String, &'static str)> =
            markers.iter().filter(|(lc, _)| normalized.contains(lc)).collect();
        for (lc, original) in &matched {
            let subsumed = matched
                .iter()
                .any(|(other_lc, _)| other_lc.len() > lc.len() && other_lc.contains(lc.as_str()));
            if subsumed {
                continue;
            }
            out.push(Violation {
                surface: surface.to_string(),
                line: idx + 1,
                marker: (*original).to_string(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicator::IndicatorStatus;

    #[test]
    fn flags_disallowed_markers() {
        let mut out = Vec::new();
        collect("s.md", "ok\ncpanm Perl::LanguageServer\n", false, &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].line, 2);
    }

    #[test]
    fn allows_native_first_negations() {
        let mut out = Vec::new();
        collect(
            "s.md",
            "The native path does not require `Perl::LanguageServer`.\n",
            false,
            &mut out,
        );
        // In non-strict (pr) mode the raw name alone is allowed.
        assert!(out.is_empty(), "{out:?}");
    }

    #[test]
    fn substring_markers_do_not_double_report() {
        // Under release-strict, "requires perltidy" (DISALLOWED) and "perltidy"
        // (RELEASE_STRICT) both match; only the broader one should be reported.
        let mut out = Vec::new();
        collect("s.md", "requires perltidy\n", true, &mut out);
        assert_eq!(out.len(), 1, "one line -> one violation, got {out:?}");
        assert_eq!(out[0].marker, "requires perltidy");
    }

    #[test]
    fn release_strict_bans_raw_external_names() {
        let text = "The native path does not require `Perl::LanguageServer`.\n";
        let mut lax = Vec::new();
        collect("s.md", text, false, &mut lax);
        assert!(lax.is_empty());

        let mut strict = Vec::new();
        collect("s.md", text, true, &mut strict);
        assert!(!strict.is_empty(), "release-strict should flag the raw name");
    }

    #[test]
    fn empty_tree_is_unverified() {
        // No first-mile surface present at all → cannot assert cleanliness.
        let dir = tempfile::tempdir().expect("tmp");
        assert_eq!(
            native_only(dir.path(), KwaliteeProfile::Pr).status,
            IndicatorStatus::Unverified
        );
    }

    #[test]
    fn clean_surface_passes() {
        let dir = tempfile::tempdir().expect("tmp");
        let p = dir.path().join("vscode-extension/package.json");
        std::fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
        std::fs::write(p, "\"description\": \"native Perl debugger\"\n").expect("write");
        assert_eq!(native_only(dir.path(), KwaliteeProfile::Pr).status, IndicatorStatus::Pass);
    }

    #[test]
    fn regressed_surface_fails() {
        let dir = tempfile::tempdir().expect("tmp");
        let p = dir.path().join("vscode-extension/package.json");
        std::fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
        std::fs::write(p, "\"x\": \"requires perltidy\"\n").expect("write");
        assert_eq!(native_only(dir.path(), KwaliteeProfile::Pr).status, IndicatorStatus::Fail);
    }
}
