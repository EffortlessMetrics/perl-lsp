use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::utils::project_root;

const COMMON_REQUIRED_FIELDS: [&str; 4] = ["check", "schema_version", "event", "verdict"];
const VERDICTS: [&str; 4] = ["pass", "fail", "warn", "skipped"];
const EVENTS: [&str; 4] = ["pull_request", "merge_group", "push", "local"];
const CLASSIFICATIONS: [&str; 6] =
    ["code_regression", "infra_failure", "stale_base", "master_red", "skipped", "unknown"];

#[derive(Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum ReceiptOutputFormat {
    Human,
    Json,
}

#[derive(Debug, Deserialize)]
pub struct ReceiptRegistry {
    pub registry: RegistryMetadata,
    #[serde(default)]
    pub schema: Vec<SchemaEntry>,
}

#[derive(Debug, Deserialize)]
pub struct RegistryMetadata {
    pub version: String,
    pub default_schema_version: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SchemaEntry {
    pub check: String,
    pub schema: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub extra_required: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ValidationReport {
    path: String,
    check: Option<String>,
    schema: Option<String>,
    valid: bool,
    errors: Vec<String>,
}

pub fn list(format: ReceiptOutputFormat) -> Result<()> {
    let registry = load_registry()?;
    match format {
        ReceiptOutputFormat::Human => {
            println!(
                "Gate receipt registry v{} (default schema version: {})",
                registry.registry.version, registry.registry.default_schema_version
            );
            for entry in registry.schema {
                println!("- {} => {}", entry.check, entry.schema);
            }
        }
        ReceiptOutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&registry.schema)?);
        }
    }
    Ok(())
}

pub fn validate(path: PathBuf, format: ReceiptOutputFormat) -> Result<()> {
    let registry = load_registry()?;
    let report = validate_path(&registry, &path)?;
    print_report(&report, format)?;
    if !report.valid {
        bail!("receipt validation failed for {}", path.display());
    }
    Ok(())
}

pub fn validate_all(dir: PathBuf, format: ReceiptOutputFormat) -> Result<()> {
    let registry = load_registry()?;
    let mut reports = Vec::new();

    for entry in WalkDir::new(&dir).into_iter().filter_map(|entry| entry.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        reports.push(validate_path(&registry, path)?);
    }

    match format {
        ReceiptOutputFormat::Human => {
            for report in &reports {
                if report.valid {
                    println!("OK: {}", report.path);
                } else {
                    println!("FAIL: {}", report.path);
                    for error in &report.errors {
                        println!("  - {error}");
                    }
                }
            }
        }
        ReceiptOutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&reports)?);
        }
    }

    let failed = reports.iter().filter(|report| !report.valid).count();
    if failed > 0 {
        bail!("{failed} receipt(s) failed validation");
    }

    Ok(())
}

fn load_registry() -> Result<ReceiptRegistry> {
    let root = project_root()?;
    let path = root.join(".ci/receipts/registry.toml");
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read registry file: {}", path.display()))?;
    toml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))
}

fn validate_path(registry: &ReceiptRegistry, path: &Path) -> Result<ValidationReport> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read receipt file: {}", path.display()))?;
    let value: Value = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse receipt JSON: {}", path.display()))?;

    let mut errors = Vec::new();

    for field in COMMON_REQUIRED_FIELDS {
        if value.get(field).is_none() {
            errors.push(format!("missing required field `{field}`"));
        }
    }

    let check = value.get("check").and_then(Value::as_str).map(str::to_owned);

    let schema_entry = if let Some(check_value) = &check {
        registry.schema.iter().find(|entry| entry.check == *check_value).map(|entry| {
            for field in &entry.extra_required {
                if value.get(field).is_none() {
                    errors.push(format!(
                        "missing required field `{field}` for check `{}`",
                        entry.check
                    ));
                }
            }
            entry
        })
    } else {
        None
    };

    if check.is_some() && schema_entry.is_none() {
        errors.push("check is not registered in .ci/receipts/registry.toml".to_string());
    }

    validate_enum_field(&value, "verdict", &VERDICTS, &mut errors);
    validate_enum_field(&value, "event", &EVENTS, &mut errors);
    validate_enum_field(&value, "classification", &CLASSIFICATIONS, &mut errors);

    Ok(ValidationReport {
        path: path.display().to_string(),
        check,
        schema: schema_entry.map(|entry| entry.schema.clone()),
        valid: errors.is_empty(),
        errors,
    })
}

fn validate_enum_field(
    value: &Value,
    field: &str,
    allowed_values: &[&str],
    errors: &mut Vec<String>,
) {
    if let Some(field_value) = value.get(field) {
        match field_value.as_str() {
            Some(raw) if allowed_values.contains(&raw) => {}
            Some(raw) => errors.push(format!(
                "field `{field}` has unsupported value `{raw}`; allowed: {}",
                allowed_values.join(", ")
            )),
            None => errors.push(format!("field `{field}` must be a string")),
        }
    }
}

fn print_report(report: &ValidationReport, format: ReceiptOutputFormat) -> Result<()> {
    match format {
        ReceiptOutputFormat::Human => {
            if report.valid {
                println!("OK: {}", report.path);
            } else {
                println!("FAIL: {}", report.path);
                for error in &report.errors {
                    println!("  - {error}");
                }
            }
        }
        ReceiptOutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(report)?);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validates_common_fields_and_enums() {
        let receipt = json!({
            "check": "methodology-gate",
            "schema_version": "1.0.0",
            "event": "pull_request",
            "verdict": "pass",
            "classification": "unknown"
        });

        let mut errors = Vec::new();
        validate_enum_field(&receipt, "verdict", &VERDICTS, &mut errors);
        validate_enum_field(&receipt, "event", &EVENTS, &mut errors);
        validate_enum_field(&receipt, "classification", &CLASSIFICATIONS, &mut errors);

        assert!(errors.is_empty());
    }

    #[test]
    fn rejects_invalid_enum_values() {
        let receipt = json!({
            "verdict": "green",
            "event": "cron",
            "classification": "weird"
        });

        let mut errors = Vec::new();
        validate_enum_field(&receipt, "verdict", &VERDICTS, &mut errors);
        validate_enum_field(&receipt, "event", &EVENTS, &mut errors);
        validate_enum_field(&receipt, "classification", &CLASSIFICATIONS, &mut errors);

        assert_eq!(errors.len(), 3);
    }

    #[test]
    fn parses_registry_schema_entries() -> Result<()> {
        let raw = r#"
[registry]
version = "1"
default_schema_version = "1.0.0"

[[schema]]
check = "fmt"
schema = ".ci/receipts/schemas/fmt.schema.json"
description = "fmt result"
extra_required = ["violations"]
"#;

        let registry: ReceiptRegistry = toml::from_str(raw)?;
        assert_eq!(registry.schema.len(), 1);
        assert_eq!(registry.schema[0].check, "fmt");
        Ok(())
    }
}
