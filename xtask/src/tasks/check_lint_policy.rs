//! Clippy lint policy coherence checks.

use chrono::{Local, NaiveDate};
use color_eyre::eyre::{Result, bail, eyre};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use toml::Value;

const ROOT_MANIFEST: &str = "Cargo.toml";
const CLIPPY_CONFIG: &str = "clippy.toml";
const LINT_LEDGER: &str = "policy/clippy-lints.toml";
const DEBT_LEDGER: &str = "policy/clippy-debt.toml";

const TEST_CARVEOUTS: &[&str] = &[
    "allow-unwrap-in-tests",
    "allow-expect-in-tests",
    "allow-panic-in-tests",
    "allow-indexing-slicing-in-tests",
    "allow-dbg-in-tests",
];

const REQUIRED_PLANNED: &[&str] = &[
    "rust::const_item_interior_mutations",
    "rust::function_casts_as_integer",
    "clippy::same_length_and_capacity",
    "clippy::manual_ilog2",
    "clippy::disallowed_fields",
    "clippy::manual_checked_ops",
    "clippy::manual_take",
    "clippy::manual_pop_if",
];

#[derive(Debug, Deserialize)]
struct LintLedger {
    schema: u64,
    msrv: String,
    policy: LintPolicy,
    #[serde(default)]
    lint: Vec<LintEntry>,
    #[serde(default)]
    planned: Vec<PlannedLint>,
}

#[derive(Debug, Deserialize)]
struct LintPolicy {
    panic_free_tests: bool,
    allow_test_carveouts: bool,
    suppression_style: String,
    blanket_categories: bool,
}

#[derive(Debug, Deserialize)]
struct LintEntry {
    name: String,
    level: String,
    status: String,
    class: String,
    reason: String,
}

#[derive(Debug, Deserialize)]
struct PlannedLint {
    name: String,
    level: String,
    activate_when_msrv: String,
    reason: String,
}

#[derive(Debug, Deserialize)]
struct DebtLedger {
    schema: u64,
    #[serde(default)]
    debt: Vec<DebtEntry>,
}

#[derive(Debug, Deserialize)]
struct DebtEntry {
    lint: String,
    path: String,
    owner: String,
    reason: String,
    expires: String,
}

pub fn run() -> Result<()> {
    let root = Path::new(".");
    let cargo = read_toml(root.join(ROOT_MANIFEST))?;
    let lint_ledger: LintLedger = read_toml_as(root.join(LINT_LEDGER))?;
    let debt_ledger: DebtLedger = read_toml_as(root.join(DEBT_LEDGER))?;

    validate_policy_header(&lint_ledger)?;
    validate_msrv(&cargo, &lint_ledger)?;
    validate_workspace_lints(&cargo, &lint_ledger)?;
    validate_workspace_members_inherit_lints(root, &cargo)?;
    validate_clippy_config(root.join(CLIPPY_CONFIG), &lint_ledger)?;
    validate_planned_lints(&cargo, &lint_ledger)?;
    validate_debt_ledger(&debt_ledger)?;

    println!(
        "lint policy ok: {} lint entries, {} planned flips, {} debt entries",
        lint_ledger.lint.len(),
        lint_ledger.planned.len(),
        debt_ledger.debt.len()
    );
    Ok(())
}

fn read_toml(path: PathBuf) -> Result<Value> {
    let content = fs::read_to_string(&path)
        .map_err(|err| eyre!("failed to read {}: {err}", path.display()))?;
    toml::from_str(&content).map_err(|err| eyre!("failed to parse {}: {err}", path.display()))
}

fn read_toml_as<T>(path: PathBuf) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let content = fs::read_to_string(&path)
        .map_err(|err| eyre!("failed to read {}: {err}", path.display()))?;
    toml::from_str(&content).map_err(|err| eyre!("failed to parse {}: {err}", path.display()))
}

fn validate_policy_header(ledger: &LintLedger) -> Result<()> {
    if ledger.schema != 1 {
        bail!("{} schema must be 1", LINT_LEDGER);
    }
    if !ledger.policy.panic_free_tests {
        bail!("{} must set policy.panic_free_tests = true", LINT_LEDGER);
    }
    if ledger.policy.suppression_style != "expect-with-reason" {
        bail!("{} must set policy.suppression_style = \"expect-with-reason\"", LINT_LEDGER);
    }
    if ledger.policy.blanket_categories {
        bail!("{} must set policy.blanket_categories = false", LINT_LEDGER);
    }
    Ok(())
}

fn validate_msrv(cargo: &Value, ledger: &LintLedger) -> Result<()> {
    let msrv = value_at(cargo, &["workspace", "package", "rust-version"])?
        .as_str()
        .ok_or_else(|| eyre!("workspace.package.rust-version must be a string"))?;
    if msrv != ledger.msrv {
        bail!(
            "workspace.package.rust-version ({msrv}) must match {LINT_LEDGER} msrv ({})",
            ledger.msrv
        );
    }
    Ok(())
}

fn validate_workspace_lints(cargo: &Value, ledger: &LintLedger) -> Result<()> {
    let cargo_lints = collect_workspace_lints(cargo)?;
    let mut seen = BTreeSet::new();
    for lint in &ledger.lint {
        validate_lint_entry(lint)?;
        if !seen.insert(lint.name.clone()) {
            bail!("duplicate lint ledger entry for {}", lint.name);
        }
        match lint.status.as_str() {
            "active" | "debt" => {
                let cargo_level = cargo_lints
                    .get(&lint.name)
                    .ok_or_else(|| eyre!("active lint {} is missing from Cargo.toml", lint.name))?;
                if cargo_level != &lint.level {
                    bail!(
                        "lint {} level mismatch: Cargo.toml has {cargo_level}, ledger has {}",
                        lint.name,
                        lint.level
                    );
                }
            }
            "tracked" => {
                if cargo_lints.contains_key(&lint.name) {
                    bail!(
                        "tracked lint {} is already active in Cargo.toml; mark it active or debt",
                        lint.name
                    );
                }
            }
            _ => bail!("lint {} has unsupported status {}", lint.name, lint.status),
        }
    }
    Ok(())
}

fn validate_lint_entry(lint: &LintEntry) -> Result<()> {
    if lint.name.trim().is_empty() {
        bail!("lint entry name cannot be empty");
    }
    if !matches!(lint.level.as_str(), "allow" | "warn" | "deny" | "forbid") {
        bail!("lint {} has unsupported level {}", lint.name, lint.level);
    }
    if !matches!(lint.status.as_str(), "active" | "debt" | "tracked") {
        bail!("lint {} must have status active, debt, or tracked", lint.name);
    }
    if lint.class.trim().is_empty() {
        bail!("lint {} must have a class", lint.name);
    }
    if lint.reason.trim().is_empty() {
        bail!("lint {} must have a reason", lint.name);
    }
    Ok(())
}

fn collect_workspace_lints(cargo: &Value) -> Result<BTreeMap<String, String>> {
    let mut output = BTreeMap::new();
    let workspace_lints = value_at(cargo, &["workspace", "lints"])?
        .as_table()
        .ok_or_else(|| eyre!("workspace.lints must be a table"))?;
    for (tool, table) in workspace_lints {
        let lint_table =
            table.as_table().ok_or_else(|| eyre!("workspace.lints.{tool} must be a table"))?;
        for (name, value) in lint_table {
            let level = lint_level(value).ok_or_else(|| {
                eyre!("workspace.lints.{tool}.{name} must be a string or table with level")
            })?;
            output.insert(format!("{tool}::{}", name.replace('-', "_")), level);
        }
    }
    Ok(output)
}

fn lint_level(value: &Value) -> Option<String> {
    if let Some(level) = value.as_str() {
        return Some(level.to_owned());
    }
    value
        .as_table()
        .and_then(|table| table.get("level"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn validate_workspace_members_inherit_lints(root: &Path, cargo: &Value) -> Result<()> {
    let members = value_at(cargo, &["workspace", "members"])?
        .as_array()
        .ok_or_else(|| eyre!("workspace.members must be an array"))?;
    let mut missing = Vec::new();
    for member in members {
        let Some(member_path) = member.as_str() else {
            bail!("workspace.members must contain only strings");
        };
        let manifest_path = root.join(member_path).join("Cargo.toml");
        let manifest = read_toml(manifest_path.clone())?;
        let inherits = value_at(&manifest, &["lints", "workspace"])
            .and_then(|value| {
                value.as_bool().ok_or_else(|| eyre!("[lints].workspace must be boolean"))
            })
            .unwrap_or(false);
        if !inherits {
            missing.push(manifest_path.display().to_string());
        }
    }
    if !missing.is_empty() {
        bail!("workspace members missing [lints] workspace = true: {}", missing.join(", "));
    }
    Ok(())
}

fn validate_clippy_config(path: PathBuf, ledger: &LintLedger) -> Result<()> {
    let content = fs::read_to_string(&path)
        .map_err(|err| eyre!("failed to read {}: {err}", path.display()))?;
    if !ledger.policy.allow_test_carveouts {
        for carveout in TEST_CARVEOUTS {
            if !content.contains(carveout) {
                continue;
            }
            bail!("{} must not contain test carveout {carveout}", path.display());
        }
    }
    let config: Value = toml::from_str(&content)
        .map_err(|err| eyre!("failed to parse {}: {err}", path.display()))?;
    if let Some(msrv) = config.get("msrv").and_then(Value::as_str) {
        if msrv != ledger.msrv {
            bail!(
                "{} msrv ({msrv}) must match {LINT_LEDGER} msrv ({})",
                path.display(),
                ledger.msrv
            );
        }
    }
    Ok(())
}

fn validate_planned_lints(cargo: &Value, ledger: &LintLedger) -> Result<()> {
    let cargo_lints = collect_workspace_lints(cargo)?;
    let planned_names: BTreeSet<_> = ledger.planned.iter().map(|lint| lint.name.as_str()).collect();
    for required in REQUIRED_PLANNED {
        if !planned_names.contains(required) {
            bail!("{} is missing planned lint {required}", LINT_LEDGER);
        }
    }
    for planned in &ledger.planned {
        validate_planned_lint(planned)?;
        if compare_versions(&planned.activate_when_msrv, &ledger.msrv)? > 0
            && cargo_lints.contains_key(&planned.name)
        {
            bail!(
                "planned lint {} must not be active before MSRV {}",
                planned.name,
                planned.activate_when_msrv
            );
        }
    }
    Ok(())
}

fn validate_planned_lint(planned: &PlannedLint) -> Result<()> {
    if planned.name.trim().is_empty() {
        bail!("planned lint name cannot be empty");
    }
    if !matches!(planned.level.as_str(), "warn" | "deny" | "forbid") {
        bail!("planned lint {} has unsupported level {}", planned.name, planned.level);
    }
    if planned.activate_when_msrv.trim().is_empty() {
        bail!("planned lint {} must have activate_when_msrv", planned.name);
    }
    if planned.reason.trim().is_empty() {
        bail!("planned lint {} must have a reason", planned.name);
    }
    Ok(())
}

fn validate_debt_ledger(ledger: &DebtLedger) -> Result<()> {
    if ledger.schema != 1 {
        bail!("{} schema must be 1", DEBT_LEDGER);
    }
    let today = Local::now().date_naive();
    for entry in &ledger.debt {
        validate_debt_entry(entry, today)?;
    }
    Ok(())
}

fn validate_debt_entry(entry: &DebtEntry, today: NaiveDate) -> Result<()> {
    if entry.lint.trim().is_empty() {
        bail!("debt entry lint cannot be empty");
    }
    if entry.path.trim().is_empty() {
        bail!("debt entry for {} must have a path", entry.lint);
    }
    if entry.owner.trim().is_empty() {
        bail!("debt entry for {} must have an owner", entry.lint);
    }
    if entry.reason.trim().is_empty() {
        bail!("debt entry for {} must have a reason", entry.lint);
    }
    let expires = NaiveDate::parse_from_str(&entry.expires, "%Y-%m-%d")
        .map_err(|err| eyre!("debt entry for {} has invalid expires date: {err}", entry.lint))?;
    if expires < today {
        bail!("debt entry for {} expired on {expires}", entry.lint);
    }
    Ok(())
}

fn value_at<'a>(value: &'a Value, path: &[&str]) -> Result<&'a Value> {
    let mut current = value;
    for segment in path {
        current =
            current.get(*segment).ok_or_else(|| eyre!("missing TOML key {}", path.join(".")))?;
    }
    Ok(current)
}

fn compare_versions(left: &str, right: &str) -> Result<i8> {
    let left_parts = version_parts(left)?;
    let right_parts = version_parts(right)?;
    for (left_part, right_part) in left_parts.iter().zip(right_parts.iter()) {
        if left_part > right_part {
            return Ok(1);
        }
        if left_part < right_part {
            return Ok(-1);
        }
    }
    Ok(0)
}

fn version_parts(version: &str) -> Result<Vec<u64>> {
    let mut parts = Vec::new();
    for part in version.split('.') {
        let parsed = part
            .parse::<u64>()
            .map_err(|err| eyre!("invalid version component {part} in {version}: {err}"))?;
        parts.push(parsed);
    }
    while parts.len() < 3 {
        parts.push(0);
    }
    Ok(parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lint_entry(name: &str, status: &str) -> LintEntry {
        LintEntry {
            name: name.to_owned(),
            level: "deny".to_owned(),
            status: status.to_owned(),
            class: "test".to_owned(),
            reason: "test reason".to_owned(),
        }
    }

    fn ledger_with(lint: LintEntry) -> LintLedger {
        LintLedger {
            schema: 1,
            msrv: "1.92".to_owned(),
            policy: LintPolicy {
                panic_free_tests: true,
                allow_test_carveouts: true,
                suppression_style: "expect-with-reason".to_owned(),
                blanket_categories: false,
            },
            lint: vec![lint],
            planned: Vec::new(),
        }
    }

    #[test]
    fn lint_entry_accepts_tracked_status() -> Result<()> {
        validate_lint_entry(&lint_entry("clippy::indexing_slicing", "tracked"))
    }

    #[test]
    fn lint_entry_rejects_unknown_status() -> Result<()> {
        let result = validate_lint_entry(&lint_entry("clippy::indexing_slicing", "candidate"));
        let Err(error) = result else {
            bail!("candidate status should be rejected");
        };
        assert!(error.to_string().contains("active, debt, or tracked"));
        Ok(())
    }

    #[test]
    fn tracked_lints_do_not_need_cargo_entries() -> Result<()> {
        let cargo = toml::from_str::<Value>(
            r#"
            [workspace.lints.clippy]
            panic = "deny"
            "#,
        )?;
        let ledger = ledger_with(lint_entry("clippy::indexing_slicing", "tracked"));

        validate_workspace_lints(&cargo, &ledger)
    }

    #[test]
    fn tracked_lints_fail_when_already_active() -> Result<()> {
        let cargo = toml::from_str::<Value>(
            r#"
            [workspace.lints.clippy]
            indexing_slicing = "deny"
            "#,
        )?;
        let ledger = ledger_with(lint_entry("clippy::indexing_slicing", "tracked"));

        let result = validate_workspace_lints(&cargo, &ledger);
        let Err(error) = result else {
            bail!("tracked active lint should fail");
        };
        assert!(error.to_string().contains("mark it active or debt"));
        Ok(())
    }
}
