// Test that agent definition files have ## Todo list as their final section.
// This enforces the swarm architecture convention: agents read top-to-bottom
// and must encounter their todo list (with terminal skills like /scout-report,
// /agent-wrapup) as the last thing they see.
//
// See: ADR work-eb418345, issue #4382, issue #4087

use std::fs;
use std::path::{Path, PathBuf};

/// Extract all level-2 markdown sections (## Section Name) from content,
/// returning them in order along with the line number where each appears.
fn extract_sections(content: &str) -> Vec<(usize, &str)> {
    content
        .lines()
        .enumerate()
        .filter(|(_, line)| line.starts_with("## "))
        .map(|(idx, line)| (idx + 1, line.trim_start_matches("## ").trim()))
        .collect()
}

/// Get the last section name from a markdown file.
fn get_last_section(file_path: &Path) -> Option<String> {
    let content = fs::read_to_string(file_path).ok()?;
    extract_sections(&content).last().map(|(_, name)| name.to_string())
}

/// Get all sections from a markdown file for diagnostics.
fn get_all_sections(file_path: &Path) -> Option<Vec<String>> {
    let content = fs::read_to_string(file_path).ok()?;
    Some(extract_sections(&content).into_iter().map(|(_, name)| name.to_string()).collect())
}

fn agent_path(name: &str) -> PathBuf {
    // Navigate from integration test (crates/perl-lsp-ux-tests/tests/) to repo root
    // CARGO_MANIFEST_DIR for integration tests is the crate root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/perl-lsp-ux-tests -> crates
        .and_then(|p| p.parent()) // crates -> repo root
        .unwrap_or_else(|| Path::new("."))
        .join(".claude/agents")
        .join(name)
}

// ---------------------------------------------------------------------------
// Tests: scout-parser.md
// ---------------------------------------------------------------------------

#[test]
fn test_scout_parser_last_section_is_todo_list() {
    let path = agent_path("scout-parser.md");
    let last_section = get_last_section(&path)
        .expect("scout-parser.md should exist and be readable");

    assert_eq!(
        last_section, "Todo list",
        "scout-parser.md: last section must be '## Todo list' per swarm architecture\n\
         All sections: {:?}\n\
         See: issue #4382, issue #4087",
        get_all_sections(&path)
    );
}

// ---------------------------------------------------------------------------
// Tests: scout-dap.md
// ---------------------------------------------------------------------------

#[test]
fn test_scout_dap_last_section_is_todo_list() {
    let path = agent_path("scout-dap.md");
    let last_section = get_last_section(&path)
        .expect("scout-dap.md should exist and be readable");

    assert_eq!(
        last_section, "Todo list",
        "scout-dap.md: last section must be '## Todo list' per swarm architecture\n\
         All sections: {:?}\n\
         See: issue #4382, issue #4087",
        get_all_sections(&path)
    );
}

// ---------------------------------------------------------------------------
// Tests: accuracy-scout.md
// ---------------------------------------------------------------------------

#[test]
fn test_accuracy_scout_last_section_is_todo_list() {
    let path = agent_path("accuracy-scout.md");
    let last_section = get_last_section(&path)
        .expect("accuracy-scout.md should exist and be readable");

    assert_eq!(
        last_section, "Todo list",
        "accuracy-scout.md: last section must be '## Todo list' per swarm architecture\n\
         All sections: {:?}\n\
         See: issue #4382, issue #4087",
        get_all_sections(&path)
    );
}

// ---------------------------------------------------------------------------
// Tests: scout-lsp.md
// ---------------------------------------------------------------------------

#[test]
fn test_scout_lsp_last_section_is_todo_list() {
    let path = agent_path("scout-lsp.md");
    let last_section = get_last_section(&path)
        .expect("scout-lsp.md should exist and be readable");

    assert_eq!(
        last_section, "Todo list",
        "scout-lsp.md: last section must be '## Todo list' per swarm architecture\n\
         All sections: {:?}\n\
         See: issue #4382, issue #4087",
        get_all_sections(&path)
    );
}
