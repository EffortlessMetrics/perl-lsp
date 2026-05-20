//! Validate Real Perl Editor Trust provider claim matrices.

use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail, eyre};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const PROVIDER_MATRIX: &str = "docs/project/status/provider_confidence_matrix.md";
const SUPPORT_TIERS: &str = "docs/project/status/SUPPORT_TIERS.md";

const PROVIDER_HEADER: &[&str] = &[
    "Provider surface",
    "Live state",
    "Fact source / provenance",
    "Confidence and freshness boundary",
    "Fallback / blocker behavior",
    "Runtime or live comparison receipt",
    "Real-workspace link",
    "Next proof",
];

const SUPPORT_HEADER: &[&str] = &[
    "Surface / claim",
    "Tier",
    "Allowed user-facing claim",
    "Proof commands",
    "Status docs",
    "Known limitation",
    "Next promotion proof",
];

const SUPPORT_TIERS_ALLOWED: &[&str] =
    &["measured-bounded", "partial-live-with-fallback", "shadowed", "deferred"];

const FORBIDDEN_SUPPORT_CLAIM_PHRASES: &[&str] = &[
    "full CPAN support",
    "all CPAN support",
    "all-CPAN support",
    "fully supports CPAN",
    "complete static analysis",
    "safe refactor everywhere",
    "generated symbols fully supported",
    "compiler-backed tokens broadly live",
    "full dynamic Perl inference",
];

const REQUIRED_PROVIDER_SURFACES: &[&str] = &[
    "Completion",
    "Goto definition",
    "References",
    "Hover",
    "Diagnostics",
    "Rename",
    "Safe delete",
    "Workspace symbols",
    "Document symbols",
    "Semantic tokens",
    "DAP module paths / Perl subprocess seams",
];

const REQUIRED_SUPPORT_SURFACES: &[&str] = &[
    "Parser compatibility",
    "Module resolution / `@INC` consistency",
    "Completion",
    "Goto definition",
    "References",
    "Hover",
    "Diagnostics",
    "Provider decision explanations",
    "Workspace trust report",
    "Rename",
    "Safe delete",
    "Workspace symbols",
    "Document symbols",
    "Semantic tokens",
    "DAP module paths / Perl subprocess seams",
    "Real-workspace editor baseline",
];

const PARTIAL_LIVE_BOUNDARY_REQUIRED_SURFACES: &[&str] = &[
    "Completion",
    "Goto definition",
    "References",
    "Hover",
    "Diagnostics",
    "Provider decision explanations",
    "Rename",
    "Safe delete",
    "Workspace symbols",
    "Document symbols",
    "Semantic tokens",
];

const EDIT_PRODUCING_SUPPORT_SURFACES: &[&str] = &["Rename", "Safe delete"];

const SUPPORT_BOUNDARY_TERMS: &[&str] = &[
    "fallback",
    "fall back",
    "block",
    "gated",
    "deferred",
    "legacy",
    "no edit",
    "no-edit",
    "zero edits",
    "refuse",
    "refused",
];

const EDIT_ROLLBACK_TERMS: &[&str] = &["rollback", "roll back"];

const EDIT_NO_EDIT_TERMS: &[&str] =
    &["no edit", "no-edit", "zero edits", "return no edits", "returns no edits"];

const EDIT_BLOCKER_TERMS: &[&str] = &["block", "blocked", "blocker", "refuse", "refused"];

#[derive(Debug)]
struct MarkdownTable {
    header: Vec<String>,
    rows: Vec<TableRow>,
}

#[derive(Debug)]
struct TableRow {
    line_number: usize,
    cells: Vec<String>,
}

#[derive(Debug, Default)]
struct ValidationStats {
    provider_rows: usize,
    support_rows: usize,
    links_checked: usize,
}

pub fn run() -> Result<()> {
    let root = project_root()?;
    let stats = validate_docs(&root, PROVIDER_MATRIX, SUPPORT_TIERS)?;
    println!(
        "provider confidence matrix check passed: {} provider rows, {} support rows, {} relative links",
        stats.provider_rows, stats.support_rows, stats.links_checked
    );
    Ok(())
}

pub fn run_support_claims() -> Result<()> {
    let root = project_root()?;
    let stats = validate_support_claim_doc(&root, SUPPORT_TIERS)?;
    println!(
        "support claim map check passed: {} support rows, {} relative links",
        stats.support_rows, stats.links_checked
    );
    Ok(())
}

fn validate_docs(root: &Path, provider_rel: &str, support_rel: &str) -> Result<ValidationStats> {
    let provider_text = read_doc(root, provider_rel)?;
    let support_text = read_doc(root, support_rel)?;

    let mut violations = Vec::new();
    let provider_table = parse_table(&provider_text, "Provider surface")
        .ok_or_else(|| eyre!("provider matrix table not found in {provider_rel}"))?;
    let support_table = parse_table(&support_text, "Surface / claim")
        .ok_or_else(|| eyre!("support tier table not found in {support_rel}"))?;

    validate_header(provider_rel, &provider_table.header, PROVIDER_HEADER, &mut violations);
    validate_header(support_rel, &support_table.header, SUPPORT_HEADER, &mut violations);
    validate_provider_rows(provider_rel, &provider_table, &mut violations);
    validate_support_rows(support_rel, &support_table, &mut violations);

    let links_checked = check_relative_links(root, provider_rel, &provider_text, &mut violations)
        + check_relative_links(root, support_rel, &support_text, &mut violations);

    if !violations.is_empty() {
        eprintln!("provider confidence matrix violations:");
        for violation in &violations {
            eprintln!("  - {violation}");
        }
        bail!("provider confidence matrix check failed with {} violation(s)", violations.len());
    }

    Ok(ValidationStats {
        provider_rows: provider_table.rows.len(),
        support_rows: support_table.rows.len(),
        links_checked,
    })
}

fn validate_support_claim_doc(root: &Path, support_rel: &str) -> Result<ValidationStats> {
    let support_text = read_doc(root, support_rel)?;
    let mut violations = Vec::new();
    let support_table = parse_table(&support_text, "Surface / claim")
        .ok_or_else(|| eyre!("support tier table not found in {support_rel}"))?;

    validate_header(support_rel, &support_table.header, SUPPORT_HEADER, &mut violations);
    validate_support_rows(support_rel, &support_table, &mut violations);
    let links_checked = check_relative_links(root, support_rel, &support_text, &mut violations);

    if !violations.is_empty() {
        eprintln!("support claim map violations:");
        for violation in &violations {
            eprintln!("  - {violation}");
        }
        bail!("support claim map check failed with {} violation(s)", violations.len());
    }

    Ok(ValidationStats { provider_rows: 0, support_rows: support_table.rows.len(), links_checked })
}

fn read_doc(root: &Path, rel: &str) -> Result<String> {
    let path = root.join(rel);
    fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))
}

fn validate_header(doc: &str, actual: &[String], expected: &[&str], violations: &mut Vec<String>) {
    if actual.len() != expected.len() {
        violations.push(format!(
            "{doc}: header has {} columns; expected {}",
            actual.len(),
            expected.len()
        ));
        return;
    }

    for (idx, (actual_cell, expected_cell)) in actual.iter().zip(expected.iter()).enumerate() {
        if actual_cell != expected_cell {
            violations.push(format!(
                "{doc}: header column {} is {:?}; expected {:?}",
                idx + 1,
                actual_cell,
                expected_cell
            ));
        }
    }
}

fn validate_provider_rows(doc: &str, table: &MarkdownTable, violations: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    for row in &table.rows {
        if row.cells.len() != PROVIDER_HEADER.len() {
            violations.push(format!(
                "{doc}:{}: provider row has {} columns; expected {}",
                row.line_number,
                row.cells.len(),
                PROVIDER_HEADER.len()
            ));
            continue;
        }

        let provider = row.cells[0].as_str();
        seen.insert(provider.to_string());
        for (column, label) in PROVIDER_HEADER.iter().enumerate().skip(1) {
            require_meaningful_cell(doc, row, column, label, violations);
        }

        require_markdown_link(doc, row, 5, "Runtime or live comparison receipt", violations);

        let live_state = row.cells[1].to_ascii_lowercase();
        let fallback = row.cells[4].to_ascii_lowercase();
        if live_state.contains("live")
            && !(fallback.contains("fallback")
                || fallback.contains("block")
                || fallback.contains("legacy"))
        {
            violations.push(format!(
                "{doc}:{}: live provider row {:?} must name fallback or blocker behavior",
                row.line_number, provider
            ));
        }
    }

    for required in REQUIRED_PROVIDER_SURFACES {
        if !seen.contains(*required) {
            violations.push(format!("{doc}: missing provider matrix row for {required:?}"));
        }
    }
}

fn validate_support_rows(doc: &str, table: &MarkdownTable, violations: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    for row in &table.rows {
        if row.cells.len() != SUPPORT_HEADER.len() {
            violations.push(format!(
                "{doc}:{}: support row has {} columns; expected {}",
                row.line_number,
                row.cells.len(),
                SUPPORT_HEADER.len()
            ));
            continue;
        }

        let surface = row.cells[0].as_str();
        seen.insert(surface.to_string());
        for (column, label) in SUPPORT_HEADER.iter().enumerate().skip(1) {
            require_meaningful_cell(doc, row, column, label, violations);
        }
        require_valid_support_tier(doc, row, violations);
        require_markdown_link(doc, row, 4, "Status docs", violations);
        if !row.cells[3].contains('`') {
            violations.push(format!(
                "{doc}:{}: support row {:?} must list proof commands in backticks",
                row.line_number, surface
            ));
        }
        reject_forbidden_support_claims(doc, row, violations);
        reject_shadow_live_cutover_claim(doc, row, violations);
        require_partial_live_boundary(doc, row, violations);
        require_edit_producing_safety_terms(doc, row, violations);
    }

    for required in REQUIRED_SUPPORT_SURFACES {
        if !seen.contains(*required) {
            violations.push(format!("{doc}: missing support tier row for {required:?}"));
        }
    }
}

fn require_partial_live_boundary(doc: &str, row: &TableRow, violations: &mut Vec<String>) {
    let tier = normalize_inline_code(&row.cells[1]);
    if tier != "partial-live-with-fallback"
        || !PARTIAL_LIVE_BOUNDARY_REQUIRED_SURFACES.contains(&row.cells[0].as_str())
    {
        return;
    }

    let claim_boundary = support_claim_boundary_text(row);
    if !contains_any_ascii_case_insensitive(&claim_boundary, SUPPORT_BOUNDARY_TERMS) {
        violations.push(format!(
            "{doc}:{}: partial-live support row {:?} must name fallback, blocker, gated, deferred, legacy, refused, or no-edit behavior",
            row.line_number, row.cells[0]
        ));
    }
}

fn require_edit_producing_safety_terms(doc: &str, row: &TableRow, violations: &mut Vec<String>) {
    let tier = normalize_inline_code(&row.cells[1]);
    if tier != "partial-live-with-fallback"
        || !EDIT_PRODUCING_SUPPORT_SURFACES.contains(&row.cells[0].as_str())
    {
        return;
    }

    let claim_boundary = support_claim_boundary_text(row);
    if !contains_any_ascii_case_insensitive(&claim_boundary, EDIT_ROLLBACK_TERMS) {
        violations.push(format!(
            "{doc}:{}: edit-producing support row {:?} must name rollback behavior",
            row.line_number, row.cells[0]
        ));
    }
    if !contains_any_ascii_case_insensitive(&claim_boundary, EDIT_NO_EDIT_TERMS) {
        violations.push(format!(
            "{doc}:{}: edit-producing support row {:?} must name no-edit behavior",
            row.line_number, row.cells[0]
        ));
    }
    if !contains_any_ascii_case_insensitive(&claim_boundary, EDIT_BLOCKER_TERMS) {
        violations.push(format!(
            "{doc}:{}: edit-producing support row {:?} must name blocker or refusal behavior",
            row.line_number, row.cells[0]
        ));
    }
}

fn require_valid_support_tier(doc: &str, row: &TableRow, violations: &mut Vec<String>) {
    let tier = normalize_inline_code(&row.cells[1]);
    if !SUPPORT_TIERS_ALLOWED.iter().any(|allowed| tier == *allowed) {
        violations.push(format!(
            "{doc}:{}: support row {:?} has unsupported tier {:?}",
            row.line_number, row.cells[0], row.cells[1]
        ));
    }
}

fn reject_forbidden_support_claims(doc: &str, row: &TableRow, violations: &mut Vec<String>) {
    let row_text = row.cells.join(" ");
    for phrase in FORBIDDEN_SUPPORT_CLAIM_PHRASES {
        if contains_ascii_case_insensitive(&row_text, phrase) {
            violations.push(format!(
                "{doc}:{}: support row {:?} contains forbidden broad claim phrase {:?}",
                row.line_number, row.cells[0], phrase
            ));
        }
    }
}

fn reject_shadow_live_cutover_claim(doc: &str, row: &TableRow, violations: &mut Vec<String>) {
    let tier = normalize_inline_code(&row.cells[1]);
    if tier != "shadowed" {
        return;
    }

    let claim = row.cells[2].to_ascii_lowercase();
    if contains_live_cutover_language(&claim) && !contains_cutover_negation(&claim) {
        violations.push(format!(
            "{doc}:{}: shadowed support row {:?} must not make a positive live-cutover claim",
            row.line_number, row.cells[0]
        ));
    }
}

fn normalize_inline_code(value: &str) -> String {
    value.trim().trim_matches('`').trim().to_string()
}

fn contains_ascii_case_insensitive(haystack: &str, needle: &str) -> bool {
    haystack.to_ascii_lowercase().contains(&needle.to_ascii_lowercase())
}

fn contains_any_ascii_case_insensitive(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| contains_ascii_case_insensitive(haystack, needle))
}

fn support_claim_boundary_text(row: &TableRow) -> String {
    [row.cells[2].as_str(), row.cells[4].as_str(), row.cells[5].as_str(), row.cells[6].as_str()]
        .join(" ")
}

fn contains_live_cutover_language(value: &str) -> bool {
    ["live cutover", "live provider", "partial-live", "live behavior", "answer live"]
        .iter()
        .any(|needle| value.contains(needle))
}

fn contains_cutover_negation(value: &str) -> bool {
    [
        "without claiming",
        "does not claim",
        "do not claim",
        "not claim",
        "no broad",
        "no dedicated",
        "before cutover",
    ]
    .iter()
    .any(|needle| value.contains(needle))
}

fn require_meaningful_cell(
    doc: &str,
    row: &TableRow,
    column: usize,
    label: &str,
    violations: &mut Vec<String>,
) {
    let value = row.cells[column].trim();
    if value.is_empty()
        || matches!(value.to_ascii_lowercase().as_str(), "n/a" | "na" | "tbd" | "_pending_")
    {
        violations.push(format!(
            "{doc}:{}: column {label:?} must not be empty or pending",
            row.line_number
        ));
    }
}

fn require_markdown_link(
    doc: &str,
    row: &TableRow,
    column: usize,
    label: &str,
    violations: &mut Vec<String>,
) {
    let value = &row.cells[column];
    if !(value.contains('[') && value.contains("](")) {
        violations.push(format!(
            "{doc}:{}: column {label:?} must include at least one markdown link",
            row.line_number
        ));
    }
}

fn parse_table(text: &str, first_header_cell: &str) -> Option<MarkdownTable> {
    let lines = text.lines().collect::<Vec<_>>();
    let header_index = lines.iter().position(|line| {
        split_table_row(line)
            .and_then(|cells| cells.first().cloned())
            .is_some_and(|cell| cell == first_header_cell)
    })?;
    let header = split_table_row(lines[header_index])?;
    let mut rows = Vec::new();

    for (offset, line) in lines.iter().enumerate().skip(header_index + 1) {
        let Some(cells) = split_table_row(line) else {
            break;
        };
        if is_separator_row(&cells) {
            continue;
        }
        rows.push(TableRow { line_number: offset + 1, cells });
    }

    Some(MarkdownTable { header, rows })
}

fn split_table_row(line: &str) -> Option<Vec<String>> {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') || !trimmed.ends_with('|') {
        return None;
    }
    Some(trimmed.trim_matches('|').split('|').map(|cell| cell.trim().to_string()).collect())
}

fn is_separator_row(cells: &[String]) -> bool {
    cells.iter().all(|cell| {
        let trimmed = cell.trim();
        !trimmed.is_empty()
            && trimmed.chars().all(|ch| matches!(ch, '-' | ':' | ' '))
            && trimmed.contains('-')
    })
}

fn check_relative_links(
    root: &Path,
    doc_rel: &str,
    text: &str,
    violations: &mut Vec<String>,
) -> usize {
    let root_canonical = match root.canonicalize() {
        Ok(path) => path,
        Err(err) => {
            violations.push(format!(
                "{doc_rel}: project root cannot be canonicalized for link validation: {err}"
            ));
            return 0;
        }
    };
    let doc_path = root.join(doc_rel);
    let doc_dir = doc_path.parent().map(Path::to_path_buf).unwrap_or_else(|| root.to_path_buf());
    let mut checked = 0usize;

    for (line_number, line) in text.lines().enumerate() {
        for target in markdown_link_targets(line) {
            if should_skip_link(&target) {
                continue;
            }
            let path_part = target.split('#').next().unwrap_or_default().trim();
            if path_part.is_empty() {
                continue;
            }

            checked += 1;
            let target_path = doc_dir.join(path_part);
            if !target_path.exists() {
                violations.push(format!(
                    "{doc_rel}:{}: markdown link target does not exist: {}",
                    line_number + 1,
                    display_relative(root, &target_path)
                ));
                continue;
            }

            match target_path.canonicalize() {
                Ok(canonical) if canonical.starts_with(&root_canonical) => {}
                Ok(canonical) => violations.push(format!(
                    "{doc_rel}:{}: markdown link target escapes project root: {}",
                    line_number + 1,
                    canonical.display()
                )),
                Err(err) => violations.push(format!(
                    "{doc_rel}:{}: markdown link target cannot be canonicalized: {} ({err})",
                    line_number + 1,
                    display_relative(root, &target_path)
                )),
            }
        }
    }

    checked
}

fn markdown_link_targets(line: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let mut rest = line;

    while let Some(start) = rest.find("](") {
        let after_open = &rest[start + 2..];
        let Some(end) = after_open.find(')') else {
            break;
        };
        targets.push(after_open[..end].trim().trim_matches(['<', '>']).to_string());
        rest = &after_open[end + 1..];
    }

    targets
}

fn should_skip_link(target: &str) -> bool {
    target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with("mailto:")
        || target.starts_with('#')
}

fn display_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| PathBuf::from(path))
        .display()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_markdown_table_rows() -> Result<()> {
        let text = "\
before
| Provider surface | Live state |
| --- | --- |
| Completion | `partial live / shadowed` |
after";
        let table = parse_table(text, "Provider surface")
            .ok_or_else(|| eyre!("expected table to parse"))?;
        assert_eq!(table.header, vec!["Provider surface", "Live state"]);
        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.rows[0].cells[0], "Completion");
        Ok(())
    }

    #[test]
    fn extracts_relative_markdown_link_targets() {
        let targets = markdown_link_targets(
            "See [matrix](provider_confidence_matrix.md), [issue](https://example.test/x), and [anchor](#local).",
        );
        assert_eq!(
            targets,
            vec![
                "provider_confidence_matrix.md".to_string(),
                "https://example.test/x".to_string(),
                "#local".to_string()
            ]
        );
    }

    #[test]
    fn validates_minimal_provider_and_support_docs() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let status_dir = root.join("docs").join("project").join("status");
        fs::create_dir_all(&status_dir)?;
        fs::write(status_dir.join("semantic_shadow_compare.md"), "# Shadow\n")?;
        fs::write(status_dir.join("provider_cutover.md"), "# Cutover\n")?;
        fs::write(status_dir.join("provider_confidence_matrix.md"), provider_fixture())?;
        fs::write(status_dir.join("SUPPORT_TIERS.md"), support_fixture())?;

        let stats = validate_docs(
            root,
            "docs/project/status/provider_confidence_matrix.md",
            "docs/project/status/SUPPORT_TIERS.md",
        )?;
        assert_eq!(stats.provider_rows, REQUIRED_PROVIDER_SURFACES.len());
        assert_eq!(stats.support_rows, REQUIRED_SUPPORT_SURFACES.len());
        assert!(stats.links_checked > 0);
        Ok(())
    }

    #[test]
    fn support_claim_map_validates_minimal_support_doc() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let status_dir = root.join("docs").join("project").join("status");
        fs::create_dir_all(&status_dir)?;
        fs::write(status_dir.join("provider_confidence_matrix.md"), provider_fixture())?;
        fs::write(status_dir.join("SUPPORT_TIERS.md"), support_fixture())?;

        let stats = validate_support_claim_doc(root, "docs/project/status/SUPPORT_TIERS.md")?;
        assert_eq!(stats.support_rows, REQUIRED_SUPPORT_SURFACES.len());
        assert!(stats.links_checked > 0);
        Ok(())
    }

    #[test]
    fn rejects_missing_required_provider_rows() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let status_dir = root.join("docs").join("project").join("status");
        fs::create_dir_all(&status_dir)?;
        fs::write(status_dir.join("semantic_shadow_compare.md"), "# Shadow\n")?;
        fs::write(status_dir.join("provider_cutover.md"), "# Cutover\n")?;
        fs::write(
            status_dir.join("provider_confidence_matrix.md"),
            provider_fixture_for(&REQUIRED_PROVIDER_SURFACES[1..], "semantic_shadow_compare.md"),
        )?;
        fs::write(status_dir.join("SUPPORT_TIERS.md"), support_fixture())?;

        let result = validate_docs(
            root,
            "docs/project/status/provider_confidence_matrix.md",
            "docs/project/status/SUPPORT_TIERS.md",
        );
        assert!(result.is_err(), "missing provider rows must fail validation");
        Ok(())
    }

    #[test]
    fn support_claim_map_rejects_forbidden_broad_claim() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let status_dir = root.join("docs").join("project").join("status");
        fs::create_dir_all(&status_dir)?;
        fs::write(status_dir.join("provider_confidence_matrix.md"), provider_fixture())?;
        fs::write(
            status_dir.join("SUPPORT_TIERS.md"),
            support_fixture_for_claim(
                REQUIRED_SUPPORT_SURFACES,
                "provider_confidence_matrix.md",
                "Parser compatibility",
                "perl-lsp has full CPAN support",
            ),
        )?;

        let result = validate_support_claim_doc(root, "docs/project/status/SUPPORT_TIERS.md");
        assert!(result.is_err(), "forbidden broad claims must fail validation");
        Ok(())
    }

    #[test]
    fn support_claim_map_rejects_missing_required_support_rows() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let status_dir = root.join("docs").join("project").join("status");
        fs::create_dir_all(&status_dir)?;
        fs::write(status_dir.join("provider_confidence_matrix.md"), provider_fixture())?;
        fs::write(
            status_dir.join("SUPPORT_TIERS.md"),
            support_fixture_for(&REQUIRED_SUPPORT_SURFACES[1..], "provider_confidence_matrix.md"),
        )?;

        let result = validate_support_claim_doc(root, "docs/project/status/SUPPORT_TIERS.md");
        assert!(result.is_err(), "missing support rows must fail validation");
        Ok(())
    }

    #[test]
    fn support_claim_map_rejects_partial_live_without_boundary_language() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let status_dir = root.join("docs").join("project").join("status");
        fs::create_dir_all(&status_dir)?;
        fs::write(status_dir.join("provider_confidence_matrix.md"), provider_fixture())?;
        fs::write(
            status_dir.join("SUPPORT_TIERS.md"),
            support_fixture_for_row(
                "Completion",
                "`partial-live-with-fallback`",
                "High-confidence facts can answer live.",
                "Current proof exists.",
                "More proof.",
            ),
        )?;

        let result = validate_support_claim_doc(root, "docs/project/status/SUPPORT_TIERS.md");
        assert!(result.is_err(), "partial-live claims must name bounded behavior");
        Ok(())
    }

    #[test]
    fn support_claim_map_allows_partial_live_with_boundary_language() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let status_dir = root.join("docs").join("project").join("status");
        fs::create_dir_all(&status_dir)?;
        fs::write(status_dir.join("provider_confidence_matrix.md"), provider_fixture())?;
        fs::write(
            status_dir.join("SUPPORT_TIERS.md"),
            support_fixture_for_row(
                "Completion",
                "`partial-live-with-fallback`",
                "High-confidence facts can answer live with fallback.",
                "Generated and dynamic candidates remain gated.",
                "More fallback proof.",
            ),
        )?;

        validate_support_claim_doc(root, "docs/project/status/SUPPORT_TIERS.md")?;
        Ok(())
    }

    #[test]
    fn support_claim_map_rejects_edit_producing_claim_without_rollback_and_no_edit() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let status_dir = root.join("docs").join("project").join("status");
        fs::create_dir_all(&status_dir)?;
        fs::write(status_dir.join("provider_confidence_matrix.md"), provider_fixture())?;
        fs::write(
            status_dir.join("SUPPORT_TIERS.md"),
            support_fixture_for_row(
                "Safe delete",
                "`partial-live-with-fallback`",
                "Source-backed symbols can return WorkspaceEdits when proof is high-confidence.",
                "Unsafe cases are blocked.",
                "More blocker proof.",
            ),
        )?;

        let result = validate_support_claim_doc(root, "docs/project/status/SUPPORT_TIERS.md");
        assert!(
            result.is_err(),
            "edit-producing support claims must name rollback and no-edit behavior"
        );
        Ok(())
    }

    #[test]
    fn support_claim_map_allows_edit_producing_claim_with_safety_language() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let status_dir = root.join("docs").join("project").join("status");
        fs::create_dir_all(&status_dir)?;
        fs::write(status_dir.join("provider_confidence_matrix.md"), provider_fixture())?;
        fs::write(
            status_dir.join("SUPPORT_TIERS.md"),
            support_fixture_for_row(
                "Safe delete",
                "`partial-live-with-fallback`",
                "Source-backed symbols can return WorkspaceEdits only with rollback proof; unsafe cases return no edit.",
                "Generated and dynamic requests are blocked.",
                "More no-edit blocker proof.",
            ),
        )?;

        validate_support_claim_doc(root, "docs/project/status/SUPPORT_TIERS.md")?;
        Ok(())
    }

    #[test]
    fn support_claim_map_rejects_shadowed_live_cutover_claim() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let status_dir = root.join("docs").join("project").join("status");
        fs::create_dir_all(&status_dir)?;
        fs::write(status_dir.join("provider_confidence_matrix.md"), provider_fixture())?;
        fs::write(
            status_dir.join("SUPPORT_TIERS.md"),
            support_fixture_for_claim(
                REQUIRED_SUPPORT_SURFACES,
                "provider_confidence_matrix.md",
                "Rename",
                "Rename has live cutover for compiler-backed facts",
            ),
        )?;

        let result = validate_support_claim_doc(root, "docs/project/status/SUPPORT_TIERS.md");
        assert!(result.is_err(), "shadowed live-cutover claims must fail validation");
        Ok(())
    }

    #[test]
    fn support_claim_map_allows_negated_shadow_live_cutover_boundary() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let status_dir = root.join("docs").join("project").join("status");
        fs::create_dir_all(&status_dir)?;
        fs::write(status_dir.join("provider_confidence_matrix.md"), provider_fixture())?;
        fs::write(
            status_dir.join("SUPPORT_TIERS.md"),
            support_fixture_for_claim(
                REQUIRED_SUPPORT_SURFACES,
                "provider_confidence_matrix.md",
                "Document symbols",
                "Document-symbol receipts exist without claiming compiler-backed live cutover",
            ),
        )?;

        validate_support_claim_doc(root, "docs/project/status/SUPPORT_TIERS.md")?;
        Ok(())
    }

    #[test]
    fn rejects_missing_relative_links() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let status_dir = root.join("docs").join("project").join("status");
        fs::create_dir_all(&status_dir)?;
        fs::write(status_dir.join("provider_confidence_matrix.md"), provider_fixture())?;
        fs::write(
            status_dir.join("SUPPORT_TIERS.md"),
            support_fixture_for(REQUIRED_SUPPORT_SURFACES, "missing-status.md"),
        )?;

        let result = validate_docs(
            root,
            "docs/project/status/provider_confidence_matrix.md",
            "docs/project/status/SUPPORT_TIERS.md",
        );
        assert!(result.is_err(), "missing relative links must fail validation");
        Ok(())
    }

    #[test]
    fn rejects_links_outside_project_root() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("repo");
        let outside_path = temp.path().join("outside-status.md");
        let status_dir = root.join("docs").join("project").join("status");
        fs::create_dir_all(&status_dir)?;
        fs::write(&outside_path, "# Outside\n")?;
        fs::write(status_dir.join("semantic_shadow_compare.md"), "# Shadow\n")?;
        fs::write(status_dir.join("provider_cutover.md"), "# Cutover\n")?;
        fs::write(status_dir.join("provider_confidence_matrix.md"), provider_fixture())?;
        fs::write(
            status_dir.join("SUPPORT_TIERS.md"),
            support_fixture_for(REQUIRED_SUPPORT_SURFACES, "../../../../outside-status.md"),
        )?;

        let result = validate_docs(
            &root,
            "docs/project/status/provider_confidence_matrix.md",
            "docs/project/status/SUPPORT_TIERS.md",
        );
        assert!(result.is_err(), "links escaping the project root must fail validation");
        Ok(())
    }

    fn provider_fixture() -> String {
        provider_fixture_for(REQUIRED_PROVIDER_SURFACES, "semantic_shadow_compare.md")
    }

    fn provider_fixture_for(surfaces: &[&str], receipt_link: &str) -> String {
        let mut text = String::from("| ");
        text.push_str(&PROVIDER_HEADER.join(" | "));
        text.push_str(" |\n| ");
        text.push_str(&PROVIDER_HEADER.iter().map(|_| "---").collect::<Vec<_>>().join(" | "));
        text.push_str(" |\n");
        for surface in surfaces {
            text.push_str(&format!(
                "| {surface} | `partial live` | source | confidence and freshness | legacy fallback blocks unsafe facts | [shadow]({receipt_link}) | real workspace link text | next proof |\n"
            ));
        }
        text
    }

    fn support_fixture() -> String {
        support_fixture_for(REQUIRED_SUPPORT_SURFACES, "provider_confidence_matrix.md")
    }

    fn support_fixture_for(surfaces: &[&str], status_link: &str) -> String {
        support_fixture_for_claim(surfaces, status_link, "", "claim")
    }

    fn support_fixture_for_claim(
        surfaces: &[&str],
        status_link: &str,
        claim_surface: &str,
        claim_text: &str,
    ) -> String {
        support_fixture_for_row_with_surfaces(
            surfaces,
            status_link,
            claim_surface,
            "`shadowed`",
            claim_text,
            "limitation",
            "next proof",
        )
    }

    fn support_fixture_for_row(
        claim_surface: &str,
        tier: &str,
        claim_text: &str,
        limitation: &str,
        next_proof: &str,
    ) -> String {
        support_fixture_for_row_with_surfaces(
            REQUIRED_SUPPORT_SURFACES,
            "provider_confidence_matrix.md",
            claim_surface,
            tier,
            claim_text,
            limitation,
            next_proof,
        )
    }

    fn support_fixture_for_row_with_surfaces(
        surfaces: &[&str],
        status_link: &str,
        claim_surface: &str,
        tier: &str,
        claim_text: &str,
        limitation: &str,
        next_proof: &str,
    ) -> String {
        let mut text = String::from("| ");
        text.push_str(&SUPPORT_HEADER.join(" | "));
        text.push_str(" |\n| ");
        text.push_str(&SUPPORT_HEADER.iter().map(|_| "---").collect::<Vec<_>>().join(" | "));
        text.push_str(" |\n");
        for surface in surfaces {
            let claim = if *surface == claim_surface { claim_text } else { "claim" };
            let row_tier = if *surface == claim_surface { tier } else { "`shadowed`" };
            let row_limitation = if *surface == claim_surface { limitation } else { "limitation" };
            let row_next_proof = if *surface == claim_surface { next_proof } else { "next proof" };
            text.push_str(&format!(
                "| {surface} | {row_tier} | {claim} | `cargo xtask semantic-shadow-compare --check` | [matrix]({status_link}) | {row_limitation} | {row_next_proof} |\n"
            ));
        }
        text
    }
}
