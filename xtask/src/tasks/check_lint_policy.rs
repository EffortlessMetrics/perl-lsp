use chrono::NaiveDate;
use color_eyre::eyre::{Result, bail};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use toml::Value;
use walkdir::WalkDir;

const ROOT_CARGO: &str = "Cargo.toml";
const CLIPPY_TOML: &str = "clippy.toml";
const LINT_POLICY: &str = "policy/clippy-lints.toml";
const LINT_DEBT: &str = "policy/clippy-debt.toml";
const NO_PANIC_ALLOWLIST: &str = "policy/no-panic-allowlist.toml";
const NON_RUST_ALLOWLIST: &str = "policy/non-rust-allowlist.toml";
const REQUIRED_POLICY_MSRV: &str = "1.93";
const TEST_CARVEOUTS: &[&str] = &[
    "allow-unwrap-in-tests",
    "allow-expect-in-tests",
    "allow-panic-in-tests",
    "allow-indexing-slicing-in-tests",
    "allow-dbg-in-tests",
];

#[derive(Debug, Deserialize)]
struct ClippyPolicy {
    schema: u64,
    msrv: String,
    policy: PolicyFlags,
    #[serde(default)]
    lint: Vec<LintEntry>,
    #[serde(default)]
    planned: Vec<PlannedLint>,
}

#[derive(Debug, Deserialize)]
struct PolicyFlags {
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
    class: String,
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
    lint: Option<String>,
    path: Option<String>,
    owner: Option<String>,
    reason: Option<String>,
    expires: Option<String>,
}

pub fn run() -> Result<()> {
    let root = Path::new(".");
    let mut violations = Vec::new();

    let cargo_text = read_to_string(ROOT_CARGO, &mut violations);
    let policy_text = read_to_string(LINT_POLICY, &mut violations);
    let debt_text = read_to_string(LINT_DEBT, &mut violations);

    if !Path::new(NO_PANIC_ALLOWLIST).exists() {
        violations.push(format!("missing required policy file `{NO_PANIC_ALLOWLIST}`"));
    }
    if !Path::new(NON_RUST_ALLOWLIST).exists() {
        violations.push(format!("missing required policy file `{NON_RUST_ALLOWLIST}`"));
    }

    let cargo_value = parse_toml_value(ROOT_CARGO, cargo_text.as_deref(), &mut violations);
    let policy = parse_toml::<ClippyPolicy>(LINT_POLICY, policy_text.as_deref(), &mut violations);
    let debt = parse_toml::<DebtLedger>(LINT_DEBT, debt_text.as_deref(), &mut violations);

    if let (Some(cargo), Some(policy)) = (cargo_value.as_ref(), policy.as_ref()) {
        check_msrv(cargo, policy, &mut violations);
        check_policy_flags(policy, &mut violations);
        check_active_lints(cargo, policy, &mut violations);
        check_planned_lints(cargo, policy, &mut violations);
        check_workspace_members_inherit_lints(root, cargo, &mut violations);
    }

    check_clippy_toml(&mut violations);

    if let Some(debt) = debt.as_ref() {
        check_debt_ledger(debt, &mut violations);
    }

    if violations.is_empty() {
        println!("lint policy OK");
        Ok(())
    } else {
        for violation in &violations {
            eprintln!("lint policy violation: {violation}");
        }
        bail!("lint policy check failed with {} violation(s)", violations.len())
    }
}

fn read_to_string(path: &str, violations: &mut Vec<String>) -> Option<String> {
    match fs::read_to_string(path) {
        Ok(text) => Some(text),
        Err(error) => {
            violations.push(format!("failed to read `{path}`: {error}"));
            None
        }
    }
}

fn parse_toml_value(path: &str, text: Option<&str>, violations: &mut Vec<String>) -> Option<Value> {
    let text = text?;
    match toml::from_str::<Value>(text) {
        Ok(value) => Some(value),
        Err(error) => {
            violations.push(format!("failed to parse `{path}` as TOML: {error}"));
            None
        }
    }
}

fn parse_toml<T>(path: &str, text: Option<&str>, violations: &mut Vec<String>) -> Option<T>
where
    T: for<'de> Deserialize<'de>,
{
    let text = text?;
    match toml::from_str::<T>(text) {
        Ok(value) => Some(value),
        Err(error) => {
            violations.push(format!("failed to parse `{path}` as TOML: {error}"));
            None
        }
    }
}

fn check_msrv(cargo: &Value, policy: &ClippyPolicy, violations: &mut Vec<String>) {
    let cargo_msrv = cargo
        .get("workspace")
        .and_then(Value::as_table)
        .and_then(|workspace| workspace.get("package"))
        .and_then(Value::as_table)
        .and_then(|package| package.get("rust-version"))
        .and_then(Value::as_str);

    if cargo_msrv != Some(policy.msrv.as_str()) {
        violations.push(format!(
            "workspace.package.rust-version must match {LINT_POLICY} msrv (Cargo.toml: {:?}, policy: {})",
            cargo_msrv, policy.msrv
        ));
    }

    if policy.msrv != REQUIRED_POLICY_MSRV {
        violations.push(format!(
            "{LINT_POLICY} msrv must stay on the shared platform MSRV {REQUIRED_POLICY_MSRV}"
        ));
    }
}

fn check_policy_flags(policy: &ClippyPolicy, violations: &mut Vec<String>) {
    if policy.schema != 1 {
        violations.push(format!("{LINT_POLICY} schema must be 1"));
    }
    if !policy.policy.panic_free_tests {
        violations.push("policy.panic_free_tests must be true".to_owned());
    }
    if policy.policy.allow_test_carveouts {
        violations.push("policy.allow_test_carveouts must be false".to_owned());
    }
    if policy.policy.suppression_style != "expect-with-reason" {
        violations.push("policy.suppression_style must be `expect-with-reason`".to_owned());
    }
    if policy.policy.blanket_categories {
        violations.push("policy.blanket_categories must be false".to_owned());
    }
}

fn check_active_lints(cargo: &Value, policy: &ClippyPolicy, violations: &mut Vec<String>) {
    let active_lints: Vec<&LintEntry> =
        policy.lint.iter().filter(|lint| lint.status == "active").collect();
    if active_lints.is_empty() {
        violations.push(format!("{LINT_POLICY} must declare active [[lint]] entries"));
    }

    let mut seen = BTreeSet::new();
    for lint in active_lints {
        if !seen.insert(lint.name.as_str()) {
            violations.push(format!("duplicate active lint `{}` in {LINT_POLICY}", lint.name));
        }
        if lint.class.trim().is_empty() {
            violations.push(format!("active lint `{}` is missing class", lint.name));
        }
        if lint.reason.trim().is_empty() {
            violations.push(format!("active lint `{}` is missing reason", lint.name));
        }
        match cargo_lint_level(cargo, &lint.name) {
            Some(level) if level == lint.level => {}
            Some(level) => violations.push(format!(
                "active lint `{}` has Cargo.toml level `{level}` but policy level `{}`",
                lint.name, lint.level
            )),
            None => violations.push(format!(
                "active lint `{}` is missing from root Cargo.toml workspace lints",
                lint.name
            )),
        }
    }
}

fn check_planned_lints(cargo: &Value, policy: &ClippyPolicy, violations: &mut Vec<String>) {
    let mut planned_versions = BTreeSet::new();
    for planned in &policy.planned {
        planned_versions.insert(planned.activate_when_msrv.as_str());
        if planned.class.trim().is_empty() {
            violations.push(format!("planned lint `{}` is missing class", planned.name));
        }
        if planned.reason.trim().is_empty() {
            violations.push(format!("planned lint `{}` is missing reason", planned.name));
        }
        if planned.level != "deny" && planned.level != "warn" {
            violations
                .push(format!("planned lint `{}` must have level `deny` or `warn`", planned.name));
        }
        if cargo_lint_level(cargo, &planned.name).is_some() {
            violations.push(format!(
                "planned lint `{}` must not be active before MSRV {}",
                planned.name, planned.activate_when_msrv
            ));
        }
    }
    for required in ["1.94", "1.95"] {
        if !planned_versions.contains(required) {
            violations.push(format!("{LINT_POLICY} must track planned Rust {required} lint flips"));
        }
    }
}

fn cargo_lint_level(cargo: &Value, lint_name: &str) -> Option<String> {
    let (family, name) = lint_name.split_once("::")?;
    let lints = cargo
        .get("workspace")
        .and_then(Value::as_table)
        .and_then(|workspace| workspace.get("lints"))
        .and_then(Value::as_table)
        .and_then(|lints| lints.get(family))
        .and_then(Value::as_table)?;
    let value = lints.get(name)?;
    if let Some(level) = value.as_str() {
        return Some(level.to_owned());
    }
    value.as_table().and_then(|table| table.get("level")).and_then(Value::as_str).map(str::to_owned)
}

fn check_workspace_members_inherit_lints(root: &Path, cargo: &Value, violations: &mut Vec<String>) {
    for manifest in workspace_member_manifests(root, cargo, violations) {
        let manifest_text = match fs::read_to_string(&manifest) {
            Ok(text) => text,
            Err(error) => {
                violations.push(format!("failed to read `{}`: {error}", manifest.display()));
                continue;
            }
        };
        let manifest_value = match toml::from_str::<Value>(&manifest_text) {
            Ok(value) => value,
            Err(error) => {
                violations.push(format!("failed to parse `{}`: {error}", manifest.display()));
                continue;
            }
        };
        let inherits = manifest_value
            .get("lints")
            .and_then(Value::as_table)
            .and_then(|lints| lints.get("workspace"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !inherits {
            violations.push(format!(
                "workspace member `{}` must include `[lints] workspace = true`",
                manifest.display()
            ));
        }
    }
}

fn workspace_member_manifests(
    root: &Path,
    cargo: &Value,
    violations: &mut Vec<String>,
) -> Vec<PathBuf> {
    let Some(members) = cargo
        .get("workspace")
        .and_then(Value::as_table)
        .and_then(|workspace| workspace.get("members"))
        .and_then(Value::as_array)
    else {
        violations.push("Cargo.toml missing workspace.members".to_owned());
        return Vec::new();
    };

    let mut manifests = Vec::new();
    for member in members {
        let Some(member) = member.as_str() else {
            violations.push("workspace.members contains a non-string entry".to_owned());
            continue;
        };
        if member.contains('*') {
            collect_globbed_member_manifests(root, member, violations, &mut manifests);
        } else {
            manifests.push(root.join(member).join("Cargo.toml"));
        }
    }
    manifests
}

fn collect_globbed_member_manifests(
    root: &Path,
    pattern: &str,
    violations: &mut Vec<String>,
    manifests: &mut Vec<PathBuf>,
) {
    let prefix = pattern.split('*').next().unwrap_or_default();
    let base = root.join(prefix);
    if !base.exists() {
        violations.push(format!("workspace member glob base `{}` does not exist", base.display()));
        return;
    }
    for entry in WalkDir::new(base).max_depth(3) {
        let Ok(entry) = entry else {
            continue;
        };
        if entry.file_name() == "Cargo.toml" {
            manifests.push(entry.into_path());
        }
    }
}

fn check_clippy_toml(violations: &mut Vec<String>) {
    let text = match fs::read_to_string(CLIPPY_TOML) {
        Ok(text) => text,
        Err(error) => {
            violations.push(format!("failed to read `{CLIPPY_TOML}`: {error}"));
            return;
        }
    };
    for carveout in TEST_CARVEOUTS {
        if text.contains(carveout) {
            violations.push(format!("{CLIPPY_TOML} must not configure test carveout `{carveout}`"));
        }
    }
    match toml::from_str::<Value>(&text) {
        Ok(value) => {
            let msrv = value.get("msrv").and_then(Value::as_str);
            if msrv != Some(REQUIRED_POLICY_MSRV) {
                violations.push(format!(
                    "{CLIPPY_TOML} msrv must be {REQUIRED_POLICY_MSRV} (found {msrv:?})"
                ));
            }
        }
        Err(error) => violations.push(format!("failed to parse `{CLIPPY_TOML}`: {error}")),
    }
}

fn check_debt_ledger(debt: &DebtLedger, violations: &mut Vec<String>) {
    if debt.schema != 1 {
        violations.push(format!("{LINT_DEBT} schema must be 1"));
    }
    let today = chrono::Utc::now().date_naive();
    for (index, entry) in debt.debt.iter().enumerate() {
        let number = index + 1;
        check_required_field(number, "lint", entry.lint.as_deref(), violations);
        check_required_field(number, "path", entry.path.as_deref(), violations);
        check_required_field(number, "owner", entry.owner.as_deref(), violations);
        check_required_field(number, "reason", entry.reason.as_deref(), violations);
        check_required_field(number, "expires", entry.expires.as_deref(), violations);
        if let Some(expires) = entry.expires.as_deref() {
            match NaiveDate::parse_from_str(expires, "%Y-%m-%d") {
                Ok(date) if date < today => {
                    violations.push(format!("debt entry #{number} expired on {expires}"))
                }
                Ok(_) => {}
                Err(error) => violations.push(format!(
                    "debt entry #{number} has invalid expires date `{expires}`: {error}"
                )),
            }
        }
    }
}

fn check_required_field(
    entry_number: usize,
    field: &str,
    value: Option<&str>,
    violations: &mut Vec<String>,
) {
    if value.is_none_or(|value| value.trim().is_empty()) {
        violations.push(format!("debt entry #{entry_number} is missing `{field}`"));
    }
}
