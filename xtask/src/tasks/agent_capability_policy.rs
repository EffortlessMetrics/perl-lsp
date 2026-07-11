//! Enforce the M4b capability boundary: review/audit agents are mechanically
//! read-only.
//!
//! Issue #3763 (ledger milestone M4 exit criterion — "review workflows
//! mechanically cannot write") requires that every review/audit agent profile
//! *excludes* write/mutating tools from its allowlist, rather than merely
//! prompting the model against writing. Workflow subagents run in
//! `acceptEdits` and inherit the parent's tools, so a prompt-level "REVIEW
//! ONLY" instruction is not a control — the tool allowlist is.
//!
//! This check parses `.claude/agents/*.md` frontmatter and fails if any agent
//! in the review/audit cohort:
//!   * has no explicit `tools:` allowlist (fail-closed: no allowlist means it
//!     inherits every tool, including Edit/Write), or
//!   * lists any forbidden write/mutating tool (Edit, Write, NotebookEdit,
//!     MultiEdit, Agent/Task sub-agent spawn, Artifact).
//!
//! Writer agents (builder, pr-responder, green-*, red-tdd, ops, lead-*,
//! spec-planner, ...) are intentionally NOT in the cohort — they legitimately
//! write. The reference read-only shape is the built-in `Explore` agent, whose
//! tools exclude Edit/Write/NotebookEdit/Agent.

use color_eyre::eyre::{Result, bail, eyre};
use serde_yaml_ng::Value as YamlValue;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Agents that inspect someone else's work and render a verdict (or file a
/// read-derived report). They must be mechanically read-only: no source edit,
/// no sub-agent spawn (which would proxy a write).
///
/// Keep this list in sync with `.claude/agents/AGENT_CATALOG.md`. Adding a new
/// review/audit agent means adding its name here so the boundary is enforced.
pub const REVIEW_AUDIT_AGENTS: &[&str] = &[
    "reviewer",
    "reviewer-deep",
    "diff-auditor",
    "maintainer-pr",
    "maintainer-issue",
    "architecture-reviewer",
    "advocatus-diaboli",
    "accuracy-scout",
    "research-verifier",
    "oppositional-planner",
    "plan-reviewer",
    "spec-test-code-match",
    "scout-find-ci-ops-gaps",
    "scout-find-dap-gaps",
    "scout-find-docs-receipt-drift",
    "scout-find-lsp-gaps",
    "scout-find-parser-gaps",
    "scout-find-robustness-gaps",
];

/// Tool names that grant write / mutation / proxy-spawn capability. A
/// read-only reviewer allowlist must contain none of these.
///
/// `Agent`/`Task` are excluded because a reviewer that can spawn a sub-agent
/// can proxy an arbitrary write through it. `Artifact` publishes a hosted
/// page (a side effect a pure reviewer does not need).
pub const FORBIDDEN_WRITE_TOOLS: &[&str] =
    &["Edit", "Write", "MultiEdit", "NotebookEdit", "Agent", "Task", "Artifact"];

/// A single policy violation for one review/audit agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub agent: String,
    pub reason: String,
}

/// Validate that every review/audit agent under `root` is mechanically
/// read-only. `root` defaults to the repository root.
pub fn run(root: Option<PathBuf>) -> Result<()> {
    let root = match root {
        Some(root) => root,
        None => crate::utils::project_root()?,
    };
    let agents_dir = root.join(".claude/agents");
    let violations = audit_dir(&agents_dir)?;

    if violations.is_empty() {
        println!(
            "M4b capability boundary OK: {} review/audit agents are mechanically read-only ({})",
            REVIEW_AUDIT_AGENTS.len(),
            agents_dir.display()
        );
        return Ok(());
    }

    let mut message = String::from(
        "M4b capability boundary FAILED: review/audit agents must exclude write/mutating tools \
         (Edit/Write/NotebookEdit/MultiEdit/Agent/Task/Artifact) and carry an explicit read-only \
         tools: allowlist. Offenders:\n",
    );
    for violation in &violations {
        message.push_str(&format!("  - {}: {}\n", violation.agent, violation.reason));
    }
    bail!(message);
}

/// Audit every review/audit cohort agent that lives in `agents_dir`.
///
/// Returns the full list of violations (missing files + non-read-only
/// allowlists) so callers can report all problems at once.
pub fn audit_dir(agents_dir: &Path) -> Result<Vec<Violation>> {
    let mut violations = Vec::new();
    for agent in REVIEW_AUDIT_AGENTS {
        let path = agents_dir.join(format!("{agent}.md"));
        if !path.exists() {
            violations.push(Violation {
                agent: (*agent).to_string(),
                reason: format!("expected agent definition {} does not exist", path.display()),
            });
            continue;
        }
        if let Err(reason) = enforce_read_only(&path)? {
            violations.push(Violation { agent: (*agent).to_string(), reason });
        }
    }
    Ok(violations)
}

/// Check one agent definition file for the read-only invariant.
///
/// The outer `Result` carries hard I/O / parse errors; the inner
/// `Result<(), String>` carries a policy violation reason (`Err`) or
/// compliance (`Ok`).
pub fn enforce_read_only(path: &Path) -> Result<std::result::Result<(), String>> {
    let frontmatter = load_frontmatter(path)?;

    let Some(tools_value) = frontmatter.get("tools") else {
        return Ok(Err(
            "no explicit tools: allowlist — inherits all tools, including Edit/Write (fail-open)"
                .to_string(),
        ));
    };

    let tools = parse_tool_list(tools_value)
        .map_err(|error| eyre!("{}: could not parse tools: allowlist: {error}", path.display()))?;

    if tools.is_empty() {
        return Ok(Err("tools: allowlist is empty".to_string()));
    }

    let forbidden: Vec<String> = tools
        .iter()
        .filter(|tool| {
            FORBIDDEN_WRITE_TOOLS.iter().any(|forbidden| forbidden.eq_ignore_ascii_case(tool))
        })
        .cloned()
        .collect();

    if forbidden.is_empty() {
        Ok(Ok(()))
    } else {
        Ok(Err(format!("allowlist grants write/mutating tool(s): {}", forbidden.join(", "))))
    }
}

/// Parse a frontmatter `tools:` value into individual tool names.
///
/// Accepts either an inline scalar (`tools: Read, Grep, Glob`) — which YAML
/// parses as a single comma-joined string — or a YAML sequence
/// (`tools: [Read, Grep]` or a block list).
fn parse_tool_list(value: &YamlValue) -> std::result::Result<Vec<String>, String> {
    match value {
        YamlValue::String(raw) => Ok(raw
            .split(',')
            .map(str::trim)
            .filter(|tool| !tool.is_empty())
            .map(str::to_string)
            .collect()),
        YamlValue::Sequence(items) => {
            let mut tools = Vec::with_capacity(items.len());
            for item in items {
                let tool = item
                    .as_str()
                    .ok_or_else(|| "tools: sequence entries must be strings".to_string())?;
                let tool = tool.trim();
                if !tool.is_empty() {
                    tools.push(tool.to_string());
                }
            }
            Ok(tools)
        }
        YamlValue::Null => Ok(Vec::new()),
        other => Err(format!("tools: must be a string or sequence, got {other:?}")),
    }
}

/// Load and parse the YAML frontmatter mapping from an agent definition file.
fn load_frontmatter(path: &Path) -> Result<serde_yaml_ng::Mapping> {
    let text = fs::read_to_string(path)
        .map_err(|error| eyre!("failed to read {}: {error}", path.display()))?;
    let normalized = text.replace("\r\n", "\n");
    let Some(rest) = normalized.strip_prefix("---\n") else {
        bail!("{} must start with YAML frontmatter", path.display());
    };
    let Some((frontmatter, _body)) = rest.split_once("\n---") else {
        bail!("{} must contain opening and closing frontmatter markers", path.display());
    };
    let data: YamlValue = serde_yaml_ng::from_str(frontmatter)
        .map_err(|error| eyre!("{} has invalid YAML frontmatter: {error}", path.display()))?;
    data.as_mapping()
        .cloned()
        .ok_or_else(|| eyre!("{} frontmatter must parse to a mapping", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const READ_ONLY_TOOLS: &str = "Read, Grep, Glob, Bash, WebSearch, WebFetch, TodoWrite";

    fn write_agent(dir: &Path, name: &str, tools_line: Option<&str>) -> Result<PathBuf> {
        fs::create_dir_all(dir)
            .map_err(|error| eyre!("failed to create {}: {error}", dir.display()))?;
        let path = dir.join(format!("{name}.md"));
        let mut frontmatter = format!("---\nname: {name}\ndescription: test agent\nmodel: haiku\n");
        if let Some(tools) = tools_line {
            frontmatter.push_str(&format!("tools: {tools}\n"));
        }
        frontmatter.push_str("---\n\nagent body\n");
        fs::write(&path, frontmatter)
            .map_err(|error| eyre!("failed to write {}: {error}", path.display()))?;
        Ok(path)
    }

    #[test]
    fn real_repository_review_agents_are_read_only() -> Result<()> {
        // The live regression guard: the committed .claude/agents surface must
        // satisfy the M4b boundary. This runs under `cargo test --workspace`.
        run(None)
    }

    #[test]
    fn accepts_read_only_allowlist() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = write_agent(tmp.path(), "reviewer", Some(READ_ONLY_TOOLS))?;
        assert_eq!(enforce_read_only(&path)?, Ok(()));
        Ok(())
    }

    #[test]
    fn rejects_write_tool_in_allowlist() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = write_agent(tmp.path(), "reviewer", Some("Read, Grep, Glob, Edit, Write"))?;
        match enforce_read_only(&path)? {
            Ok(()) => bail!("a reviewer with Edit/Write must be rejected"),
            Err(reason) => {
                assert!(reason.contains("Edit"), "reason should name Edit: {reason}");
                assert!(reason.contains("Write"), "reason should name Write: {reason}");
            }
        }
        Ok(())
    }

    #[test]
    fn rejects_agent_sub_spawn_tool() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = write_agent(tmp.path(), "reviewer", Some("Read, Grep, Glob, Agent"))?;
        match enforce_read_only(&path)? {
            Ok(()) => bail!("a reviewer that can spawn sub-agents must be rejected"),
            Err(reason) => assert!(reason.contains("Agent"), "reason should name Agent: {reason}"),
        }
        Ok(())
    }

    #[test]
    fn rejects_missing_tools_allowlist_fail_closed() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = write_agent(tmp.path(), "reviewer", None)?;
        match enforce_read_only(&path)? {
            Ok(()) => bail!("a reviewer with no tools: allowlist inherits all tools and must fail"),
            Err(reason) => assert!(reason.contains("no explicit"), "reason: {reason}"),
        }
        Ok(())
    }

    #[test]
    fn audit_dir_flags_broken_fixture_and_missing_files() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let agents_dir = tmp.path().join(".claude/agents");
        // One cohort agent present but broken (has Write), the rest missing.
        write_agent(&agents_dir, "reviewer", Some("Read, Grep, Write"))?;
        let violations = audit_dir(&agents_dir)?;

        let reviewer = violations
            .iter()
            .find(|violation| violation.agent == "reviewer")
            .ok_or_else(|| eyre!("expected a reviewer violation"))?;
        assert!(reviewer.reason.contains("Write"), "reviewer reason: {}", reviewer.reason);

        // Every other cohort agent is missing => also a violation.
        assert_eq!(violations.len(), REVIEW_AUDIT_AGENTS.len());
        Ok(())
    }

    #[test]
    fn run_rejects_broken_agents_dir() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let agents_dir = tmp.path().join(".claude/agents");
        write_agent(&agents_dir, "reviewer", Some("Read, Grep, Edit"))?;
        match run(Some(tmp.path().to_path_buf())) {
            Ok(()) => bail!("run() must fail on a review agent with a write tool"),
            Err(error) => assert!(
                error.to_string().contains("M4b capability boundary FAILED"),
                "error: {error}"
            ),
        }
        Ok(())
    }

    #[test]
    fn parses_inline_and_sequence_tool_lists() -> Result<()> {
        let inline = serde_yaml_ng::from_str::<YamlValue>("Read, Grep, Glob")?;
        assert_eq!(
            parse_tool_list(&inline).map_err(|error| eyre!(error))?,
            vec!["Read".to_string(), "Grep".to_string(), "Glob".to_string()]
        );
        let seq = serde_yaml_ng::from_str::<YamlValue>("[Read, Write]")?;
        assert_eq!(
            parse_tool_list(&seq).map_err(|error| eyre!(error))?,
            vec!["Read".to_string(), "Write".to_string()]
        );
        Ok(())
    }
}
