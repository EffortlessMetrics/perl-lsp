//! Validate the workspace Clippy policy ledger against Cargo configuration.

use crate::utils::project_root;
use chrono::{NaiveDate, Utc};
use color_eyre::eyre::{Context, Result, bail, eyre};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use toml::Value;

const ROOT_CARGO_TOML: &str = "Cargo.toml";
const CLIPPY_TOML: &str = "clippy.toml";
const CLIPPY_LINTS_TOML: &str = "policy/clippy-lints.toml";
const CLIPPY_DEBT_TOML: &str = "policy/clippy-debt.toml";
const FORBIDDEN_TEST_CARVEOUTS: &[&str] = &[
    "allow-unwrap-in-tests",
    "allow-expect-in-tests",
    "allow-panic-in-tests",
    "allow-indexing-slicing-in-tests",
    "allow-dbg-in-tests",
];

#[derive(Debug, Deserialize)]
struct LintPolicyLedger {
    schema: u64,
    msrv: String,
    policy: LintPolicySettings,
    #[serde(default)]
    lint: Vec<LintEntry>,
}

#[derive(Debug, Deserialize)]
struct LintPolicySettings {
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
    #[serde(default)]
    activate_when_msrv: Option<String>,
    class: String,
    reason: String,
}

#[derive(Debug, Deserialize)]
struct ClippyDebtLedger {
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
    let root = project_root()?;
    let cargo = read_toml(root.join(ROOT_CARGO_TOML))?;
    let ledger: LintPolicyLedger = read_toml(root.join(CLIPPY_LINTS_TOML))?;
    let debt: ClippyDebtLedger = read_toml(root.join(CLIPPY_DEBT_TOML))?;

    check_policy_header(&ledger)?;
    check_msrv(&root, &cargo, &ledger)?;
    check_clippy_toml(&root, &ledger, &debt)?;
    check_workspace_lints(&cargo, &ledger)?;
    check_workspace_members_inherit_lints(&root, &cargo)?;
    check_debt_entries(&debt)?;

    println!("lint policy check passed");
    println!(
        "  active lints: {}",
        ledger.lint.iter().filter(|lint| lint.status == "active").count()
    );
    println!(
        "  planned lints: {}",
        ledger.lint.iter().filter(|lint| lint.status == "planned").count()
    );
    println!("  debt entries: {}", debt.debt.len());

    Ok(())
}

fn read_toml<T>(path: PathBuf) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let content =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    toml::from_str(&content).with_context(|| format!("failed to parse {}", path.display()))
}

fn check_policy_header(ledger: &LintPolicyLedger) -> Result<()> {
    if ledger.schema != 1 {
        bail!("{CLIPPY_LINTS_TOML} schema must be 1");
    }
    if !ledger.policy.panic_free_tests {
        bail!("{CLIPPY_LINTS_TOML} must require panic_free_tests = true");
    }
    if ledger.policy.allow_test_carveouts {
        bail!("{CLIPPY_LINTS_TOML} must forbid test carveouts");
    }
    if ledger.policy.suppression_style != "expect-with-reason" {
        bail!("{CLIPPY_LINTS_TOML} must use suppression_style = \"expect-with-reason\"");
    }
    if ledger.policy.blanket_categories {
        bail!("{CLIPPY_LINTS_TOML} must set blanket_categories = false");
    }
    Ok(())
}

fn check_msrv(root: &Path, cargo: &Value, ledger: &LintPolicyLedger) -> Result<()> {
    let workspace_msrv = cargo
        .get("workspace")
        .and_then(|workspace| workspace.get("package"))
        .and_then(|package| package.get("rust-version"))
        .and_then(Value::as_str)
        .ok_or_else(|| eyre!("workspace.package.rust-version is missing"))?;
    if workspace_msrv != ledger.msrv {
        bail!(
            "workspace.package.rust-version ({workspace_msrv}) must match {CLIPPY_LINTS_TOML} msrv ({})",
            ledger.msrv
        );
    }

    let toolchain_path = root.join("rust-toolchain.toml");
    if toolchain_path.exists() {
        let toolchain: Value = read_toml(toolchain_path)?;
        let channel = toolchain
            .get("toolchain")
            .and_then(|toolchain| toolchain.get("channel"))
            .and_then(Value::as_str)
            .ok_or_else(|| eyre!("rust-toolchain.toml toolchain.channel is missing"))?;
        let channel_msrv = channel.strip_suffix(".0").unwrap_or(channel);
        if channel_msrv != workspace_msrv {
            bail!(
                "rust-toolchain.toml channel ({channel}) must match workspace.package.rust-version ({workspace_msrv})"
            );
        }
    }

    Ok(())
}

fn check_clippy_toml(
    root: &Path,
    ledger: &LintPolicyLedger,
    debt: &ClippyDebtLedger,
) -> Result<()> {
    let path = root.join(CLIPPY_TOML);
    let content =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let clippy: Value = toml::from_str(&content).context("failed to parse clippy.toml")?;

    let msrv = clippy
        .get("msrv")
        .and_then(Value::as_str)
        .ok_or_else(|| eyre!("clippy.toml msrv is missing"))?;
    if msrv != ledger.msrv {
        bail!("clippy.toml msrv ({msrv}) must match {CLIPPY_LINTS_TOML} msrv ({})", ledger.msrv);
    }

    for carveout in FORBIDDEN_TEST_CARVEOUTS {
        if clippy.get(*carveout).is_some()
            || content.lines().any(|line| line.trim_start().starts_with(carveout))
        {
            let has_debt = debt.debt.iter().any(|entry| entry.path == CLIPPY_TOML);
            if !has_debt {
                bail!(
                    "clippy.toml test carveout `{carveout}` requires explicit {CLIPPY_DEBT_TOML} debt"
                );
            }
        }
    }

    Ok(())
}

fn check_workspace_lints(cargo: &Value, ledger: &LintPolicyLedger) -> Result<()> {
    let workspace_lints = cargo
        .get("workspace")
        .and_then(|workspace| workspace.get("lints"))
        .ok_or_else(|| eyre!("[workspace.lints] is missing"))?;
    let rust_lints = lint_table(workspace_lints, "rust")?;
    let clippy_lints = lint_table(workspace_lints, "clippy")?;

    let mut active_names = BTreeSet::new();
    for lint in &ledger.lint {
        validate_lint_entry(lint)?;
        match lint.status.as_str() {
            "active" => {
                active_names.insert(lint.name.clone());
                check_active_lint(lint, rust_lints, clippy_lints)?;
            }
            "planned" => check_planned_lint(lint, rust_lints, clippy_lints, &ledger.msrv)?,
            other => bail!("lint {} has unsupported status `{other}`", lint.name),
        }
    }

    for name in rust_lints.keys() {
        let full_name = format!("rust::{name}");
        if !active_names.contains(&full_name) {
            bail!("active rust lint {full_name} is missing from {CLIPPY_LINTS_TOML}");
        }
    }
    for name in clippy_lints.keys() {
        let full_name = format!("clippy::{name}");
        if !active_names.contains(&full_name) {
            bail!("active clippy lint {full_name} is missing from {CLIPPY_LINTS_TOML}");
        }
    }

    Ok(())
}

fn lint_table<'a>(
    workspace_lints: &'a Value,
    name: &str,
) -> Result<&'a toml::map::Map<String, Value>> {
    workspace_lints
        .get(name)
        .and_then(Value::as_table)
        .ok_or_else(|| eyre!("[workspace.lints.{name}] is missing"))
}

fn validate_lint_entry(lint: &LintEntry) -> Result<()> {
    if lint.name.trim().is_empty()
        || lint.level.trim().is_empty()
        || lint.class.trim().is_empty()
        || lint.reason.trim().is_empty()
    {
        bail!("lint entries require name, level, class, and reason");
    }
    if !matches!(lint.level.as_str(), "allow" | "warn" | "deny" | "forbid") {
        bail!("lint {} has unsupported level `{}`", lint.name, lint.level);
    }
    Ok(())
}

fn check_active_lint(
    lint: &LintEntry,
    rust_lints: &toml::map::Map<String, Value>,
    clippy_lints: &toml::map::Map<String, Value>,
) -> Result<()> {
    let (family, name) = split_lint_name(&lint.name)?;
    let table = match family {
        "rust" => rust_lints,
        "clippy" => clippy_lints,
        _ => bail!("active lint {} must start with rust:: or clippy::", lint.name),
    };
    let actual = table
        .get(name)
        .ok_or_else(|| eyre!("active lint {} is missing from Cargo.toml", lint.name))?;
    let level = lint_level(actual)
        .ok_or_else(|| eyre!("could not read Cargo.toml level for {}", lint.name))?;
    if level != lint.level {
        bail!(
            "active lint {} level is `{level}` in Cargo.toml but `{}` in {CLIPPY_LINTS_TOML}",
            lint.name,
            lint.level
        );
    }
    Ok(())
}

fn check_planned_lint(
    lint: &LintEntry,
    rust_lints: &toml::map::Map<String, Value>,
    clippy_lints: &toml::map::Map<String, Value>,
    msrv: &str,
) -> Result<()> {
    let activate_when_msrv = lint
        .activate_when_msrv
        .as_deref()
        .ok_or_else(|| eyre!("planned lint {} is missing activate_when_msrv", lint.name))?;
    let (family, name) = split_lint_name(&lint.name)?;
    if version_at_least(msrv, activate_when_msrv)? {
        bail!(
            "planned lint {} must be promoted because MSRV {msrv} >= {activate_when_msrv}",
            lint.name
        );
    }
    let is_active = match family {
        "rust" => rust_lints.contains_key(name),
        "clippy" => clippy_lints.contains_key(name),
        _ => bail!("planned lint {} must start with rust:: or clippy::", lint.name),
    };
    if is_active {
        bail!("planned lint {} is already active before MSRV {activate_when_msrv}", lint.name);
    }
    Ok(())
}

fn split_lint_name(name: &str) -> Result<(&str, &str)> {
    name.split_once("::").ok_or_else(|| eyre!("lint name `{name}` must include a family prefix"))
}

fn lint_level(value: &Value) -> Option<String> {
    value.as_str().map(ToOwned::to_owned).or_else(|| {
        value
            .as_table()
            .and_then(|table| table.get("level"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
    })
}

fn version_at_least(current: &str, required: &str) -> Result<bool> {
    let current_parts = parse_minor_version(current)?;
    let required_parts = parse_minor_version(required)?;
    Ok(current_parts >= required_parts)
}

fn parse_minor_version(version: &str) -> Result<(u64, u64)> {
    let mut parts = version.split('.');
    let major = parts
        .next()
        .ok_or_else(|| eyre!("version `{version}` is missing a major component"))?
        .parse::<u64>()
        .with_context(|| format!("version `{version}` has an invalid major component"))?;
    let minor = parts
        .next()
        .ok_or_else(|| eyre!("version `{version}` is missing a minor component"))?
        .parse::<u64>()
        .with_context(|| format!("version `{version}` has an invalid minor component"))?;
    Ok((major, minor))
}

fn check_workspace_members_inherit_lints(root: &Path, cargo: &Value) -> Result<()> {
    let members = cargo
        .get("workspace")
        .and_then(|workspace| workspace.get("members"))
        .and_then(Value::as_array)
        .ok_or_else(|| eyre!("workspace.members is missing"))?;

    let mut missing = Vec::new();
    for member in members {
        let member_path =
            member.as_str().ok_or_else(|| eyre!("workspace member entries must be strings"))?;
        if member_path.contains('*') {
            continue;
        }
        let manifest_path = root.join(member_path).join("Cargo.toml");
        let member_manifest: Value = read_toml(manifest_path.clone())?;
        let inherits = member_manifest
            .get("lints")
            .and_then(|lints| lints.get("workspace"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !inherits {
            missing.push(manifest_path.display().to_string());
        }
    }

    if !missing.is_empty() {
        bail!("workspace members must inherit workspace lints:\n{}", missing.join("\n"));
    }

    Ok(())
}

fn check_debt_entries(debt: &ClippyDebtLedger) -> Result<()> {
    if debt.schema != 1 {
        bail!("{CLIPPY_DEBT_TOML} schema must be 1");
    }
    let today = Utc::now().date_naive();
    let mut seen = BTreeSet::new();
    let mut by_lint = BTreeMap::<&str, usize>::new();
    for entry in &debt.debt {
        if entry.lint.trim().is_empty()
            || entry.path.trim().is_empty()
            || entry.owner.trim().is_empty()
            || entry.reason.trim().is_empty()
            || entry.expires.trim().is_empty()
        {
            bail!(
                "every {CLIPPY_DEBT_TOML} entry must include lint, path, owner, reason, and expires"
            );
        }
        if !entry.lint.starts_with("clippy::") && !entry.lint.starts_with("rust::") {
            bail!("debt lint `{}` must start with clippy:: or rust::", entry.lint);
        }
        let expires = NaiveDate::parse_from_str(&entry.expires, "%Y-%m-%d").with_context(|| {
            format!("debt entry for {} has invalid expires date `{}`", entry.path, entry.expires)
        })?;
        if expires < today {
            bail!("debt entry for {} expired on {}", entry.path, entry.expires);
        }
        let key = (&entry.lint, &entry.path);
        if !seen.insert(key) {
            bail!("duplicate debt entry for {} at {}", entry.lint, entry.path);
        }
        *by_lint.entry(entry.lint.as_str()).or_default() += 1;
    }
    Ok(())
}
