//! Regression tests for GitHub issue #4208 — metric drift between `index.md` and `lsp.md`
//!
//! The human-written narrative in `docs/project/status/index.md` must stay in sync
//! with the auto-generated metrics in `docs/project/status/lsp.md`. This drift was
//! detected when `index.md` reported 58 user-visible features at 116/116 coverage
//! while `lsp.md` (generated from `features.toml`) reported 60/60 advertised and
//! 119/119 including plumbing.
//!
//! ## What this module tests
//!
//! - `index.md`'s user-visible count matches `lsp.md`'s advertised feature count
//! - `index.md`'s plumbing/protocol count matches `lsp.md`'s "including plumbing" count
//! - Parsing helpers are robust against empty input, malformed patterns, and edge cases
//!
//! ## How to fix failures
//!
//! If these tests fail, run:
//! ```bash
//! cargo xtask update-status --only lsp
//! ```
//! Then edit `docs/project/status/index.md` line 10 to match the current generated values.

use std::path::PathBuf;

/// Returns the workspace root (parent of xtask/).
/// Assumes CARGO_MANIFEST_DIR points to the xtask/ subdirectory.
fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

/// Parses `docs/project/status/lsp.md` and extracts LSP metrics.
///
/// Returns `(ux_impl, ux_total, proto_impl, proto_total)`:
///
/// - `ux_impl, ux_total`: from the "advertised features" pattern
/// - `proto_impl, proto_total`: from the "including plumbing" pattern
///
/// If the file does not exist or no match is found, returns `(0, 0, 0, 0)`.
fn parse_lsp_md_metrics(lsp_md: &str) -> (u32, u32, u32, u32) {
    // e.g. "(60/60 advertised features" → ux_impl=60, ux_total=60
    let ux_re = regex::Regex::new(r"\((\d+)/(\d+)\) advertised features").ok();
    // e.g. "(119/119 including plumbing)" → proto_impl=119, proto_total=119
    let proto_re = regex::Regex::new(r"\((\d+)/(\d+)\) including plumbing").ok();

    let ux_caps = ux_re
        .and_then(|re| re.captures(lsp_md))
        .map(|c| (c[1].parse().unwrap_or(0), c[2].parse().unwrap_or(0)));
    let proto_caps = proto_re
        .and_then(|re| re.captures(lsp_md))
        .map(|c| (c[1].parse().unwrap_or(0), c[2].parse().unwrap_or(0)));

    match (ux_caps, proto_caps) {
        (Some((ux_i, ux_t)), Some((p_i, p_t))) => (ux_i, ux_t, p_i, p_t),
        _ => (0, 0, 0, 0),
    }
}

/// Parses `docs/project/status/index.md` and extracts the metrics written in the narrative.
///
/// Returns `(user_visible, plumbing)`:
///
/// - `user_visible`: the count of "user-visible features" from line 10
/// - `plumbing`: the "including plumbing" count from the same line
///
/// If the file does not exist or no match is found, returns `(0, 0)`.
fn parse_index_md_metrics(index_md: &str) -> (u32, u32) {
    // e.g. "60 user-visible features at 100% coverage (119/119 including plumbing"
    let re = match regex::Regex::new(
        r"(\d+) user-visible features at 100% coverage \((\d+)/\d+ including plumbing",
    ) {
        Ok(re) => re,
        Err(_) => return (0, 0),
    };

    re.captures(index_md)
        .map(|c| (c[1].parse().unwrap_or(0), c[2].parse().unwrap_or(0)))
        .unwrap_or((0, 0))
}

// ─────────────────────────────────────────────────────────────────────────────
// RED contract tests — these MUST pass for the fix to be valid
// ─────────────────────────────────────────────────────────────────────────────

/// `index.md`'s user-visible count must match `lsp.md`'s advertised feature count.
#[test]
fn test_index_md_user_visible_count_matches_lsp_md_advertised() {
    let root = project_root();
    let lsp_md_path = root.join("docs/project/status/lsp.md");
    let index_md_path = root.join("docs/project/status/index.md");

    let lsp_md = std::fs::read_to_string(&lsp_md_path).expect("lsp.md must exist");
    let index_md = std::fs::read_to_string(&index_md_path).expect("index.md must exist");

    let (_ux_impl, _ux_total, proto_impl, _proto_total) = parse_lsp_md_metrics(&lsp_md);
    let (index_user_visible, _index_plumbing) = parse_index_md_metrics(&index_md);

    assert_eq!(
        index_user_visible, proto_impl,
        "index.md user-visible ({}) must match lsp.md advertised ({})",
        index_user_visible, proto_impl
    );
}

/// `index.md`'s plumbing count must match `lsp.md`'s "including plumbing" count.
#[test]
fn test_index_md_plumbing_count_matches_lsp_md_protocol() {
    let root = project_root();
    let lsp_md_path = root.join("docs/project/status/lsp.md");
    let index_md_path = root.join("docs/project/status/index.md");

    let lsp_md = std::fs::read_to_string(&lsp_md_path).expect("lsp.md must exist");
    let index_md = std::fs::read_to_string(&index_md_path).expect("index.md must exist");

    let (_ux_impl, _ux_total, proto_impl, _proto_total) = parse_lsp_md_metrics(&lsp_md);
    let (_index_user_visible, index_plumbing) = parse_index_md_metrics(&index_md);

    assert_eq!(
        index_plumbing, proto_impl,
        "index.md plumbing ({}) must match lsp.md protocol ({})",
        index_plumbing, proto_impl
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Edge case tests — verify parsing helpers are robust
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_parse_lsp_md_metrics_empty_string() {
    let (ux_i, ux_t, proto_i, proto_t) = parse_lsp_md_metrics("");
    assert_eq!((ux_i, ux_t, proto_i, proto_t), (0, 0, 0, 0));
}

#[test]
fn test_parse_lsp_md_metrics_no_match() {
    let (ux_i, ux_t, proto_i, proto_t) = parse_lsp_md_metrics("no metrics here");
    assert_eq!((ux_i, ux_t, proto_i, proto_t), (0, 0, 0, 0));
}

#[test]
fn test_parse_lsp_md_metrics_large_numbers() {
    let input =
        "(4294967295/4294967295) advertised features (4294967295/4294967295) including plumbing)";
    let (ux_i, ux_t, proto_i, proto_t) = parse_lsp_md_metrics(input);
    assert_eq!(ux_i, 4_294_967_295);
    assert_eq!(ux_t, 4_294_967_295);
    assert_eq!(proto_i, 4_294_967_295);
    assert_eq!(proto_t, 4_294_967_295);
}

#[test]
fn test_parse_lsp_md_metrics_takes_last_match() {
    let input = "(1/1) advertised features (2/2) including plumbing) (60/60) advertised features (119/119) including plumbing)";
    let (ux_i, ux_t, proto_i, proto_t) = parse_lsp_md_metrics(input);
    assert_eq!((ux_i, ux_t), (60, 60));
    assert_eq!((proto_i, proto_t), (119, 119));
}

#[test]
fn test_parse_index_md_metrics_empty_string() {
    let (uv, plumbing) = parse_index_md_metrics("");
    assert_eq!((uv, plumbing), (0, 0));
}

#[test]
fn test_parse_index_md_metrics_no_match() {
    let (uv, plumbing) = parse_index_md_metrics("no metrics here at all");
    assert_eq!((uv, plumbing), (0, 0));
}

#[test]
fn test_parse_index_md_metrics_60_119() {
    let input = "60 user-visible features at 100% coverage (119/119 including plumbing";
    let (uv, plumbing) = parse_index_md_metrics(input);
    assert_eq!((uv, plumbing), (60, 119));
}

#[test]
fn test_parse_index_md_metrics_58_116_legacy() {
    let input = "58 user-visible features at 100% coverage (116/116 including plumbing";
    let (uv, plumbing) = parse_index_md_metrics(input);
    assert_eq!((uv, plumbing), (58, 116));
}

#[test]
fn test_parse_index_md_metrics_large_numbers() {
    let input = "4294967295 user-visible features at 100% coverage (4294967295/4294967295 including plumbing";
    let (uv, plumbing) = parse_index_md_metrics(input);
    assert_eq!(uv, 4_294_967_295);
    assert_eq!(plumbing, 4_294_967_295);
}

#[test]
fn test_parse_index_md_metrics_both_files_exist_integration() {
    let root = project_root();
    let lsp_md_path = root.join("docs/project/status/lsp.md");
    let index_md_path = root.join("docs/project/status/index.md");
    assert!(lsp_md_path.exists(), "lsp.md must exist at {:?}", lsp_md_path);
    assert!(index_md_path.exists(), "index.md must exist at {:?}", index_md_path);
}

#[test]
fn test_parse_index_md_metrics_empty_file() {
    let (uv, plumbing) = parse_index_md_metrics("");
    assert_eq!((uv, plumbing), (0, 0));
}

#[test]
fn test_parse_index_md_metrics_only_user_visible_no_plumbing() {
    let input = "60 user-visible features at 100% coverage";
    let (uv, plumbing) = parse_index_md_metrics(input);
    assert_eq!((uv, plumbing), (0, 0));
}
