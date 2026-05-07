//! Diff-aware DevEx proof planner.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use chrono::Utc;
use color_eyre::eyre::{Context, Result, bail};
use serde::Serialize;

use crate::utils::project_root;

#[derive(Debug, Clone)]
pub struct DevexPlanConfig {
    pub base: String,
}

#[derive(Debug, Clone)]
pub struct DevexReceiptConfig {
    pub base: String,
    pub output: PathBuf,
}

#[derive(Debug, Clone)]
pub struct DevexCockpitConfig {
    pub base: String,
    pub receipt: PathBuf,
}

#[derive(Debug, Clone)]
pub struct DevexPrBodyConfig {
    pub base: String,
    pub receipt: PathBuf,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
enum Surface {
    ParserAccuracy,
    GeneratedStatusDocs,
    MemorySensitiveRuntime,
    RetainedOwnerCandidate,
    ReleaseVersion,
    PolicyOrCi,
    RustCode,
    Docs,
}

impl Surface {
    fn label(&self) -> &'static str {
        match self {
            Self::ParserAccuracy => "parser accuracy",
            Self::GeneratedStatusDocs => "generated status docs",
            Self::MemorySensitiveRuntime => "memory-sensitive runtime",
            Self::RetainedOwnerCandidate => "retained-owner candidate",
            Self::ReleaseVersion => "release/version surface",
            Self::PolicyOrCi => "policy/CI configuration",
            Self::RustCode => "Rust code",
            Self::Docs => "docs/prose",
        }
    }

    fn id(&self) -> &'static str {
        match self {
            Self::ParserAccuracy => "parser_accuracy",
            Self::GeneratedStatusDocs => "generated_status_docs",
            Self::MemorySensitiveRuntime => "memory_sensitive_runtime",
            Self::RetainedOwnerCandidate => "retained_owner_candidate",
            Self::ReleaseVersion => "release_version",
            Self::PolicyOrCi => "policy_ci",
            Self::RustCode => "rust_code",
            Self::Docs => "docs",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct Plan {
    base: String,
    head: String,
    changed_files: Vec<String>,
    surfaces: BTreeSet<Surface>,
    required_commands: Vec<ProofCommand>,
    optional_commands: Vec<ProofCommand>,
    agent_hints: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct ProofCommand {
    command: String,
    why: String,
    evidence: String,
}

#[derive(Debug, Serialize)]
struct DevexReceipt {
    base: String,
    head: String,
    changed_files: Vec<String>,
    changed_surfaces: Vec<String>,
    required_proof: Vec<ProofCommandReceipt>,
    optional_proof: Vec<ProofCommandReceipt>,
    agent_hints: Vec<String>,
    worktree_clean: bool,
    generated_at: String,
}

#[derive(Debug, Serialize)]
struct ProofCommandReceipt {
    command: String,
    why: String,
    evidence: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct WorktreeState {
    branch: String,
    head_short: String,
    clean: bool,
    staged_present: bool,
    unstaged_present: bool,
    untracked_present: bool,
}

pub fn run(config: DevexPlanConfig) -> Result<()> {
    let plan = load_plan(&config.base)?;
    print_plan(&plan);
    Ok(())
}

pub fn write_receipt(config: DevexReceiptConfig) -> Result<()> {
    let root = project_root()?;
    let plan = load_plan_from_root(&root, &config.base)?;
    let receipt = build_receipt(&root, plan)?;
    write_receipt_json(&config.output, &receipt)?;

    println!("Wrote DevEx local proof receipt to {}", config.output.display());
    Ok(())
}

pub fn cockpit(config: DevexCockpitConfig) -> Result<()> {
    let root = project_root()?;
    let plan = load_plan_from_root(&root, &config.base)?;
    let receipt = build_receipt(&root, plan.clone())?;
    write_receipt_json(&config.receipt, &receipt)?;
    let worktree = load_worktree_state(&root)?;

    print!(
        "{}",
        render_cockpit(CockpitView {
            plan: &plan,
            worktree: &worktree,
            receipt_path: &config.receipt,
        })
    );
    Ok(())
}

pub fn pr_body(config: DevexPrBodyConfig) -> Result<()> {
    let plan = load_plan(&config.base)?;
    print!("{}", render_pr_body(PrBodyView { plan: &plan, receipt_path: &config.receipt }));
    Ok(())
}

fn load_plan(requested_base: &str) -> Result<Plan> {
    let root = project_root()?;
    load_plan_from_root(&root, requested_base)
}

fn load_plan_from_root(root: &Path, requested_base: &str) -> Result<Plan> {
    let base = resolve_diff_base(root, requested_base)?;
    let changed_files = changed_files(root, &base)?;
    let head = git_stdout(root, &["rev-parse", "HEAD"])?;
    Ok(build_plan(base, head.trim().to_string(), changed_files))
}

fn resolve_diff_base(root: &Path, requested_base: &str) -> Result<String> {
    if requested_base != "auto" && git_ref_exists(root, requested_base)? {
        return Ok(requested_base.to_string());
    }

    for candidate in ["origin/HEAD", "origin/master", "origin/main", "master", "main", "HEAD~1"] {
        if git_ref_exists(root, candidate)? {
            return Ok(candidate.to_string());
        }
    }

    bail!("could not resolve a diff base for devex plan");
}

fn git_ref_exists(root: &Path, reference: &str) -> Result<bool> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "--verify", reference])
        .output()
        .with_context(|| format!("failed to resolve git ref {reference}"))?;
    Ok(output.status.success())
}

fn changed_files(root: &Path, base: &str) -> Result<Vec<String>> {
    let committed = git_stdout(root, &["diff", "--name-only", &format!("{base}...HEAD")])?;
    let staged = git_stdout(root, &["diff", "--cached", "--name-only"])?;
    let unstaged = git_stdout(root, &["diff", "--name-only"])?;
    let untracked = git_stdout(root, &["ls-files", "--others", "--exclude-standard"])?;
    Ok(merge_changed_file_lists(&[&committed, &staged, &unstaged, &untracked]))
}

fn git_stdout(root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .with_context(|| format!("failed to run git {}", args.join(" ")))?;

    if !output.status.success() {
        bail!("git {} failed", args.join(" "));
    }

    String::from_utf8(output.stdout).context("git output was not UTF-8")
}

fn is_worktree_clean(root: &Path) -> Result<bool> {
    Ok(git_stdout(root, &["status", "--porcelain"])?.trim().is_empty())
}

fn load_worktree_state(root: &Path) -> Result<WorktreeState> {
    let branch_raw = git_stdout(root, &["branch", "--show-current"])?;
    let branch = match branch_raw.trim() {
        "" => "detached HEAD".to_string(),
        branch => branch.to_string(),
    };
    let head = git_stdout(root, &["rev-parse", "HEAD"])?;
    let staged = git_stdout(root, &["diff", "--cached", "--name-only"])?;
    let unstaged = git_stdout(root, &["diff", "--name-only"])?;
    let untracked = git_stdout(root, &["ls-files", "--others", "--exclude-standard"])?;

    let staged_present = !staged.trim().is_empty();
    let unstaged_present = !unstaged.trim().is_empty();
    let untracked_present = !untracked.trim().is_empty();

    Ok(WorktreeState {
        branch,
        head_short: short_sha(head.trim()),
        clean: !(staged_present || unstaged_present || untracked_present),
        staged_present,
        unstaged_present,
        untracked_present,
    })
}

fn short_sha(sha: &str) -> String {
    sha.chars().take(9).collect()
}

fn build_plan(base: String, head: String, changed_files: Vec<String>) -> Plan {
    let surfaces = classify_surfaces(&changed_files);
    let required_commands = required_commands(&surfaces, &base);
    let optional_commands = optional_commands(&surfaces);
    let agent_hints = agent_hints(&surfaces);

    Plan { base, head, changed_files, surfaces, required_commands, optional_commands, agent_hints }
}

fn build_receipt(root: &Path, plan: Plan) -> Result<DevexReceipt> {
    build_receipt_payload(plan, is_worktree_clean(root)?, Utc::now().to_rfc3339())
}

fn build_receipt_payload(
    plan: Plan,
    worktree_clean: bool,
    generated_at: String,
) -> Result<DevexReceipt> {
    Ok(DevexReceipt {
        base: plan.base,
        head: plan.head,
        changed_files: plan.changed_files,
        changed_surfaces: plan.surfaces.iter().map(|surface| surface.id().to_string()).collect(),
        required_proof: proof_receipts(plan.required_commands),
        optional_proof: proof_receipts(plan.optional_commands),
        agent_hints: plan.agent_hints,
        worktree_clean,
        generated_at,
    })
}

fn proof_receipts(commands: Vec<ProofCommand>) -> Vec<ProofCommandReceipt> {
    commands
        .into_iter()
        .map(|proof| ProofCommandReceipt {
            command: proof.command,
            why: proof.why,
            evidence: proof.evidence,
        })
        .collect()
}

fn surface_ids(plan: &Plan) -> Vec<String> {
    plan.surfaces.iter().map(|surface| surface.id().to_string()).collect()
}

fn proof_command_names(commands: &[ProofCommand]) -> Vec<String> {
    commands.iter().map(|proof| proof.command.clone()).collect()
}

fn write_receipt_json(path: &Path, receipt: &DevexReceipt) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating receipt directory {}", parent.display()))?;
    }

    let rendered = serde_json::to_string_pretty(receipt).context("serializing devex receipt")?;
    fs::write(path, format!("{rendered}\n"))
        .with_context(|| format!("writing devex receipt {}", path.display()))
}

struct CockpitView<'a> {
    plan: &'a Plan,
    worktree: &'a WorktreeState,
    receipt_path: &'a Path,
}

fn render_cockpit(view: CockpitView<'_>) -> String {
    let surfaces = surface_ids(view.plan);
    let required = proof_command_names(&view.plan.required_commands);
    let optional = proof_command_names(&view.plan.optional_commands);

    let mut out = String::new();
    out.push_str("PR Cockpit\n");
    out.push_str("----------\n");
    out.push_str(&format!("Base:                  {}\n", view.plan.base));
    out.push_str(&format!("Head:                  {}\n", view.worktree.head_short));
    out.push_str(&format!("Branch:                {}\n", view.worktree.branch));
    out.push_str(&format!("Worktree clean:        {}\n", yes_no(view.worktree.clean)));
    out.push_str(&format!("Staged changes:        {}\n", yes_no(view.worktree.staged_present)));
    out.push_str(&format!("Unstaged changes:      {}\n", yes_no(view.worktree.unstaged_present)));
    out.push_str(&format!("Untracked files:       {}\n", yes_no(view.worktree.untracked_present)));
    out.push_str(&format!("Changed files:         {}\n", view.plan.changed_files.len()));
    out.push_str(&format!("Changed surfaces:      {}\n", list_or_none(&surfaces)));
    out.push_str(&format!("Required proof:        {} commands\n", required.len()));
    for command in &required {
        out.push_str(&format!("  - {command}\n"));
    }
    out.push_str(&format!("Optional proof:        {} commands\n", optional.len()));
    for command in &optional {
        out.push_str(&format!("  - {command}\n"));
    }
    out.push_str(&format!(
        "Memory owner drift:    {}\n",
        applicability(
            view.plan.surfaces.contains(&Surface::RetainedOwnerCandidate),
            "run retained-owner drift proof",
        )
    ));
    out.push_str(&format!(
        "Release/version drift: {}\n",
        applicability(view.plan.surfaces.contains(&Surface::ReleaseVersion), "run release proof")
    ));
    out.push_str(&format!(
        "Agent-safe path:       {}\n",
        if view.plan.agent_hints.is_empty() { "none" } else { "available" }
    ));
    out.push_str(&format!("Receipt written:       {}\n", view.receipt_path.display()));
    out.push('\n');
    out.push_str("Agent hints:\n");
    for hint in &view.plan.agent_hints {
        out.push_str(&format!("  - {hint}\n"));
    }
    out.push('\n');
    out.push_str("Next:\n");
    out.push_str("  - Paste receipt summary into the PR body.\n");
    out.push_str("  - Run missing required proof commands.\n");
    out.push_str(&format!(
        "  - Use `cargo xtask devex receipt --base {} --output {}` for JSON handoff.\n",
        view.plan.base,
        view.receipt_path.display()
    ));
    out
}

struct PrBodyView<'a> {
    plan: &'a Plan,
    receipt_path: &'a Path,
}

fn render_pr_body(view: PrBodyView<'_>) -> String {
    let surfaces = surface_ids(view.plan);
    let mut out = String::new();
    out.push_str("## Proof packet\n\n");
    out.push_str("Changed surfaces:\n");
    if surfaces.is_empty() {
        out.push_str("- none\n");
    } else {
        for surface in &surfaces {
            out.push_str(&format!("- {surface}\n"));
        }
    }
    out.push('\n');

    out.push_str("Required proof:\n");
    render_pr_body_proof_commands(&mut out, &view.plan.required_commands, true);
    out.push('\n');

    out.push_str("Optional proof:\n");
    render_pr_body_proof_commands(&mut out, &view.plan.optional_commands, false);
    out.push('\n');

    if !view.plan.agent_hints.is_empty() {
        out.push_str("Agent hints:\n");
        for hint in &view.plan.agent_hints {
            out.push_str(&format!("- {hint}\n"));
        }
        out.push('\n');
    }

    out.push_str("Receipt:\n");
    out.push_str(&format!("- {}\n", view.receipt_path.display()));
    out
}

fn render_pr_body_proof_commands(
    out: &mut String,
    commands: &[ProofCommand],
    include_details: bool,
) {
    if commands.is_empty() {
        out.push_str("- none\n");
        return;
    }

    for proof in commands {
        out.push_str(&format!("- [ ] {}\n", proof.command));
        if include_details {
            out.push_str(&format!("  - why: {}\n", proof.why));
            out.push_str(&format!("  - evidence: {}\n", proof.evidence));
        }
        out.push('\n');
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn list_or_none(values: &[String]) -> String {
    if values.is_empty() { "none".to_string() } else { values.join(", ") }
}

fn applicability(applies: bool, action: &'static str) -> &'static str {
    if applies { action } else { "not applicable" }
}

fn merge_changed_file_lists(lists: &[&str]) -> Vec<String> {
    lists
        .iter()
        .flat_map(|list| list.lines())
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn classify_surfaces(files: &[String]) -> BTreeSet<Surface> {
    let mut surfaces = BTreeSet::new();

    for file in files {
        if is_docs_file(file) {
            surfaces.insert(Surface::Docs);
        }
        if file.ends_with(".rs") {
            surfaces.insert(Surface::RustCode);
        }
        if is_parser_accuracy_path(file) {
            surfaces.insert(Surface::ParserAccuracy);
        }
        if file.starts_with("docs/project/status/") {
            surfaces.insert(Surface::GeneratedStatusDocs);
        }
        if is_memory_sensitive_path(file) {
            surfaces.insert(Surface::MemorySensitiveRuntime);
        }
        if is_retained_owner_candidate_path(file) {
            surfaces.insert(Surface::RetainedOwnerCandidate);
        }
        if is_release_version_path(file) {
            surfaces.insert(Surface::ReleaseVersion);
        }
        if is_policy_or_ci_path(file) {
            surfaces.insert(Surface::PolicyOrCi);
        }
    }

    surfaces
}

fn required_commands(surfaces: &BTreeSet<Surface>, base: &str) -> Vec<ProofCommand> {
    let mut commands = vec![proof_command(
        "cargo xtask fmt",
        "keeps formatting deterministic before any other proof",
        "PR body lists `cargo xtask fmt` as passed and the branch has no formatting-only drift",
    )];

    if surfaces.contains(&Surface::ParserAccuracy) {
        commands.push(proof_command(
            "just ci-metrics-ratchet-check parser_accuracy",
            "parser fixtures, baselines, or parser status changed",
            "attach/pass parser accuracy ratchet output or explain an intentional baseline update",
        ));
    }
    if surfaces.contains(&Surface::GeneratedStatusDocs) {
        commands.push(proof_command(
            "just status-update",
            "generated status docs changed and should be regenerated from source data",
            "generated status diffs are present when expected",
        ));
        commands.push(proof_command(
            "just status-check",
            "generated status docs should match their checked-in sources",
            "PR body lists `just status-check` as passed",
        ));
    }
    if surfaces.contains(&Surface::MemorySensitiveRuntime) {
        commands.push(proof_command(
            "cargo xtask check-memory-lifecycle-policy",
            "memory-sensitive lifecycle, cache, or retained-state surfaces changed",
            "PR body includes the policy pass and any focused lifecycle/cache test evidence",
        ));
    }
    if surfaces.contains(&Surface::RetainedOwnerCandidate) {
        commands.push(proof_command(
            format!("cargo xtask check-memory-retained-owner-drift --base {base}"),
            "a Rust file in a retained-owner-sensitive path changed",
            "show no owner drift, or include the retained-state inventory/counter/test update",
        ));
    }
    if surfaces.contains(&Surface::ReleaseVersion) {
        commands.push(proof_command(
            "just version-check",
            "release/version surfaces changed and version declarations must stay aligned",
            "PR body lists `just version-check` as passed",
        ));
        commands.push(proof_command(
            "just release-check",
            "release-facing files changed and release hygiene should be validated",
            "PR body lists `just release-check` as passed",
        ));
    }

    commands.push(proof_command(
        "git diff --check",
        "guards against whitespace errors in the final patch",
        "command exits cleanly after all edits",
    ));
    commands
}

fn optional_commands(surfaces: &BTreeSet<Surface>) -> Vec<ProofCommand> {
    let mut commands = vec![
        proof_command(
            "just pr-fast",
            "cheap broader proof when the change spans more than docs or one small module",
            "useful PR-body evidence when you want confidence before pushing",
        ),
        proof_command(
            "just ci-gate",
            "local approximation of the merge-blocking CI gate",
            "optional unless the change is broad, risky, or CI-only behavior is unclear",
        ),
    ];

    if surfaces.contains(&Surface::ParserAccuracy) {
        commands.push(proof_command(
            "just cpan-corpus-check",
            "broader parser corpus confidence after parser accuracy changes",
            "attach only when parser grammar/accuracy changes need extra confidence",
        ));
        commands.push(proof_command(
            "just corpus-sweep-check",
            "expensive corpus sweep for parser changes with broad blast radius",
            "usually saved for risky parser edits or follow-up validation",
        ));
    }
    if surfaces.contains(&Surface::PolicyOrCi) {
        commands.push(proof_command(
            "cargo xtask workflow-policy-lint",
            "policy or CI files changed",
            "use when workflow/policy semantics changed, not for every xtask-only edit",
        ));
        commands.push(proof_command(
            "cargo xtask workflow-trigger-lint",
            "workflow trigger behavior may have changed",
            "use when GitHub workflow trigger files are touched",
        ));
    }

    commands
}

fn proof_command(
    command: impl Into<String>,
    why: impl Into<String>,
    evidence: impl Into<String>,
) -> ProofCommand {
    ProofCommand { command: command.into(), why: why.into(), evidence: evidence.into() }
}

fn agent_hints(surfaces: &BTreeSet<Surface>) -> Vec<String> {
    let mut hints = vec![
        "Use `just agent-check`, `just agent-test`, and `just agent-clippy` for large agent-run compile/test loops.".to_string(),
        "Use `just agent-pr-fast` when you need the PR-fast gate through cargo-safe agent profiles.".to_string(),
    ];

    if surfaces.contains(&Surface::MemorySensitiveRuntime) {
        hints.push("For memory-sensitive edits, keep focused lifecycle/cache tests in the PR body alongside policy proof.".to_string());
    }
    if surfaces.contains(&Surface::ParserAccuracy) {
        hints.push("For parser-accuracy edits, attach ratchet output or explain intentional baseline/status changes.".to_string());
    }

    hints
}

fn is_docs_file(file: &str) -> bool {
    file.ends_with(".md")
        || file.starts_with("docs/")
        || file == "README.md"
        || file == "CONTRIBUTING.md"
}

fn is_parser_accuracy_path(file: &str) -> bool {
    file.starts_with("crates/perl-corpus/fixtures/parser_accuracy/")
        || file.starts_with(".ci/metrics/baselines/parser_accuracy")
        || file == ".ci/schemas/parser-accuracy.schema.json"
        || file.starts_with("docs/project/status/parser_accuracy")
        || file == "docs/project/status/parser.md"
        || file.starts_with("xtask/src/tasks/metrics/parser_accuracy")
}

fn is_memory_sensitive_path(file: &str) -> bool {
    file.starts_with("crates/perl-lsp-rs/src/runtime/")
        || file.starts_with("crates/perl-lsp-rs/src/runtime/language/")
        || file.starts_with("crates/perl-workspace/src/workspace/")
        || file.starts_with("crates/perl-lsp-perltidy/src/")
        || file.starts_with("crates/perl-lsp-rs-core/src/tooling/")
        || file.starts_with("crates/perl-dap/src/")
        || file.starts_with("docs/large-workspaces/")
        || file.starts_with("scripts/repro_lsp_storm")
        || file.starts_with("scripts/assert_rss_plateau")
}

fn is_retained_owner_candidate_path(file: &str) -> bool {
    file.ends_with(".rs")
        && (file.starts_with("crates/perl-lsp-rs/src/runtime/")
            || file.starts_with("crates/perl-workspace/src/workspace/")
            || file.starts_with("crates/perl-lsp-perltidy/src/")
            || file.starts_with("crates/perl-lsp-rs-core/src/tooling/")
            || file.starts_with("crates/perl-dap/src/"))
}

fn is_release_version_path(file: &str) -> bool {
    matches!(
        file,
        "Cargo.toml"
            | "Cargo.lock"
            | "CHANGELOG.md"
            | "README.md"
            | "rust-toolchain.toml"
            | "vscode-extension/package.json"
    ) || file.starts_with("docs/releases/")
        || file.starts_with("docs/release/")
        || file.starts_with("docs/project/RELEASE")
        || file.starts_with(".github/workflows/release")
}

fn is_policy_or_ci_path(file: &str) -> bool {
    file.starts_with(".github/workflows/")
        || file.starts_with(".ci/")
        || file.starts_with("policy/")
        || file.starts_with("xtask/src/tasks/ci_")
        || file.starts_with("xtask/src/tasks/devex_")
        || file.starts_with("xtask/src/tasks/workflow_")
        || file.starts_with("xtask/src/tasks/gate")
        || file == "xtask/src/main.rs"
        || file == "justfile"
        || file.starts_with("scripts/")
}

fn print_plan(plan: &Plan) {
    print!("{}", render_plan(plan));
}

fn render_plan(plan: &Plan) -> String {
    let mut out = String::new();
    out.push_str("DevEx local proof plan\n");
    out.push_str(&format!("Base: {}\n", plan.base));
    out.push_str(&format!("Head: {}\n", plan.head.trim()));
    out.push('\n');

    out.push_str("Changed files:\n");
    if plan.changed_files.is_empty() {
        out.push_str("- none\n");
    } else {
        for file in &plan.changed_files {
            out.push_str(&format!("- {file}\n"));
        }
    }
    out.push('\n');

    out.push_str("Changed surfaces:\n");
    if plan.surfaces.is_empty() {
        out.push_str("- none\n");
    } else {
        for surface in &plan.surfaces {
            out.push_str(&format!("- {}\n", surface.label()));
        }
    }
    out.push('\n');

    out.push_str("Required local proof:\n");
    for proof in &plan.required_commands {
        out.push_str(&format!("- {}\n", proof.command));
        out.push_str(&format!("  why: {}\n", proof.why));
        out.push_str(&format!("  evidence: {}\n", proof.evidence));
    }
    out.push('\n');

    out.push_str("Optional / expensive:\n");
    for proof in &plan.optional_commands {
        out.push_str(&format!("- {}\n", proof.command));
        out.push_str(&format!("  why: {}\n", proof.why));
        out.push_str(&format!("  evidence: {}\n", proof.evidence));
    }
    out.push('\n');

    out.push_str("Agent-safe hints:\n");
    for hint in &plan.agent_hints {
        out.push_str(&format!("- {hint}\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|path| path.to_string()).collect()
    }

    fn command_strings(commands: &[ProofCommand]) -> Vec<String> {
        commands.iter().map(|proof| proof.command.clone()).collect()
    }

    fn assert_object_keys(object: &serde_json::Map<String, serde_json::Value>, expected: &[&str]) {
        let mut actual = object.keys().map(String::as_str).collect::<Vec<_>>();
        let mut expected = expected.to_vec();
        actual.sort_unstable();
        expected.sort_unstable();
        assert_eq!(actual, expected);
    }

    fn plan_for(files: &[&str]) -> Plan {
        build_plan("origin/master".to_string(), "abcdef1234567890".to_string(), strings(files))
    }

    fn surface_strings(plan: &Plan) -> Vec<String> {
        plan.surfaces.iter().map(|surface| surface.id().to_string()).collect()
    }

    struct DevexRoutingFixture {
        name: &'static str,
        files: &'static [&'static str],
        expected_surfaces: &'static [&'static str],
        expected_required_commands: &'static [&'static str],
        expected_optional_commands: &'static [&'static str],
        expected_agent_hint_snippets: &'static [&'static str],
    }

    #[test]
    fn routing_fixture_matrix_locks_surface_and_proof_selection() {
        let fixtures = [
            DevexRoutingFixture {
                name: "docs only",
                files: &["docs/how-to/LOCAL_WATCH_MODE.md"],
                expected_surfaces: &["docs"],
                expected_required_commands: &["cargo xtask fmt", "git diff --check"],
                expected_optional_commands: &["just pr-fast", "just ci-gate"],
                expected_agent_hint_snippets: &[
                    "Use `just agent-check`",
                    "Use `just agent-pr-fast`",
                ],
            },
            DevexRoutingFixture {
                name: "rust code only",
                files: &["crates/perl-parser-core/src/lib.rs"],
                expected_surfaces: &["rust_code"],
                expected_required_commands: &["cargo xtask fmt", "git diff --check"],
                expected_optional_commands: &["just pr-fast", "just ci-gate"],
                expected_agent_hint_snippets: &[
                    "Use `just agent-check`",
                    "Use `just agent-pr-fast`",
                ],
            },
            DevexRoutingFixture {
                name: "parser accuracy",
                files: &["crates/perl-corpus/fixtures/parser_accuracy/scalar_ref.pl"],
                expected_surfaces: &["parser_accuracy"],
                expected_required_commands: &[
                    "cargo xtask fmt",
                    "just ci-metrics-ratchet-check parser_accuracy",
                    "git diff --check",
                ],
                expected_optional_commands: &[
                    "just pr-fast",
                    "just ci-gate",
                    "just cpan-corpus-check",
                    "just corpus-sweep-check",
                ],
                expected_agent_hint_snippets: &[
                    "Use `just agent-check`",
                    "Use `just agent-pr-fast`",
                    "For parser-accuracy edits",
                ],
            },
            DevexRoutingFixture {
                name: "generated status docs",
                files: &["docs/project/status/lsp.md"],
                expected_surfaces: &["generated_status_docs", "docs"],
                expected_required_commands: &[
                    "cargo xtask fmt",
                    "just status-update",
                    "just status-check",
                    "git diff --check",
                ],
                expected_optional_commands: &["just pr-fast", "just ci-gate"],
                expected_agent_hint_snippets: &[
                    "Use `just agent-check`",
                    "Use `just agent-pr-fast`",
                ],
            },
            DevexRoutingFixture {
                name: "memory-sensitive runtime",
                files: &["docs/large-workspaces/MEMORY_CONTROL_CLOSEOUT.md"],
                expected_surfaces: &["memory_sensitive_runtime", "docs"],
                expected_required_commands: &[
                    "cargo xtask fmt",
                    "cargo xtask check-memory-lifecycle-policy",
                    "git diff --check",
                ],
                expected_optional_commands: &["just pr-fast", "just ci-gate"],
                expected_agent_hint_snippets: &[
                    "Use `just agent-check`",
                    "Use `just agent-pr-fast`",
                    "For memory-sensitive edits",
                ],
            },
            DevexRoutingFixture {
                name: "retained-owner candidate",
                files: &["crates/perl-workspace/src/workspace/workspace_index.rs"],
                expected_surfaces: &[
                    "memory_sensitive_runtime",
                    "retained_owner_candidate",
                    "rust_code",
                ],
                expected_required_commands: &[
                    "cargo xtask fmt",
                    "cargo xtask check-memory-lifecycle-policy",
                    "cargo xtask check-memory-retained-owner-drift --base origin/master",
                    "git diff --check",
                ],
                expected_optional_commands: &["just pr-fast", "just ci-gate"],
                expected_agent_hint_snippets: &[
                    "Use `just agent-check`",
                    "Use `just agent-pr-fast`",
                    "For memory-sensitive edits",
                ],
            },
            DevexRoutingFixture {
                name: "release/version surface",
                files: &["rust-toolchain.toml"],
                expected_surfaces: &["release_version"],
                expected_required_commands: &[
                    "cargo xtask fmt",
                    "just version-check",
                    "just release-check",
                    "git diff --check",
                ],
                expected_optional_commands: &["just pr-fast", "just ci-gate"],
                expected_agent_hint_snippets: &[
                    "Use `just agent-check`",
                    "Use `just agent-pr-fast`",
                ],
            },
            DevexRoutingFixture {
                name: "policy/CI surface",
                files: &[".github/workflows/ci.yml"],
                expected_surfaces: &["policy_ci"],
                expected_required_commands: &["cargo xtask fmt", "git diff --check"],
                expected_optional_commands: &[
                    "just pr-fast",
                    "just ci-gate",
                    "cargo xtask workflow-policy-lint",
                    "cargo xtask workflow-trigger-lint",
                ],
                expected_agent_hint_snippets: &[
                    "Use `just agent-check`",
                    "Use `just agent-pr-fast`",
                ],
            },
            DevexRoutingFixture {
                name: "xtask/devex tooling",
                files: &["xtask/src/tasks/devex_plan.rs"],
                expected_surfaces: &["policy_ci", "rust_code"],
                expected_required_commands: &["cargo xtask fmt", "git diff --check"],
                expected_optional_commands: &[
                    "just pr-fast",
                    "just ci-gate",
                    "cargo xtask workflow-policy-lint",
                    "cargo xtask workflow-trigger-lint",
                ],
                expected_agent_hint_snippets: &[
                    "Use `just agent-check`",
                    "Use `just agent-pr-fast`",
                ],
            },
            DevexRoutingFixture {
                name: "mixed parser + status",
                files: &["docs/project/status/parser.md"],
                expected_surfaces: &["parser_accuracy", "generated_status_docs", "docs"],
                expected_required_commands: &[
                    "cargo xtask fmt",
                    "just ci-metrics-ratchet-check parser_accuracy",
                    "just status-update",
                    "just status-check",
                    "git diff --check",
                ],
                expected_optional_commands: &[
                    "just pr-fast",
                    "just ci-gate",
                    "just cpan-corpus-check",
                    "just corpus-sweep-check",
                ],
                expected_agent_hint_snippets: &[
                    "Use `just agent-check`",
                    "Use `just agent-pr-fast`",
                    "For parser-accuracy edits",
                ],
            },
            DevexRoutingFixture {
                name: "mixed memory + retained owner",
                files: &["crates/perl-lsp-rs/src/runtime/text_sync.rs"],
                expected_surfaces: &[
                    "memory_sensitive_runtime",
                    "retained_owner_candidate",
                    "rust_code",
                ],
                expected_required_commands: &[
                    "cargo xtask fmt",
                    "cargo xtask check-memory-lifecycle-policy",
                    "cargo xtask check-memory-retained-owner-drift --base origin/master",
                    "git diff --check",
                ],
                expected_optional_commands: &["just pr-fast", "just ci-gate"],
                expected_agent_hint_snippets: &[
                    "Use `just agent-check`",
                    "Use `just agent-pr-fast`",
                    "For memory-sensitive edits",
                ],
            },
            DevexRoutingFixture {
                name: "mixed release + changelog",
                files: &["CHANGELOG.md"],
                expected_surfaces: &["release_version", "docs"],
                expected_required_commands: &[
                    "cargo xtask fmt",
                    "just version-check",
                    "just release-check",
                    "git diff --check",
                ],
                expected_optional_commands: &["just pr-fast", "just ci-gate"],
                expected_agent_hint_snippets: &[
                    "Use `just agent-check`",
                    "Use `just agent-pr-fast`",
                ],
            },
        ];

        for fixture in fixtures {
            let plan = build_plan(
                "origin/master".to_string(),
                "abc123".to_string(),
                strings(fixture.files),
            );

            assert_eq!(
                surface_strings(&plan),
                strings(fixture.expected_surfaces),
                "{}: changed surfaces",
                fixture.name
            );
            assert_eq!(
                command_strings(&plan.required_commands),
                strings(fixture.expected_required_commands),
                "{}: required proof commands",
                fixture.name
            );
            assert_eq!(
                command_strings(&plan.optional_commands),
                strings(fixture.expected_optional_commands),
                "{}: optional proof commands",
                fixture.name
            );
            assert_eq!(
                plan.agent_hints.len(),
                fixture.expected_agent_hint_snippets.len(),
                "{}: agent hint count",
                fixture.name
            );
            for snippet in fixture.expected_agent_hint_snippets {
                assert!(
                    plan.agent_hints.iter().any(|hint| hint.contains(snippet)),
                    "{}: missing agent hint containing `{snippet}` in {:?}",
                    fixture.name,
                    plan.agent_hints
                );
            }
        }
    }

    #[test]
    fn golden_plan_output_for_empty_change_set() {
        let plan = plan_for(&[]);

        assert_eq!(
            render_plan(&plan),
            r#"DevEx local proof plan
Base: origin/master
Head: abcdef1234567890

Changed files:
- none

Changed surfaces:
- none

Required local proof:
- cargo xtask fmt
  why: keeps formatting deterministic before any other proof
  evidence: PR body lists `cargo xtask fmt` as passed and the branch has no formatting-only drift
- git diff --check
  why: guards against whitespace errors in the final patch
  evidence: command exits cleanly after all edits

Optional / expensive:
- just pr-fast
  why: cheap broader proof when the change spans more than docs or one small module
  evidence: useful PR-body evidence when you want confidence before pushing
- just ci-gate
  why: local approximation of the merge-blocking CI gate
  evidence: optional unless the change is broad, risky, or CI-only behavior is unclear

Agent-safe hints:
- Use `just agent-check`, `just agent-test`, and `just agent-clippy` for large agent-run compile/test loops.
- Use `just agent-pr-fast` when you need the PR-fast gate through cargo-safe agent profiles.
"#
        );
    }

    #[test]
    fn golden_receipt_json_for_release_version_surface() {
        let receipt = build_receipt_payload(
            plan_for(&["rust-toolchain.toml"]),
            true,
            "2026-05-07T12:00:00Z".to_string(),
        )
        .unwrap();

        let rendered = serde_json::to_string_pretty(&receipt).expect("receipt serializes");

        assert_eq!(
            rendered,
            r#"{
  "base": "origin/master",
  "head": "abcdef1234567890",
  "changed_files": [
    "rust-toolchain.toml"
  ],
  "changed_surfaces": [
    "release_version"
  ],
  "required_proof": [
    {
      "command": "cargo xtask fmt",
      "why": "keeps formatting deterministic before any other proof",
      "evidence": "PR body lists `cargo xtask fmt` as passed and the branch has no formatting-only drift"
    },
    {
      "command": "just version-check",
      "why": "release/version surfaces changed and version declarations must stay aligned",
      "evidence": "PR body lists `just version-check` as passed"
    },
    {
      "command": "just release-check",
      "why": "release-facing files changed and release hygiene should be validated",
      "evidence": "PR body lists `just release-check` as passed"
    },
    {
      "command": "git diff --check",
      "why": "guards against whitespace errors in the final patch",
      "evidence": "command exits cleanly after all edits"
    }
  ],
  "optional_proof": [
    {
      "command": "just pr-fast",
      "why": "cheap broader proof when the change spans more than docs or one small module",
      "evidence": "useful PR-body evidence when you want confidence before pushing"
    },
    {
      "command": "just ci-gate",
      "why": "local approximation of the merge-blocking CI gate",
      "evidence": "optional unless the change is broad, risky, or CI-only behavior is unclear"
    }
  ],
  "agent_hints": [
    "Use `just agent-check`, `just agent-test`, and `just agent-clippy` for large agent-run compile/test loops.",
    "Use `just agent-pr-fast` when you need the PR-fast gate through cargo-safe agent profiles."
  ],
  "worktree_clean": true,
  "generated_at": "2026-05-07T12:00:00Z"
}"#
        );
    }

    #[test]
    fn receipt_json_contract_keeps_agent_handoff_fields_stable()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let receipt = build_receipt_payload(
            plan_for(&[
                "docs/project/status/parser.md",
                "crates/perl-lsp-rs/src/runtime/text_sync.rs",
                "CHANGELOG.md",
            ]),
            false,
            "2026-05-07T12:34:56Z".to_string(),
        )?;

        let value = serde_json::to_value(&receipt)?;
        let object = value
            .as_object()
            .ok_or_else(|| std::io::Error::other("receipt should serialize as an object"))?;
        assert_object_keys(
            object,
            &[
                "base",
                "head",
                "changed_files",
                "changed_surfaces",
                "required_proof",
                "optional_proof",
                "agent_hints",
                "worktree_clean",
                "generated_at",
            ],
        );
        assert_eq!(value["base"], "origin/master");
        assert_eq!(value["head"], "abcdef1234567890");
        assert_eq!(value["worktree_clean"], false);
        assert_eq!(value["generated_at"], "2026-05-07T12:34:56Z");
        assert_eq!(
            value["changed_files"],
            serde_json::json!([
                "docs/project/status/parser.md",
                "crates/perl-lsp-rs/src/runtime/text_sync.rs",
                "CHANGELOG.md"
            ])
        );
        assert_eq!(
            value["changed_surfaces"],
            serde_json::json!([
                "parser_accuracy",
                "generated_status_docs",
                "memory_sensitive_runtime",
                "retained_owner_candidate",
                "release_version",
                "rust_code",
                "docs"
            ])
        );

        let required = value["required_proof"]
            .as_array()
            .ok_or_else(|| std::io::Error::other("required proof array"))?;
        assert!(required.len() >= 2, "required proof should not collapse to an empty contract");
        for proof in required {
            let proof_object = proof
                .as_object()
                .ok_or_else(|| std::io::Error::other("proof command should be an object"))?;
            assert_object_keys(proof_object, &["command", "why", "evidence"]);
            assert!(proof["command"].as_str().is_some_and(|command| !command.is_empty()));
            assert!(proof["why"].as_str().is_some_and(|why| !why.is_empty()));
            assert!(proof["evidence"].as_str().is_some_and(|evidence| !evidence.is_empty()));
        }

        let optional = value["optional_proof"]
            .as_array()
            .ok_or_else(|| std::io::Error::other("optional proof array"))?;
        assert!(optional.iter().any(|proof| proof["command"] == "just pr-fast"));
        for proof in optional {
            let proof_object = proof
                .as_object()
                .ok_or_else(|| std::io::Error::other("optional proof should be an object"))?;
            assert_object_keys(proof_object, &["command", "why", "evidence"]);
        }

        let hints = value["agent_hints"]
            .as_array()
            .ok_or_else(|| std::io::Error::other("agent hints array"))?;
        assert!(
            hints.iter().any(|hint| {
                hint.as_str().is_some_and(|hint| hint.contains("just agent-check"))
            })
        );
        assert!(hints.iter().any(|hint| {
            hint.as_str().is_some_and(|hint| hint.contains("For memory-sensitive edits"))
        }));
        assert!(hints.iter().any(|hint| {
            hint.as_str().is_some_and(|hint| hint.contains("For parser-accuracy edits"))
        }));

        Ok(())
    }

    #[test]
    fn golden_cockpit_output_for_memory_retained_owner_surface() {
        let plan = plan_for(&["crates/perl-lsp-rs/src/runtime/text_sync.rs"]);
        let worktree = WorktreeState {
            branch: "codex/devex-golden-renderer-tests".to_string(),
            head_short: "abcdef123".to_string(),
            clean: false,
            staged_present: true,
            unstaged_present: false,
            untracked_present: true,
        };

        let rendered = render_cockpit(CockpitView {
            plan: &plan,
            worktree: &worktree,
            receipt_path: Path::new("target/devex/local-proof.json"),
        });

        assert_eq!(
            rendered,
            r#"PR Cockpit
----------
Base:                  origin/master
Head:                  abcdef123
Branch:                codex/devex-golden-renderer-tests
Worktree clean:        no
Staged changes:        yes
Unstaged changes:      no
Untracked files:       yes
Changed files:         1
Changed surfaces:      memory_sensitive_runtime, retained_owner_candidate, rust_code
Required proof:        4 commands
  - cargo xtask fmt
  - cargo xtask check-memory-lifecycle-policy
  - cargo xtask check-memory-retained-owner-drift --base origin/master
  - git diff --check
Optional proof:        2 commands
  - just pr-fast
  - just ci-gate
Memory owner drift:    run retained-owner drift proof
Release/version drift: not applicable
Agent-safe path:       available
Receipt written:       target/devex/local-proof.json

Agent hints:
  - Use `just agent-check`, `just agent-test`, and `just agent-clippy` for large agent-run compile/test loops.
  - Use `just agent-pr-fast` when you need the PR-fast gate through cargo-safe agent profiles.
  - For memory-sensitive edits, keep focused lifecycle/cache tests in the PR body alongside policy proof.

Next:
  - Paste receipt summary into the PR body.
  - Run missing required proof commands.
  - Use `cargo xtask devex receipt --base origin/master --output target/devex/local-proof.json` for JSON handoff.
"#
        );
    }

    #[test]
    fn golden_pr_body_output_for_parser_status_surface() {
        let plan = plan_for(&["docs/project/status/parser.md"]);

        assert_eq!(
            render_pr_body(PrBodyView {
                plan: &plan,
                receipt_path: Path::new("target/devex/local-proof.json"),
            }),
            r#"## Proof packet

Changed surfaces:
- parser_accuracy
- generated_status_docs
- docs

Required proof:
- [ ] cargo xtask fmt
  - why: keeps formatting deterministic before any other proof
  - evidence: PR body lists `cargo xtask fmt` as passed and the branch has no formatting-only drift

- [ ] just ci-metrics-ratchet-check parser_accuracy
  - why: parser fixtures, baselines, or parser status changed
  - evidence: attach/pass parser accuracy ratchet output or explain an intentional baseline update

- [ ] just status-update
  - why: generated status docs changed and should be regenerated from source data
  - evidence: generated status diffs are present when expected

- [ ] just status-check
  - why: generated status docs should match their checked-in sources
  - evidence: PR body lists `just status-check` as passed

- [ ] git diff --check
  - why: guards against whitespace errors in the final patch
  - evidence: command exits cleanly after all edits


Optional proof:
- [ ] just pr-fast

- [ ] just ci-gate

- [ ] just cpan-corpus-check

- [ ] just corpus-sweep-check


Agent hints:
- Use `just agent-check`, `just agent-test`, and `just agent-clippy` for large agent-run compile/test loops.
- Use `just agent-pr-fast` when you need the PR-fast gate through cargo-safe agent profiles.
- For parser-accuracy edits, attach ratchet output or explain intentional baseline/status changes.

Receipt:
- target/devex/local-proof.json
"#
        );
    }

    #[test]
    fn golden_scenarios_cover_devex_surface_families() {
        let cases = [
            ("docs-only", &["docs/how-to/LOCAL_WATCH_MODE.md"][..]),
            ("parser + generated status", &["docs/project/status/parser.md"][..]),
            ("memory + retained owner", &["crates/perl-lsp-rs/src/runtime/text_sync.rs"][..]),
            ("release/version", &["rust-toolchain.toml"][..]),
            ("policy/CI", &[".github/workflows/ci.yml"][..]),
            (
                "mixed broad change",
                &[
                    "docs/project/status/parser.md",
                    "crates/perl-lsp-rs/src/runtime/text_sync.rs",
                    "CHANGELOG.md",
                    ".github/workflows/ci.yml",
                ][..],
            ),
            ("empty/no-change", &[][..]),
        ];

        for (name, files) in cases {
            let plan = plan_for(files);
            assert!(
                !render_plan(&plan).is_empty(),
                "{name}: plan renderer should have golden coverage input"
            );
            assert!(
                !render_pr_body(PrBodyView {
                    plan: &plan,
                    receipt_path: Path::new("target/devex/local-proof.json"),
                })
                .is_empty(),
                "{name}: PR-body renderer should have golden coverage input"
            );
            assert!(
                build_receipt_payload(plan, true, "2026-05-07T12:00:00Z".to_string()).is_ok(),
                "{name}: receipt payload should build for golden coverage input"
            );
        }
    }

    #[test]
    fn plan_routes_parser_accuracy_status_memory_and_release_surfaces() {
        let plan = build_plan(
            "origin/master".to_string(),
            "abc123".to_string(),
            strings(&[
                "xtask/src/tasks/metrics/parser_accuracy.rs",
                "docs/project/status/parser.md",
                "crates/perl-lsp-rs/src/runtime/text_sync.rs",
                "CHANGELOG.md",
            ]),
        );

        assert!(plan.surfaces.contains(&Surface::ParserAccuracy));
        assert!(plan.surfaces.contains(&Surface::GeneratedStatusDocs));
        assert!(plan.surfaces.contains(&Surface::MemorySensitiveRuntime));
        assert!(plan.surfaces.contains(&Surface::RetainedOwnerCandidate));
        assert!(plan.surfaces.contains(&Surface::ReleaseVersion));
        let commands = command_strings(&plan.required_commands);
        assert!(commands.contains(&"just ci-metrics-ratchet-check parser_accuracy".to_string()));
        assert!(commands.contains(&"just status-update".to_string()));
        assert!(commands.contains(&"just status-check".to_string()));
        assert!(commands.contains(&"cargo xtask check-memory-lifecycle-policy".to_string()));
        assert!(commands.contains(
            &"cargo xtask check-memory-retained-owner-drift --base origin/master".to_string()
        ));
        assert!(commands.contains(&"just version-check".to_string()));
        assert!(commands.contains(&"just release-check".to_string()));
        assert!(commands.contains(&"git diff --check".to_string()));
        assert!(plan.required_commands.iter().all(|proof| !proof.why.is_empty()));
        assert!(plan.required_commands.iter().all(|proof| !proof.evidence.is_empty()));
    }

    #[test]
    fn plan_keeps_docs_only_changes_lightweight() {
        let plan = build_plan(
            "origin/master".to_string(),
            "abc123".to_string(),
            strings(&["docs/reference/COMMANDS_REFERENCE.md"]),
        );

        assert!(plan.surfaces.contains(&Surface::Docs));
        assert!(!plan.surfaces.contains(&Surface::ParserAccuracy));
        assert_eq!(
            command_strings(&plan.required_commands),
            vec!["cargo xtask fmt".to_string(), "git diff --check".to_string()]
        );
        assert!(command_strings(&plan.optional_commands).contains(&"just pr-fast".to_string()));
    }

    #[test]
    fn changed_file_lists_include_committed_staged_unstaged_and_untracked_paths() {
        let files = merge_changed_file_lists(&[
            "CONTRIBUTING.md\n",
            "docs/reference/COMMANDS_REFERENCE.md\nCONTRIBUTING.md\n",
            "xtask/src/tasks/devex_plan.rs\n",
            "xtask/src/tasks/devex_receipt.rs\n",
        ]);

        assert_eq!(
            files,
            vec![
                "CONTRIBUTING.md".to_string(),
                "docs/reference/COMMANDS_REFERENCE.md".to_string(),
                "xtask/src/tasks/devex_plan.rs".to_string(),
                "xtask/src/tasks/devex_receipt.rs".to_string(),
            ]
        );
    }

    #[test]
    fn receipt_serializes_plan_with_machine_readable_surfaces() {
        let plan = build_plan(
            "origin/master".to_string(),
            "abcdef1234567890".to_string(),
            strings(&[
                "docs/project/status/parser.md",
                "crates/perl-lsp-rs/src/runtime/text_sync.rs",
                "CHANGELOG.md",
            ]),
        );

        let receipt =
            build_receipt_payload(plan, true, "2026-05-07T12:00:00Z".to_string()).unwrap();
        let value = serde_json::to_value(&receipt).expect("receipt should serialize");

        assert_eq!(value["base"], "origin/master");
        assert_eq!(value["head"], "abcdef1234567890");
        assert_eq!(value["worktree_clean"], true);
        assert_eq!(value["generated_at"], "2026-05-07T12:00:00Z");
        let surfaces = value["changed_surfaces"].as_array().expect("surfaces array");
        assert!(surfaces.iter().any(|surface| surface == "generated_status_docs"));
        assert!(surfaces.iter().any(|surface| surface == "memory_sensitive_runtime"));
        assert!(surfaces.iter().any(|surface| surface == "retained_owner_candidate"));
        assert!(surfaces.iter().any(|surface| surface == "release_version"));

        let required = value["required_proof"].as_array().expect("required proof array");
        assert!(required.iter().any(|proof| {
            proof["command"] == "cargo xtask check-memory-lifecycle-policy"
                && proof["why"].as_str().is_some_and(|why| !why.is_empty())
                && proof["evidence"].as_str().is_some_and(|evidence| !evidence.is_empty())
        }));
    }

    #[test]
    fn cockpit_renders_branch_state_proof_summary_and_receipt_path() {
        let plan = build_plan(
            "origin/master".to_string(),
            "abcdef1234567890".to_string(),
            strings(&[
                "docs/project/status/parser.md",
                "crates/perl-lsp-rs/src/runtime/text_sync.rs",
                "CHANGELOG.md",
            ]),
        );
        let worktree = WorktreeState {
            branch: "feature/devex".to_string(),
            head_short: "abcdef123".to_string(),
            clean: true,
            staged_present: false,
            unstaged_present: false,
            untracked_present: false,
        };

        let rendered = render_cockpit(CockpitView {
            plan: &plan,
            worktree: &worktree,
            receipt_path: Path::new("target/devex/local-proof.json"),
        });

        assert!(rendered.contains("PR Cockpit"));
        assert!(rendered.contains("Base:                  origin/master"));
        assert!(rendered.contains("Head:                  abcdef123"));
        assert!(rendered.contains("Branch:                feature/devex"));
        assert!(rendered.contains("Worktree clean:        yes"));
        assert!(rendered.contains("Changed surfaces:      parser_accuracy"));
        assert!(rendered.contains("generated_status_docs"));
        assert!(rendered.contains("memory_sensitive_runtime"));
        assert!(rendered.contains("Required proof:"));
        assert!(rendered.contains("just ci-metrics-ratchet-check parser_accuracy"));
        assert!(
            rendered.contains("cargo xtask check-memory-retained-owner-drift --base origin/master")
        );
        assert!(rendered.contains("just release-check"));
        assert!(rendered.contains("cargo xtask check-memory-lifecycle-policy"));
        assert!(rendered.contains("Memory owner drift:    run retained-owner drift proof"));
        assert!(rendered.contains("Release/version drift: run release proof"));
        assert!(rendered.contains("Agent-safe path:       available"));
        assert!(rendered.contains("Receipt written:       target/devex/local-proof.json"));
        assert!(rendered.contains("Next:"));
    }

    #[test]
    fn pr_body_renders_paste_ready_proof_packet() {
        let plan = build_plan(
            "origin/master".to_string(),
            "abcdef1234567890".to_string(),
            strings(&[
                "docs/project/status/parser.md",
                "crates/perl-lsp-rs/src/runtime/text_sync.rs",
                "CHANGELOG.md",
            ]),
        );

        let rendered = render_pr_body(PrBodyView {
            plan: &plan,
            receipt_path: Path::new("target/devex/local-proof.json"),
        });

        assert!(rendered.starts_with("## Proof packet"));
        assert!(rendered.contains("Changed surfaces:"));
        assert!(rendered.contains("- parser_accuracy"));
        assert!(rendered.contains("- generated_status_docs"));
        assert!(rendered.contains("- memory_sensitive_runtime"));
        assert!(rendered.contains("Required proof:"));
        assert!(rendered.contains("- [ ] cargo xtask fmt"));
        assert!(rendered.contains("- [ ] just ci-metrics-ratchet-check parser_accuracy"));
        assert!(rendered.contains("- [ ] cargo xtask check-memory-lifecycle-policy"));
        assert!(rendered.contains("  - why: "));
        assert!(rendered.contains("  - evidence: "));
        assert!(rendered.contains("Optional proof:"));
        assert!(rendered.contains("- [ ] just pr-fast"));
        assert!(rendered.contains("Agent hints:"));
        assert!(rendered.contains("Receipt:"));
        assert!(rendered.contains("- target/devex/local-proof.json"));
    }
}
