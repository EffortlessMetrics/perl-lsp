use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_CONFIG_PATH: &str = ".ci/state/label-projection.toml";

#[derive(Debug, Clone)]
pub struct LabelProjectorArgs {
    pub state: PathBuf,
    pub dry_run: bool,
    pub apply: bool,
    pub receipt: Option<PathBuf>,
    pub config: PathBuf,
}

#[derive(Debug, Deserialize)]
struct ProjectionConfig {
    state: BTreeMap<String, ProjectionRule>,
}

#[derive(Debug, Deserialize)]
struct ProjectionRule {
    #[serde(default)]
    apply: Vec<String>,
    #[serde(default)]
    remove: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct QueueStateEntry {
    #[serde(default, alias = "pr", alias = "number")]
    pr_number: Option<u64>,
    state: String,
    #[serde(default)]
    current_labels: Vec<String>,
    #[serde(default, alias = "has_merge_ready_receipt", alias = "merge_ready_receipt_exists")]
    valid_merge_ready_receipt: Option<bool>,
}

#[derive(Debug, Serialize)]
struct LabelProjectionReceipt {
    dry_run: bool,
    verdict: String,
    projections: Vec<LabelProjectionEntry>,
}

#[derive(Debug, Serialize)]
struct LabelProjectionEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pr_number: Option<u64>,
    state: String,
    current_labels: Vec<String>,
    projected_apply: Vec<String>,
    projected_remove: Vec<String>,
    skipped: bool,
    reason: String,
    dry_run: bool,
    verdict: String,
}

pub fn run_project_labels(args: LabelProjectorArgs) -> Result<()> {
    if args.apply && args.dry_run {
        bail!("--apply and --dry-run cannot be used together");
    }

    let config_path = resolve_path(&args.config)?;
    let state_path = resolve_path(&args.state)?;
    let receipt_path = match args.receipt.as_ref() {
        Some(path) => Some(resolve_path(path)?),
        None => None,
    };

    let config = load_config(&config_path)?;
    let state_text = fs::read_to_string(&state_path)
        .with_context(|| format!("failed to read queue state: {}", state_path.display()))?;
    let state_value: Value = serde_json::from_str(&state_text)
        .with_context(|| format!("failed to parse queue state JSON: {}", state_path.display()))?;

    let entries = extract_entries(&state_value)?;
    let dry_run = !args.apply;

    if args.apply {
        ensure_gh_token()?;
    }

    let mut projections = Vec::new();
    for entry in entries {
        let mut projection = project_entry(entry, &config, dry_run);
        if args.apply && !projection.skipped {
            match projection.pr_number {
                Some(pr_number) => {
                    apply_projection(
                        pr_number,
                        &projection.projected_apply,
                        &projection.projected_remove,
                    )?;
                    projection.reason = "labels reconciled".to_string();
                    projection.verdict = "applied".to_string();
                }
                None => {
                    projection.skipped = true;
                    projection.reason =
                        "apply mode requires pr_number in state receipt".to_string();
                    projection.verdict = "skipped".to_string();
                }
            }
        }

        projections.push(projection);
    }

    let verdict = if dry_run {
        if projections.iter().all(|item| item.verdict == "skipped") {
            "skipped"
        } else if projections.iter().all(|item| item.verdict == "projected") {
            "projected"
        } else {
            "mixed"
        }
    } else if projections.iter().all(|item| item.verdict == "applied") {
        "applied"
    } else if projections.iter().all(|item| item.verdict == "skipped") {
        "skipped"
    } else {
        "mixed"
    }
    .to_string();

    let receipt = LabelProjectionReceipt { dry_run, verdict, projections };

    if let Some(path) = receipt_path {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create receipt directory: {}", parent.display())
            })?;
        }
        let serialized = serde_json::to_string_pretty(&receipt)
            .context("failed to serialize label projection receipt")?;
        fs::write(&path, serialized)
            .with_context(|| format!("failed to write receipt: {}", path.display()))?;
    }

    println!("label projection verdict: {}", receipt.verdict);
    Ok(())
}

fn load_config(path: &Path) -> Result<ProjectionConfig> {
    let config_text = fs::read_to_string(path)
        .with_context(|| format!("failed to read projection config: {}", path.display()))?;
    toml::from_str(&config_text)
        .with_context(|| format!("failed to parse projection config: {}", path.display()))
}

fn resolve_path(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    Ok(crate::utils::project_root()?.join(path))
}

fn extract_entries(state_value: &Value) -> Result<Vec<QueueStateEntry>> {
    if state_value.is_array() {
        return serde_json::from_value(state_value.clone())
            .context("failed to parse queue state array");
    }

    if let Some(obj) = state_value.as_object() {
        for key in ["pull_requests", "prs", "entries", "items"] {
            if let Some(value) = obj.get(key) {
                let entries: Vec<QueueStateEntry> = serde_json::from_value(value.clone())
                    .with_context(|| format!("failed to parse queue state field '{key}'"))?;
                return Ok(entries);
            }
        }

        if obj.contains_key("state") {
            let entry: QueueStateEntry = serde_json::from_value(state_value.clone())
                .context("failed to parse single queue state object")?;
            return Ok(vec![entry]);
        }
    }

    bail!(
        "unsupported queue state shape; expected object with 'state' or list under pull_requests/prs/entries/items"
    )
}

fn project_entry(
    entry: QueueStateEntry,
    config: &ProjectionConfig,
    dry_run: bool,
) -> LabelProjectionEntry {
    let state = entry.state;
    let current_labels = entry.current_labels;

    let Some(rule) = config.state.get(&state) else {
        return LabelProjectionEntry {
            pr_number: entry.pr_number,
            state,
            current_labels,
            projected_apply: Vec::new(),
            projected_remove: Vec::new(),
            skipped: true,
            reason: "no projection configured for state".to_string(),
            dry_run,
            verdict: "skipped".to_string(),
        };
    };

    if state == "MERGE_READY" && entry.valid_merge_ready_receipt != Some(true) {
        return LabelProjectionEntry {
            pr_number: entry.pr_number,
            state,
            current_labels,
            projected_apply: Vec::new(),
            projected_remove: Vec::new(),
            skipped: true,
            reason: "missing valid merge-ready receipt".to_string(),
            dry_run,
            verdict: "skipped".to_string(),
        };
    }

    let projected_apply = rule
        .apply
        .iter()
        .filter(|label| !current_labels.iter().any(|existing| existing == *label))
        .cloned()
        .collect::<Vec<_>>();

    let projected_remove = rule
        .remove
        .iter()
        .filter(|label| current_labels.iter().any(|existing| existing == *label))
        .cloned()
        .collect::<Vec<_>>();

    LabelProjectionEntry {
        pr_number: entry.pr_number,
        state,
        current_labels,
        projected_apply,
        projected_remove,
        skipped: false,
        reason: if dry_run { "dry run".to_string() } else { "ready to apply".to_string() },
        dry_run,
        verdict: if dry_run { "projected".to_string() } else { "planned".to_string() },
    }
}

fn ensure_gh_token() -> Result<()> {
    match std::env::var("GH_TOKEN") {
        Ok(token) if !token.trim().is_empty() => Ok(()),
        _ => bail!("--apply requires GH_TOKEN to be set"),
    }
}

fn apply_projection(pr_number: u64, to_apply: &[String], to_remove: &[String]) -> Result<()> {
    let repository = std::env::var("GITHUB_REPOSITORY").ok();

    for label in to_apply {
        let mut command = Command::new("gh");
        command.args(["pr", "edit", &pr_number.to_string(), "--add-label", label]);
        if let Some(repo) = repository.as_ref() {
            command.args(["--repo", repo]);
        }
        let output = command
            .output()
            .with_context(|| format!("failed to add label '{label}' to PR #{pr_number}"))?;

        if !output.status.success() {
            bail!(
                "failed to add label '{}' to PR #{}: {}",
                label,
                pr_number,
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    for label in to_remove {
        let mut command = Command::new("gh");
        command.args(["pr", "edit", &pr_number.to_string(), "--remove-label", label]);
        if let Some(repo) = repository.as_ref() {
            command.args(["--repo", repo]);
        }
        let output = command
            .output()
            .with_context(|| format!("failed to remove label '{label}' from PR #{pr_number}"))?;

        if !output.status.success() {
            bail!(
                "failed to remove label '{}' from PR #{}: {}",
                label,
                pr_number,
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    Ok(())
}

impl Default for LabelProjectorArgs {
    fn default() -> Self {
        Self {
            state: PathBuf::from("target/receipts/queue-state.json"),
            dry_run: true,
            apply: false,
            receipt: None,
            config: PathBuf::from(DEFAULT_CONFIG_PATH),
        }
    }
}
