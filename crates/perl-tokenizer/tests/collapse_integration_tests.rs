//! Integration tests for perl-lexer Wave C collapse (#4444).
//!
//! These tests verify the microcrate collapse of 4 lexer satellite crates
//! (`perl-tokenizer`, `perl-keywords`, `perl-builtins`, `perl-builtins-phf`)
//! into the existing published `perl-lexer` crate, plus the architect-mandated
//! relocation of trivia modules (`trivia.rs`, `trivia_parser.rs`) from
//! `perl-tokenizer` into `perl-parser-core/src/tokens/`.
//!
//! Scope: These tests check what can be verified from the workspace
//! filesystem and Cargo.toml. Behavioural tests for the absorbed modules
//! are migrated into `crates/perl-lexer/tests/` by the builder.
//!
//! Placement note: This file lives in `perl-tokenizer/tests/` — one of the 4
//! crates scheduled for deletion. When the collapse is complete, this host
//! crate disappears (Phase 7 of the builder checklist), and these tests
//! disappear with it — having served their purpose as the red gate.
//!
//! `perl-token` is NOT absorbed — the ADR amendment in #4446 keeps it as a
//! separately-published crate. Tests here explicitly verify it remains in the
//! allowlist.

use std::env;
use std::path::{Path, PathBuf};

/// Baseline captured before Wave C begins: workspace currently has 101 members
/// and all 4 satellite crates are present. After collapse, member count drops
/// by 4 (4 removed, 0 added — perl-lexer already exists).
const PRE_COLLAPSE_MEMBER_COUNT: usize = 101;

/// Baseline publish allowlist count before Wave C. After collapse, drops by 4
/// (4 old names removed, 0 added — perl-lexer already on allowlist).
const PRE_COLLAPSE_ALLOWLIST_COUNT: usize = 98;

/// The 4 satellite crate directory names that must be deleted by the builder.
const OLD_SATELLITES: &[&str] = &[
    "perl-tokenizer",
    "perl-keywords",
    "perl-builtins",
    "perl-builtins-phf",
];

/// The 9 consumer crates whose Cargo.toml must no longer reference any of the
/// 4 old satellites. Consumers may already depend on `perl-lexer` (absorber)
/// or will be edited to do so; `perl-lexer` itself appears here because its
/// own Cargo.toml lists `perl-keywords` pre-collapse and must be scrubbed.
const CONSUMER_CRATES: &[&str] = &[
    "perl-parser-core",
    "perl-dap",
    "perl-lsp",
    "perl-lsp-code-actions",
    "perl-lsp-completion",
    "perl-lsp-inlay-hints",
    "perl-lsp-rename",
    "perl-parser",
    "perl-lexer",
];

fn workspace_root() -> PathBuf {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR must be set when running under cargo test");
    // CARGO_MANIFEST_DIR is .../crates/perl-tokenizer → parent().parent() is workspace root.
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
/// is complete (Phase 7 of the builder checklist).
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
        "Wave C collapse incomplete — these satellite directories still exist:\n  {}",
        still_present.join("\n  ")
    );
}

// =============================================================================
// Test 2: Absorbed modules exist in perl-lexer
// =============================================================================

/// The new internal modules must exist inside `perl-lexer/src/`:
/// - `keywords/mod.rs` (from perl-keywords)
/// - `builtins/mod.rs` (from perl-builtins)
/// - `builtins/builtin_signatures.rs` (from perl-builtins)
/// - `tokenizer/mod.rs` (from perl-tokenizer, AST-agnostic slice)
/// - `api.rs` (new public facade)
#[test]
fn test_perl_lexer_absorbed_modules_exist() {
    let root = workspace_root();
    let lexer_src = root.join("crates").join("perl-lexer").join("src");

    let required_files = [
        "keywords/mod.rs",
        "builtins/mod.rs",
        "builtins/builtin_signatures.rs",
        "tokenizer/mod.rs",
        "api.rs",
    ];

    let mut missing = Vec::new();
    for rel in required_files {
        let path = lexer_src.join(rel);
        if !path.exists() {
            missing.push(path.display().to_string());
        }
    }

    assert!(
        missing.is_empty(),
        "Wave C collapse incomplete — expected perl-lexer module files missing:\n  {}",
        missing.join("\n  ")
    );
}

// =============================================================================
// Test 3: builtins/phf_lookup.rs is a file, not a folder
// =============================================================================

/// `perl-builtins-phf` absorbs as a sibling FILE `phf_lookup.rs` next to
/// `builtins/mod.rs`, not as its own subfolder. This matches checklist
/// Phase 2.2 File A which specifies "sibling of mod.rs".
#[test]
fn test_builtins_phf_lookup_is_a_file() {
    let root = workspace_root();
    let phf_lookup = root
        .join("crates")
        .join("perl-lexer")
        .join("src")
        .join("builtins")
        .join("phf_lookup.rs");

    assert!(
        phf_lookup.exists(),
        "expected phf_lookup.rs at {}",
        phf_lookup.display()
    );
    assert!(
        phf_lookup.is_file(),
        "expected {} to be a file, not a directory",
        phf_lookup.display()
    );

    // Assert that it's not accidentally created as a folder (which would
    // create phf_lookup/mod.rs instead).
    let phf_lookup_as_dir = root
        .join("crates")
        .join("perl-lexer")
        .join("src")
        .join("builtins")
        .join("phf_lookup");
    assert!(
        !phf_lookup_as_dir.exists() || !phf_lookup_as_dir.is_dir(),
        "unexpected directory at {} — builder should create phf_lookup.rs as a file",
        phf_lookup_as_dir.display()
    );
}

// =============================================================================
// Test 4: Trivia modules moved to perl-parser-core
// =============================================================================

/// Per architect recommendation (context decision 8), trivia modules move to
/// `perl-parser-core/src/tokens/` because they depend on `perl-ast-v2` which
/// `perl-lexer` must not pull in (leaf-layer contract).
#[test]
fn test_trivia_modules_in_parser_core() {
    let root = workspace_root();
    let tokens_dir = root
        .join("crates")
        .join("perl-parser-core")
        .join("src")
        .join("tokens");

    let required_files = ["trivia.rs", "trivia_parser.rs"];
    let mut missing = Vec::new();
    for rel in required_files {
        let path = tokens_dir.join(rel);
        if !path.exists() {
            missing.push(path.display().to_string());
        }
    }

    assert!(
        missing.is_empty(),
        "Wave C collapse incomplete — trivia modules missing from perl-parser-core/src/tokens/:\n  {}",
        missing.join("\n  ")
    );
}

// =============================================================================
// Test 5: Workspace member count decreased by 4
// =============================================================================

/// Workspace member count must drop by exactly 4 (4 satellites removed, 0
/// added — perl-lexer already existed).
#[test]
fn test_workspace_member_count_decreased_by_four() {
    let content = read_workspace_toml();
    let member_count = content.matches("\"crates/").count();
    let expected = PRE_COLLAPSE_MEMBER_COUNT - 4;

    assert_eq!(
        member_count, expected,
        "Workspace member count should be {expected} ({PRE_COLLAPSE_MEMBER_COUNT} - 4) \
         after Wave C collapse, but is {member_count}. Old satellites may not all have \
         been removed from [workspace.members]."
    );
}

// =============================================================================
// Test 6: Publish allowlist — 4 old removed, perl-lexer + perl-token remain
// =============================================================================

/// The `[workspace.metadata.publish].allow` list must NOT contain any of the 4
/// old satellite names, and MUST still contain `perl-lexer` (absorber) and
/// `perl-token` (ADR amendment #4446 — stays published).
#[test]
fn test_publish_allowlist_post_collapse() {
    let content = read_workspace_toml();

    let allowlist_section = content
        .split("[workspace.metadata.publish]")
        .nth(1)
        .expect("workspace Cargo.toml should have a [workspace.metadata.publish] section");

    // Required survivors.
    assert!(
        allowlist_section.contains("\"perl-lexer\""),
        "publish allowlist must still contain \"perl-lexer\" after Wave C collapse"
    );
    assert!(
        allowlist_section.contains("\"perl-token\""),
        "publish allowlist must still contain \"perl-token\" per ADR amendment #4446"
    );

    // Forbidden holdouts.
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

    // Expected count — this is a quick integrity check; exact count catches
    // silent-insertion regressions.
    let allow_entries = allowlist_section
        .split("allow")
        .nth(1)
        .and_then(|s| s.split('[').nth(1))
        .and_then(|s| s.split(']').next())
        .expect("allowlist should have an allow = [...] array");
    let allow_count = allow_entries.matches('"').count() / 2;
    let expected = PRE_COLLAPSE_ALLOWLIST_COUNT - 4;
    assert_eq!(
        allow_count, expected,
        "publish allowlist count should be {expected} ({PRE_COLLAPSE_ALLOWLIST_COUNT} - 4) \
         after Wave C collapse, but is {allow_count}."
    );
}

// =============================================================================
// Test 7: Consumer Cargo.toml files no longer reference old satellites
// =============================================================================

/// All 9 consumer crates' Cargo.toml files must be scrubbed of dependencies on
/// the 4 absorbed satellites. They should depend on `perl-lexer` (directly or
/// transitively).
#[test]
fn test_consumer_cargo_tomls_no_old_references() {
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

        // Negative check: must not depend on any absorbed satellite.
        for old in OLD_SATELLITES {
            // Match Cargo.toml key form: `<old-name> = {` (workspace or path).
            let needle = format!("{old} = {{");
            if content.contains(&needle) {
                failures.push(format!(
                    "{consumer}/Cargo.toml still depends on absorbed satellite {old}"
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "consumer Cargo.toml files still reference old satellites:\n  {}",
        failures.join("\n  ")
    );
}
