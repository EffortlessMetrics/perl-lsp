//! Validate the machine-readable provider promotion ledger.

use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const POLICY_PATH: &str = "policy/provider-promotion-ledger.toml";
const HUMAN_LEDGER: &str = "docs/project/status/provider_promotion_ledger.md";
const POLICY_NAME: &str = "provider-promotion-ledger";

const REQUIRED_DECISIONS: &[&str] = &["promote", "fallback", "block", "defer"];

const REQUIRED_BLOCKERS: &[&str] = &[
    "generated_no_source",
    "dynamic_boundary",
    "stale_fact",
    "low_confidence",
    "ambiguous_identity",
    "imported_exported",
    "typeglob_alias",
    "autoload",
    "symbolic_ref",
    "dynamic_require",
    "rollback_missing",
    "current_source_reference",
    "workspace_reference",
    "unsupported_fact_class",
    "unsafe_edit_blocked",
    "missing_fact",
    "fallback_policy",
    "unknown",
];

#[derive(Debug, Deserialize)]
struct ProviderPromotionLedger {
    schema_version: u32,
    policy: String,
    human_ledger: String,
    decision_states: Vec<String>,
    blocker_registry: Vec<String>,
    #[serde(default)]
    provider_class: Vec<ProviderClass>,
}

#[derive(Debug, Deserialize)]
struct ProviderClass {
    surface: String,
    fact_class: String,
    current_state: String,
    decision: String,
    next_proof: String,
    promotion_conditions: Vec<String>,
    fallback_conditions: Vec<String>,
    blocker_conditions: Vec<String>,
    required_receipts: Vec<String>,
}

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

#[derive(Debug)]
struct ValidationStats {
    policy_rows: usize,
    human_rows: usize,
    blockers: usize,
}

pub fn run() -> Result<()> {
    let root = project_root()?;
    let stats = validate(&root)?;
    println!(
        "provider promotion ledger check passed: {} policy rows, {} human rows, {} blocker registry entries",
        stats.policy_rows, stats.human_rows, stats.blockers
    );
    Ok(())
}

fn validate(root: &Path) -> Result<ValidationStats> {
    let policy = read_policy(root, POLICY_PATH)?;
    let human_text = read_text(root, HUMAN_LEDGER)?;
    let human_table = parse_table(&human_text, "Surface")
        .ok_or_else(|| color_eyre::eyre::eyre!("provider promotion ledger table not found"))?;

    let mut violations = Vec::new();
    validate_policy_shape(&policy, &mut violations);
    validate_human_table(&human_table, &mut violations);
    validate_policy_rows(root, &policy, &mut violations);
    validate_policy_matches_human(&policy, &human_table, &mut violations);

    if !violations.is_empty() {
        eprintln!("provider promotion ledger violations:");
        for violation in &violations {
            eprintln!("  - {violation}");
        }
        bail!("provider promotion ledger check failed with {} violation(s)", violations.len());
    }

    Ok(ValidationStats {
        policy_rows: policy.provider_class.len(),
        human_rows: human_table.rows.len(),
        blockers: policy.blocker_registry.len(),
    })
}

fn read_policy(root: &Path, rel: &str) -> Result<ProviderPromotionLedger> {
    let text = read_text(root, rel)?;
    toml::from_str(&text).with_context(|| format!("failed to parse {rel}"))
}

fn read_text(root: &Path, rel: &str) -> Result<String> {
    let path = root.join(rel);
    fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))
}

fn validate_policy_shape(policy: &ProviderPromotionLedger, violations: &mut Vec<String>) {
    if policy.schema_version != 1 {
        violations.push(format!(
            "{POLICY_PATH}: schema_version is {}; expected 1",
            policy.schema_version
        ));
    }
    if policy.policy != POLICY_NAME {
        violations.push(format!(
            "{POLICY_PATH}: policy is {:?}; expected {:?}",
            policy.policy, POLICY_NAME
        ));
    }
    if policy.human_ledger != HUMAN_LEDGER {
        violations.push(format!(
            "{POLICY_PATH}: human_ledger is {:?}; expected {:?}",
            policy.human_ledger, HUMAN_LEDGER
        ));
    }

    require_exact_set(
        POLICY_PATH,
        "decision_states",
        &policy.decision_states,
        REQUIRED_DECISIONS,
        violations,
    );
    require_exact_set(
        POLICY_PATH,
        "blocker_registry",
        &policy.blocker_registry,
        REQUIRED_BLOCKERS,
        violations,
    );
    if policy.provider_class.is_empty() {
        violations.push(format!("{POLICY_PATH}: provider_class must not be empty"));
    }
}

fn validate_human_table(table: &MarkdownTable, violations: &mut Vec<String>) {
    let expected = [
        "Surface",
        "Fact class",
        "Current state",
        "Next proof",
        "Promotion condition",
        "Fallback condition",
        "Blocker condition",
    ];
    if table.header.len() != expected.len() {
        violations.push(format!(
            "{HUMAN_LEDGER}: header has {} columns; expected {}",
            table.header.len(),
            expected.len()
        ));
        return;
    }

    for (index, (actual, expected_cell)) in table.header.iter().zip(expected.iter()).enumerate() {
        if actual != expected_cell {
            violations.push(format!(
                "{HUMAN_LEDGER}: header column {} is {:?}; expected {:?}",
                index + 1,
                actual,
                expected_cell
            ));
        }
    }
}

fn validate_policy_rows(
    root: &Path,
    policy: &ProviderPromotionLedger,
    violations: &mut Vec<String>,
) {
    let decision_states =
        policy.decision_states.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let blocker_registry =
        policy.blocker_registry.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let mut seen_rows = BTreeSet::new();

    for row in &policy.provider_class {
        let key = row_key(&row.surface, &row.fact_class);
        if !seen_rows.insert(key.clone()) {
            violations.push(format!("{POLICY_PATH}: duplicate provider_class row {key}"));
        }

        require_non_empty(&key, "surface", &row.surface, violations);
        require_non_empty(&key, "fact_class", &row.fact_class, violations);
        require_non_empty(&key, "current_state", &row.current_state, violations);
        require_non_empty(&key, "next_proof", &row.next_proof, violations);

        if !decision_states.contains(row.decision.as_str()) {
            violations.push(format!(
                "{POLICY_PATH}: {key} decision {:?} is not listed in decision_states",
                row.decision
            ));
        }

        require_non_empty_list(&key, "promotion_conditions", &row.promotion_conditions, violations);
        require_non_empty_list(&key, "fallback_conditions", &row.fallback_conditions, violations);
        require_non_empty_list(&key, "blocker_conditions", &row.blocker_conditions, violations);
        require_non_empty_list(&key, "required_receipts", &row.required_receipts, violations);

        for blocker in &row.blocker_conditions {
            if !blocker_registry.contains(blocker.as_str()) {
                violations.push(format!(
                    "{POLICY_PATH}: {key} blocker condition {:?} is missing from blocker_registry",
                    blocker
                ));
            }
        }

        for receipt in &row.required_receipts {
            if !root.join(receipt).exists() {
                violations.push(format!(
                    "{POLICY_PATH}: {key} required receipt does not exist: {receipt}"
                ));
            }
        }
    }
}

fn validate_policy_matches_human(
    policy: &ProviderPromotionLedger,
    table: &MarkdownTable,
    violations: &mut Vec<String>,
) {
    let mut human_rows = BTreeSet::new();
    for row in &table.rows {
        if row.cells.len() != table.header.len() {
            violations.push(format!(
                "{HUMAN_LEDGER}:{}: row has {} columns; expected {}",
                row.line_number,
                row.cells.len(),
                table.header.len()
            ));
            continue;
        }
        human_rows.insert(HumanRow {
            key: row_key(&row.cells[0], &row.cells[1]),
            current_state: normalize_inline_code(&row.cells[2]),
            next_proof: row.cells[3].clone(),
        });
    }

    let policy_rows = policy
        .provider_class
        .iter()
        .map(|row| HumanRow {
            key: row_key(&row.surface, &row.fact_class),
            current_state: row.current_state.clone(),
            next_proof: row.next_proof.clone(),
        })
        .collect::<BTreeSet<_>>();

    for row in human_rows.difference(&policy_rows) {
        violations
            .push(format!("{POLICY_PATH}: missing TOML row matching human ledger row {}", row.key));
    }
    for row in policy_rows.difference(&human_rows) {
        violations
            .push(format!("{POLICY_PATH}: TOML row has no matching human ledger row {}", row.key));
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct HumanRow {
    key: String,
    current_state: String,
    next_proof: String,
}

fn require_exact_set(
    doc: &str,
    field: &str,
    actual: &[String],
    expected: &[&str],
    violations: &mut Vec<String>,
) {
    let actual_set = actual.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let expected_set = expected.iter().copied().collect::<BTreeSet<_>>();

    for missing in expected_set.difference(&actual_set) {
        violations.push(format!("{doc}: {field} missing required entry {missing:?}"));
    }
    for unexpected in actual_set.difference(&expected_set) {
        violations.push(format!("{doc}: {field} contains unsupported entry {unexpected:?}"));
    }
}

fn require_non_empty(row: &str, field: &str, value: &str, violations: &mut Vec<String>) {
    if value.trim().is_empty() {
        violations.push(format!("{POLICY_PATH}: {row} field {field} must not be empty"));
    }
}

fn require_non_empty_list(row: &str, field: &str, values: &[String], violations: &mut Vec<String>) {
    if values.is_empty() {
        violations.push(format!("{POLICY_PATH}: {row} field {field} must not be empty"));
        return;
    }

    for value in values {
        if value.trim().is_empty() {
            violations.push(format!("{POLICY_PATH}: {row} field {field} contains an empty item"));
        }
    }
}

fn row_key(surface: &str, fact_class: &str) -> String {
    format!("{}::{}", surface.trim(), fact_class.trim())
}

fn normalize_inline_code(value: &str) -> String {
    value.trim().trim_matches('`').trim().to_string()
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

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult<T = ()> = Result<T>;

    #[test]
    fn parses_markdown_ledger_rows() -> TestResult {
        let table = parse_table(
            "\
| Surface | Fact class | Current state | Next proof | Promotion condition | Fallback condition | Blocker condition |
| --- | --- | --- | --- | --- | --- | --- |
| Completion | Source-backed receiver fact | `pilot` | next | promote | fallback | block |
",
            "Surface",
        )
        .ok_or_else(|| color_eyre::eyre::eyre!("expected provider ledger table"))?;

        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.rows[0].cells[0], "Completion");
        assert_eq!(normalize_inline_code(&table.rows[0].cells[2]), "pilot");
        Ok(())
    }

    #[test]
    fn rejects_policy_missing_required_blocker() -> TestResult {
        let policy = ProviderPromotionLedger {
            schema_version: 1,
            policy: POLICY_NAME.to_string(),
            human_ledger: HUMAN_LEDGER.to_string(),
            decision_states: REQUIRED_DECISIONS.iter().map(|value| (*value).to_string()).collect(),
            blocker_registry: REQUIRED_BLOCKERS[1..]
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
            provider_class: Vec::new(),
        };
        let mut violations = Vec::new();

        validate_policy_shape(&policy, &mut violations);

        assert!(
            violations.iter().any(|violation| violation.contains("generated_no_source")),
            "missing blocker should be reported: {violations:?}"
        );
        Ok(())
    }
}
