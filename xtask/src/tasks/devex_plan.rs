//! Diff-aware DevEx proof planner.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use color_eyre::eyre::{Context, Result, bail};

use crate::utils::project_root;

#[derive(Debug, Clone)]
pub struct DevexPlanConfig {
    pub base: String,
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

pub fn run(config: DevexPlanConfig) -> Result<()> {
    let root = project_root()?;
    let base = resolve_diff_base(&root, &config.base)?;
    let changed_files = changed_files(&root, &base)?;
    let head = git_stdout(&root, &["rev-parse", "--short", "HEAD"])?;
    let plan = build_plan(base, head, changed_files);
    print_plan(&plan);
    Ok(())
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

fn build_plan(base: String, head: String, changed_files: Vec<String>) -> Plan {
    let surfaces = classify_surfaces(&changed_files);
    let required_commands = required_commands(&surfaces, &base);
    let optional_commands = optional_commands(&surfaces);
    let agent_hints = agent_hints(&surfaces);

    Plan { base, head, changed_files, surfaces, required_commands, optional_commands, agent_hints }
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
    println!("DevEx local proof plan");
    println!("Base: {}", plan.base);
    println!("Head: {}", plan.head.trim());
    println!();

    println!("Changed files:");
    if plan.changed_files.is_empty() {
        println!("- none");
    } else {
        for file in &plan.changed_files {
            println!("- {file}");
        }
    }
    println!();

    println!("Changed surfaces:");
    if plan.surfaces.is_empty() {
        println!("- none");
    } else {
        for surface in &plan.surfaces {
            println!("- {}", surface.label());
        }
    }
    println!();

    println!("Required local proof:");
    for proof in &plan.required_commands {
        println!("- {}", proof.command);
        println!("  why: {}", proof.why);
        println!("  evidence: {}", proof.evidence);
    }
    println!();

    println!("Optional / expensive:");
    for proof in &plan.optional_commands {
        println!("- {}", proof.command);
        println!("  why: {}", proof.why);
        println!("  evidence: {}", proof.evidence);
    }
    println!();

    println!("Agent-safe hints:");
    for hint in &plan.agent_hints {
        println!("- {hint}");
    }
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
}
