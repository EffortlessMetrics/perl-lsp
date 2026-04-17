//! Integration tests for perl-symbol collapse (Wave B #4428).
//!
//! These tests verify the microcrate collapse from 4 satellite crates
//! (`perl-symbol-types`, `perl-symbol-cursor`, `perl-symbol-index`,
//! `perl-symbol-surface`) into a new published `perl-symbol` facade crate.
//!
//! Scope: These tests check only what can be verified from the workspace
//! filesystem and Cargo.toml. Behavioural tests for the absorbed modules
//! are migrated into `crates/perl-symbol/tests/` by the builder.
//!
//! When the collapse is complete, this test file disappears along with its
//! host crate (`perl-symbol-types` is one of the 4 deleted directories).

use std::env;
use std::path::{Path, PathBuf};

/// Baseline captured before Wave B begins: workspace currently has 200 members
/// and all 4 satellite crates are present. After collapse, member count drops
/// by 3 (4 removed + 1 added).
const PRE_COLLAPSE_MEMBER_COUNT: usize = 200;

/// The 4 satellite crate directory names that must be deleted by the builder.
const OLD_SATELLITES: &[&str] =
    &["perl-symbol-types", "perl-symbol-cursor", "perl-symbol-index", "perl-symbol-surface"];

/// The 5 consumer crates whose Cargo.toml must reference the new `perl-symbol`
/// instead of any old satellite name.
const CONSUMER_CRATES: &[&str] = &[
    "perl-workspace-index",
    "perl-semantic-analyzer",
    "perl-lsp",
    "perl-lsp-rename",
    "perl-lsp-performance",
];

fn workspace_root() -> PathBuf {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR must be set when running under cargo test");
    // CARGO_MANIFEST_DIR is .../crates/perl-symbol-types → parent().parent() is workspace root.
    PathBuf::from(manifest_dir)
        .parent()
        .and_then(Path::parent)
        .expect("workspace root two levels above CARGO_MANIFEST_DIR")
        .to_path_buf()
}

fn read_workspace_toml() -> String {
    let root = workspace_root();
    let cargo_toml = root.join("Cargo.toml");
    std::fs::read_to_string(&cargo_toml)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", cargo_toml.display()))
}

// =============================================================================
// Test 1: Satellite directories deleted
// =============================================================================

/// The 4 old satellite crate directories must all be removed when the collapse
/// is complete.
#[test]
fn test_satellite_directories_deleted() {
    let root = workspace_root();
    let mut still_present = Vec::new();
    for name in OLD_SATELLITES {
        let dir = root.join("crates").join(name);
        if dir.exists() {
            still_present.push(dir.display().to_string());
        }
    }
    assert!(
        still_present.is_empty(),
        "Wave B collapse incomplete — these satellite directories still exist:\n  {}",
        still_present.join("\n  ")
    );
}

// =============================================================================
// Test 2: New `perl-symbol` crate skeleton exists
// =============================================================================

/// The new unified crate must exist with the expected flat-module layout:
/// `crates/perl-symbol/src/{lib,api}.rs` plus `{types,cursor,index,surface}/mod.rs`.
#[test]
fn test_perl_symbol_crate_skeleton_exists() {
    let root = workspace_root();
    let crate_root = root.join("crates").join("perl-symbol");

    let required_files = [
        "Cargo.toml",
        "src/lib.rs",
        "src/api.rs",
        "src/types/mod.rs",
        "src/cursor/mod.rs",
        "src/index/mod.rs",
        "src/surface/mod.rs",
    ];

    let mut missing = Vec::new();
    for rel in required_files {
        let path = crate_root.join(rel);
        if !path.exists() {
            missing.push(path.display().to_string());
        }
    }

    assert!(
        missing.is_empty(),
        "Wave B collapse incomplete — expected perl-symbol skeleton files missing:\n  {}",
        missing.join("\n  ")
    );
}

// =============================================================================
// Test 3: Workspace member count decreased by 3
// =============================================================================

/// Workspace member count must drop by exactly 3 (4 satellites removed,
/// 1 new `perl-symbol` added).
#[test]
fn test_workspace_member_count_decreased_by_three() {
    let content = read_workspace_toml();
    let member_count = content.matches("\"crates/").count();
    let expected = PRE_COLLAPSE_MEMBER_COUNT - 3;

    assert_eq!(
        member_count, expected,
        "Workspace member count should be {expected} ({PRE_COLLAPSE_MEMBER_COUNT} - 3) \
         after Wave B collapse, but is {member_count}. Old satellites may not all have \
         been removed, or the new perl-symbol entry is missing."
    );
}

// =============================================================================
// Test 4: Publish allowlist updated — perl-symbol added, 4 old names removed
// =============================================================================

/// The `[workspace.metadata.publish].allow` list must contain `perl-symbol`
/// and must NOT contain any of the 4 old satellite names.
#[test]
fn test_publish_allowlist_contains_perl_symbol_only() {
    let content = read_workspace_toml();

    let allowlist_section = content
        .split("[workspace.metadata.publish]")
        .nth(1)
        .expect("workspace Cargo.toml should have a [workspace.metadata.publish] section");

    assert!(
        allowlist_section.contains("\"perl-symbol\""),
        "publish allowlist should contain \"perl-symbol\" after Wave B collapse"
    );

    let mut still_present = Vec::new();
    for old in OLD_SATELLITES {
        let needle = format!("\"{old}\"");
        if allowlist_section.contains(&needle) {
            still_present.push(*old);
        }
    }

    assert!(
        still_present.is_empty(),
        "publish allowlist still contains old satellite names after collapse: {still_present:?}"
    );
}

// =============================================================================
// Test 5: Workspace dependencies cleaned up
// =============================================================================

/// `[workspace.dependencies]` must expose `perl-symbol` and no longer list any
/// of the 4 old satellites.
#[test]
fn test_workspace_dependencies_cleaned_up() {
    let content = read_workspace_toml();

    assert!(
        content.contains("perl-symbol = { path = \"crates/perl-symbol\""),
        "workspace.dependencies should contain a perl-symbol entry pointing at crates/perl-symbol"
    );

    let mut still_present = Vec::new();
    for old in OLD_SATELLITES {
        // Match the workspace.dependencies line pattern: `<name> = { path = ...`
        let needle = format!("{old} = {{ path =");
        if content.contains(&needle) {
            still_present.push(*old);
        }
    }

    assert!(
        still_present.is_empty(),
        "workspace.dependencies still references old satellite crates after collapse: {still_present:?}"
    );
}

// =============================================================================
// Test 6: Consumer Cargo.toml files reference perl-symbol, not old names
// =============================================================================

/// All 5 consumer crates must depend on `perl-symbol` and must NOT depend on
/// any of the 4 old satellite names.
#[test]
fn test_consumer_cargo_tomls_reference_perl_symbol() {
    let root = workspace_root();
    let mut failures = Vec::new();

    for consumer in CONSUMER_CRATES {
        let cargo_toml = root.join("crates").join(consumer).join("Cargo.toml");
        let content = match std::fs::read_to_string(&cargo_toml) {
            Ok(c) => c,
            Err(e) => {
                failures.push(format!("{}: failed to read ({e})", cargo_toml.display()));
                continue;
            }
        };

        // Positive check: must now depend on perl-symbol
        if !content.contains("perl-symbol = { workspace = true }")
            && !content.contains("perl-symbol = {workspace = true}")
        {
            failures.push(format!(
                "{consumer}/Cargo.toml does not depend on perl-symbol (workspace = true)"
            ));
        }

        // Negative check: must not depend on any old satellite
        for old in OLD_SATELLITES {
            // Match Cargo.toml key form: `<old-name> = {` (workspace or path).
            let needle = format!("{old} = {{");
            if content.contains(&needle) {
                failures
                    .push(format!("{consumer}/Cargo.toml still depends on old satellite {old}"));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "consumer Cargo.toml files not yet updated:\n  {}",
        failures.join("\n  ")
    );
}

// =============================================================================
// Test 7: No stale `use perl_symbol_*` imports in consumer source
// =============================================================================

/// Scan each consumer crate's `src/` tree for lingering imports of the old
/// satellite crate module names. Everything must be rewritten to
/// `perl_symbol::{...}` (or `perl_symbol::cursor::...`, etc.).
#[test]
fn test_consumer_sources_have_no_stale_imports() {
    let root = workspace_root();

    let forbidden_imports =
        ["perl_symbol_types", "perl_symbol_cursor", "perl_symbol_index", "perl_symbol_surface"];

    let mut offenders = Vec::new();

    for consumer in CONSUMER_CRATES {
        let src_dir = root.join("crates").join(consumer).join("src");
        if !src_dir.exists() {
            continue;
        }
        visit_rs_files(&src_dir, &mut |path, content| {
            for forbidden in forbidden_imports {
                if content.contains(forbidden) {
                    offenders
                        .push(format!("{} references old module `{forbidden}`", path.display()));
                }
            }
        });
    }

    assert!(
        offenders.is_empty(),
        "Consumer source files still reference old satellite module names:\n  {}",
        offenders.join("\n  ")
    );
}

/// Recursively walk `dir` and invoke `f(path, content)` for each `.rs` file.
/// Intentionally dependency-free — this test crate has no walkdir on its path.
fn visit_rs_files(dir: &Path, f: &mut dyn FnMut(&Path, &str)) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            visit_rs_files(&path, f);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                f(&path, &content);
            }
        }
    }
}
