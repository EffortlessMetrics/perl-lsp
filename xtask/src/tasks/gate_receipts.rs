use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum GateReceiptsFormat {
    Human,
    Json,
}

#[derive(Debug, Deserialize)]
struct RegistryCommon {
    required: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RegistrySchema {
    name: String,
    check: String,
    path: String,
}

#[derive(Debug, Deserialize)]
struct Registry {
    common: RegistryCommon,
    schemas: Vec<RegistrySchema>,
}

#[derive(Debug, Serialize)]
struct ValidationIssue {
    path: String,
    code: String,
    message: String,
}

#[derive(Debug, Serialize)]
struct ValidationResult {
    receipt_path: String,
    valid: bool,
    schema: Option<String>,
    issues: Vec<ValidationIssue>,
}

const REGISTRY_PATH: &str = ".ci/receipts/registry.toml";
const SUPPORTED_VERDICTS: &[&str] = &["pass", "fail", "warn", "skipped"];
const SUPPORTED_EVENTS: &[&str] = &["pull_request", "merge_group", "push", "local"];
const SUPPORTED_CLASSIFICATIONS: &[&str] =
    &["code_regression", "infra_failure", "stale_base", "master_red", "skipped", "unknown"];

pub fn list(format: GateReceiptsFormat) -> Result<()> {
    let root = crate::utils::project_root()?;
    let registry = load_registry(&root)?;

    match format {
        GateReceiptsFormat::Human => {
            println!("Registry: {}", root.join(REGISTRY_PATH).display());
            for schema in registry.schemas {
                println!("- {} (check: {}) -> {}", schema.name, schema.check, schema.path);
            }
        }
        GateReceiptsFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&registry.schemas)?);
        }
    }

    Ok(())
}

pub fn validate(path: &Path, format: GateReceiptsFormat) -> Result<()> {
    let root = crate::utils::project_root()?;
    let registry = load_registry(&root)?;
    let result = validate_receipt_path(&root, &registry, path)?;
    emit_validation_result(&result, format)?;

    if !result.valid {
        bail!("receipt validation failed for {}", path.display());
    }

    Ok(())
}

pub fn validate_all(dir: &Path, format: GateReceiptsFormat) -> Result<()> {
    let root = crate::utils::project_root()?;
    let registry = load_registry(&root)?;
    let mut results = Vec::new();

    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let entry = entry.with_context(|| format!("reading entry under {}", dir.display()))?;
        let path = entry.path();
        if path.extension() != Some(OsStr::new("json")) {
            continue;
        }
        results.push(validate_receipt_path(&root, &registry, &path)?);
    }

    match format {
        GateReceiptsFormat::Human => {
            for result in &results {
                emit_validation_result(result, format)?;
            }
        }
        GateReceiptsFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&results)?);
        }
    }

    if results.iter().any(|result| !result.valid) {
        bail!("one or more receipts failed validation");
    }

    Ok(())
}

fn emit_validation_result(result: &ValidationResult, format: GateReceiptsFormat) -> Result<()> {
    match format {
        GateReceiptsFormat::Human => {
            if result.valid {
                println!(
                    "VALID {} ({})",
                    result.receipt_path,
                    result.schema.as_deref().unwrap_or("unknown")
                );
            } else {
                println!(
                    "INVALID {} ({})",
                    result.receipt_path,
                    result.schema.as_deref().unwrap_or("unknown")
                );
                for issue in &result.issues {
                    println!("  - [{}] {}: {}", issue.code, issue.path, issue.message);
                }
            }
        }
        GateReceiptsFormat::Json => {
            println!("{}", serde_json::to_string_pretty(result)?);
        }
    }

    Ok(())
}

fn validate_receipt_path(
    root: &Path,
    registry: &Registry,
    path: &Path,
) -> Result<ValidationResult> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("reading receipt {}", path.display()))?;
    let receipt: Value = serde_json::from_str(&raw)
        .with_context(|| format!("parsing receipt JSON {}", path.display()))?;

    let mut issues = Vec::new();

    for field in &registry.common.required {
        if lookup_field(&receipt, field).is_none() {
            issues.push(ValidationIssue {
                path: field.clone(),
                code: "missing_required".to_string(),
                message: format!("Required field `{field}` is missing"),
            });
        }
    }

    if let Some(event) = receipt.get("event").and_then(Value::as_str)
        && !SUPPORTED_EVENTS.contains(&event)
    {
        issues.push(ValidationIssue {
            path: "event".to_string(),
            code: "invalid_enum".to_string(),
            message: format!("Unsupported event `{event}`"),
        });
    }

    if let Some(verdict) = receipt.get("verdict").and_then(Value::as_str)
        && !SUPPORTED_VERDICTS.contains(&verdict)
    {
        issues.push(ValidationIssue {
            path: "verdict".to_string(),
            code: "invalid_enum".to_string(),
            message: format!("Unsupported verdict `{verdict}`"),
        });
    }

    if let Some(classification) = receipt.get("classification").and_then(Value::as_str)
        && !SUPPORTED_CLASSIFICATIONS.contains(&classification)
    {
        issues.push(ValidationIssue {
            path: "classification".to_string(),
            code: "invalid_enum".to_string(),
            message: format!("Unsupported classification `{classification}`"),
        });
    }

    let check_name =
        receipt.get("check").and_then(Value::as_str).map(std::borrow::ToOwned::to_owned);
    let schema = check_name
        .as_deref()
        .and_then(|check| registry.schemas.iter().find(|entry| entry.check == check))
        .or_else(|| registry.schemas.iter().find(|entry| entry.check == "*"));

    if check_name.is_some() && schema.is_none() {
        issues.push(ValidationIssue {
            path: "check".to_string(),
            code: "unknown_check".to_string(),
            message: "No schema mapped for check in .ci/receipts/registry.toml".to_string(),
        });
    }

    if let Some(schema) = schema {
        let schema_path = root.join(&schema.path);
        if !schema_path.exists() {
            issues.push(ValidationIssue {
                path: "schema".to_string(),
                code: "missing_schema".to_string(),
                message: format!("Schema path does not exist: {}", schema_path.display()),
            });
        }
    }

    Ok(ValidationResult {
        receipt_path: path.display().to_string(),
        valid: issues.is_empty(),
        schema: schema.map(|entry| entry.name.clone()),
        issues,
    })
}

fn lookup_field<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut cursor = value;
    for segment in path.split('.') {
        cursor = cursor.get(segment)?;
    }
    Some(cursor)
}

fn load_registry(root: &Path) -> Result<Registry> {
    let path = root.join(REGISTRY_PATH);
    let raw = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let registry: Registry =
        toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;

    if registry.schemas.is_empty() {
        bail!("receipt schema registry is empty: {}", path.display());
    }

    Ok(registry)
}

#[cfg(test)]
mod tests {
    use super::{SUPPORTED_EVENTS, SUPPORTED_VERDICTS, lookup_field};
    use serde_json::json;

    #[test]
    fn lookup_field_resolves_nested_segments() {
        let value = json!({"repro": {"command": "cargo xtask"}});
        assert_eq!(
            lookup_field(&value, "repro.command").and_then(|v| v.as_str()),
            Some("cargo xtask")
        );
        assert!(lookup_field(&value, "repro.missing").is_none());
    }

    #[test]
    fn enums_cover_expected_values() {
        assert!(SUPPORTED_VERDICTS.contains(&"pass"));
        assert!(SUPPORTED_EVENTS.contains(&"pull_request"));
    }
}
