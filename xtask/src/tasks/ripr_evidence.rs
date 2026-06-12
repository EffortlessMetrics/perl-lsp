//! Portable RIPR PR evidence and routing tasks.
//!
//! README badges stay repo-scoped. These commands produce diff-scoped artifacts
//! under `target/` for PR review, annotations, and mutation routing.

use color_eyre::eyre::{Context, Result, bail, eyre};
use glob::Pattern;
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[cfg(test)]
static RIPR_BIN_OVERRIDE: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

const DEFAULT_ROOT: &str = ".";
const DEFAULT_BASE: &str = "origin/main";
const DEFAULT_HEAD: &str = "HEAD";
const PR_EVIDENCE_JSON: &str = "target/ripr/pr/repo-exposure.json";
const PR_EVIDENCE_MD: &str = "target/ripr/pr/repo-exposure.md";
const PR_DIFF: &str = "target/ripr/pr/pr.diff";
const REVIEW_COMMENTS_JSON: &str = "target/ripr/review/comments.json";
const REVIEW_COMMENTS_MD: &str = "target/ripr/review/comments.md";
const ANNOTATIONS_TXT: &str = "target/ripr/review/annotations.txt";
const PR_SUMMARY_MD: &str = "target/ripr/pr/summary.md";
const IMPACTED_JSON: &str = "target/xtask/impacted-evidence/latest.json";
const IMPACTED_MD: &str = "target/xtask/impacted-evidence/latest.md";
const DEFAULT_RIPR_SUPPRESSIONS: &str = "policy/ripr-suppressions.toml";

pub fn ripr_pr(root: &str, base: &str, head: &str, check: bool) -> Result<()> {
    let repo = repo_root()?;
    let options = PrEvidenceOptions {
        root: normalized_option(root, DEFAULT_ROOT),
        base: normalized_option(base, DEFAULT_BASE),
        head: normalized_option(head, DEFAULT_HEAD),
    };
    if check { check_pr_evidence(&repo, &options) } else { write_pr_evidence(&repo, &options) }
}

pub fn ripr_plus(root: &str, receipt: &Path, suppressions: &Path, check: bool) -> Result<()> {
    let repo = repo_root()?;
    let options = RiprPlusOptions {
        root: normalized_option(root, DEFAULT_ROOT),
        suppressions: suppressions.to_path_buf(),
    };
    let packet = ripr_plus_packet(&repo, &options)?;
    let rendered = format_json(&packet)?;
    let receipt_path = repo.join(receipt);
    if check {
        let actual = fs::read_to_string(&receipt_path)
            .with_context(|| format!("missing or unreadable {}", receipt_path.display()))?;
        if actual != rendered {
            bail!(
                "{} is stale; run `cargo xtask ripr-plus --root {} --receipt {}`",
                receipt_path.display(),
                options.root,
                receipt.display()
            );
        }
        println!("RIPR+ receipt is current: {}", receipt_path.display());
    } else {
        write_text(&receipt_path, &rendered)?;
        println!("Wrote {}", receipt_path.display());
    }
    Ok(())
}

pub fn ripr_review_comments(
    root: &str,
    base: &str,
    head: &str,
    timeout_seconds: Option<u64>,
    check: bool,
) -> Result<()> {
    let repo = repo_root()?;
    let options = ReviewCommentsOptions {
        root: normalized_option(root, DEFAULT_ROOT),
        base: normalized_option(base, DEFAULT_BASE),
        head: normalized_option(head, DEFAULT_HEAD),
        timeout_seconds: timeout_seconds.filter(|seconds| *seconds > 0),
    };
    if check {
        check_review_comments(&repo, &options)
    } else {
        write_review_comments(&repo, &options)
    }
}

pub fn ripr_pr_summary(check: bool) -> Result<()> {
    let repo = repo_root()?;
    let summary = render_pr_evidence_summary(&repo);
    let path = repo.join(PR_SUMMARY_MD);
    if check {
        let actual = fs::read_to_string(&path)
            .with_context(|| format!("missing or unreadable {PR_SUMMARY_MD}"))?;
        if actual != summary {
            bail!("{PR_SUMMARY_MD} is stale; run `cargo xtask ripr-pr-summary`");
        }
        println!("PR evidence summary is current.");
    } else {
        write_text(&path, &summary)?;
        println!("Wrote {PR_SUMMARY_MD}");
    }
    Ok(())
}

pub fn ripr_annotations(comments: &str, out: &str, check: bool) -> Result<()> {
    let repo = repo_root()?;
    let comments = normalized_option(comments, REVIEW_COMMENTS_JSON);
    let out = normalized_option(out, ANNOTATIONS_TXT);
    let generated = render_annotations(&repo, &comments)?;
    let out_path = repo.join(&out);

    if check {
        if generated.comments_missing && !out_path.exists() {
            println!("RIPR annotations skipped: {comments} is missing.");
            return Ok(());
        }
        let actual = fs::read_to_string(&out_path)
            .with_context(|| format!("missing or unreadable {out}"))?;
        if actual != generated.text {
            bail!("{out} is stale; run `cargo xtask ripr-annotations`");
        }
        println!("RIPR annotations are current.");
    } else {
        write_text(&out_path, &generated.text)?;
        if generated.comments_missing {
            println!("RIPR annotations skipped: {comments} is missing.");
        } else if generated.text.is_empty() {
            println!("RIPR annotations: no comments[] guidance to emit.");
        } else {
            print!("{}", generated.text);
        }
        println!("Wrote {out}");
    }
    Ok(())
}

pub fn impacted_evidence(
    pr_evidence: &str,
    labels: &[String],
    labels_csv: Option<&str>,
    check: bool,
) -> Result<()> {
    let repo = repo_root()?;
    let options = ImpactedEvidenceOptions {
        pr_evidence: normalized_option(pr_evidence, PR_EVIDENCE_JSON),
        labels: merged_labels(labels, labels_csv),
    };
    let packet = impacted_evidence_packet(&repo, &options);
    let json_text = format_json(&packet)?;
    let markdown = render_impacted_evidence_markdown(&packet);

    if check {
        let actual_json = fs::read_to_string(repo.join(IMPACTED_JSON))
            .with_context(|| format!("missing or unreadable {IMPACTED_JSON}"))?;
        let actual_md = fs::read_to_string(repo.join(IMPACTED_MD))
            .with_context(|| format!("missing or unreadable {IMPACTED_MD}"))?;
        if actual_json != json_text || actual_md != markdown {
            bail!("impacted evidence is stale; run `cargo xtask impacted-evidence`");
        }
        println!("Impacted evidence is current.");
    } else {
        write_text(&repo.join(IMPACTED_JSON), &json_text)?;
        write_text(&repo.join(IMPACTED_MD), &markdown)?;
        println!("Wrote {IMPACTED_JSON}");
        println!("Wrote {IMPACTED_MD}");
    }
    Ok(())
}

fn normalized_option(value: &str, default: &str) -> String {
    if value.trim().is_empty() { default.to_string() } else { value.to_string() }
}

fn repo_root() -> Result<PathBuf> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| eyre!("failed to resolve repository root from {}", manifest_dir.display()))
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PrEvidenceOptions {
    root: String,
    base: String,
    head: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RiprPlusOptions {
    root: String,
    suppressions: PathBuf,
}

fn ripr_plus_packet(repo: &Path, options: &RiprPlusOptions) -> Result<Value> {
    let root = command_root_arg(repo, &options.root)?;
    // Fetch repo-badge-json for the canonical actionable-gap counts (authoritative headline).
    let badge_raw = run_ripr(&[
        "check".to_string(),
        "--root".to_string(),
        root.clone(),
        "--format".to_string(),
        "repo-badge-json".to_string(),
    ])?;
    // Fetch repo-seams-json for the triage inventory (top_files / top_gap_kinds / clusters).
    let seams_raw = run_ripr(&[
        "check".to_string(),
        "--root".to_string(),
        root,
        "--format".to_string(),
        "repo-seams-json".to_string(),
    ])?;
    let suppressions = read_ripr_suppression_rules(repo, &options.suppressions)?;
    ripr_plus_packet_from_raw(options, &current_head(repo)?, &suppressions, &badge_raw, &seams_raw)
}

/// Parse both `repo-badge-json` and `repo-seams-json` output and build the
/// RIPR+ baseline receipt. Kept separate from the git/ripr I/O above so the
/// parsing and canonical-gap accounting are exercised by unit tests.
fn ripr_plus_packet_from_raw(
    options: &RiprPlusOptions,
    head: &str,
    suppressions: &RiprSuppressionRules,
    badge_raw: &str,
    seams_raw: &str,
) -> Result<Value> {
    let badge: Value =
        serde_json::from_str(badge_raw).context("ripr repo-badge-json was invalid JSON")?;
    let seams_value: Value =
        serde_json::from_str(seams_raw).context("ripr repo-seams-json was invalid JSON")?;
    let seams = seams_value
        .get("seams")
        .and_then(Value::as_array)
        .ok_or_else(|| eyre!("ripr repo-seams-json output did not include seams[]"))?;
    let seam_summary = ripr_plus_seam_summary(seams, suppressions, 10);
    Ok(ripr_plus_receipt_packet(options, head, suppressions, &badge, seam_summary))
}

fn ripr_plus_receipt_packet(
    options: &RiprPlusOptions,
    head: &str,
    suppressions: &RiprSuppressionRules,
    badge: &Value,
    seam_summary: RiprPlusSeamSummary,
) -> Value {
    // Canonical counts: authoritative headline from repo-badge-json.
    let counts = badge.get("counts");
    let count = |key: &str| counts.and_then(|v| v.get(key)).and_then(Value::as_u64).unwrap_or(0);
    let active_unresolved =
        count("unsuppressed_exposure_gaps") + count("unsuppressed_test_efficiency_findings");
    let suppressed_unresolved =
        count("suppressed_exposure_gaps") + count("suppressed_test_efficiency_findings");
    let basis = badge
        .get("basis")
        .and_then(Value::as_str)
        .unwrap_or("canonical_actionable_gap")
        .to_string();

    // Triage inventory: from the seam summary (repo-seams-json).
    let top_active_files = seam_summary.top_files;
    let top_suppressed_files = seam_summary.top_suppressed_files;
    let top_active_gap_kinds = seam_summary.top_gap_kinds;
    let top_suppressed_gap_kinds = seam_summary.top_suppressed_gap_kinds;

    json!({
        "schema_version": 2,
        "kind": "ripr_plus_baseline",
        "mode": "advisory",
        "head": head,
        "root": options.root,
        "source_format": "ripr check --format repo-badge-json (counts) + repo-seams-json (triage inventory)",
        "basis": basis,
        "unresolved": active_unresolved,
        "active_unresolved": active_unresolved,
        "suppressed_unresolved": suppressed_unresolved,
        "new_unresolved": null,
        "counts": counts.cloned().unwrap_or_else(|| json!({})),
        "reason_counts": badge.get("reason_counts").cloned().unwrap_or_else(|| json!({})),
        "top_files": top_active_files.clone(),
        "top_active_files": top_active_files,
        "top_suppressed_files": top_suppressed_files,
        "top_gap_kinds": top_active_gap_kinds.clone(),
        "top_active_gap_kinds": top_active_gap_kinds,
        "top_suppressed_gap_kinds": top_suppressed_gap_kinds,
        "recommended_first_clusters": seam_summary.recommended_first_clusters,
        "suppression_rule_count": suppressions.display_patterns.len(),
        "suppressions": {
            "path": display_path(&options.suppressions),
            "path_patterns": suppressions.display_patterns.clone(),
            "invalid_patterns": suppressions.invalid_patterns.clone(),
            "reasons": suppressions.suppression_reasons.clone(),
        },
        "decision": "advisory",
        "claim_boundary": [
            "Measurement only; this receipt does not enforce ripr+ zero.",
            "unresolved is the canonical_actionable_gap count from RIPR repo-badge-json: unsuppressed exposure gaps plus actionable test-efficiency findings.",
            "active_unresolved equals unresolved; suppressed_unresolved is suppressed_exposure_gaps + suppressed_test_efficiency_findings from repo-badge-json.",
            "top_files, top_gap_kinds, and recommended_first_clusters are triage aids derived from the seam inventory (repo-seams-json), not the headline count.",
            "new_unresolved is null until PR diff comparison is wired in the quality gate."
        ]
    })
}

#[derive(Debug)]
struct RiprPlusSeamSummary {
    /// Raw seam count of active (unsuppressed) seams in the inventory.
    /// Not used for the headline `unresolved` count in the receipt — that
    /// comes from `repo-badge-json` (canonical actionable gap basis). Kept
    /// here so that `ripr_plus_seam_summary` tests can assert the seam split.
    #[allow(dead_code)]
    unresolved: usize,
    /// Raw seam count of suppressed seams in the inventory. Same note as above.
    #[allow(dead_code)]
    suppressed: usize,
    top_files: Vec<Value>,
    top_suppressed_files: Vec<Value>,
    top_gap_kinds: Vec<Value>,
    top_suppressed_gap_kinds: Vec<Value>,
    recommended_first_clusters: Vec<Value>,
}

fn ripr_plus_seam_summary(
    seams: &[Value],
    suppressions: &RiprSuppressionRules,
    limit: usize,
) -> RiprPlusSeamSummary {
    let active = seams
        .iter()
        .filter(|seam| !suppression_matches_seam(suppressions, seam))
        .collect::<Vec<_>>();
    let suppressed = seams
        .iter()
        .filter(|seam| suppression_matches_seam(suppressions, seam))
        .collect::<Vec<_>>();

    let top_files = ripr_plus_top_files(active.iter().copied(), limit);
    let top_gap_kinds = ripr_plus_top_gap_kinds(active.iter().copied(), limit);
    let recommended_first_clusters =
        ripr_plus_recommended_first_clusters_from_seams(active.iter().copied(), limit);

    RiprPlusSeamSummary {
        unresolved: active.len(),
        suppressed: suppressed.len(),
        top_files,
        top_suppressed_files: ripr_plus_top_files(suppressed.iter().copied(), limit),
        top_gap_kinds,
        top_suppressed_gap_kinds: ripr_plus_top_gap_kinds(suppressed.iter().copied(), limit),
        recommended_first_clusters,
    }
}

fn ripr_plus_top_files<'a>(seams: impl IntoIterator<Item = &'a Value>, limit: usize) -> Vec<Value> {
    ripr_plus_count_rows(seams.into_iter().filter_map(ripr_plus_seam_path), limit)
}

fn ripr_plus_top_gap_kinds<'a>(
    seams: impl IntoIterator<Item = &'a Value>,
    limit: usize,
) -> Vec<Value> {
    ripr_plus_count_rows(seams.into_iter().filter_map(ripr_plus_seam_gap_kind), limit)
}

fn ripr_plus_count_rows(values: impl IntoIterator<Item = String>, limit: usize) -> Vec<Value> {
    let mut counts = BTreeMap::<String, u64>::new();
    for value in values {
        *counts.entry(value).or_default() += 1;
    }
    let mut rows = counts.into_iter().collect::<Vec<_>>();
    rows.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    rows.truncate(limit);
    rows.into_iter().map(|(name, count)| json!({ "name": name, "count": count })).collect()
}

fn ripr_plus_recommended_first_clusters_from_seams<'a>(
    seams: impl IntoIterator<Item = &'a Value>,
    limit: usize,
) -> Vec<Value> {
    let seams = seams.into_iter().collect::<Vec<_>>();
    let file_rows = ripr_plus_top_files(seams.iter().copied(), usize::MAX);
    let gap_kind_rows = ripr_plus_top_gap_kinds(seams.iter().copied(), usize::MAX);
    ripr_plus_recommended_first_clusters(&file_rows, &gap_kind_rows, limit)
}

fn ripr_plus_seam_path(seam: &Value) -> Option<String> {
    let direct = ["path", "file"].into_iter().find_map(|key| seam.get(key).and_then(Value::as_str));
    let nested = seam
        .get("location")
        .and_then(|value| value.get("path"))
        .and_then(Value::as_str)
        .or_else(|| {
            seam.get("placement").and_then(|value| value.get("path")).and_then(Value::as_str)
        })
        .or_else(|| {
            seam.get("evidence_record").and_then(|value| value.get("path")).and_then(Value::as_str)
        });
    direct.or(nested).map(normalize_path_text).filter(|path| !path.trim().is_empty())
}

fn ripr_plus_seam_gap_kind(seam: &Value) -> Option<String> {
    let direct = ["gap_kind", "seam_kind", "kind", "classification", "category", "reason"]
        .into_iter()
        .find_map(|key| ripr_plus_text_value(seam.get(key)));
    let nested = ["location", "placement", "evidence_record", "evidence"].into_iter().find_map(
        |object_key| {
            let object = seam.get(object_key)?;
            ["gap_kind", "seam_kind", "kind", "classification", "category", "reason"]
                .into_iter()
                .find_map(|key| ripr_plus_text_value(object.get(key)))
        },
    );
    direct.or(nested).map(normalize_inventory_label).filter(|kind| !kind.is_empty())
}

fn ripr_plus_text_value(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(value) => Some(value.clone()),
        Value::Array(values) => {
            let parts = values
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>();
            if parts.is_empty() { None } else { Some(parts.join(",")) }
        }
        _ => None,
    }
}

fn normalize_inventory_label(value: String) -> String {
    value.trim().replace('\\', "/").to_ascii_lowercase()
}

fn ripr_plus_recommended_first_clusters(
    top_files: &[Value],
    top_gap_kinds: &[Value],
    limit: usize,
) -> Vec<Value> {
    let mut clusters = BTreeMap::<String, RiprPlusClusterRecommendation>::new();
    for file in top_files {
        let Some(path) = file.get("name").and_then(Value::as_str) else {
            continue;
        };
        let count = file.get("count").and_then(Value::as_u64).unwrap_or(0);
        let (name, reason) = ripr_plus_cluster_for_path(path);
        clusters
            .entry(name.to_string())
            .or_insert_with(|| RiprPlusClusterRecommendation::new(name, reason))
            .push_file(path, count);
    }
    for gap_kind in top_gap_kinds {
        let Some(kind) = gap_kind.get("name").and_then(Value::as_str) else {
            continue;
        };
        let count = gap_kind.get("count").and_then(Value::as_u64).unwrap_or(0);
        let (name, reason) = ripr_plus_cluster_for_gap_kind(kind);
        clusters
            .entry(name.to_string())
            .or_insert_with(|| RiprPlusClusterRecommendation::new(name, reason))
            .push_gap_kind(kind, count);
    }

    let mut rows = clusters.into_values().collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right.score.cmp(&left.score).then_with(|| left.name.cmp(&right.name))
    });
    rows.truncate(limit);
    rows.into_iter().map(RiprPlusClusterRecommendation::into_json).collect()
}

fn ripr_plus_cluster_for_path(path: &str) -> (&'static str, &'static str) {
    let normalized = normalize_path_text(path);
    if normalized.starts_with("xtask/")
        || normalized.starts_with(".github/")
        || normalized.starts_with("policy/")
        || normalized.starts_with("scripts/")
        || normalized.starts_with("docs/ci/")
    {
        (
            "proof-infrastructure",
            "Proof tooling, policy, workflow, and report surfaces are owned by this lane.",
        )
    } else if normalized.contains("receipt")
        || normalized.contains("quality")
        || normalized.contains("ripr")
        || normalized.contains("coverage")
        || normalized.contains("report")
        || normalized.contains("summary")
    {
        (
            "ci-report-formatting",
            "Receipt and report formatting gaps should become agent repair packets.",
        )
    } else if normalized.contains("config") {
        ("config-parsing", "Configuration paths should be covered with focused parse cases.")
    } else if normalized.contains("error") || normalized.contains("diagnostic") {
        ("error-variants", "Failure variants should be covered with behavior assertions.")
    } else {
        (
            "active-ripr-inventory",
            "Use the top active files and gap kinds to split a focused burn-down PR.",
        )
    }
}

fn ripr_plus_cluster_for_gap_kind(kind: &str) -> (&'static str, &'static str) {
    if kind.contains("receipt") || kind.contains("report") || kind.contains("summary") {
        (
            "ci-report-formatting",
            "Receipt and report formatting gaps should become agent repair packets.",
        )
    } else if kind.contains("config") {
        ("config-parsing", "Configuration paths should be covered with focused parse cases.")
    } else if kind.contains("error") || kind.contains("failure") {
        ("error-variants", "Failure variants should be covered with behavior assertions.")
    } else if kind.contains("boundary") || kind.contains("predicate") {
        ("boundary-predicates", "Boundary branches should be covered with below/equal/above cases.")
    } else {
        (
            "active-ripr-inventory",
            "Use the top active files and gap kinds to split a focused burn-down PR.",
        )
    }
}

#[derive(Debug)]
struct RiprPlusClusterRecommendation {
    name: String,
    reason: String,
    score: u64,
    active_file_count: u64,
    gap_kind_count: u64,
    example_files: BTreeSet<String>,
    example_gap_kinds: BTreeSet<String>,
}

impl RiprPlusClusterRecommendation {
    fn new(name: &str, reason: &str) -> Self {
        Self {
            name: name.to_string(),
            reason: reason.to_string(),
            score: 0,
            active_file_count: 0,
            gap_kind_count: 0,
            example_files: BTreeSet::new(),
            example_gap_kinds: BTreeSet::new(),
        }
    }

    fn push_file(&mut self, path: &str, count: u64) {
        self.score += count;
        self.active_file_count += count;
        if self.example_files.len() < 3 {
            self.example_files.insert(path.to_string());
        }
    }

    fn push_gap_kind(&mut self, kind: &str, count: u64) {
        self.score += count;
        self.gap_kind_count += count;
        if self.example_gap_kinds.len() < 3 {
            self.example_gap_kinds.insert(kind.to_string());
        }
    }

    fn into_json(self) -> Value {
        json!({
            "name": self.name,
            "score": self.score,
            "active_file_count": self.active_file_count,
            "gap_kind_count": self.gap_kind_count,
            "reason": self.reason,
            "example_files": self.example_files.into_iter().collect::<Vec<_>>(),
            "example_gap_kinds": self.example_gap_kinds.into_iter().collect::<Vec<_>>(),
        })
    }
}

#[derive(Debug, Default, Deserialize)]
struct RiprSuppressionPolicy {
    #[serde(default, rename = "suppress")]
    suppressions: Vec<RiprSuppression>,
}

#[derive(Debug, Default, Deserialize)]
struct RiprSuppression {
    #[serde(default)]
    id: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    paths: Vec<String>,
    #[serde(default)]
    reason: String,
}

#[derive(Debug, Default)]
struct RiprSuppressionRules {
    display_patterns: Vec<String>,
    path_patterns: Vec<Pattern>,
    invalid_patterns: Vec<String>,
    suppression_reasons: Vec<Value>,
}

fn read_ripr_suppression_rules(repo: &Path, path: &Path) -> Result<RiprSuppressionRules> {
    let policy_path = if path.is_absolute() { path.to_path_buf() } else { repo.join(path) };
    let raw = fs::read_to_string(&policy_path)
        .with_context(|| format!("reading RIPR suppressions {}", policy_path.display()))?;
    let policy: RiprSuppressionPolicy = toml::from_str(&raw)
        .with_context(|| format!("parsing RIPR suppressions {}", policy_path.display()))?;

    let mut rules = RiprSuppressionRules::default();
    for suppression in policy.suppressions {
        let paths =
            suppression.paths.iter().map(|path| normalize_path_text(path)).collect::<Vec<_>>();
        if !suppression.id.trim().is_empty()
            || !suppression.kind.trim().is_empty()
            || !suppression.reason.trim().is_empty()
        {
            rules.suppression_reasons.push(json!({
                "id": suppression.id,
                "kind": suppression.kind,
                "reason": suppression.reason,
                "paths": paths.clone(),
            }));
        }
        for path_pattern in paths {
            match Pattern::new(&path_pattern) {
                Ok(pattern) => {
                    rules.display_patterns.push(path_pattern);
                    rules.path_patterns.push(pattern);
                }
                Err(_) => rules.invalid_patterns.push(path_pattern),
            }
        }
    }
    if !rules.invalid_patterns.is_empty() {
        bail!("invalid RIPR suppression path pattern(s): {}", rules.invalid_patterns.join(", "));
    }
    Ok(rules)
}

fn suppression_matches_seam(rules: &RiprSuppressionRules, seam: &Value) -> bool {
    let Some(path) = ripr_plus_seam_path(seam) else {
        return false;
    };
    let path = normalize_suppression_match_path(&path);
    rules.path_patterns.iter().any(|pattern| pattern.matches(&path))
}

fn current_head(repo: &Path) -> Result<String> {
    revision_sha(repo, "HEAD")
}

fn revision_sha(repo: &Path, revision: &str) -> Result<String> {
    Ok(run_git_output(repo, &["rev-parse", revision])?.trim().to_string())
}

fn write_pr_evidence(repo: &Path, options: &PrEvidenceOptions) -> Result<()> {
    verify_revision(repo, &options.base)?;
    verify_revision(repo, &options.head)?;
    let base_sha = revision_sha(repo, &options.base)?;
    let head_sha = revision_sha(repo, &options.head)?;
    let changed_files = changed_files(repo, &options.base, &options.head)?;
    write_pr_diff(repo, &options.base, &options.head)?;
    let check_json = run_ripr_check(repo, options)?;
    let check_value: Value =
        serde_json::from_str(&check_json).context("ripr check output was not valid JSON")?;
    let suppressions = read_ripr_suppression_rules(repo, Path::new(DEFAULT_RIPR_SUPPRESSIONS))?;
    let packet = pr_evidence_packet(
        options,
        &changed_files,
        &check_value,
        &base_sha,
        &head_sha,
        &suppressions,
    );
    validate_pr_evidence_packet(&packet, options, changed_files.len(), true, &base_sha, &head_sha)?;
    write_text(&repo.join(PR_EVIDENCE_JSON), &format_json(&packet)?)?;
    write_text(&repo.join(PR_EVIDENCE_MD), &render_pr_evidence_markdown(&packet))?;
    println!("Wrote {PR_EVIDENCE_JSON}");
    println!("Wrote {PR_EVIDENCE_MD}");
    Ok(())
}

fn check_pr_evidence(repo: &Path, options: &PrEvidenceOptions) -> Result<()> {
    verify_revision(repo, &options.base)?;
    verify_revision(repo, &options.head)?;
    let base_sha = revision_sha(repo, &options.base)?;
    let head_sha = revision_sha(repo, &options.head)?;
    let changed_files = changed_files(repo, &options.base, &options.head)?;
    let text = fs::read_to_string(repo.join(PR_EVIDENCE_JSON))
        .with_context(|| format!("missing or unreadable {PR_EVIDENCE_JSON}"))?;
    let packet: Value =
        serde_json::from_str(&text).with_context(|| format!("{PR_EVIDENCE_JSON} is invalid"))?;
    validate_pr_evidence_packet(
        &packet,
        options,
        changed_files.len(),
        repo.join(PR_EVIDENCE_MD).exists(),
        &base_sha,
        &head_sha,
    )?;
    println!("PR evidence contract ok: {PR_EVIDENCE_JSON}");
    Ok(())
}

fn run_ripr_check(repo: &Path, options: &PrEvidenceOptions) -> Result<String> {
    let diff = repo.join(PR_DIFF).display().to_string();
    let root = command_root_arg(repo, &options.root)?;
    run_ripr(&[
        "check".to_string(),
        "--root".to_string(),
        root,
        "--diff".to_string(),
        diff,
        "--format".to_string(),
        "json".to_string(),
    ])
}

#[derive(Debug, Default)]
struct RiprPrSummaryCounts {
    weakly_exposed: usize,
    reachable_unrevealed: usize,
    no_static_path: usize,
    suppressed_by_policy: usize,
}

fn ripr_pr_summary_counts(
    check_value: &Value,
    check_summary: Option<&Map<String, Value>>,
    suppressions: &RiprSuppressionRules,
) -> RiprPrSummaryCounts {
    let summary_counts = RiprPrSummaryCounts {
        weakly_exposed: count_field(check_summary, "weakly_exposed"),
        reachable_unrevealed: count_field(check_summary, "reachable_unrevealed"),
        no_static_path: count_field(check_summary, "no_static_path"),
        suppressed_by_policy: 0,
    };
    let Some(findings) = check_value.get("findings").and_then(Value::as_array) else {
        return summary_counts;
    };

    let mut suppressed = RiprPrSummaryCounts::default();
    let mut unsuppressed_from_findings = RiprPrSummaryCounts::default();
    for finding in findings {
        // ripr 0.5.x: "classification" field, values "weakly_exposed" | "reachable_unrevealed" | "no_static_path".
        // ripr 0.9.x: "grip_class" field, values "weakly_gripped" | "reachable_unrevealed" | "no_static_path".
        //   "weakly_gripped" findings are counted in summary.reachable_unrevealed in 0.9.x.
        // Accept both so suppression policy applies across ripr versions.
        let raw_class = finding
            .get("classification")
            .and_then(Value::as_str)
            .or_else(|| finding.get("grip_class").and_then(Value::as_str));
        // Map to the canonical summary-counter name for correct bucket subtraction.
        let canonical: Option<&str> = match raw_class {
            Some("weakly_exposed") => Some("weakly_exposed"),
            // ripr 0.9.x: weakly_gripped is reported under summary.reachable_unrevealed
            Some("weakly_gripped") => Some("reachable_unrevealed"),
            Some("reachable_unrevealed") => Some("reachable_unrevealed"),
            Some("no_static_path") => Some("no_static_path"),
            _ => None,
        };
        let Some(canonical) = canonical else {
            continue;
        };
        let counts = if suppression_matches_finding(suppressions, finding) {
            suppressed.suppressed_by_policy += 1;
            &mut suppressed
        } else {
            &mut unsuppressed_from_findings
        };
        match canonical {
            "weakly_exposed" => counts.weakly_exposed += 1,
            "reachable_unrevealed" => counts.reachable_unrevealed += 1,
            "no_static_path" => counts.no_static_path += 1,
            _ => {}
        }
    }
    if check_summary.is_some() {
        return RiprPrSummaryCounts {
            weakly_exposed: summary_counts.weakly_exposed.saturating_sub(suppressed.weakly_exposed),
            reachable_unrevealed: summary_counts
                .reachable_unrevealed
                .saturating_sub(suppressed.reachable_unrevealed),
            no_static_path: summary_counts.no_static_path.saturating_sub(suppressed.no_static_path),
            suppressed_by_policy: suppressed.suppressed_by_policy,
        };
    }
    RiprPrSummaryCounts {
        suppressed_by_policy: suppressed.suppressed_by_policy,
        ..unsuppressed_from_findings
    }
}

fn suppression_matches_finding(rules: &RiprSuppressionRules, finding: &Value) -> bool {
    let Some(path) = ripr_finding_path(finding) else {
        return false;
    };
    let path = normalize_suppression_match_path(&path);
    rules.path_patterns.iter().zip(rules.display_patterns.iter()).any(|(pattern, pattern_text)| {
        pattern.matches(&path) || suppression_directory_pattern_matches(pattern_text, &path)
    })
}

fn suppression_directory_pattern_matches(pattern: &str, path: &str) -> bool {
    let Some(prefix) = pattern.strip_suffix("/**").or_else(|| pattern.strip_suffix("/*")) else {
        return false;
    };
    let prefix = prefix.trim_end_matches('/');
    path == prefix || path.strip_prefix(prefix).is_some_and(|rest| rest.starts_with('/'))
}

fn ripr_finding_path(finding: &Value) -> Option<String> {
    // ripr 0.5.x: path lives under finding["probe"]["path"] or finding["probe"]["file"].
    // ripr 0.9.x: path lives under finding["seam"]["file"] (and probe may be absent).
    // Accept both so suppression policy applies across ripr versions.
    finding
        .get("probe")
        .and_then(|probe| {
            ["path", "file"].into_iter().find_map(|key| probe.get(key).and_then(Value::as_str))
        })
        .map(normalize_path_text)
        .or_else(|| {
            finding
                .get("seam")
                .and_then(|seam| {
                    ["file", "path"]
                        .into_iter()
                        .find_map(|key| seam.get(key).and_then(Value::as_str))
                })
                .map(normalize_path_text)
        })
        .or_else(|| ripr_plus_seam_path(finding))
        .filter(|path| !path.trim().is_empty())
}

fn normalize_suppression_match_path(path: &str) -> String {
    let normalized = normalize_path_text(path);
    let normalized = normalized.strip_prefix("./").unwrap_or(&normalized);
    ["crates/", "docs/", "archive/", "xtask/", "scripts/", "policy/", ".ci/"]
        .into_iter()
        .filter_map(|anchor| normalized.find(anchor))
        .min()
        .map_or_else(|| normalized.to_string(), |index| normalized[index..].to_string())
}

fn pr_evidence_packet(
    options: &PrEvidenceOptions,
    changed_files: &[String],
    check_value: &Value,
    base_sha: &str,
    head_sha: &str,
    suppressions: &RiprSuppressionRules,
) -> Value {
    let check_summary = check_value.get("summary").and_then(Value::as_object);
    let summary = ripr_pr_summary_counts(check_value, check_summary, suppressions);
    let weakly_exposed = summary.weakly_exposed;
    let reachable_unrevealed = summary.reachable_unrevealed;
    let no_static_path = summary.no_static_path;
    let severe_gaps =
        weakly_exposed.saturating_add(reachable_unrevealed).saturating_add(no_static_path);
    let ripr_severe_gap = severe_gaps > 0;
    let warnings = if check_summary.is_some() {
        Vec::new()
    } else {
        vec![json!({
            "kind": "invalid_json",
            "message": "RIPR check output did not include a summary object.",
            "path": null
        })]
    };
    json!({
        "schema_version": "0.1",
        "tool": "ripr",
        "kind": "pr_evidence",
        "scope": "diff",
        "status": if warnings.is_empty() { "advisory" } else { "incomplete" },
        "root": options.root,
        "base": options.base,
        "base_sha": base_sha,
        "head": options.head,
        "head_sha": head_sha,
        "summary": {
            "changed_files": changed_files.len(),
            "comments": 0,
            "summary_only": 0,
            "suppressed": 0,
            "weakly_exposed": weakly_exposed,
            "reachable_unrevealed": reachable_unrevealed,
            "no_static_path": no_static_path,
            "severe_gaps": severe_gaps,
            "requires_targeted_mutation": ripr_severe_gap,
            "ripr_severe_gap": ripr_severe_gap,
            "routing_reason": if ripr_severe_gap { json!("ripr severe gap") } else { Value::Null },
            "suppressed_by_policy": summary.suppressed_by_policy,
            "suppression_patterns": suppressions.display_patterns.clone(),
        },
        "artifacts": [
            {
                "label": "PR evidence JSON",
                "path": PR_EVIDENCE_JSON,
                "kind": "json",
                "scope": "diff",
                "available": true,
                "required": true
            },
            {
                "label": "PR evidence Markdown",
                "path": PR_EVIDENCE_MD,
                "kind": "markdown",
                "scope": "diff",
                "available": true
            },
            {
                "label": "Analyzed PR diff",
                "path": PR_DIFF,
                "kind": "other",
                "scope": "diff",
                "available": true
            }
        ],
        "warnings": warnings,
        "advisory_limits": [
            "RIPR evidence is static and advisory by default.",
            "This packet does not post review comments or execute mutation.",
            "Public badge state must not be derived from this diff-scoped packet."
        ]
    })
}

fn validate_pr_evidence_packet(
    packet: &Value,
    options: &PrEvidenceOptions,
    expected_changed_files: usize,
    markdown_exists: bool,
    expected_base_sha: &str,
    expected_head_sha: &str,
) -> Result<()> {
    let mut violations = Vec::new();
    expect_string(packet, "schema_version", "0.1", &mut violations);
    expect_string(packet, "tool", "ripr", &mut violations);
    expect_string(packet, "kind", "pr_evidence", &mut violations);
    expect_string(packet, "scope", "diff", &mut violations);
    expect_string(packet, "root", &options.root, &mut violations);
    expect_string(packet, "base", &options.base, &mut violations);
    expect_string(packet, "base_sha", expected_base_sha, &mut violations);
    expect_string(packet, "head", &options.head, &mut violations);
    expect_string(packet, "head_sha", expected_head_sha, &mut violations);
    match packet.get("status").and_then(Value::as_str) {
        Some("advisory" | "incomplete" | "error") => {}
        Some(other) => violations.push(format!("status {other:?} is not valid")),
        None => violations.push("status is missing or not a string".to_string()),
    }
    let Some(summary) = packet.get("summary").and_then(Value::as_object) else {
        bail!("summary is missing or not an object");
    };
    for key in [
        "comments",
        "summary_only",
        "suppressed",
        "weakly_exposed",
        "reachable_unrevealed",
        "no_static_path",
        "severe_gaps",
    ] {
        if !summary.get(key).is_some_and(Value::is_u64) {
            violations.push(format!("summary.{key} is missing or not an integer"));
        }
    }
    match summary.get("changed_files").and_then(Value::as_u64) {
        Some(value) if value == expected_changed_files as u64 => {}
        Some(value) => violations
            .push(format!("summary.changed_files is {value}, expected {expected_changed_files}")),
        None => violations.push("summary.changed_files is missing or not an integer".to_string()),
    }
    for key in ["requires_targeted_mutation", "ripr_severe_gap"] {
        if !summary.get(key).is_some_and(Value::is_boolean) {
            violations.push(format!("summary.{key} is missing or not a boolean"));
        }
    }
    if !(summary.get("routing_reason").is_some_and(Value::is_string)
        || summary.get("routing_reason").is_some_and(Value::is_null))
    {
        violations.push("summary.routing_reason must be string or null".to_string());
    }
    if !packet.get("warnings").is_some_and(Value::is_array) {
        violations.push("warnings is missing or not an array".to_string());
    }
    match packet.get("advisory_limits").and_then(Value::as_array) {
        Some(values) if !values.is_empty() => {}
        _ => violations.push("advisory_limits is missing or empty".to_string()),
    }
    for required in [PR_EVIDENCE_JSON, PR_EVIDENCE_MD] {
        if !artifact_available(packet, required) {
            violations.push(format!("artifacts[] is missing {required}"));
        }
    }
    if !markdown_exists {
        violations.push(format!("{PR_EVIDENCE_MD} is missing"));
    }
    if violations.is_empty() {
        Ok(())
    } else {
        bail!("PR evidence contract violations:\n{}", bullet_list(&violations));
    }
}

fn render_pr_evidence_markdown(packet: &Value) -> String {
    let summary = packet.get("summary").and_then(Value::as_object);
    let mut out = String::new();
    out.push_str("# PR Evidence Summary\n\n");
    out.push_str("## Fast Gate\n\n");
    out.push_str(&format!("- status: `{}`\n", string_field(packet, "status", "unknown")));
    out.push_str(&format!("- root: `{}`\n", md_escape(string_field(packet, "root", DEFAULT_ROOT))));
    out.push_str(&format!("- base: `{}`\n", md_escape(string_field(packet, "base", DEFAULT_BASE))));
    out.push_str(&format!("- head: `{}`\n", md_escape(string_field(packet, "head", DEFAULT_HEAD))));
    out.push_str(&format!("- changed files: {}\n\n", count_field(summary, "changed_files")));
    out.push_str("## RIPR\n\n");
    out.push_str(&format!("- changed-line comments: {}\n", count_field(summary, "comments")));
    out.push_str(&format!("- summary-only guidance: {}\n", count_field(summary, "summary_only")));
    out.push_str(&format!("- suppressed guidance: {}\n", count_field(summary, "suppressed")));
    out.push_str(&format!("- weakly_exposed: {}\n", count_field(summary, "weakly_exposed")));
    out.push_str(&format!(
        "- reachable_unrevealed: {}\n",
        count_field(summary, "reachable_unrevealed")
    ));
    out.push_str(&format!("- no_static_path: {}\n", count_field(summary, "no_static_path")));
    out.push_str(&format!(
        "- suppressed_by_policy: {}\n",
        count_field(summary, "suppressed_by_policy")
    ));
    out.push_str(&format!("- severe gaps: {}\n\n", count_field(summary, "severe_gaps")));
    out.push_str("## Targeted Mutation\n\n");
    out.push_str(&format!(
        "- requires_targeted_mutation: {}\n",
        bool_field(summary, "requires_targeted_mutation")
    ));
    out.push_str(&format!(
        "- routing_reason: `{}`\n\n",
        summary_string_or_null(summary, "routing_reason")
    ));
    out.push_str("## Artifacts\n\n");
    out.push_str("| Artifact | Path | Scope | Available |\n");
    out.push_str("| --- | --- | --- | --- |\n");
    if let Some(artifacts) = packet.get("artifacts").and_then(Value::as_array) {
        for artifact in artifacts {
            out.push_str(&format!(
                "| {} | `{}` | {} | {} |\n",
                md_escape(string_field(artifact, "label", "artifact")),
                md_escape(string_field(artifact, "path", "unknown")),
                md_escape(string_field(artifact, "scope", "unknown")),
                artifact.get("available").and_then(Value::as_bool).unwrap_or(false)
            ));
        }
    }
    out.push_str(
        "\n_This packet is diff-scoped and advisory. Do not copy it into public badge state._\n",
    );
    out
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReviewCommentsOptions {
    root: String,
    base: String,
    head: String,
    timeout_seconds: Option<u64>,
}

fn write_review_comments(repo: &Path, options: &ReviewCommentsOptions) -> Result<()> {
    verify_revision(repo, &options.base)?;
    verify_revision(repo, &options.head)?;
    let root = command_root_arg(repo, &options.root)?;
    if current_pr_evidence_has_no_severe_gaps(repo, options)? {
        write_clean_review_comments(repo, options, &root)?;
    } else if let Err(err) = run_ripr_review_comments(repo, options, &root) {
        write_error_review_comments(repo, options, &root, &err.to_string())?;
    }
    stamp_review_comments_receipt(repo, options)?;
    validate_review_comments(repo, options, true)?;
    println!("Wrote {REVIEW_COMMENTS_JSON}");
    println!("Wrote {REVIEW_COMMENTS_MD}");
    Ok(())
}

fn check_review_comments(repo: &Path, options: &ReviewCommentsOptions) -> Result<()> {
    verify_revision(repo, &options.base)?;
    verify_revision(repo, &options.head)?;
    validate_review_comments(repo, options, true)?;
    println!("Review comments contract ok: {REVIEW_COMMENTS_JSON}");
    Ok(())
}

fn run_ripr_review_comments(
    repo: &Path,
    options: &ReviewCommentsOptions,
    root: &str,
) -> Result<()> {
    let out = repo.join(REVIEW_COMMENTS_JSON);
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let out_arg = out.display().to_string();
    run_ripr_with_timeout(
        &[
            "review-comments".to_string(),
            "--root".to_string(),
            root.to_string(),
            "--base".to_string(),
            options.base.clone(),
            "--head".to_string(),
            options.head.clone(),
            "--out".to_string(),
            out_arg,
        ],
        options.timeout_seconds,
    )
    .map(|_| ())
}

fn current_pr_evidence_has_no_severe_gaps(
    repo: &Path,
    options: &ReviewCommentsOptions,
) -> Result<bool> {
    let path = repo.join(PR_EVIDENCE_JSON);
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(false);
    };
    let Ok(packet) = serde_json::from_str::<Value>(&text) else {
        return Ok(false);
    };
    if packet.get("base").and_then(Value::as_str) != Some(options.base.as_str())
        || packet.get("head").and_then(Value::as_str) != Some(options.head.as_str())
    {
        return Ok(false);
    }
    let base_sha = revision_sha(repo, &options.base)?;
    let head_sha = revision_sha(repo, &options.head)?;
    if packet.get("base_sha").and_then(Value::as_str) != Some(base_sha.as_str())
        || packet.get("head_sha").and_then(Value::as_str) != Some(head_sha.as_str())
    {
        return Ok(false);
    }
    Ok(packet.pointer("/summary/severe_gaps").and_then(Value::as_u64) == Some(0))
}

fn validate_review_comments(
    repo: &Path,
    options: &ReviewCommentsOptions,
    markdown_required: bool,
) -> Result<()> {
    let text = fs::read_to_string(repo.join(REVIEW_COMMENTS_JSON))
        .with_context(|| format!("missing or unreadable {REVIEW_COMMENTS_JSON}"))?;
    let packet: Value = serde_json::from_str(&text)
        .with_context(|| format!("{REVIEW_COMMENTS_JSON} is not valid JSON"))?;
    let mut violations = Vec::new();
    expect_string(&packet, "schema_version", "0.1", &mut violations);
    expect_string(&packet, "tool", "ripr", &mut violations);
    expect_string(&packet, "base", &options.base, &mut violations);
    expect_string(&packet, "base_sha", &revision_sha(repo, &options.base)?, &mut violations);
    expect_string(&packet, "head", &options.head, &mut violations);
    expect_string(&packet, "head_sha", &revision_sha(repo, &options.head)?, &mut violations);
    match packet.get("status").and_then(Value::as_str) {
        Some("advisory" | "incomplete" | "error") => {}
        Some(other) => violations.push(format!("status {other:?} is not valid")),
        None => violations.push("status is missing or not a string".to_string()),
    }
    for key in ["comments", "summary_only", "suppressed", "warnings"] {
        if !packet.get(key).is_some_and(Value::is_array) {
            violations.push(format!("{key} is missing or not an array"));
        }
    }
    if !packet.get("summary").is_some_and(Value::is_object) {
        violations.push("summary is missing or not an object".to_string());
    }
    if markdown_required && !repo.join(REVIEW_COMMENTS_MD).exists() {
        violations.push(format!("{REVIEW_COMMENTS_MD} is missing"));
    }
    if violations.is_empty() {
        Ok(())
    } else {
        bail!("review comments contract violations:\n{}", bullet_list(&violations));
    }
}

fn stamp_review_comments_receipt(repo: &Path, options: &ReviewCommentsOptions) -> Result<()> {
    let path = repo.join(REVIEW_COMMENTS_JSON);
    let text = fs::read_to_string(&path)
        .with_context(|| format!("missing or unreadable {REVIEW_COMMENTS_JSON}"))?;
    let mut packet: Value = serde_json::from_str(&text)
        .with_context(|| format!("{REVIEW_COMMENTS_JSON} is not valid JSON"))?;
    let Some(object) = packet.as_object_mut() else {
        bail!("{REVIEW_COMMENTS_JSON} is not a JSON object");
    };
    object.insert("base_sha".to_string(), json!(revision_sha(repo, &options.base)?));
    object.insert("head_sha".to_string(), json!(revision_sha(repo, &options.head)?));
    write_text(&path, &format_json(&packet)?)
}

fn write_clean_review_comments(
    repo: &Path,
    options: &ReviewCommentsOptions,
    root: &str,
) -> Result<()> {
    let packet = json!({
        "schema_version": "0.1",
        "tool": "ripr",
        "status": "advisory",
        "root": normalize_path_text(root),
        "base": options.base,
        "head": options.head,
        "mode": "pr_evidence_clean",
        "rendering_limits": {
            "max_inline_comments": 0,
            "max_summary_items": 0
        },
        "summary": {
            "comments": 0,
            "summary_only": 0,
            "suppressed": 0,
            "unchanged_tests": true,
            "source": "pr_evidence",
            "skip_reason": "pr_evidence_zero_severe_gaps"
        },
        "comments": [],
        "summary_only": [],
        "suppressed": [],
        "warnings": [],
        "limits_note": "Review guidance generation skipped because diff-scoped PR evidence reported zero severe gaps."
    });
    write_text(&repo.join(REVIEW_COMMENTS_JSON), &format_json(&packet)?)?;
    write_text(&repo.join(REVIEW_COMMENTS_MD), &render_clean_review_comments_markdown(&packet))
}

fn render_clean_review_comments_markdown(packet: &Value) -> String {
    format!(
        "# RIPR PR Guidance\n\n- status: advisory\n- base: `{}`\n- head: `{}`\n- line annotations: 0\n- summary-only recommendations: 0\n- suppressed recommendations: 0\n\nNo review guidance was generated because diff-scoped PR evidence reported zero severe gaps.\n",
        string_field(packet, "base", DEFAULT_BASE),
        string_field(packet, "head", DEFAULT_HEAD)
    )
}

fn write_error_review_comments(
    repo: &Path,
    options: &ReviewCommentsOptions,
    root: &str,
    error: &str,
) -> Result<()> {
    let packet = json!({
        "schema_version": "0.1",
        "tool": "ripr",
        "status": "error",
        "root": normalize_path_text(root),
        "base": options.base,
        "head": options.head,
        "mode": "fast",
        "rendering_limits": {
            "max_inline_comments": 0,
            "max_summary_items": 0
        },
        "summary": {
            "comments": 0,
            "summary_only": 0,
            "suppressed": 0,
            "unchanged_tests": true
        },
        "comments": [],
        "summary_only": [],
        "suppressed": [],
        "warnings": [
            {
                "kind": "tool_error",
                "message": first_line(error),
                "path": null
            }
        ],
        "limits_note": "Review guidance generation is advisory. The producer did not complete, so no comments are emitted."
    });
    write_text(&repo.join(REVIEW_COMMENTS_JSON), &format_json(&packet)?)?;
    write_text(&repo.join(REVIEW_COMMENTS_MD), &render_error_review_comments_markdown(&packet))
}

fn render_error_review_comments_markdown(packet: &Value) -> String {
    let warning = packet
        .get("warnings")
        .and_then(Value::as_array)
        .and_then(|warnings| warnings.first())
        .and_then(|warning| warning.get("message"))
        .and_then(Value::as_str)
        .unwrap_or("review guidance generation did not complete");
    format!(
        "# RIPR PR Guidance\n\n- status: error\n- base: `{}`\n- head: `{}`\n- line annotations: 0\n- summary-only recommendations: 0\n- suppressed recommendations: 0\n\nNo review guidance was generated.\n\n## Warnings\n\n- tool_error: {}\n",
        string_field(packet, "base", DEFAULT_BASE),
        string_field(packet, "head", DEFAULT_HEAD),
        md_escape(warning)
    )
}

fn render_pr_evidence_summary(repo: &Path) -> String {
    let pr_evidence = load_json(repo, PR_EVIDENCE_JSON);
    let review_comments = load_json(repo, REVIEW_COMMENTS_JSON);
    let impacted = load_json(repo, IMPACTED_JSON);
    let pr_value = pr_evidence.value.as_ref();
    let review_value = review_comments.value.as_ref();
    let impacted_value = impacted.value.as_ref();
    let mut out = String::new();
    out.push_str("# PR Evidence Summary\n\n");
    out.push_str("## Fast Gate\n\n");
    out.push_str(&format!("- PR evidence JSON: {}\n", pr_evidence.state));
    out.push_str(&format!("- review guidance JSON: {}\n", review_comments.state));
    out.push_str(&format!("- impacted evidence JSON: {}\n", impacted.state));
    out.push_str(&format!("- PR evidence status: `{}`\n", option_string_field(pr_value, "status")));
    out.push_str(&format!(
        "- review guidance status: `{}`\n",
        option_string_field(review_value, "status")
    ));
    out.push_str(&format!("- base: `{}`\n", option_string_field(pr_value, "base")));
    out.push_str(&format!("- head: `{}`\n", option_string_field(pr_value, "head")));
    out.push_str(&format!(
        "- changed files: {}\n\n",
        option_summary_u64(pr_value, "changed_files")
    ));
    out.push_str("## RIPR\n\n");
    out.push_str(&format!(
        "- changed-line comments: {}\n",
        option_summary_u64(review_value.or(pr_value), "comments")
    ));
    out.push_str(&format!(
        "- summary-only guidance: {}\n",
        option_summary_u64(review_value.or(pr_value), "summary_only")
    ));
    out.push_str(&format!(
        "- suppressed guidance: {}\n",
        option_summary_u64(review_value.or(pr_value), "suppressed")
    ));
    out.push_str(&format!(
        "- weakly_exposed: {}\n",
        option_summary_u64(pr_value, "weakly_exposed")
    ));
    out.push_str(&format!(
        "- reachable_unrevealed: {}\n",
        option_summary_u64(pr_value, "reachable_unrevealed")
    ));
    out.push_str(&format!(
        "- no_static_path: {}\n",
        option_summary_u64(pr_value, "no_static_path")
    ));
    out.push_str(&format!("- severe gaps: {}\n\n", option_summary_u64(pr_value, "severe_gaps")));
    out.push_str("## Targeted Mutation\n\n");
    out.push_str(&format!(
        "- mutation_mode: `{}`\n",
        option_summary_string(impacted_value, "mutation_mode")
    ));
    out.push_str(&format!(
        "- requires_targeted_mutation: {}\n",
        option_summary_bool(impacted_value.or(pr_value), "requires_targeted_mutation")
    ));
    out.push_str(&format!(
        "- ripr_severe_gap: {}\n",
        option_summary_bool(impacted_value.or(pr_value), "ripr_severe_gap")
    ));
    out.push_str(&format!(
        "- routing_reason: `{}`\n\n",
        option_summary_string_or_null(impacted_value.or(pr_value), "routing_reason")
    ));
    out.push_str("## Artifacts\n\n");
    out.push_str("| Artifact | Path | State |\n");
    out.push_str("| --- | --- | --- |\n");
    out.push_str(&format!("| PR evidence JSON | `{PR_EVIDENCE_JSON}` | {} |\n", pr_evidence.state));
    out.push_str(&format!(
        "| PR evidence Markdown | `{PR_EVIDENCE_MD}` | {} |\n",
        file_state(repo, PR_EVIDENCE_MD)
    ));
    out.push_str(&format!(
        "| Review guidance JSON | `{REVIEW_COMMENTS_JSON}` | {} |\n",
        review_comments.state
    ));
    out.push_str(&format!(
        "| Review guidance Markdown | `{REVIEW_COMMENTS_MD}` | {} |\n",
        file_state(repo, REVIEW_COMMENTS_MD)
    ));
    out.push_str(&format!("| Impacted evidence JSON | `{IMPACTED_JSON}` | {} |\n", impacted.state));
    out.push_str(&format!(
        "| Impacted evidence Markdown | `{IMPACTED_MD}` | {} |\n",
        file_state(repo, IMPACTED_MD)
    ));
    out.push_str(&format!("| PR evidence summary Markdown | `{PR_SUMMARY_MD}` | generated |\n"));
    out.push_str("\n_This summary is generated from diff-scoped artifacts. Do not copy it into public badge state._\n");
    out
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct JsonInput {
    state: InputState,
    value: Option<Value>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum InputState {
    Present,
    Missing,
    Invalid(String),
}

impl std::fmt::Display for InputState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Present => f.write_str("present"),
            Self::Missing => f.write_str("missing"),
            Self::Invalid(err) => write!(f, "invalid: {}", md_escape(err)),
        }
    }
}

fn load_json(repo: &Path, relative: &str) -> JsonInput {
    let path = repo.join(relative);
    let Ok(text) = fs::read_to_string(&path) else {
        return JsonInput { state: InputState::Missing, value: None };
    };
    match serde_json::from_str::<Value>(&text) {
        Ok(value) => JsonInput { state: InputState::Present, value: Some(value) },
        Err(err) => {
            JsonInput { state: InputState::Invalid(first_line(&err.to_string())), value: None }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AnnotationOutput {
    text: String,
    comments_missing: bool,
}

fn render_annotations(repo: &Path, comments: &str) -> Result<AnnotationOutput> {
    let comments_path = repo.join(comments);
    if !comments_path.exists() {
        return Ok(AnnotationOutput { text: String::new(), comments_missing: true });
    }
    let text =
        fs::read_to_string(&comments_path).with_context(|| format!("failed to read {comments}"))?;
    let packet: Value =
        serde_json::from_str(&text).with_context(|| format!("{comments} is not valid JSON"))?;
    let comments_array = packet
        .get("comments")
        .and_then(Value::as_array)
        .ok_or_else(|| eyre!("{comments} is missing comments[]"))?;
    let mut out = String::new();
    for item in comments_array {
        out.push_str(&annotation_from_comment(item)?);
        out.push('\n');
    }
    Ok(AnnotationOutput { text: out, comments_missing: false })
}

fn annotation_from_comment(item: &Value) -> Result<String> {
    let placement = item
        .get("placement")
        .and_then(Value::as_object)
        .ok_or_else(|| eyre!("comments[] item is missing placement object"))?;
    let path = string_key(placement, "path")?;
    let line = placement
        .get("line")
        .and_then(Value::as_u64)
        .ok_or_else(|| eyre!("comments[] placement.line is missing or not an integer"))?;
    let mode = string_key(placement, "mode")?;
    if !matches!(
        mode.as_str(),
        "exact_seam_line" | "owner_function_changed_line" | "same_file_changed_line"
    ) {
        bail!("comments[] placement mode {mode:?} is not annotation-safe");
    }
    let severity = item.get("severity").and_then(Value::as_str).unwrap_or("advisory");
    let kind = item.get("kind").and_then(Value::as_str).unwrap_or("focused_test");
    let reason = item.get("reason").and_then(Value::as_str).unwrap_or("RIPR review guidance");
    let intent =
        item.get("suggested_test").and_then(|test| test.get("intent")).and_then(Value::as_str);
    let mut message = reason.to_string();
    if let Some(intent) = intent {
        message.push_str(" Suggested test: ");
        message.push_str(intent);
    }
    Ok(format!(
        "::warning file={},line={},title={}::{}",
        escape_cmd(&path),
        line,
        escape_cmd(&format!("ripr {severity} {kind}")),
        escape_cmd(&message)
    ))
}

fn string_key(object: &Map<String, Value>, key: &str) -> Result<String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| eyre!("comments[] placement.{key} is missing or empty"))
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ImpactedEvidenceOptions {
    pr_evidence: String,
    labels: Vec<String>,
}

fn impacted_evidence_packet(repo: &Path, options: &ImpactedEvidenceOptions) -> Value {
    let input = load_pr_evidence(repo, &options.pr_evidence);
    let ripr_severe_gap = input
        .value
        .as_ref()
        .and_then(|value| value.pointer("/summary/ripr_severe_gap"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let pr_requires_targeted = input
        .value
        .as_ref()
        .and_then(|value| value.pointer("/summary/requires_targeted_mutation"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let decision = routing_decision(&options.labels, ripr_severe_gap || pr_requires_targeted);
    let warnings = input.warning(&options.pr_evidence);
    json!({
        "schema_version": "0.1",
        "tool": "ripr",
        "kind": "impacted_evidence",
        "scope": "diff",
        "status": if warnings.is_empty() { "advisory" } else { "incomplete" },
        "inputs": {
            "pr_evidence": options.pr_evidence,
            "labels": options.labels
        },
        "summary": {
            "mutation_mode": decision.mode,
            "requires_targeted_mutation": decision.requires_targeted_mutation,
            "requires_full_owner_mutation": decision.requires_full_owner_mutation,
            "ripr_severe_gap": ripr_severe_gap,
            "routing_reason": decision.reason
        },
        "artifacts": [
            {
                "label": "impacted evidence JSON",
                "path": IMPACTED_JSON,
                "kind": "json",
                "scope": "diff",
                "available": true,
                "required": true
            },
            {
                "label": "impacted evidence Markdown",
                "path": IMPACTED_MD,
                "kind": "markdown",
                "scope": "diff",
                "available": true
            },
            {
                "label": "PR evidence JSON",
                "path": options.pr_evidence,
                "kind": "json",
                "scope": "diff",
                "available": input.value.is_some()
            }
        ],
        "warnings": warnings,
        "advisory_limits": [
            "Impacted evidence routes mutation; it does not execute mutation.",
            "Full-owner mutation requires an explicit mutation/full-owner label."
        ]
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RoutingDecision {
    mode: &'static str,
    requires_targeted_mutation: bool,
    requires_full_owner_mutation: bool,
    reason: Value,
}

fn routing_decision(labels: &[String], ripr_routes_targeted: bool) -> RoutingDecision {
    let has = |needle: &str| labels.iter().any(|label| label == needle);
    if has("mutation/full-owner") {
        return RoutingDecision {
            mode: "full_owner",
            requires_targeted_mutation: false,
            requires_full_owner_mutation: true,
            reason: json!("mutation/full-owner label"),
        };
    }
    if has("mutation") || has("mutation/targeted") {
        return targeted("mutation label");
    }
    if has("release-risk") {
        return targeted("release-risk label");
    }
    if ripr_routes_targeted {
        return targeted("ripr severe gap");
    }
    RoutingDecision {
        mode: "fast_only",
        requires_targeted_mutation: false,
        requires_full_owner_mutation: false,
        reason: Value::Null,
    }
}

fn targeted(reason: &'static str) -> RoutingDecision {
    RoutingDecision {
        mode: "targeted",
        requires_targeted_mutation: true,
        requires_full_owner_mutation: false,
        reason: json!(reason),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PrEvidenceInput {
    value: Option<Value>,
    state: InputState,
}

impl PrEvidenceInput {
    fn warning(&self, path: &str) -> Vec<Value> {
        match &self.state {
            InputState::Present => Vec::new(),
            InputState::Missing => vec![json!({
                "kind": "missing_artifact",
                "message": "PR evidence JSON is missing; mutation routing uses labels only.",
                "path": path
            })],
            InputState::Invalid(err) => vec![json!({
                "kind": "invalid_json",
                "message": format!("PR evidence JSON is invalid: {err}"),
                "path": path
            })],
        }
    }
}

fn load_pr_evidence(repo: &Path, relative: &str) -> PrEvidenceInput {
    let input = load_json(repo, relative);
    PrEvidenceInput { value: input.value, state: input.state }
}

fn render_impacted_evidence_markdown(packet: &Value) -> String {
    let summary = packet.get("summary").and_then(Value::as_object);
    let inputs = packet.get("inputs").and_then(Value::as_object);
    let labels = inputs
        .and_then(|inputs| inputs.get("labels"))
        .and_then(Value::as_array)
        .map(|labels| labels.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", "))
        .filter(|labels| !labels.is_empty())
        .unwrap_or_else(|| "none".to_string());
    let mut out = String::new();
    out.push_str("# Impacted Evidence\n\n");
    out.push_str("## Routing\n\n");
    out.push_str(&format!(
        "- mutation_mode: `{}`\n",
        summary_string(summary, "mutation_mode", "unknown")
    ));
    out.push_str(&format!(
        "- requires_targeted_mutation: {}\n",
        bool_field(summary, "requires_targeted_mutation")
    ));
    out.push_str(&format!(
        "- requires_full_owner_mutation: {}\n",
        bool_field(summary, "requires_full_owner_mutation")
    ));
    out.push_str(&format!("- ripr_severe_gap: {}\n", bool_field(summary, "ripr_severe_gap")));
    out.push_str(&format!(
        "- routing_reason: `{}`\n\n",
        summary_string_or_null(summary, "routing_reason")
    ));
    out.push_str("## Inputs\n\n");
    out.push_str(&format!(
        "- PR evidence: `{}`\n",
        inputs
            .and_then(|inputs| inputs.get("pr_evidence"))
            .and_then(Value::as_str)
            .map(md_escape)
            .unwrap_or_else(|| "not_available".to_string())
    ));
    out.push_str(&format!("- labels: `{}`\n\n", md_escape(&labels)));
    out.push_str("## Artifacts\n\n");
    out.push_str("| Artifact | Path | Available |\n");
    out.push_str("| --- | --- | --- |\n");
    if let Some(artifacts) = packet.get("artifacts").and_then(Value::as_array) {
        for artifact in artifacts {
            out.push_str(&format!(
                "| {} | `{}` | {} |\n",
                md_escape(string_field(artifact, "label", "artifact")),
                md_escape(string_field(artifact, "path", "unknown")),
                artifact.get("available").and_then(Value::as_bool).unwrap_or(false)
            ));
        }
    }
    if let Some(warnings) = packet.get("warnings").and_then(Value::as_array)
        && !warnings.is_empty()
    {
        out.push_str("\n## Warnings\n\n");
        for warning in warnings {
            out.push_str(&format!(
                "- {}: {}\n",
                md_escape(string_field(warning, "kind", "warning")),
                md_escape(string_field(warning, "message", "unknown warning"))
            ));
        }
    }
    out.push_str("\n_This receipt routes verification work. It does not execute mutation._\n");
    out
}

fn merged_labels(labels: &[String], labels_csv: Option<&str>) -> Vec<String> {
    let mut all = Vec::new();
    all.extend(labels.iter().cloned());
    if let Some(csv) = labels_csv {
        all.extend(split_labels(csv));
    }
    if all.is_empty() {
        all.extend(labels_from_env());
    }
    normalize_labels(&all)
}

fn labels_from_env() -> Vec<String> {
    env::var("GITHUB_PR_LABELS")
        .or_else(|_| env::var("PR_LABELS"))
        .map(|labels| split_labels(&labels))
        .unwrap_or_default()
}

fn split_labels(labels: &str) -> Vec<String> {
    labels
        .split([',', '\n', ';'])
        .map(str::trim)
        .filter(|label| !label.is_empty())
        .map(str::to_string)
        .collect()
}

fn normalize_labels(labels: &[String]) -> Vec<String> {
    labels
        .iter()
        .map(|label| label.trim().to_ascii_lowercase())
        .filter(|label| !label.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn verify_revision(repo: &Path, rev: &str) -> Result<()> {
    let commit = format!("{rev}^{{commit}}");
    run_git_output(repo, &["rev-parse", "--verify", commit.as_str()])
        .map(|_| ())
        .with_context(|| format!("bad base/head revision {rev:?}"))
}

fn changed_files(repo: &Path, base: &str, head: &str) -> Result<Vec<String>> {
    let range = format!("{base}...{head}");
    // A shallow clone (Claude Code on the web, default Actions checkout) has no
    // common history, so `base...head` has no merge base and git diff fails with
    // an opaque message. Attach actionable guidance as context while preserving
    // the underlying git error as the cause; CI is unaffected (ripr.yml uses
    // fetch-depth: 0).
    let output =
        run_git_output(repo, &["diff", "--name-only", "--diff-filter=ACMR", range.as_str()])
            .with_context(|| merge_base_failure_guidance(base, head, is_shallow_clone(repo)))?;
    Ok(output.lines().map(str::trim).filter(|line| !line.is_empty()).map(str::to_string).collect())
}

fn is_shallow_clone(repo: &Path) -> bool {
    run_git_output(repo, &["rev-parse", "--is-shallow-repository"])
        .map(|out| out.trim() == "true")
        .unwrap_or(false)
}

fn merge_base_failure_guidance(base: &str, head: &str, shallow: bool) -> String {
    let mut message =
        format!("cannot compute diff range `{base}...{head}`: no merge base between them.");
    if shallow {
        message.push_str(&format!(
            " This checkout is a shallow clone, so `{base}` and `{head}` share no common history. \
             Deepen the clone before running diff-scoped RIPR locally, e.g. \
             `git fetch --unshallow` or `git fetch --deepen=200 origin {base}`. \
             CI is unaffected: the RIPR workflow checks out with fetch-depth: 0."
        ));
    } else {
        message.push_str(&format!(
            " Ensure `{base}` is fetched and shares history with `{head}`, \
             e.g. `git fetch origin {base}`."
        ));
    }
    message
}

fn write_pr_diff(repo: &Path, base: &str, head: &str) -> Result<()> {
    let range = format!("{base}...{head}");
    let diff = run_git_output(repo, &["diff", "--binary", "--no-ext-diff", range.as_str()])?;
    write_text(&repo.join(PR_DIFF), &diff)
}

fn run_git_output(repo: &Path, args: &[&str]) -> Result<String> {
    let mut git_args = vec!["-C".to_string(), repo.display().to_string()];
    git_args.extend(args.iter().map(|arg| (*arg).to_string()));
    run_output("git", &git_args)
}

fn run_ripr(args: &[String]) -> Result<String> {
    let binary = ripr_binary()?;
    run_output(&binary, args)
}

fn run_ripr_with_timeout(args: &[String], timeout_seconds: Option<u64>) -> Result<String> {
    let binary = ripr_binary()?;
    match timeout_seconds {
        Some(seconds) => run_output_with_timeout(&binary, args, Duration::from_secs(seconds)),
        None => run_output(&binary, args),
    }
}

fn ripr_binary() -> Result<String> {
    #[cfg(test)]
    {
        let guard =
            RIPR_BIN_OVERRIDE.lock().map_err(|_| eyre!("RIPR_BIN test override lock poisoned"))?;
        if let Some(binary) = guard.as_ref() {
            return Ok(binary.clone());
        }
    }

    let binary = match env::var("RIPR_BIN") {
        Ok(value) if !value.trim().is_empty() => value,
        Ok(_) => bail!("RIPR_BIN is set but empty"),
        Err(_) => "ripr".to_string(),
    };
    Ok(binary)
}

fn run_output(cmd: &str, args: &[String]) -> Result<String> {
    let output =
        Command::new(cmd).args(args).output().with_context(|| format!("failed to run {cmd}"))?;
    output_to_string(cmd, output)
}

fn run_output_with_timeout(cmd: &str, args: &[String], timeout: Duration) -> Result<String> {
    let mut child = Command::new(cmd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to run {cmd}"))?;
    let started = Instant::now();
    loop {
        if child.try_wait().with_context(|| format!("failed to poll {cmd}"))?.is_some() {
            let output =
                child.wait_with_output().with_context(|| format!("failed to collect {cmd}"))?;
            return output_to_string(cmd, output);
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let output =
                child.wait_with_output().with_context(|| format!("failed to collect {cmd}"))?;
            bail!(
                "{cmd} timed out after {}s\nstdout:\n{}\nstderr:\n{}",
                timeout.as_secs(),
                String::from_utf8_lossy(&output.stdout).trim(),
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

fn output_to_string(cmd: &str, output: std::process::Output) -> Result<String> {
    if !output.status.success() {
        bail!(
            "{cmd} failed with status {}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout).trim(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    String::from_utf8(output.stdout).with_context(|| format!("{cmd} stdout was not UTF-8"))
}

fn command_root_arg(repo: &Path, root: &str) -> Result<String> {
    let repo = repo
        .canonicalize()
        .with_context(|| format!("failed to resolve repository root {}", repo.display()))?;
    let root_path = Path::new(root);
    let candidate =
        if root_path.is_absolute() { root_path.to_path_buf() } else { repo.join(root_path) };
    let canonical = candidate
        .canonicalize()
        .with_context(|| format!("failed to resolve RIPR root {}", candidate.display()))?;
    if !canonical.starts_with(&repo) {
        bail!(
            "RIPR root {} resolves outside repository root {}",
            canonical.display(),
            repo.display()
        );
    }
    Ok(canonical.display().to_string())
}

fn write_text(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(path, text).with_context(|| format!("failed to write {}", path.display()))
}

fn format_json(value: &Value) -> Result<String> {
    Ok(format!("{}\n", serde_json::to_string_pretty(value)?))
}

fn expect_string(packet: &Value, key: &str, expected: &str, violations: &mut Vec<String>) {
    match packet.get(key).and_then(Value::as_str) {
        Some(actual) if actual == expected => {}
        Some(actual) => violations.push(format!("{key} is {actual:?}, expected {expected:?}")),
        None => violations.push(format!("{key} is missing or not a string")),
    }
}

fn count_field(summary: Option<&Map<String, Value>>, key: &str) -> usize {
    summary
        .and_then(|summary| summary.get(key))
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(0)
}

fn bool_field(summary: Option<&Map<String, Value>>, key: &str) -> bool {
    summary.and_then(|summary| summary.get(key)).and_then(Value::as_bool).unwrap_or(false)
}

fn string_field<'a>(packet: &'a Value, key: &str, fallback: &'a str) -> &'a str {
    packet.get(key).and_then(Value::as_str).unwrap_or(fallback)
}

fn option_string_field(value: Option<&Value>, key: &str) -> String {
    value
        .and_then(|value| value.get(key))
        .and_then(Value::as_str)
        .map(md_escape)
        .unwrap_or_else(|| "not_available".to_string())
}

fn option_summary_u64(value: Option<&Value>, key: &str) -> String {
    value
        .and_then(|value| value.get("summary"))
        .and_then(|summary| summary.get(key))
        .and_then(Value::as_u64)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "not_available".to_string())
}

fn option_summary_bool(value: Option<&Value>, key: &str) -> String {
    value
        .and_then(|value| value.get("summary"))
        .and_then(|summary| summary.get(key))
        .and_then(Value::as_bool)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "not_available".to_string())
}

fn option_summary_string(value: Option<&Value>, key: &str) -> String {
    value
        .and_then(|value| value.get("summary"))
        .and_then(|summary| summary.get(key))
        .and_then(Value::as_str)
        .map(md_escape)
        .unwrap_or_else(|| "not_available".to_string())
}

fn option_summary_string_or_null(value: Option<&Value>, key: &str) -> String {
    let Some(value) =
        value.and_then(|value| value.get("summary")).and_then(|summary| summary.get(key))
    else {
        return "not_available".to_string();
    };
    if value.is_null() {
        "none".to_string()
    } else {
        value.as_str().map(md_escape).unwrap_or_else(|| "invalid".to_string())
    }
}

fn summary_string(summary: Option<&Map<String, Value>>, key: &str, fallback: &str) -> String {
    summary
        .and_then(|summary| summary.get(key))
        .and_then(Value::as_str)
        .map(md_escape)
        .unwrap_or_else(|| fallback.to_string())
}

fn summary_string_or_null(summary: Option<&Map<String, Value>>, key: &str) -> String {
    let Some(value) = summary.and_then(|summary| summary.get(key)) else {
        return "not_available".to_string();
    };
    if value.is_null() {
        "none".to_string()
    } else {
        value.as_str().map(md_escape).unwrap_or_else(|| "invalid".to_string())
    }
}

fn artifact_available(packet: &Value, required_path: &str) -> bool {
    packet.get("artifacts").and_then(Value::as_array).is_some_and(|artifacts| {
        artifacts.iter().any(|artifact| {
            artifact.get("path").and_then(Value::as_str) == Some(required_path)
                && artifact.get("scope").and_then(Value::as_str) == Some("diff")
                && artifact.get("available").and_then(Value::as_bool).unwrap_or(false)
        })
    })
}

fn file_state(repo: &Path, relative: &str) -> &'static str {
    if repo.join(relative).exists() { "present" } else { "missing" }
}

fn first_line(value: &str) -> String {
    value.lines().next().unwrap_or("unknown error").trim().to_string()
}

fn md_escape(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn normalize_path_text(value: &str) -> String {
    value.replace('\\', "/")
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn escape_cmd(value: &str) -> String {
    value
        .replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
        .replace(',', "%2C")
        .replace(':', "%3A")
}

fn bullet_list(values: &[String]) -> String {
    values.iter().map(|value| format!("- {value}")).collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_root_arg_allows_repo_relative_root() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let repo = temp.path();
        fs::create_dir(repo.join("crates"))?;

        let root = command_root_arg(repo, "crates")?;

        assert_eq!(PathBuf::from(root), repo.join("crates").canonicalize()?);
        Ok(())
    }

    #[test]
    fn command_root_arg_rejects_absolute_root_outside_repo() -> Result<()> {
        let repo = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        let outside_arg = outside.path().display().to_string();

        assert!(command_root_arg(repo.path(), &outside_arg).is_err());
        Ok(())
    }

    #[test]
    fn command_root_arg_rejects_relative_parent_escape() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let repo = temp.path().join("repo");
        let outside = temp.path().join("outside");
        fs::create_dir(&repo)?;
        fs::create_dir(&outside)?;

        assert!(command_root_arg(&repo, "../outside").is_err());
        Ok(())
    }

    #[test]
    fn ripr_plus_top_files_rank_repo_seams_across_path_shapes() {
        let seams = vec![
            json!({"file": "crates/perl-parser/src/lib.rs"}),
            json!({"path": "crates/perl-lexer/src/lib.rs"}),
            json!({"location": {"path": r"crates\perl-parser\src\lib.rs"}}),
            json!({"placement": {"path": "crates/perl-workspace/src/index.rs"}}),
            json!({"evidence_record": {"path": "crates/perl-lexer/src/lib.rs"}}),
            json!({"file": ""}),
            json!({}),
        ];

        let rows = ripr_plus_top_files(seams.iter(), 2);

        assert_eq!(
            rows,
            vec![
                json!({"name": "crates/perl-lexer/src/lib.rs", "count": 2}),
                json!({"name": "crates/perl-parser/src/lib.rs", "count": 2}),
            ]
        );
    }

    #[test]
    fn ripr_plus_top_gap_kinds_rank_repo_seams_across_kind_shapes() {
        let seams = vec![
            json!({"kind": "ReceiptParsing"}),
            json!({"gap_kind": "BoundaryPredicate"}),
            json!({"classification": ["StaticUnknown", "NoStaticPath"]}),
            json!({"evidence_record": {"kind": "ReceiptParsing"}}),
            json!({"location": {"reason": "BoundaryPredicate"}}),
            json!({"kind": false}),
            json!({"kind": ""}),
            json!({}),
        ];

        let rows = ripr_plus_top_gap_kinds(seams.iter(), 3);

        assert_eq!(
            rows,
            vec![
                json!({"name": "boundarypredicate", "count": 2}),
                json!({"name": "receiptparsing", "count": 2}),
                json!({"name": "staticunknown,nostaticpath", "count": 1}),
            ]
        );
    }

    #[test]
    fn ripr_plus_recommended_clusters_group_files_and_gap_kinds() {
        let top_files = vec![
            json!({"name": "xtask/src/tasks/ripr_evidence.rs", "count": 5}),
            json!({"name": "crates/perl-lsp-quality/src/lib.rs", "count": 4}),
            json!({"name": "crates/perl-config/src/lib.rs", "count": 3}),
            json!({"name": "crates/perl-diagnostics/src/error.rs", "count": 2}),
            json!({"name": "crates/perl-parser/src/lib.rs", "count": 1}),
            json!({"count": 99}),
        ];
        let top_gap_kinds = vec![
            json!({"name": "receipt_missing", "count": 8}),
            json!({"name": "config_parse", "count": 7}),
            json!({"name": "error_variant", "count": 6}),
            json!({"name": "boundary_predicate", "count": 5}),
            json!({"name": "call_presence", "count": 4}),
            json!({"count": 99}),
        ];

        let rows = ripr_plus_recommended_first_clusters(&top_files, &top_gap_kinds, 10);

        assert_eq!(rows[0].pointer("/name"), Some(&json!("ci-report-formatting")));
        assert_eq!(rows[0].pointer("/score"), Some(&json!(12)));
        assert_eq!(rows[0].pointer("/active_file_count"), Some(&json!(4)));
        assert_eq!(rows[0].pointer("/gap_kind_count"), Some(&json!(8)));
        assert!(rows.iter().any(|row| {
            row.pointer("/name") == Some(&json!("proof-infrastructure"))
                && row.pointer("/example_files/0")
                    == Some(&json!("xtask/src/tasks/ripr_evidence.rs"))
        }));
        assert!(rows.iter().any(|row| {
            row.pointer("/name") == Some(&json!("boundary-predicates"))
                && row.pointer("/example_gap_kinds/0") == Some(&json!("boundary_predicate"))
        }));
        assert!(rows.iter().any(|row| row.pointer("/name") == Some(&json!("config-parsing"))));
        assert!(rows.iter().any(|row| row.pointer("/name") == Some(&json!("error-variants"))));
        assert!(
            rows.iter().any(|row| row.pointer("/name") == Some(&json!("active-ripr-inventory")))
        );
    }

    #[test]
    fn ripr_plus_recommended_clusters_keep_proof_infra_when_below_top_files() {
        let mut seams = Vec::new();
        for file_index in 0..12 {
            for _ in 0..3 {
                seams.push(json!({
                    "file": format!("crates/product-{file_index}/src/lib.rs"),
                    "kind": "CallPresence"
                }));
            }
        }
        seams.push(json!({
            "file": "xtask/src/tasks/quality_gate.rs",
            "kind": "CallPresence"
        }));

        let summary = ripr_plus_seam_summary(&seams, &RiprSuppressionRules::default(), 10);

        assert!(
            !summary.top_files.iter().any(|row| {
                row.get("name").and_then(Value::as_str) == Some("xtask/src/tasks/quality_gate.rs")
            }),
            "xtask proof-infra file should sit below the truncated top_files list"
        );
        assert!(
            summary.recommended_first_clusters.iter().any(|row| {
                row.get("name").and_then(Value::as_str) == Some("proof-infrastructure")
                    && row.get("example_files").and_then(Value::as_array).is_some_and(|files| {
                        files
                            .iter()
                            .any(|file| file.as_str() == Some("xtask/src/tasks/quality_gate.rs"))
                    })
            }),
            "cluster recommendations must preserve proof-infra work below the display top list: {:?}",
            summary.recommended_first_clusters
        );
    }

    #[test]
    fn ripr_plus_cluster_mapping_covers_inventory_buckets() {
        assert_eq!(
            ripr_plus_cluster_for_path("xtask/src/tasks/ripr_evidence.rs").0,
            "proof-infrastructure"
        );
        assert_eq!(
            ripr_plus_cluster_for_path("crates/perl-lsp-quality/src/lib.rs").0,
            "ci-report-formatting"
        );
        assert_eq!(ripr_plus_cluster_for_path("crates/perl-config/src/lib.rs").0, "config-parsing");
        assert_eq!(
            ripr_plus_cluster_for_path("crates/perl-diagnostics/src/error.rs").0,
            "error-variants"
        );
        assert_eq!(
            ripr_plus_cluster_for_path("crates/perl-parser/src/lib.rs").0,
            "active-ripr-inventory"
        );

        assert_eq!(ripr_plus_cluster_for_gap_kind("receipt_missing").0, "ci-report-formatting");
        assert_eq!(ripr_plus_cluster_for_gap_kind("config_parse").0, "config-parsing");
        assert_eq!(ripr_plus_cluster_for_gap_kind("error_variant").0, "error-variants");
        assert_eq!(ripr_plus_cluster_for_gap_kind("boundary_predicate").0, "boundary-predicates");
        assert_eq!(ripr_plus_cluster_for_gap_kind("call_presence").0, "active-ripr-inventory");
    }

    #[test]
    fn ripr_plus_seam_summary_splits_active_and_suppressed_paths() -> Result<()> {
        let seams = vec![
            json!({"file": "crates/perl-parser/src/lib.rs", "kind": "BoundaryPredicate"}),
            json!({"path": "archive/crates/perl-parser/src/lib.rs", "kind": "Archived"}),
            json!({"location": {"path": r"docs\project\status\quality.rs", "kind": "Generated"}}),
            json!({"placement": {"path": "crates/perl-parser/src/lib.rs", "kind": "BoundaryPredicate"}}),
            json!({"file": ""}),
        ];
        let suppressions = RiprSuppressionRules {
            display_patterns: vec!["archive/**".to_string(), "docs/project/status/**".to_string()],
            path_patterns: vec![
                Pattern::new("archive/**")?,
                Pattern::new("docs/project/status/**")?,
            ],
            invalid_patterns: Vec::new(),
            suppression_reasons: Vec::new(),
        };

        let summary = ripr_plus_seam_summary(&seams, &suppressions, 10);

        assert_eq!(summary.unresolved, 3);
        assert_eq!(summary.suppressed, 2);
        assert_eq!(
            summary.top_files,
            vec![json!({"name": "crates/perl-parser/src/lib.rs", "count": 2})]
        );
        assert_eq!(
            summary.top_suppressed_files,
            vec![
                json!({"name": "archive/crates/perl-parser/src/lib.rs", "count": 1}),
                json!({"name": "docs/project/status/quality.rs", "count": 1}),
            ]
        );
        assert_eq!(summary.top_gap_kinds, vec![json!({"name": "boundarypredicate", "count": 2})]);
        assert_eq!(
            summary.top_suppressed_gap_kinds,
            vec![json!({"name": "archived", "count": 1}), json!({"name": "generated", "count": 1}),]
        );
        Ok(())
    }

    #[test]
    fn ripr_plus_receipt_packet_reports_active_and_suppressed_totals() {
        let options = RiprPlusOptions {
            root: ".".to_string(),
            suppressions: PathBuf::from("policy/ripr-suppressions.toml"),
        };
        let suppressions = RiprSuppressionRules {
            display_patterns: vec!["archive/**".to_string()],
            path_patterns: Vec::new(),
            invalid_patterns: vec!["archive/[".to_string()],
            suppression_reasons: vec![json!({
                "id": "ripr-suppress-archive",
                "kind": "generated_or_non_production_surface",
                "reason": "Archived source is not active behavior.",
                "paths": ["archive/**"],
            })],
        };
        // Badge supplies the canonical counts; seam summary supplies the triage inventory.
        let badge = json!({
            "basis": "canonical_actionable_gap",
            "counts": {
                "unsuppressed_exposure_gaps": 2,
                "unsuppressed_test_efficiency_findings": 0,
                "suppressed_exposure_gaps": 1,
                "suppressed_test_efficiency_findings": 0
            }
        });
        let packet = ripr_plus_receipt_packet(
            &options,
            "head-sha",
            &suppressions,
            &badge,
            RiprPlusSeamSummary {
                unresolved: 2,
                suppressed: 1,
                top_files: vec![json!({"name": "xtask/src/tasks/ripr_evidence.rs", "count": 2})],
                top_suppressed_files: vec![json!({"name": "archive/old.rs", "count": 1})],
                top_gap_kinds: vec![json!({"name": "receiptparsing", "count": 2})],
                top_suppressed_gap_kinds: vec![json!({"name": "archived", "count": 1})],
                recommended_first_clusters: vec![json!({
                    "name": "proof-infrastructure",
                    "score": 2,
                    "active_file_count": 2,
                    "gap_kind_count": 0,
                    "reason": "Proof tooling, policy, workflow, and report surfaces are owned by this lane.",
                    "example_files": ["xtask/src/tasks/ripr_evidence.rs"],
                    "example_gap_kinds": [],
                })],
            },
        );

        assert_eq!(packet["head"], json!("head-sha"));
        assert_eq!(packet["root"], json!("."));
        // Canonical counts from badge: 2 unsuppressed exposure gaps.
        assert_eq!(packet["unresolved"], json!(2));
        assert_eq!(packet["active_unresolved"], json!(2));
        assert_eq!(packet["suppressed_unresolved"], json!(1));
        assert_eq!(packet["basis"], json!("canonical_actionable_gap"));
        assert_eq!(packet["schema_version"], json!(2));
        // Triage inventory from seam summary.
        assert_eq!(
            packet.pointer("/top_files/0/name"),
            Some(&json!("xtask/src/tasks/ripr_evidence.rs"))
        );
        assert_eq!(
            packet.pointer("/top_active_files/0/name"),
            Some(&json!("xtask/src/tasks/ripr_evidence.rs"))
        );
        assert_eq!(packet.pointer("/top_suppressed_files/0/name"), Some(&json!("archive/old.rs")));
        assert_eq!(packet.pointer("/top_active_gap_kinds/0/name"), Some(&json!("receiptparsing")));
        assert_eq!(packet.pointer("/top_suppressed_gap_kinds/0/name"), Some(&json!("archived")));
        assert_eq!(
            packet.pointer("/recommended_first_clusters/0/name"),
            Some(&json!("proof-infrastructure"))
        );
        assert_eq!(
            packet.pointer("/suppressions/path"),
            Some(&json!("policy/ripr-suppressions.toml"))
        );
        assert_eq!(packet.pointer("/suppressions/path_patterns/0"), Some(&json!("archive/**")));
        assert_eq!(packet.pointer("/suppressions/invalid_patterns/0"), Some(&json!("archive/[")));
        assert_eq!(
            packet.pointer("/suppressions/reasons/0/id"),
            Some(&json!("ripr-suppress-archive"))
        );
        assert_eq!(packet["decision"], json!("advisory"));
    }

    #[test]
    fn ripr_plus_receipt_counts_canonical_gaps_not_raw_seam_inventory() {
        // Regression guard for the 120k-vs-2.7k over-count: the receipt must
        // count canonical_actionable_gap findings from repo-badge-json, never
        // the raw analyzed_seams inventory. A seam that already has a
        // discriminating test is an analyzed seam but not an actionable gap.
        let options = RiprPlusOptions {
            root: ".".to_string(),
            suppressions: PathBuf::from("policy/ripr-suppressions.toml"),
        };
        let suppressions = RiprSuppressionRules::default();
        let badge = json!({
            "basis": "canonical_actionable_gap",
            "counts": {
                "unsuppressed_exposure_gaps": 2722,
                "unsuppressed_test_efficiency_findings": 0,
                "suppressed_exposure_gaps": 0,
                "suppressed_test_efficiency_findings": 0,
                "analyzed_seams": 120408,
                "analyzed_gap_records": 90255
            },
            "reason_counts": { "smoke_oracle_only": 0, "no_assertion_detected": 0 }
        });
        let seam_summary = RiprPlusSeamSummary {
            unresolved: 120408,
            suppressed: 0,
            top_files: vec![],
            top_suppressed_files: vec![],
            top_gap_kinds: vec![],
            top_suppressed_gap_kinds: vec![],
            recommended_first_clusters: vec![],
        };

        let packet =
            ripr_plus_receipt_packet(&options, "head-sha", &suppressions, &badge, seam_summary);

        // unresolved is the actionable-gap count, not the 120_408 raw seam inventory.
        assert_eq!(packet["unresolved"], json!(2722));
        assert_ne!(packet["unresolved"], json!(120_408));
        assert_eq!(packet["basis"], json!("canonical_actionable_gap"));
        assert_eq!(
            packet["source_format"],
            json!(
                "ripr check --format repo-badge-json (counts) + repo-seams-json (triage inventory)"
            )
        );
        assert_eq!(packet.pointer("/counts/analyzed_seams"), Some(&json!(120_408)));
        assert_eq!(packet.pointer("/reason_counts/smoke_oracle_only"), Some(&json!(0)));
    }

    #[test]
    fn ripr_plus_packet_from_raw_parses_and_builds_receipt() -> Result<()> {
        let options = RiprPlusOptions {
            root: ".".to_string(),
            suppressions: PathBuf::from("policy/ripr-suppressions.toml"),
        };
        let badge_raw = json!({
            "basis": "canonical_actionable_gap",
            "counts": {
                "unsuppressed_exposure_gaps": 2722,
                "unsuppressed_test_efficiency_findings": 0,
                "analyzed_seams": 120408
            }
        })
        .to_string();
        let seams_raw = json!({"seams": []}).to_string();

        let packet = ripr_plus_packet_from_raw(
            &options,
            "head-sha",
            &RiprSuppressionRules::default(),
            &badge_raw,
            &seams_raw,
        )?;

        assert_eq!(packet["unresolved"], json!(2722));
        assert_eq!(packet["head"], json!("head-sha"));
        assert_eq!(packet["basis"], json!("canonical_actionable_gap"));
        Ok(())
    }

    #[test]
    fn ripr_plus_packet_invokes_badge_counts_and_seam_inventory_sources() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let repo = temp.path();
        init_git_repo(repo)?;
        fs::create_dir_all(repo.join("policy"))?;
        fs::write(
            repo.join("policy/ripr-suppressions.toml"),
            r#"schema_version = 1
policy = "ripr-suppressions"
owner = "EffortlessMetrics"
status = "advisory"
updated = "2026-05-28"

[[suppress]]
id = "ripr-suppress-archive"
kind = "generated_or_non_production_surface"
paths = ["archive/**"]
reason = "Archived source is not active workspace behavior."
"#,
        )?;
        let ripr = write_fake_ripr_binary(repo)?;
        let _override = override_ripr_bin(&ripr)?;
        let options = RiprPlusOptions {
            root: ".".to_string(),
            suppressions: PathBuf::from("policy/ripr-suppressions.toml"),
        };

        let packet = ripr_plus_packet(repo, &options)?;

        assert_eq!(packet["head"], json!(current_head(repo)?));
        assert_eq!(packet["basis"], json!("canonical_actionable_gap"));
        assert_eq!(packet["unresolved"], json!(9));
        assert_eq!(packet["active_unresolved"], json!(9));
        assert_eq!(packet["suppressed_unresolved"], json!(4));
        assert_eq!(packet.pointer("/counts/unsuppressed_exposure_gaps"), Some(&json!(7)));
        assert_eq!(packet.pointer("/reason_counts/no_assertion_detected"), Some(&json!(7)));
        assert_eq!(
            packet.pointer("/top_files/0/name"),
            Some(&json!("xtask/src/tasks/ripr_evidence.rs"))
        );
        assert_eq!(packet.pointer("/top_suppressed_files/0/name"), Some(&json!("archive/old.rs")));
        assert_eq!(packet.pointer("/top_active_gap_kinds/0/name"), Some(&json!("receipt parsing")));
        let clusters = packet
            .get("recommended_first_clusters")
            .and_then(Value::as_array)
            .ok_or_else(|| eyre!("missing recommended_first_clusters"))?;
        assert!(
            clusters.iter().any(|cluster| {
                cluster.get("name").and_then(Value::as_str) == Some("proof-infrastructure")
            }),
            "xtask seam inventory must recommend the proof-infrastructure cluster: {clusters:?}"
        );
        Ok(())
    }

    #[test]
    fn ripr_plus_packet_from_raw_rejects_invalid_badge_json() {
        let options = RiprPlusOptions {
            root: ".".to_string(),
            suppressions: PathBuf::from("policy/ripr-suppressions.toml"),
        };
        let result = ripr_plus_packet_from_raw(
            &options,
            "head-sha",
            &RiprSuppressionRules::default(),
            "this is not json",
            r#"{"seams":[]}"#,
        );
        assert!(result.is_err(), "invalid repo-badge-json must be rejected");
    }

    #[test]
    fn ripr_plus_packet_from_raw_rejects_invalid_seams_json() {
        let options = RiprPlusOptions {
            root: ".".to_string(),
            suppressions: PathBuf::from("policy/ripr-suppressions.toml"),
        };
        let result = ripr_plus_packet_from_raw(
            &options,
            "head-sha",
            &RiprSuppressionRules::default(),
            r#"{"basis":"canonical_actionable_gap","counts":{}}"#,
            "this is not json",
        );
        assert!(result.is_err(), "invalid repo-seams-json must be rejected");
    }

    #[test]
    fn ripr_plus_packet_from_raw_rejects_missing_seams_array() {
        let options = RiprPlusOptions {
            root: ".".to_string(),
            suppressions: PathBuf::from("policy/ripr-suppressions.toml"),
        };
        let result = ripr_plus_packet_from_raw(
            &options,
            "head-sha",
            &RiprSuppressionRules::default(),
            r#"{"basis":"canonical_actionable_gap","counts":{}}"#,
            r#"{"not_seams": []}"#,
        );
        assert!(result.is_err(), "seams-json without seams[] key must be rejected");
    }

    #[test]
    fn ripr_plus_suppression_rules_match_non_production_paths() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let repo = temp.path();
        fs::create_dir_all(repo.join("policy"))?;
        fs::write(
            repo.join("policy/ripr-suppressions.toml"),
            r#"schema_version = 1
policy = "ripr-suppressions"
owner = "EffortlessMetrics"
status = "advisory"
updated = "2026-05-28"

[[suppress]]
id = "ripr-suppress-archive"
kind = "generated_or_non_production_surface"
paths = ["archive/**"]
reason = "Archived source is not active workspace behavior."

[[suppress]]
id = "ripr-suppress-generated-status-docs"
paths = ["docs/project/status/**"]

[[suppress]]
id = "ripr-suppress-ux-receipt-tests"
kind = "test_receipt_surface"
paths = ["crates/perl-lsp-ux-tests/tests/**"]
reason = "UX receipt tests are proof inputs."
"#,
        )?;

        let rules = read_ripr_suppression_rules(repo, Path::new("policy/ripr-suppressions.toml"))?;

        assert!(suppression_matches_seam(
            &rules,
            &json!({"file": "archive/crates/perl-parser/src/lib.rs"})
        ));
        assert!(suppression_matches_seam(
            &rules,
            &json!({"location": {"path": r"docs\project\status\quality.rs"}})
        ));
        assert!(suppression_matches_finding(
            &rules,
            &json!({"probe": {"file": r".\crates/perl-lsp-ux-tests/tests/ux_scenario_62_project_test_assertion_inline_completion_quality.rs"}})
        ));
        assert!(!suppression_matches_seam(
            &rules,
            &json!({"file": "crates/perl-parser/src/lib.rs"})
        ));
        assert_eq!(
            rules.suppression_reasons[0],
            json!({
                "id": "ripr-suppress-archive",
                "kind": "generated_or_non_production_surface",
                "reason": "Archived source is not active workspace behavior.",
                "paths": ["archive/**"],
            })
        );
        Ok(())
    }

    #[test]
    fn ripr_plus_suppression_rules_reject_invalid_path_globs() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let repo = temp.path();
        fs::create_dir_all(repo.join("policy"))?;
        fs::write(
            repo.join("policy/ripr-suppressions.toml"),
            r#"schema_version = 1
policy = "ripr-suppressions"
owner = "EffortlessMetrics"
status = "advisory"
updated = "2026-05-28"

[[suppress]]
id = "ripr-suppress-invalid"
paths = ["archive/["]
"#,
        )?;

        assert!(
            read_ripr_suppression_rules(repo, Path::new("policy/ripr-suppressions.toml")).is_err()
        );
        Ok(())
    }

    #[test]
    fn mutation_label_routes_targeted() {
        let decision = routing_decision(&["mutation".to_string()], false);
        assert!(decision.requires_targeted_mutation);
        assert!(!decision.requires_full_owner_mutation);
        assert_eq!(decision.mode, "targeted");
    }

    #[test]
    fn full_owner_label_routes_full_owner() {
        let decision = routing_decision(&["mutation/full-owner".to_string()], false);
        assert!(!decision.requires_targeted_mutation);
        assert!(decision.requires_full_owner_mutation);
        assert_eq!(decision.mode, "full_owner");
    }

    #[test]
    fn empty_labels_keep_fast_only() {
        let decision = routing_decision(&[], false);
        assert!(!decision.requires_targeted_mutation);
        assert!(!decision.requires_full_owner_mutation);
        assert_eq!(decision.mode, "fast_only");
    }

    #[test]
    fn normalized_option_uses_default_for_blank_values() {
        assert_eq!(normalized_option("", DEFAULT_ROOT), DEFAULT_ROOT);
        assert_eq!(normalized_option("  ", DEFAULT_BASE), DEFAULT_BASE);
        assert_eq!(normalized_option("HEAD", DEFAULT_HEAD), "HEAD");
    }

    #[test]
    fn revision_sha_reads_current_head() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let repo = temp.path();
        init_git_repo(repo)?;

        let head = revision_sha(repo, "HEAD")?;

        assert_eq!(current_head(repo)?, head);
        assert_eq!(head.len(), 40);
        Ok(())
    }

    #[test]
    fn revision_sha_rejects_missing_revision() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let repo = temp.path();
        init_git_repo(repo)?;

        assert!(revision_sha(repo, "refs/heads/does-not-exist").is_err());
        Ok(())
    }

    #[test]
    fn pr_evidence_packet_carries_revision_shas() -> Result<()> {
        let options = PrEvidenceOptions {
            root: ".".to_string(),
            base: "origin/main".to_string(),
            head: "HEAD".to_string(),
        };
        let check_value = json!({
            "summary": {
                "weakly_exposed": 1,
                "reachable_unrevealed": 0,
                "no_static_path": 0
            }
        });

        let packet = pr_evidence_packet(
            &options,
            &["xtask/src/tasks/ripr_evidence.rs".to_string()],
            &check_value,
            "base-sha",
            "head-sha",
            &RiprSuppressionRules::default(),
        );

        assert_eq!(packet["base_sha"], json!("base-sha"));
        assert_eq!(packet["head_sha"], json!("head-sha"));
        validate_pr_evidence_packet(&packet, &options, 1, true, "base-sha", "head-sha")?;
        Ok(())
    }

    #[test]
    fn pr_evidence_packet_suppresses_non_production_test_receipt_findings() -> Result<()> {
        let options = PrEvidenceOptions {
            root: ".".to_string(),
            base: "origin/main".to_string(),
            head: "HEAD".to_string(),
        };
        let check_value = json!({
            "summary": {
                "weakly_exposed": 1,
                "reachable_unrevealed": 1,
                "no_static_path": 0
            },
            "findings": [
                {
                    "classification": "reachable_unrevealed",
                    "probe": {
                        "file": r".\crates/perl-lsp-ux-tests/tests/ux_scenario_62_project_test_assertion_inline_completion_quality.rs"
                    }
                },
                {
                    "classification": "weakly_exposed",
                    "probe": {
                        "file": "crates/perl-lsp-rs-core/src/providers/inline_completion/mod.rs"
                    }
                }
            ]
        });
        let suppressions = RiprSuppressionRules {
            display_patterns: vec!["crates/perl-lsp-ux-tests/tests/**".to_string()],
            path_patterns: vec![Pattern::new("crates/perl-lsp-ux-tests/tests/**")?],
            invalid_patterns: Vec::new(),
            suppression_reasons: Vec::new(),
        };

        let packet = pr_evidence_packet(
            &options,
            &[
                "crates/perl-lsp-ux-tests/tests/ux_scenario_62_project_test_assertion_inline_completion_quality.rs".to_string(),
                "crates/perl-lsp-rs-core/src/providers/inline_completion/mod.rs".to_string(),
            ],
            &check_value,
            "base-sha",
            "head-sha",
            &suppressions,
        );

        assert_eq!(packet.pointer("/summary/weakly_exposed"), Some(&json!(1)));
        assert_eq!(packet.pointer("/summary/reachable_unrevealed"), Some(&json!(0)));
        assert_eq!(packet.pointer("/summary/severe_gaps"), Some(&json!(1)));
        assert_eq!(packet.pointer("/summary/suppressed_by_policy"), Some(&json!(1)));
        assert_eq!(
            packet.pointer("/summary/suppression_patterns/0"),
            Some(&json!("crates/perl-lsp-ux-tests/tests/**"))
        );
        Ok(())
    }

    #[test]
    fn suppression_matches_windows_probe_paths_under_receipt_test_directory() -> Result<()> {
        let rules = RiprSuppressionRules {
            display_patterns: vec![
                "crates/perl-lsp-ux-tests/tests/*".to_string(),
                "crates/perl-lsp-ux-tests/tests/**".to_string(),
            ],
            path_patterns: vec![
                Pattern::new("crates/perl-lsp-ux-tests/tests/*")?,
                Pattern::new("crates/perl-lsp-ux-tests/tests/**")?,
            ],
            invalid_patterns: Vec::new(),
            suppression_reasons: Vec::new(),
        };
        let finding = json!({
            "classification": "reachable_unrevealed",
            "probe": {
                "file": r".\crates/perl-lsp-ux-tests/tests/ux_scenario_62_project_test_assertion_inline_completion_quality.rs"
            }
        });

        assert!(suppression_matches_finding(&rules, &finding));
        Ok(())
    }

    #[test]
    fn suppression_matches_absolute_probe_paths_under_receipt_test_directory() -> Result<()> {
        let rules = RiprSuppressionRules {
            display_patterns: vec!["crates/perl-lsp-ux-tests/tests/**".to_string()],
            path_patterns: vec![Pattern::new("crates/perl-lsp-ux-tests/tests/**")?],
            invalid_patterns: Vec::new(),
            suppression_reasons: Vec::new(),
        };
        let finding = json!({
            "classification": "weakly_exposed",
            "probe": {
                "file": "//?/H:/Code/Rust3/perl-lsp-swarm\\crates/perl-lsp-ux-tests/tests/ux_scenario_62_project_test_assertion_inline_completion_quality.rs"
            }
        });

        assert!(suppression_matches_finding(&rules, &finding));
        Ok(())
    }

    #[test]
    fn ripr_0_9_x_grip_class_seam_file_suppressed_by_policy() -> Result<()> {
        // ripr 0.9.x uses "grip_class" (not "classification") and "seam.file" (not "probe.file").
        // Verify that the suppression machinery handles both field shapes so that
        // path-scoped suppressions in policy/ripr-suppressions.toml fire under 0.9.x.
        let options = PrEvidenceOptions {
            root: ".".to_string(),
            base: "origin/main".to_string(),
            head: "HEAD".to_string(),
        };
        // Simulate ripr 0.9.x check output: summary.reachable_unrevealed=3, findings use grip_class+seam.
        let check_value = json!({
            "summary": {
                "weakly_exposed": 0,
                "reachable_unrevealed": 3,
                "no_static_path": 0
            },
            "findings": [
                {
                    "grip_class": "weakly_gripped",
                    "kind": "call_presence",
                    "seam": {
                        "file": "crates/perl-dap/src/debug_adapter/execution.rs",
                        "line": 22
                    }
                },
                {
                    "grip_class": "weakly_gripped",
                    "kind": "call_presence",
                    "seam": {
                        "file": "crates/perl-dap/src/debug_adapter/execution.rs",
                        "line": 28
                    }
                },
                {
                    "grip_class": "weakly_gripped",
                    "kind": "call_presence",
                    "seam": {
                        "file": "crates/perl-dap/src/debug_adapter/execution.rs",
                        "line": 30
                    }
                }
            ]
        });
        let suppressions = RiprSuppressionRules {
            display_patterns: vec!["crates/perl-dap/src/debug_adapter/execution.rs".to_string()],
            path_patterns: vec![Pattern::new("crates/perl-dap/src/debug_adapter/execution.rs")?],
            invalid_patterns: Vec::new(),
            suppression_reasons: Vec::new(),
        };

        let packet = pr_evidence_packet(
            &options,
            &["crates/perl-dap/src/debug_adapter/execution.rs".to_string()],
            &check_value,
            "base-sha",
            "head-sha",
            &suppressions,
        );

        // All 3 weakly_gripped findings are in the suppressed path and map to reachable_unrevealed.
        // After suppression: reachable_unrevealed = 3 - 3 = 0, severe_gaps = 0.
        assert_eq!(packet.pointer("/summary/reachable_unrevealed"), Some(&json!(0)));
        assert_eq!(packet.pointer("/summary/weakly_exposed"), Some(&json!(0)));
        assert_eq!(packet.pointer("/summary/severe_gaps"), Some(&json!(0)));
        assert_eq!(packet.pointer("/summary/suppressed_by_policy"), Some(&json!(3)));
        Ok(())
    }

    #[test]
    fn ripr_0_9_x_unsuppressed_grip_class_produces_severe_gaps() -> Result<()> {
        // Gate teeth: a ripr 0.9.x weakly_gripped finding on a path NOT covered by any
        // suppression rule must produce severe_gaps > 0, causing the quality gate to FAIL.
        // Before the grip_class fix, the gate silently skipped such findings because
        // grip_class was not recognized, so severe_gaps stayed 0 — the gate had no teeth.
        let options = PrEvidenceOptions {
            root: ".".to_string(),
            base: "origin/main".to_string(),
            head: "HEAD".to_string(),
        };
        // ripr 0.9.x output: 2 weakly_gripped findings on a file not in any suppression.
        let check_value = json!({
            "summary": {
                "weakly_exposed": 0,
                "reachable_unrevealed": 2,
                "no_static_path": 0
            },
            "findings": [
                {
                    "grip_class": "weakly_gripped",
                    "kind": "call_presence",
                    "seam": {
                        "file": "crates/perl-lsp-rs/src/some_new_file.rs",
                        "line": 10
                    }
                },
                {
                    "grip_class": "weakly_gripped",
                    "kind": "call_presence",
                    "seam": {
                        "file": "crates/perl-lsp-rs/src/some_new_file.rs",
                        "line": 20
                    }
                }
            ]
        });
        // Suppression only covers the DAP execution.rs — the LSP file is NOT suppressed.
        let suppressions = RiprSuppressionRules {
            display_patterns: vec!["crates/perl-dap/src/debug_adapter/execution.rs".to_string()],
            path_patterns: vec![Pattern::new("crates/perl-dap/src/debug_adapter/execution.rs")?],
            invalid_patterns: Vec::new(),
            suppression_reasons: Vec::new(),
        };

        let packet = pr_evidence_packet(
            &options,
            &["crates/perl-lsp-rs/src/some_new_file.rs".to_string()],
            &check_value,
            "base-sha",
            "head-sha",
            &suppressions,
        );

        // The 2 unsuppressed weakly_gripped findings map to reachable_unrevealed bucket.
        // severe_gaps must be 2 (> 0) so the quality gate rejects this PR.
        assert_eq!(packet.pointer("/summary/reachable_unrevealed"), Some(&json!(2)));
        assert_eq!(packet.pointer("/summary/severe_gaps"), Some(&json!(2)));
        assert_eq!(packet.pointer("/summary/suppressed_by_policy"), Some(&json!(0)));
        assert_eq!(packet.pointer("/summary/ripr_severe_gap"), Some(&json!(true)));
        Ok(())
    }

    #[test]
    fn write_review_comments_skips_ripr_when_current_pr_evidence_has_no_severe_gaps() -> Result<()>
    {
        let temp = tempfile::tempdir()?;
        let repo = temp.path();
        init_git_repo(repo)?;
        let ripr = write_fake_ripr_binary(repo)?;
        let _override = override_ripr_bin(&ripr)?;
        let options = ReviewCommentsOptions {
            root: ".".to_string(),
            base: "HEAD".to_string(),
            head: "HEAD".to_string(),
            timeout_seconds: Some(1),
        };
        let pr_options = PrEvidenceOptions {
            root: ".".to_string(),
            base: "HEAD".to_string(),
            head: "HEAD".to_string(),
        };
        let head = revision_sha(repo, "HEAD")?;
        let pr_packet = pr_evidence_packet(
            &pr_options,
            &["crates/perl-lsp-ux-tests/tests/ux_scenario_62_project_test_assertion_inline_completion_quality.rs".to_string()],
            &json!({
                "summary": {
                    "weakly_exposed": 0,
                    "reachable_unrevealed": 0,
                    "no_static_path": 0
                }
            }),
            &head,
            &head,
            &RiprSuppressionRules::default(),
        );
        write_text(&repo.join(PR_EVIDENCE_JSON), &format_json(&pr_packet)?)?;

        write_review_comments(repo, &options)?;

        let packet: Value =
            serde_json::from_str(&fs::read_to_string(repo.join(REVIEW_COMMENTS_JSON))?)?;
        let markdown = fs::read_to_string(repo.join(REVIEW_COMMENTS_MD))?;
        assert_eq!(packet["status"], json!("advisory"));
        assert_eq!(packet["mode"], json!("pr_evidence_clean"));
        assert_eq!(
            packet.pointer("/summary/skip_reason"),
            Some(&json!("pr_evidence_zero_severe_gaps"))
        );
        assert!(
            packet.get("comments").and_then(Value::as_array).is_some_and(|items| items.is_empty())
        );
        assert_eq!(packet["head_sha"], json!(head));
        assert!(markdown.contains("zero severe gaps"), "{markdown}");
        Ok(())
    }

    #[test]
    fn current_pr_evidence_has_no_severe_gaps_rejects_stale_receipt() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let repo = temp.path();
        init_git_repo(repo)?;
        fs::create_dir_all(repo.join("target/ripr/pr"))?;
        fs::write(
            repo.join(PR_EVIDENCE_JSON),
            format_json(&json!({
                "base": "HEAD",
                "base_sha": "stale-base",
                "head": "HEAD",
                "head_sha": "stale-head",
                "summary": {
                    "severe_gaps": 0
                }
            }))?,
        )?;
        let options = ReviewCommentsOptions {
            root: ".".to_string(),
            base: "HEAD".to_string(),
            head: "HEAD".to_string(),
            timeout_seconds: None,
        };

        assert!(!current_pr_evidence_has_no_severe_gaps(repo, &options)?);
        Ok(())
    }

    #[test]
    fn stamp_review_comments_receipt_records_current_revisions() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let repo = temp.path();
        init_git_repo(repo)?;
        fs::create_dir_all(repo.join("target/ripr/review"))?;
        fs::write(repo.join(REVIEW_COMMENTS_MD), "# Review comments\n")?;
        fs::write(
            repo.join(REVIEW_COMMENTS_JSON),
            format_json(&json!({
                "schema_version": "0.1",
                "tool": "ripr",
                "status": "advisory",
                "root": ".",
                "base": "HEAD",
                "head": "HEAD",
                "mode": "fast",
                "summary": {},
                "comments": [],
                "summary_only": [],
                "suppressed": [],
                "warnings": []
            }))?,
        )?;
        let options = ReviewCommentsOptions {
            root: ".".to_string(),
            base: "HEAD".to_string(),
            head: "HEAD".to_string(),
            timeout_seconds: None,
        };

        stamp_review_comments_receipt(repo, &options)?;
        validate_review_comments(repo, &options, true)?;
        let packet: Value =
            serde_json::from_str(&fs::read_to_string(repo.join(REVIEW_COMMENTS_JSON))?)?;
        let head = revision_sha(repo, "HEAD")?;

        assert_eq!(packet["base_sha"], json!(head));
        assert_eq!(packet["head_sha"], json!(head));
        Ok(())
    }

    #[test]
    fn stamp_review_comments_receipt_rejects_non_object() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let repo = temp.path();
        init_git_repo(repo)?;
        fs::create_dir_all(repo.join("target/ripr/review"))?;
        fs::write(repo.join(REVIEW_COMMENTS_JSON), "[]\n")?;
        let options = ReviewCommentsOptions {
            root: ".".to_string(),
            base: "HEAD".to_string(),
            head: "HEAD".to_string(),
            timeout_seconds: None,
        };

        assert!(stamp_review_comments_receipt(repo, &options).is_err());
        Ok(())
    }

    #[test]
    fn validate_review_comments_rejects_stale_revision_receipt() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let repo = temp.path();
        init_git_repo(repo)?;
        fs::create_dir_all(repo.join("target/ripr/review"))?;
        fs::write(repo.join(REVIEW_COMMENTS_MD), "# Review comments\n")?;
        fs::write(
            repo.join(REVIEW_COMMENTS_JSON),
            format_json(&json!({
                "schema_version": "0.1",
                "tool": "ripr",
                "status": "advisory",
                "root": ".",
                "base": "HEAD",
                "base_sha": "stale-base",
                "head": "HEAD",
                "head_sha": "stale-head",
                "mode": "fast",
                "summary": {},
                "comments": [],
                "summary_only": [],
                "suppressed": [],
                "warnings": []
            }))?,
        )?;
        let options = ReviewCommentsOptions {
            root: ".".to_string(),
            base: "HEAD".to_string(),
            head: "HEAD".to_string(),
            timeout_seconds: None,
        };

        assert!(validate_review_comments(repo, &options, true).is_err());
        Ok(())
    }

    #[test]
    fn write_error_review_comments_writes_stamped_error_receipts() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let repo = temp.path();
        init_git_repo(repo)?;
        let options = ReviewCommentsOptions {
            root: ".".to_string(),
            base: "HEAD".to_string(),
            head: "HEAD".to_string(),
            timeout_seconds: None,
        };

        write_error_review_comments(
            repo,
            &options,
            r"crates\perl-parser",
            "ripr review-comments failed | timeout\nsecondary detail",
        )?;
        stamp_review_comments_receipt(repo, &options)?;
        validate_review_comments(repo, &options, true)?;

        let packet: Value =
            serde_json::from_str(&fs::read_to_string(repo.join(REVIEW_COMMENTS_JSON))?)?;
        let markdown = fs::read_to_string(repo.join(REVIEW_COMMENTS_MD))?;
        let head = revision_sha(repo, "HEAD")?;

        assert_eq!(packet["status"], json!("error"));
        assert_eq!(packet["root"], json!("crates/perl-parser"));
        assert_eq!(packet["base_sha"], json!(head));
        assert_eq!(packet["head_sha"], json!(head));
        assert_eq!(
            packet.pointer("/warnings/0/message"),
            Some(&json!("ripr review-comments failed | timeout"))
        );
        assert_eq!(packet.pointer("/summary/comments"), Some(&json!(0)));
        assert_eq!(packet.pointer("/summary/summary_only"), Some(&json!(0)));
        assert_eq!(packet.pointer("/summary/suppressed"), Some(&json!(0)));
        assert!(markdown.contains("- status: error"), "{markdown}");
        assert!(markdown.contains("tool_error: ripr review-comments failed \\| timeout"));
        assert!(!markdown.contains("secondary detail"), "{markdown}");
        Ok(())
    }

    #[test]
    fn render_pr_evidence_summary_surfaces_error_review_guidance() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let repo = temp.path();
        init_git_repo(repo)?;
        let options = ReviewCommentsOptions {
            root: ".".to_string(),
            base: "HEAD".to_string(),
            head: "HEAD".to_string(),
            timeout_seconds: None,
        };
        let pr_options = PrEvidenceOptions {
            root: ".".to_string(),
            base: "HEAD".to_string(),
            head: "HEAD".to_string(),
        };
        let head = revision_sha(repo, "HEAD")?;
        let pr_packet = pr_evidence_packet(
            &pr_options,
            &["xtask/src/tasks/ripr_evidence.rs".to_string()],
            &json!({
                "summary": {
                    "weakly_exposed": 0,
                    "reachable_unrevealed": 0,
                    "no_static_path": 0
                }
            }),
            &head,
            &head,
            &RiprSuppressionRules::default(),
        );
        write_text(&repo.join(PR_EVIDENCE_JSON), &format_json(&pr_packet)?)?;
        write_text(&repo.join(PR_EVIDENCE_MD), &render_pr_evidence_markdown(&pr_packet))?;
        write_error_review_comments(repo, &options, ".", "ripr review-comments failed")?;
        stamp_review_comments_receipt(repo, &options)?;

        let summary = render_pr_evidence_summary(repo);

        assert!(summary.contains("- PR evidence JSON: present"), "{summary}");
        assert!(summary.contains("- review guidance JSON: present"), "{summary}");
        assert!(summary.contains("- review guidance status: `error`"), "{summary}");
        assert!(summary.contains("- changed-line comments: 0"), "{summary}");
        assert!(summary.contains("- summary-only guidance: 0"), "{summary}");
        assert!(summary.contains("- suppressed guidance: 0"), "{summary}");
        assert!(
            summary.contains(
                "| Review guidance Markdown | `target/ripr/review/comments.md` | present |"
            ),
            "{summary}"
        );
        Ok(())
    }

    #[test]
    fn render_annotations_emits_escaped_github_warning_packets() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let repo = temp.path();
        fs::create_dir_all(repo.join("target/ripr/review"))?;
        fs::write(
            repo.join(REVIEW_COMMENTS_JSON),
            format_json(&json!({
                "comments": [
                    {
                        "placement": {
                            "path": "crates/perl-parser/src/lib.rs",
                            "line": 42,
                            "mode": "exact_seam_line"
                        },
                        "severity": "strong:gap",
                        "kind": "focused,test",
                        "reason": "branch lacks boundary proof: below, equal, above",
                        "suggested_test": {
                            "intent": "add % branch table"
                        }
                    }
                ]
            }))?,
        )?;

        let rendered = render_annotations(repo, REVIEW_COMMENTS_JSON)?;

        assert!(!rendered.comments_missing);
        assert!(rendered.text.contains("::warning file=crates/perl-parser/src/lib.rs,line=42"));
        assert!(rendered.text.contains("title=ripr strong%3Agap focused%2Ctest"));
        assert!(rendered.text.contains("boundary proof%3A below%2C equal%2C above"));
        assert!(rendered.text.contains("Suggested test%3A add %25 branch table"));
        Ok(())
    }

    #[test]
    fn render_annotations_rejects_summary_only_placement_as_not_annotation_safe() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let repo = temp.path();
        fs::create_dir_all(repo.join("target/ripr/review"))?;
        fs::write(
            repo.join(REVIEW_COMMENTS_JSON),
            format_json(&json!({
                "comments": [
                    {
                        "placement": {
                            "path": "crates/perl-parser/src/lib.rs",
                            "line": 42,
                            "mode": "summary_only"
                        },
                        "reason": "summary-only guidance should not become a line annotation"
                    }
                ]
            }))?,
        )?;

        let err = match render_annotations(repo, REVIEW_COMMENTS_JSON) {
            Ok(_) => bail!("summary-only placement must not become a warning annotation"),
            Err(err) => err,
        };

        assert!(err.to_string().contains("not annotation-safe"));
        Ok(())
    }

    #[test]
    fn impacted_evidence_routes_severe_ripr_gap_without_label() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let repo = temp.path();
        fs::create_dir_all(repo.join("target/ripr/pr"))?;
        fs::write(
            repo.join(PR_EVIDENCE_JSON),
            format_json(&json!({
                "summary": {
                    "ripr_severe_gap": true,
                    "requires_targeted_mutation": false
                }
            }))?,
        )?;
        let options = ImpactedEvidenceOptions {
            pr_evidence: PR_EVIDENCE_JSON.to_string(),
            labels: Vec::new(),
        };

        let packet = impacted_evidence_packet(repo, &options);

        assert_eq!(packet.pointer("/status"), Some(&json!("advisory")));
        assert_eq!(packet.pointer("/summary/mutation_mode"), Some(&json!("targeted")));
        assert_eq!(packet.pointer("/summary/requires_targeted_mutation"), Some(&json!(true)));
        assert_eq!(packet.pointer("/summary/routing_reason"), Some(&json!("ripr severe gap")));
        assert_eq!(packet.pointer("/artifacts/2/available"), Some(&json!(true)));
        Ok(())
    }

    #[test]
    fn impacted_evidence_missing_pr_receipt_keeps_label_route_and_warns() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let repo = temp.path();
        let options = ImpactedEvidenceOptions {
            pr_evidence: PR_EVIDENCE_JSON.to_string(),
            labels: vec!["release-risk".to_string()],
        };

        let packet = impacted_evidence_packet(repo, &options);
        let markdown = render_impacted_evidence_markdown(&packet);

        assert_eq!(packet.pointer("/status"), Some(&json!("incomplete")));
        assert_eq!(packet.pointer("/summary/mutation_mode"), Some(&json!("targeted")));
        assert_eq!(packet.pointer("/summary/routing_reason"), Some(&json!("release-risk label")));
        assert_eq!(packet.pointer("/artifacts/2/available"), Some(&json!(false)));
        assert_eq!(packet.pointer("/warnings/0/kind"), Some(&json!("missing_artifact")));
        assert!(markdown.contains("missing_artifact"));
        assert!(markdown.contains("release-risk label"));
        Ok(())
    }

    #[test]
    fn render_pr_evidence_summary_names_missing_and_invalid_inputs() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let repo = temp.path();
        fs::create_dir_all(repo.join("target/ripr/pr"))?;
        fs::create_dir_all(repo.join("target/xtask/impacted-evidence"))?;
        fs::write(repo.join(PR_EVIDENCE_JSON), "{not json\n")?;
        fs::write(
            repo.join(IMPACTED_JSON),
            format_json(&json!({
                "status": "advisory",
                "summary": {
                    "mutation_mode": "fast_only",
                    "requires_targeted_mutation": false,
                    "ripr_severe_gap": false,
                    "routing_reason": null
                }
            }))?,
        )?;

        let summary = render_pr_evidence_summary(repo);

        assert!(summary.contains("- PR evidence JSON: invalid:"), "{summary}");
        assert!(summary.contains("- review guidance JSON: missing"), "{summary}");
        assert!(summary.contains("- impacted evidence JSON: present"), "{summary}");
        assert!(
            summary
                .contains("| PR evidence Markdown | `target/ripr/pr/repo-exposure.md` | missing |")
        );
        assert!(summary.contains("- mutation_mode: `fast_only`"), "{summary}");
        Ok(())
    }

    fn init_git_repo(repo: &Path) -> Result<()> {
        fs::write(repo.join("tracked.txt"), "base\n")?;
        run_git(repo, &["init"])?;
        run_git(repo, &["add", "tracked.txt"])?;
        run_git(
            repo,
            &["-c", "user.name=test", "-c", "user.email=test@example.com", "commit", "-m", "base"],
        )?;
        Ok(())
    }

    fn run_git(repo: &Path, args: &[&str]) -> Result<String> {
        let output = Command::new("git").args(args).current_dir(repo).output()?;
        if !output.status.success() {
            bail!("git {:?} failed with status {}", args, output.status);
        }
        Ok(String::from_utf8(output.stdout)
            .context("git command returned non-UTF8 output")?
            .trim()
            .to_string())
    }

    struct RiprBinOverrideGuard;

    impl Drop for RiprBinOverrideGuard {
        fn drop(&mut self) {
            if let Ok(mut guard) = RIPR_BIN_OVERRIDE.lock() {
                *guard = None;
            }
        }
    }

    fn override_ripr_bin(binary: &Path) -> Result<RiprBinOverrideGuard> {
        let mut guard =
            RIPR_BIN_OVERRIDE.lock().map_err(|_| eyre!("RIPR_BIN test override lock poisoned"))?;
        *guard = Some(binary.display().to_string());
        Ok(RiprBinOverrideGuard)
    }

    fn write_fake_ripr_binary(dir: &Path) -> Result<PathBuf> {
        let badge_json = r#"{"basis":"canonical_actionable_gap","counts":{"unsuppressed_exposure_gaps":7,"unsuppressed_test_efficiency_findings":2,"suppressed_exposure_gaps":1,"suppressed_test_efficiency_findings":3},"reason_counts":{"no_assertion_detected":7}}"#;
        let seams_json = r#"{"seams":[{"file":"xtask/src/tasks/ripr_evidence.rs","gap_kind":"receipt parsing"},{"file":"archive/old.rs","gap_kind":"archived"}]}"#;

        #[cfg(windows)]
        {
            let path = dir.join("ripr.cmd");
            write_text(
                &path,
                &format!(
                    r#"@echo off
echo %* | findstr /C:"repo-badge-json" >NUL
if %ERRORLEVEL%==0 (
  echo {badge_json}
  exit /b 0
)
echo %* | findstr /C:"repo-seams-json" >NUL
if %ERRORLEVEL%==0 (
  echo {seams_json}
  exit /b 0
)
echo unexpected ripr args: %* 1>&2
exit /b 2
"#
                ),
            )?;
            Ok(path)
        }

        #[cfg(not(windows))]
        {
            let path = dir.join("ripr");
            write_text(
                &path,
                &format!(
                    r#"#!/bin/sh
case "$*" in
  *repo-badge-json*)
    printf '%s\n' '{badge_json}'
    ;;
  *repo-seams-json*)
    printf '%s\n' '{seams_json}'
    ;;
  *)
    echo "unexpected ripr args: $*" >&2
    exit 2
    ;;
esac
"#
                ),
            )?;
            let mut permissions = fs::metadata(&path)?.permissions();
            use std::os::unix::fs::PermissionsExt;
            permissions.set_mode(0o755);
            fs::set_permissions(&path, permissions)?;
            Ok(path)
        }
    }

    #[test]
    fn run_git_reports_failure_status() -> Result<()> {
        let temp = tempfile::tempdir()?;

        assert!(run_git(temp.path(), &["definitely-not-a-git-command"]).is_err());
        Ok(())
    }

    #[test]
    fn merge_base_guidance_points_to_unshallow_for_shallow_clone() {
        let message = merge_base_failure_guidance("origin/main", "HEAD", true);
        assert!(message.contains("origin/main...HEAD"), "range echoed: {message}");
        assert!(message.contains("no merge base"), "diagnosis: {message}");
        assert!(message.contains("shallow clone"), "shallow cause: {message}");
        assert!(message.contains("git fetch --unshallow"), "remedy: {message}");
        assert!(message.contains("fetch-depth: 0"), "CI note: {message}");
    }

    #[test]
    fn merge_base_guidance_suggests_fetch_for_non_shallow() {
        let message = merge_base_failure_guidance("origin/main", "HEAD", false);
        assert!(message.contains("no merge base"), "diagnosis: {message}");
        assert!(!message.contains("shallow"), "must not blame shallow: {message}");
        assert!(message.contains("git fetch origin origin/main"), "fetch remedy: {message}");
    }

    #[test]
    fn changed_files_reports_missing_merge_base_with_guidance() -> Result<()> {
        // The workspace root is a real git repo; a bogus base has no merge base
        // with HEAD, so changed_files must bail with the actionable guidance
        // instead of propagating a raw git failure.
        let repo = repo_root()?;
        match changed_files(&repo, "ripr-no-such-base-xyz", "HEAD") {
            Ok(files) => Err(eyre!("expected missing-merge-base error, got {files:?}")),
            Err(err) => {
                let message = format!("{err:#}");
                assert!(message.contains("no merge base"), "guidance surfaced: {message}");
                Ok(())
            }
        }
    }

    #[test]
    fn changed_files_succeeds_for_valid_range() -> Result<()> {
        // The workspace root is a real git repo; `HEAD...HEAD` is a valid range
        // with an empty symmetric diff, exercising the success path.
        let repo = repo_root()?;
        let files = changed_files(&repo, "HEAD", "HEAD")?;
        assert!(files.is_empty(), "HEAD...HEAD has no changed files: {files:?}");
        // Exercise the shallow probe; its value is environment-dependent, so we
        // only assert it returns without error.
        let _ = is_shallow_clone(&repo);
        Ok(())
    }

    #[test]
    fn split_labels_splits_on_comma_semicolon_and_newline_with_trim() {
        let labels = split_labels(" mutation , needs-ci-fix ;size/M\nsize/L ");
        assert_eq!(labels, vec!["mutation", "needs-ci-fix", "size/M", "size/L"]);
    }

    #[test]
    fn split_labels_drops_empty_and_whitespace_only_segments() {
        // Trailing/leading separators and blank segments must not yield empty
        // labels, otherwise downstream routing would match a "" label.
        let labels = split_labels(",, mutation ;; \n ; ,");
        assert_eq!(labels, vec!["mutation"]);
        assert!(split_labels("   ").is_empty());
        assert!(split_labels("").is_empty());
    }

    #[test]
    fn normalize_labels_lowercases_dedupes_and_sorts() {
        let input = vec!["Mutation".to_string(), "mutation".to_string(), "  CI  ".to_string()];
        let normalized = normalize_labels(&input);
        // case-folded, de-duplicated across cases, trimmed, and sorted.
        assert_eq!(normalized, vec!["ci".to_string(), "mutation".to_string()]);
    }

    #[test]
    fn normalize_labels_filters_blank_after_trim() {
        let input = vec!["   ".to_string(), "\t".to_string(), "keep".to_string()];
        assert_eq!(normalize_labels(&input), vec!["keep".to_string()]);
    }

    #[test]
    fn merged_labels_unions_explicit_and_csv_then_normalizes() {
        // Explicit labels are non-empty, so the env fallback path is not taken.
        // Duplicates across the two sources collapse; output is folded and sorted.
        let merged = merged_labels(&["Zeta".to_string()], Some("alpha, Zeta; BETA"));
        assert_eq!(merged, vec!["alpha".to_string(), "beta".to_string(), "zeta".to_string()]);
    }

    #[test]
    fn merged_labels_accepts_csv_only_without_explicit_labels() {
        let merged = merged_labels(&[], Some("needs-ci-fix,needs-ci-fix"));
        assert_eq!(merged, vec!["needs-ci-fix".to_string()]);
    }
}
