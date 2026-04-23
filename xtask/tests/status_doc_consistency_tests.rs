//! Status Documentation Consistency Tests
//!
//! Tests for GitHub issue #4208: verify that docs/project/status/index.md
//! narrative numbers match the auto-generated docs/project/status/lsp.md metrics.
//!
//! The index.md narrative must stay in sync with lsp.md because:
//! - index.md is hand-edited narrative (human-owned)
//! - lsp.md is auto-generated from features.toml via `cargo xtask update-status --only lsp`
//!
//! This test ensures that when code-builder fixes the contradiction,
//! the numbers in index.md line 10 match what's reported in lsp.md.

use regex::Regex;
use std::fs;
use std::path::PathBuf;

/// Extract "(UX_implemented/UX_total, plumbing_implemented/plumbing_total)" counts from lsp.md.
///
/// Example line from lsp.md:
///   "| **LSP Coverage** | 100% (60/60 advertised features, `features.toml`) | 100% | PASS |"
///   "- **Protocol Compliance**: 100% overall LSP protocol support (119/119 including plumbing)"
fn parse_lsp_md_metrics(lsp_md_content: &str) -> (u32, u32, u32, u32) {
    // Parse the LSP_COVERAGE line: "100% (60/60 advertised features, `features.toml`)"
    // We want: ux_implemented=60, ux_total=60
    let ux_re = Regex::new(r"\((\d+)/(\d+) advertised features").unwrap();

    // Parse the protocol compliance line: "(119/119 including plumbing)"
    let protocol_re = Regex::new(r"\((\d+)/(\d+) including plumbing\)").unwrap();

    let mut ux_impl = 0u32;
    let mut ux_total = 0u32;
    let mut proto_impl = 0u32;
    let mut proto_total = 0u32;

    for line in lsp_md_content.lines() {
        if let Some(caps) = ux_re.captures(line) {
            ux_impl = caps.get(1).unwrap().as_str().parse().unwrap();
            ux_total = caps.get(2).unwrap().as_str().parse().unwrap();
        }
        if let Some(caps) = protocol_re.captures(line) {
            proto_impl = caps.get(1).unwrap().as_str().parse().unwrap();
            proto_total = caps.get(2).unwrap().as_str().parse().unwrap();
        }
    }

    (ux_impl, ux_total, proto_impl, proto_total)
}

/// Extract "N user-visible features at 100% coverage (M/M including plumbing...)" from index.md line 10.
///
/// The problematic line is:
///   "- **LSP server**: `features.toml` is the canonical capability catalog; 58 user-visible features at 100% coverage (116/116 including plumbing protocol methods and DAP handlers — corrected in PR #4107 after the DAP catalog undercount audit) — computed coverage is generated from it"
///
/// After the fix, it should say:
///   "60 user-visible features at 100% coverage (119/119 including plumbing...)"
fn parse_index_md_metrics(index_md_content: &str) -> (u32, u32) {
    // Match "N user-visible features at 100% coverage (M/M including plumbing...)"
    let re =
        Regex::new(r"(\d+) user-visible features at 100% coverage \((\d+)/\d+ including plumbing")
            .unwrap();

    if let Some(caps) = re.captures(index_md_content) {
        let user_visible: u32 = caps.get(1).unwrap().as_str().parse().unwrap();
        let plumbing: u32 = caps.get(2).unwrap().as_str().parse().unwrap();
        return (user_visible, plumbing);
    }

    (0, 0)
}

fn project_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop(); // xtask is at <root>/xtask -- go up one level
    dir
}

/// The index.md narrative LSP server line must report the same user-visible count as lsp.md.
///
/// index.md line 10: "... N user-visible features at 100% coverage (M/M including plumbing...)"
/// lsp.md generated: "... (60/60 advertised features...)" for UX
///
/// Currently FAILS: index.md says 58, lsp.md says 60/60
/// After fix: index.md should say 60, matching lsp.md's 60/60
#[test]
fn test_index_md_user_visible_count_matches_lsp_md_advertised()
-> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let index_path = root.join("docs/project/status/index.md");
    let lsp_path = root.join("docs/project/status/lsp.md");

    let index_content = fs::read_to_string(&index_path)?;
    let lsp_content = fs::read_to_string(&lsp_path)?;

    let (index_user_visible, _index_plumbing) = parse_index_md_metrics(&index_content);
    let (lsp_ux_impl, lsp_ux_total, _, _) = parse_lsp_md_metrics(&lsp_content);

    assert!(
        index_user_visible > 0,
        "index.md line 10 must contain user-visible feature count (found 0)"
    );

    let failure_msg = format!(
        "index.md user-visible count ({}) must match lsp.md advertised count ({}/{}).\n\
         Issue #4208: index.md line 10 says '{} user-visible' but lsp.md says '{}/{} advertised'.\n\
         The narrative in index.md must be manually synced to match the auto-generated lsp.md metrics.\n\
         Run `cargo xtask update-status --only lsp` to see current metrics, then edit index.md line 10.",
        index_user_visible,
        lsp_ux_impl,
        lsp_ux_total,
        index_user_visible,
        lsp_ux_impl,
        lsp_ux_total
    );

    assert_eq!(index_user_visible, lsp_ux_impl, "{}", failure_msg);

    Ok(())
}

/// The index.md narrative LSP server line must report the same plumbing/protocol count as lsp.md.
///
/// index.md line 10: "... (M/M including plumbing...)"
/// lsp.md generated: "... (119/119 including plumbing)" for Protocol Compliance
///
/// Currently FAILS: index.md says 116, lsp.md says 119/119
/// After fix: index.md should say 119, matching lsp.md's 119/119
#[test]
fn test_index_md_plumbing_count_matches_lsp_md_protocol() -> Result<(), Box<dyn std::error::Error>>
{
    let root = project_root();
    let index_path = root.join("docs/project/status/index.md");
    let lsp_path = root.join("docs/project/status/lsp.md");

    let index_content = fs::read_to_string(&index_path)?;
    let lsp_content = fs::read_to_string(&lsp_path)?;

    let (_index_user_visible, index_plumbing) = parse_index_md_metrics(&index_content);
    let (_, _, lsp_proto_impl, lsp_proto_total) = parse_lsp_md_metrics(&lsp_content);

    assert!(index_plumbing > 0, "index.md line 10 must contain plumbing/protocol count (found 0)");

    let failure_msg = format!(
        "index.md plumbing count ({}) must match lsp.md protocol compliance count ({}/{}).\n\
         Issue #4208: index.md line 10 says '({}/... including plumbing)' but lsp.md says '({}/{} including plumbing)'.\n\
         The narrative in index.md must be manually synced to match the auto-generated lsp.md metrics.\n\
         Run `cargo xtask update-status --only lsp` to see current metrics, then edit index.md line 10.",
        index_plumbing,
        lsp_proto_impl,
        lsp_proto_total,
        index_plumbing,
        lsp_proto_impl,
        lsp_proto_total
    );

    assert_eq!(index_plumbing, lsp_proto_impl, "{}", failure_msg);

    Ok(())
}
