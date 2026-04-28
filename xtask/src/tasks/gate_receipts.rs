//! Gate receipt schema registry helpers.
//!
//! This task provides a lightweight control-plane registry for CI receipts.
//! It validates registry membership and required/common fields, with optional
//! JSON output for machine consumers.

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

const REGISTRY_PATH: &str = ".ci/receipts/registry.toml";

const REQUIRED_COMMON_FIELDS: &[&str] = &["check", "schema_version", "event", "verdict"];

const SUPPORTED_VERDICTS: &[&str] = &["pass", "fail", "warn", "skipped"];
const SUPPORTED_EVENTS: &[&str] = &["pull_request", "merge_group", "push", "local"];
const SUPPORTED_CLASSIFICATIONS: &[&str] =
    &["code_regression", "infra_failure", "stale_base", "master_red", "skipped", "unknown"];

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Human,
    Json,
}

#[derive(Debug, Deserialize)]
struct Registry {
    registry_version: String,
    #[serde(default)]
    receipt: Vec<RegistryEntry>,
}

#[derive(Debug, Deserialize)]
struct RegistryEntry {
    check: String,
    schema: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    producer: Option<String>,
    #[serde(default)]
    required_fields: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ListItem {
    check: String,
    schema: String,
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    producer: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    required_fields: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ValidationResult {
    path: String,
    ok: bool,
    check: Option<String>,
    schema: Option<String>,
    errors: Vec<String>,
}

pub fn list(format: OutputFormat) -> Result<()> {
    let registry = load_registry()?;

    let items = registry
        .receipt
        .iter()
        .map(|entry| ListItem {
            check: entry.check.clone(),
            schema: entry.schema.clone(),
            description: entry.description.clone(),
            producer: entry.producer.clone(),
            required_fields: entry.required_fields.clone(),
        })
        .collect::<Vec<_>>();

    match format {
        OutputFormat::Human => {
            println!("Gate receipt registry v{}", registry.registry_version);
            for item in &items {
                if let Some(description) = &item.description {
                    println!("- {} => {} ({description})", item.check, item.schema);
                } else {
                    println!("- {} => {}", item.check, item.schema);
                }
                if let Some(producer) = &item.producer {
                    println!("    producer: {producer}");
                }
                if !item.required_fields.is_empty() {
                    println!("    required_fields: {}", item.required_fields.join(", "));
                }
            }
        }
        OutputFormat::Json => {
            let payload = json!({
                "registry_version": registry.registry_version,
                "receipts": items,
            });
            println!("{}", serde_json::to_string_pretty(&payload)?);
        }
    }

    Ok(())
}

pub fn validate(path: &Path, format: OutputFormat) -> Result<()> {
    let registry = load_registry()?;
    let result = validate_receipt(path, &registry)?;
    emit_results(vec![result], format)
}

pub fn validate_all(dir: &Path, format: OutputFormat) -> Result<()> {
    let registry = load_registry()?;

    let mut results = Vec::new();
    for entry in WalkDir::new(dir) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.path().extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }

        let result = validate_receipt(entry.path(), &registry)?;
        results.push(result);
    }

    if results.is_empty() {
        return Err(anyhow!("no .json files found in {}", dir.display()));
    }

    emit_results(results, format)
}

fn emit_results(results: Vec<ValidationResult>, format: OutputFormat) -> Result<()> {
    let has_errors = results.iter().any(|result| !result.ok);

    match format {
        OutputFormat::Human => {
            for result in &results {
                if result.ok {
                    println!("PASS {}", result.path);
                } else {
                    println!("FAIL {}", result.path);
                    for error in &result.errors {
                        println!("  - {error}");
                    }
                }
            }
        }
        OutputFormat::Json => {
            let payload = json!({
                "ok": !has_errors,
                "results": results,
            });
            println!("{}", serde_json::to_string_pretty(&payload)?);
        }
    }

    if has_errors { Err(anyhow!("gate receipt validation failed")) } else { Ok(()) }
}

fn validate_receipt(path: &Path, registry: &Registry) -> Result<ValidationResult> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read receipt {}", path.display()))?;
    let receipt: Value = serde_json::from_str(&content)
        .with_context(|| format!("invalid JSON in {}", path.display()))?;

    let mut errors = Vec::new();
    validate_common_fields(&receipt, &mut errors);

    let check = receipt.get("check").and_then(Value::as_str).map(ToOwned::to_owned);

    let registry_map = registry_map(registry)?;
    let schema = check.as_ref().and_then(|check_name| registry_map.get(check_name)).cloned();

    match &check {
        Some(check_name) => {
            if !registry_map.contains_key(check_name) {
                errors.push(format!("unknown check '{check_name}' (not in registry)"));
            }
        }
        None => {
            errors.push("missing or non-string field: check".to_string());
        }
    }

    if let Some(schema_path) = &schema {
        validate_schema_required_fields(&receipt, schema_path, &mut errors)?;
    }

    Ok(ValidationResult {
        path: path.display().to_string(),
        ok: errors.is_empty(),
        check,
        schema,
        errors,
    })
}

fn validate_common_fields(receipt: &Value, errors: &mut Vec<String>) {
    for field in REQUIRED_COMMON_FIELDS {
        let value = receipt.get(field);
        if value.is_none() {
            errors.push(format!("missing required field: {field}"));
            continue;
        }
        if value.and_then(Value::as_str).is_none() {
            errors.push(format!("field '{field}' must be a string"));
        }
    }

    if let Some(event) = receipt.get("event").and_then(Value::as_str)
        && !SUPPORTED_EVENTS.contains(&event)
    {
        errors.push(format!(
            "unsupported event '{event}', expected one of {}",
            SUPPORTED_EVENTS.join(", ")
        ));
    }

    if let Some(verdict) = receipt.get("verdict").and_then(Value::as_str)
        && !SUPPORTED_VERDICTS.contains(&verdict)
    {
        errors.push(format!(
            "unsupported verdict '{verdict}', expected one of {}",
            SUPPORTED_VERDICTS.join(", ")
        ));
    }

    if let Some(classification) = receipt.get("classification").and_then(Value::as_str)
        && !SUPPORTED_CLASSIFICATIONS.contains(&classification)
    {
        errors.push(format!(
            "unsupported classification '{classification}', expected one of {}",
            SUPPORTED_CLASSIFICATIONS.join(", ")
        ));
    }
}

fn validate_schema_required_fields(
    receipt: &Value,
    schema_path: &str,
    errors: &mut Vec<String>,
) -> Result<()> {
    let schema_content = fs::read_to_string(schema_path)
        .with_context(|| format!("failed to read schema {schema_path}"))?;
    let schema_value: Value = serde_json::from_str(&schema_content)
        .with_context(|| format!("invalid schema JSON in {schema_path}"))?;

    let mut required = HashSet::new();
    collect_required_fields(&schema_value, &mut required);

    for field in required {
        if receipt.get(&field).is_none() {
            errors.push(format!("missing schema-required field: {field}"));
        }
    }

    Ok(())
}

fn collect_required_fields(schema_value: &Value, required: &mut HashSet<String>) {
    if let Some(required_fields) = schema_value.get("required").and_then(Value::as_array) {
        for field in required_fields.iter().filter_map(Value::as_str) {
            required.insert(field.to_string());
        }
    }

    if let Some(all_of) = schema_value.get("allOf").and_then(Value::as_array) {
        for sub_schema in all_of {
            collect_required_fields(sub_schema, required);
        }
    }
}

fn load_registry() -> Result<Registry> {
    let content = fs::read_to_string(REGISTRY_PATH)
        .with_context(|| format!("failed to read registry at {REGISTRY_PATH}"))?;
    let registry: Registry = toml::from_str(&content).context("invalid registry TOML")?;

    if registry.receipt.is_empty() {
        return Err(anyhow!("registry has no receipt entries"));
    }

    Ok(registry)
}

fn registry_map(registry: &Registry) -> Result<BTreeMap<String, String>> {
    let mut map = BTreeMap::new();
    for entry in &registry.receipt {
        if map.insert(entry.check.clone(), entry.schema.clone()).is_some() {
            return Err(anyhow!("duplicate registry check '{}'", entry.check));
        }
    }
    Ok(map)
}
