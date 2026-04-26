use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

use crate::utils::project_root;

const REGISTRY_PATH: &str = ".ci/receipts/registry.toml";

const COMMON_REQUIRED_FIELDS: [&str; 4] = ["check", "schema_version", "event", "verdict"];
const SUPPORTED_VERDICTS: [&str; 4] = ["pass", "fail", "warn", "skipped"];
const SUPPORTED_EVENTS: [&str; 4] = ["pull_request", "merge_group", "push", "local"];
const SUPPORTED_CLASSIFICATIONS: [&str; 6] =
    ["code_regression", "infra_failure", "stale_base", "master_red", "skipped", "unknown"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputFormat {
    Human,
    Json,
}

#[derive(Debug, Deserialize)]
struct Registry {
    schemas: Vec<RegistrySchema>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RegistrySchema {
    name: String,
    schema: String,
    description: String,
    checks: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct TopLevelSchema {
    required: Option<Vec<String>>,
    properties: Option<Map<String, Value>>,
}

#[derive(Debug, Serialize)]
struct ValidationOutcome {
    path: String,
    ok: bool,
    check: Option<String>,
    schema: Option<String>,
    errors: Vec<String>,
}

pub fn list(format: OutputFormat) -> Result<()> {
    let registry = load_registry()?;
    match format {
        OutputFormat::Human => {
            for schema in &registry.schemas {
                println!(
                    "{}\n  schema: {}\n  checks: {}\n  description: {}",
                    schema.name,
                    schema.schema,
                    schema.checks.join(", "),
                    schema.description
                );
            }
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&json!({"schemas": registry.schemas}))?);
        }
    }
    Ok(())
}

pub fn validate(path: &Path, format: OutputFormat) -> Result<()> {
    let registry = load_registry()?;
    let outcome = validate_path(path, &registry)?;
    emit_outcome(&outcome, format)?;
    if outcome.ok { Ok(()) } else { bail!("gate receipt validation failed: {}", path.display()) }
}

pub fn validate_all(dir: &Path, format: OutputFormat) -> Result<()> {
    let registry = load_registry()?;
    let mut outcomes = Vec::new();

    for entry in WalkDir::new(dir).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if entry.file_type().is_file() && path.extension() == Some(OsStr::new("json")) {
            outcomes.push(validate_path(path, &registry)?);
        }
    }

    let has_failures = outcomes.iter().any(|o| !o.ok);
    match format {
        OutputFormat::Human => {
            for outcome in &outcomes {
                if outcome.ok {
                    println!("PASS {}", outcome.path);
                } else {
                    println!("FAIL {}", outcome.path);
                    for error in &outcome.errors {
                        println!("  - {error}");
                    }
                }
            }
        }
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "ok": !has_failures,
                    "count": outcomes.len(),
                    "results": outcomes
                }))?
            );
        }
    }

    if has_failures {
        bail!("one or more gate receipts failed validation")
    }
    Ok(())
}

fn emit_outcome(outcome: &ValidationOutcome, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Human => {
            if outcome.ok {
                println!(
                    "PASS {} (check: {}, schema: {})",
                    outcome.path,
                    outcome.check.as_deref().unwrap_or("unknown"),
                    outcome.schema.as_deref().unwrap_or("unknown")
                );
            } else {
                println!("FAIL {}", outcome.path);
                for error in &outcome.errors {
                    println!("  - {error}");
                }
            }
        }
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(outcome)?),
    }

    Ok(())
}

fn validate_path(path: &Path, registry: &Registry) -> Result<ValidationOutcome> {
    let raw = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let doc: Value = serde_json::from_str(&raw)
        .with_context(|| format!("parsing JSON receipt {}", path.display()))?;

    let mut errors = Vec::new();
    let obj = match doc.as_object() {
        Some(obj) => obj,
        None => {
            return Ok(ValidationOutcome {
                path: path.display().to_string(),
                ok: false,
                check: None,
                schema: None,
                errors: vec!["receipt root must be an object".to_string()],
            });
        }
    };

    let check = obj.get("check").and_then(Value::as_str).map(ToOwned::to_owned);
    let Some(check_name) = check.clone() else {
        return Ok(ValidationOutcome {
            path: path.display().to_string(),
            ok: false,
            check,
            schema: None,
            errors: vec!["missing required field: check".to_string()],
        });
    };

    let schema_entry =
        registry.schemas.iter().find(|entry| entry.checks.iter().any(|c| c == &check_name));
    let Some(schema_entry) = schema_entry else {
        return Ok(ValidationOutcome {
            path: path.display().to_string(),
            ok: false,
            check,
            schema: None,
            errors: vec![format!("no schema registry entry for check '{check_name}'")],
        });
    };

    let schema_path = project_root()?.join(&schema_entry.schema);
    let schema = read_schema(&schema_path)?;

    validate_common_contract(obj, &mut errors);
    validate_required_fields(obj, &schema, &mut errors);
    validate_enum_fields(obj, &schema, &mut errors);

    Ok(ValidationOutcome {
        path: path.display().to_string(),
        ok: errors.is_empty(),
        check,
        schema: Some(schema_entry.schema.clone()),
        errors,
    })
}

fn validate_common_contract(obj: &Map<String, Value>, errors: &mut Vec<String>) {
    for field in COMMON_REQUIRED_FIELDS {
        if !obj.contains_key(field) {
            errors.push(format!("missing required field: {field}"));
        }
    }

    validate_allowed_value(obj, "event", &SUPPORTED_EVENTS, errors);
    validate_allowed_value(obj, "verdict", &SUPPORTED_VERDICTS, errors);

    if obj.contains_key("classification") {
        validate_allowed_value(obj, "classification", &SUPPORTED_CLASSIFICATIONS, errors);
    }
}

fn validate_allowed_value(
    obj: &Map<String, Value>,
    field: &str,
    allowed_values: &[&str],
    errors: &mut Vec<String>,
) {
    let Some(value) = obj.get(field).and_then(Value::as_str) else {
        return;
    };

    if !allowed_values.iter().any(|allowed| allowed == &value) {
        errors.push(format!(
            "invalid value for {field}: '{value}' (allowed: {})",
            allowed_values.join(", ")
        ));
    }
}

fn validate_required_fields(
    obj: &Map<String, Value>,
    schema: &TopLevelSchema,
    errors: &mut Vec<String>,
) {
    if let Some(required) = &schema.required {
        for field in required {
            if !obj.contains_key(field) {
                errors.push(format!("missing required field: {field}"));
            }
        }
    }
}

fn validate_enum_fields(
    obj: &Map<String, Value>,
    schema: &TopLevelSchema,
    errors: &mut Vec<String>,
) {
    let Some(properties) = &schema.properties else {
        return;
    };

    for (name, prop) in properties {
        let Some(allowed_values) = prop.get("enum").and_then(Value::as_array) else {
            continue;
        };
        let Some(value) = obj.get(name).and_then(Value::as_str) else {
            continue;
        };

        let contains =
            allowed_values.iter().filter_map(Value::as_str).any(|allowed| allowed == value);
        if !contains {
            let allowed =
                allowed_values.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", ");
            errors.push(format!("invalid value for {name}: '{value}' (allowed: {allowed})"));
        }
    }
}

fn load_registry() -> Result<Registry> {
    let path = project_root()?.join(REGISTRY_PATH);
    let raw = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
}

fn read_schema(path: &Path) -> Result<TopLevelSchema> {
    let raw = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parsing schema {}", path.display()))
}
