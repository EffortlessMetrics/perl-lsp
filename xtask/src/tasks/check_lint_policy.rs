//! Validate the workspace Clippy policy ledger and inheritance.

use chrono::{NaiveDate, Utc};
use color_eyre::eyre::{Result, bail, eyre};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use toml::Value;

const CARGO_TOML: &str = "Cargo.toml";
const CLIPPY_TOML: &str = "clippy.toml";
const CLIPPY_LINTS: &str = "policy/clippy-lints.toml";
const CLIPPY_DEBT: &str = "policy/clippy-debt.toml";
const FORBIDDEN_TEST_CARVEOUTS: &[&str] = &[
    "allow-unwrap-in-tests",
    "allow-expect-in-tests",
    "allow-panic-in-tests",
    "allow-indexing-slicing-in-tests",
    "allow-dbg-in-tests",
];

#[derive(Debug, Deserialize)]
struct PolicyLedger {
    schema: u64,
    msrv: String,
    policy: PolicyPosture,
    #[serde(default)]
    lint: Vec<LintEntry>,
    #[serde(default)]
    planned: Vec<PlannedLint>,
}

#[derive(Debug, Deserialize)]
struct PolicyPosture {
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
    let root = workspace_root()?;
    let cargo = parse_toml_file(&root.join(CARGO_TOML))?;
    let policy: PolicyLedger = parse_toml_file(&root.join(CLIPPY_LINTS))?;
    let debt: DebtLedger = parse_toml_file(&root.join(CLIPPY_DEBT))?;

    let mut errors = Vec::new();

    check_policy_shape(&policy, &mut errors);
    check_msrv(&cargo, &policy, &mut errors);
    check_workspace_lints(&cargo, &policy, &debt, &mut errors);
    check_workspace_inheritance(&root, &cargo, &mut errors);
    check_clippy_toml(&root.join(CLIPPY_TOML), &policy, &mut errors);
    check_debt(&debt, &mut errors);

    if errors.is_empty() {
        println!(
            "lint policy OK: {} active lints, {} planned flips, {} debt entries",
            policy.lint.len(),
            policy.planned.len(),
            debt.debt.len()
        );
        return Ok(());
    }

    for error in &errors {
        eprintln!("lint policy error: {error}");
    }
    bail!("lint policy check failed with {} error(s)", errors.len())
}

fn workspace_root() -> Result<PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        if dir.join(CARGO_TOML).is_file() && dir.join("xtask").is_dir() {
            return Ok(dir);
        }
        if !dir.pop() {
            bail!("could not locate workspace root containing Cargo.toml and xtask")
        }
    }
}

fn parse_toml_file<T>(path: &Path) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let contents = fs::read_to_string(path)
        .map_err(|source| eyre!("failed to read {}: {source}", path.display()))?;
    toml::from_str(&contents)
        .map_err(|source| eyre!("failed to parse {}: {source}", path.display()))
}

fn check_policy_shape(policy: &PolicyLedger, errors: &mut Vec<String>) {
    if policy.schema != 1 {
        errors.push(format!("{CLIPPY_LINTS} schema must be 1"));
    }
    if !policy.policy.panic_free_tests {
        errors.push("policy must require panic-free tests".to_string());
    }
    if policy.policy.allow_test_carveouts {
        errors.push("policy must not allow test carveouts".to_string());
    }
    if policy.policy.suppression_style != "expect-with-reason" {
        errors.push("policy suppression_style must be expect-with-reason".to_string());
    }
    if policy.policy.blanket_categories {
        errors.push("policy must not allow blanket lint categories".to_string());
    }

    let mut active_names = BTreeSet::new();
    for lint in &policy.lint {
        require_field(&lint.name, "active lint name", errors);
        require_field(&lint.level, "active lint level", errors);
        require_field(&lint.status, "active lint status", errors);
        require_field(&lint.class, "active lint class", errors);
        require_field(&lint.reason, "active lint reason", errors);
        if lint.status != "active" {
            errors.push(format!("{} must have status = \"active\"", lint.name));
        }
        if !active_names.insert(lint.name.clone()) {
            errors.push(format!("duplicate active lint {}", lint.name));
        }
    }

    let mut planned_names = BTreeSet::new();
    for planned in &policy.planned {
        require_field(&planned.name, "planned lint name", errors);
        require_field(&planned.level, "planned lint level", errors);
        require_field(&planned.activate_when_msrv, "planned lint activate_when_msrv", errors);
        require_field(&planned.reason, "planned lint reason", errors);
        if !planned_names.insert(planned.name.clone()) {
            errors.push(format!("duplicate planned lint {}", planned.name));
        }
    }
}

fn check_msrv(cargo: &Value, policy: &PolicyLedger, errors: &mut Vec<String>) {
    let workspace_msrv = cargo
        .get("workspace")
        .and_then(|workspace| workspace.get("package"))
        .and_then(|package| package.get("rust-version"))
        .and_then(Value::as_str);
    if workspace_msrv != Some(policy.msrv.as_str()) {
        errors.push(format!(
            "workspace.package.rust-version ({}) must match {CLIPPY_LINTS} msrv ({})",
            workspace_msrv.unwrap_or("missing"),
            policy.msrv
        ));
    }
}

fn check_workspace_lints(
    cargo: &Value,
    policy: &PolicyLedger,
    debt: &DebtLedger,
    errors: &mut Vec<String>,
) {
    let Some(workspace_lints) = cargo.get("workspace").and_then(|workspace| workspace.get("lints"))
    else {
        errors.push("root Cargo.toml must define [workspace.lints]".to_string());
        return;
    };

    let active = active_lints(workspace_lints);
    let debt_lints: BTreeSet<&str> = debt.debt.iter().map(|entry| entry.lint.as_str()).collect();
    for lint in &policy.lint {
        match active.get(lint.name.as_str()) {
            Some(level) if level == &lint.level => {}
            Some(level) => errors.push(format!(
                "{} level is {level} in Cargo.toml but {} in {CLIPPY_LINTS}",
                lint.name, lint.level
            )),
            None => errors.push(format!(
                "{} is active in {CLIPPY_LINTS} but missing from Cargo.toml",
                lint.name
            )),
        }
    }

    let policy_lints: BTreeSet<&str> = policy.lint.iter().map(|lint| lint.name.as_str()).collect();
    for lint in active.keys() {
        if !policy_lints.contains(lint.as_str()) && !debt_lints.contains(lint.as_str()) {
            errors.push(format!("{lint} is configured in Cargo.toml but missing from {CLIPPY_LINTS} or {CLIPPY_DEBT}"));
        }
    }

    for planned in &policy.planned {
        if msrv_less_than(&policy.msrv, &planned.activate_when_msrv)
            && active.contains_key(planned.name.as_str())
        {
            errors.push(format!(
                "{} is planned for MSRV {} but is already active at MSRV {}",
                planned.name, planned.activate_when_msrv, policy.msrv
            ));
        }
    }
}

fn active_lints(workspace_lints: &Value) -> BTreeMap<String, String> {
    let mut lints = BTreeMap::new();
    for namespace in ["rust", "clippy"] {
        let Some(table) = workspace_lints.get(namespace).and_then(Value::as_table) else {
            continue;
        };
        for (name, value) in table {
            if let Some(level) = lint_level(value) {
                lints.insert(format!("{namespace}::{name}"), level.to_string());
            }
        }
    }
    lints
}

fn lint_level(value: &Value) -> Option<&str> {
    value
        .as_str()
        .or_else(|| value.as_table().and_then(|table| table.get("level")).and_then(Value::as_str))
}

fn check_workspace_inheritance(root: &Path, cargo: &Value, errors: &mut Vec<String>) {
    let Some(members) = cargo
        .get("workspace")
        .and_then(|workspace| workspace.get("members"))
        .and_then(Value::as_array)
    else {
        errors.push("workspace members must be listed in Cargo.toml".to_string());
        return;
    };

    for member in members {
        let Some(member_path) = member.as_str() else {
            errors.push("workspace member entries must be strings".to_string());
            continue;
        };
        let manifest = root.join(member_path).join(CARGO_TOML);
        if !manifest.is_file() {
            errors.push(format!("workspace member {member_path} is missing Cargo.toml"));
            continue;
        }
        let Ok(member_toml): Result<Value> = parse_toml_file(&manifest) else {
            errors.push(format!("could not parse {}", manifest.display()));
            continue;
        };
        let inherits = member_toml
            .get("lints")
            .and_then(|lints| lints.get("workspace"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !inherits {
            errors.push(format!("{member_path}/Cargo.toml must contain [lints] workspace = true"));
        }
    }
}

fn check_clippy_toml(path: &Path, policy: &PolicyLedger, errors: &mut Vec<String>) {
    let Ok(contents) = fs::read_to_string(path) else {
        errors.push(format!("{} must exist", path.display()));
        return;
    };
    let parsed: Value = match toml::from_str(&contents) {
        Ok(value) => value,
        Err(source) => {
            errors.push(format!("failed to parse {}: {source}", path.display()));
            return;
        }
    };
    if parsed.get("msrv").and_then(Value::as_str) != Some(policy.msrv.as_str()) {
        errors.push(format!("clippy.toml msrv must match policy msrv {}", policy.msrv));
    }
    for key in FORBIDDEN_TEST_CARVEOUTS {
        if parsed.get(*key).is_some() {
            errors.push(format!("clippy.toml must not set forbidden test carveout {key}"));
        }
    }
}

fn check_debt(debt: &DebtLedger, errors: &mut Vec<String>) {
    if debt.schema != 1 {
        errors.push(format!("{CLIPPY_DEBT} schema must be 1"));
    }
    let today = Utc::now().date_naive();
    for entry in &debt.debt {
        require_field(&entry.lint, "debt lint", errors);
        require_field(&entry.path, "debt path", errors);
        require_field(&entry.owner, "debt owner", errors);
        require_field(&entry.reason, "debt reason", errors);
        require_field(&entry.expires, "debt expires", errors);
        match NaiveDate::parse_from_str(&entry.expires, "%Y-%m-%d") {
            Ok(expires) if expires >= today => {}
            Ok(expires) => errors.push(format!(
                "debt entry for {} at {} expired on {expires}",
                entry.lint, entry.path
            )),
            Err(source) => errors.push(format!(
                "debt entry for {} at {} has invalid expires date {}: {source}",
                entry.lint, entry.path, entry.expires
            )),
        }
    }
}

fn require_field(value: &str, field: &str, errors: &mut Vec<String>) {
    if value.trim().is_empty() {
        errors.push(format!("{field} must not be empty"));
    }
}

fn msrv_less_than(current: &str, threshold: &str) -> bool {
    version_tuple(current) < version_tuple(threshold)
}

fn version_tuple(version: &str) -> (u64, u64, u64) {
    let mut parts = version.split('.').map(|part| part.parse::<u64>().unwrap_or(0));
    let major = parts.next().unwrap_or(0);
    let minor = parts.next().unwrap_or(0);
    let patch = parts.next().unwrap_or(0);
    (major, minor, patch)
}
