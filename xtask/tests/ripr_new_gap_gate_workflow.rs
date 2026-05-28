//! Contract tests for ready-for-review RIPR workflow routing.

use std::fs;
use std::path::PathBuf;

fn project_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("CARGO_MANIFEST_DIR has no parent")?
        .to_path_buf())
}

#[test]
fn ripr_workflow_runs_on_ready_for_review_without_path_filter()
-> Result<(), Box<dyn std::error::Error>> {
    let root = project_root()?;
    let workflow = fs::read_to_string(root.join(".github/workflows/ripr.yml"))?;
    let xtask_main = fs::read_to_string(root.join("xtask/src/main.rs"))?;

    assert!(workflow.contains("pull_request:"), "ripr.yml must run from pull_request events");
    assert!(
        workflow.contains("types: [opened, synchronize, reopened, ready_for_review]"),
        "ripr.yml must rerun when a draft PR becomes ready for review because the job skips draft PRs"
    );
    assert!(
        !workflow.contains("\n    paths:"),
        "ripr.yml must not path-filter the ready-for-review proof run"
    );
    assert!(
        workflow.contains("if: github.event.pull_request.draft != true"),
        "ripr.yml may skip draft PRs while they are still draft"
    );
    let gate_step = workflow_step(&workflow, "Enforce new RIPR gap quality gate")
        .ok_or("missing RIPR gate step")?;
    assert!(
        !gate_step.contains("continue-on-error: true"),
        "RIPR workflow is now promoted past PR1 routing-only mode and must block new-gap failures"
    );
    assert!(
        workflow.contains("cargo xtask ripr-pr --base") && workflow.contains("target/ripr/pr/**"),
        "ripr.yml must produce and upload diff-scoped RIPR PR receipts"
    );
    assert!(
        workflow.contains("cargo xtask ripr-plus --receipt target/receipts/quality/ripr-plus.json")
            && workflow.contains(
                "cargo xtask ripr-plus --receipt target/receipts/quality/ripr-plus.json --check"
            )
            && workflow.contains("target/receipts/quality/ripr-plus.json"),
        "ripr.yml must generate, check, and upload the repo-wide RIPR+ receipt"
    );
    assert!(
        xtask_main.contains("RiprPlus")
            && xtask_main.contains("ripr_plus(&root, &receipt, &suppressions, check)")
            && xtask_main.contains("default_value = \"policy/ripr-suppressions.toml\""),
        "PR1 workflow must not call a missing `cargo xtask ripr-plus` command"
    );
    assert!(
        workflow.contains("cargo xtask ripr-review-comments --base")
            && workflow.contains("target/ripr/review/**"),
        "ripr.yml must generate and upload review-guidance receipts"
    );
    let validate_step = workflow_step(&workflow, "Validate PR evidence contracts")
        .ok_or("missing validate step")?;
    for check_command in [
        "cargo xtask ripr-pr --base",
        "cargo xtask ripr-plus --receipt target/receipts/quality/ripr-plus.json --check",
        "cargo xtask ripr-review-comments --base",
    ] {
        assert!(
            validate_step.contains(check_command) && validate_step.contains("--check"),
            "PR1 validate step must check `{check_command}`"
        );
    }
    let upload_step =
        workflow_step(&workflow, "Upload ripr PR evidence").ok_or("missing upload step")?;
    for artifact_path in [
        "target/ripr/pr/**",
        "target/ripr/review/**",
        "target/xtask/impacted-evidence/**",
        "target/receipts/quality/ripr-plus.json",
    ] {
        assert!(
            upload_step.contains(artifact_path),
            "PR1 upload step must include `{artifact_path}`"
        );
    }
    assert!(
        upload_step.contains("if-no-files-found: error"),
        "RIPR proof artifacts are required after PR8"
    );
    let summary_step =
        workflow_step(&workflow, "Append PR evidence summary").ok_or("missing summary step")?;
    assert!(
        summary_step.contains("if: always()") && summary_step.contains("target/ripr/pr/summary.md"),
        "RIPR summary step must publish PR evidence even when earlier receipt steps fail"
    );

    Ok(())
}

#[test]
fn ripr_docs_describe_unfiltered_ready_for_review_receipt_routing()
-> Result<(), Box<dyn std::error::Error>> {
    let root = project_root()?;
    let docs = fs::read_to_string(root.join("docs/ci/ripr.md"))?;
    let posture = section_block(&docs, "## Current routing posture")
        .ok_or("docs/ci/ripr.md is missing the current routing posture section")?;
    let when_it_runs = section_block(&docs, "## When it runs")
        .ok_or("docs/ci/ripr.md is missing the When it runs section")?;
    let behavior = section_block(&docs, "## Behavior")
        .ok_or("docs/ci/ripr.md is missing the Behavior section")?;

    assert!(
        posture.contains("blocks PRs that introduce")
            && posture.contains("Repo-wide")
            && posture.contains("RIPR+ total zero remains a burn-down target"),
        "RIPR docs must describe the promoted new-gap blocking posture without final total-zero enforcement"
    );
    assert!(
        when_it_runs.contains("Every PR targeting `master` or `main`"),
        "RIPR docs must describe the workflow as an every-PR proof run"
    );
    assert!(
        when_it_runs.contains("No path filter is applied")
            && when_it_runs.contains("docs-only")
            && when_it_runs.contains("policy-only")
            && when_it_runs.contains("workflow-only"),
        "RIPR docs must make docs/policy/workflow-only PR coverage explicit"
    );
    assert!(
        when_it_runs.contains("ready_for_review"),
        "RIPR docs must say draft PRs run the workflow when they become ready"
    );
    assert!(
        behavior.contains("target/ripr/pr/")
            && behavior.contains("target/receipts/quality/ripr-plus.json")
            && behavior.contains("target/ripr/review/"),
        "RIPR docs must name diff-scoped, repo-wide, and review-guidance receipts"
    );
    for forbidden in ["Blocks merges", "quality-gate --mode enforce "] {
        assert!(
            !docs.contains(forbidden),
            "RIPR docs must not carry final quality-gate text `{forbidden}`"
        );
    }

    Ok(())
}

#[test]
fn ripr_docs_use_rtk_for_local_proof_commands() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root()?;
    let docs = fs::read_to_string(root.join("docs/ci/ripr.md"))?;
    let block = fenced_block_after(&docs, "## Running locally")
        .ok_or("docs/ci/ripr.md is missing the Running locally command block")?;
    let commands = block.lines().filter(|line| !line.trim().is_empty()).collect::<Vec<_>>();
    assert!(!commands.is_empty(), "RIPR local proof block must list commands");
    for command in &commands {
        assert!(command.starts_with("rtk "), "RIPR local proof command must use rtk: {command}");
        assert!(
            !command.contains("quality-gate --mode enforce "),
            "RIPR local proof commands must not run final enforcement before burn-down: {command}"
        );
    }
    for required in [
        "cargo xtask ripr-pr --base origin/HEAD --head HEAD",
        "cargo xtask ripr-plus --receipt target/receipts/quality/ripr-plus.json",
        "cargo xtask ripr-review-comments --base origin/HEAD --head HEAD",
        "cargo xtask quality-gate --mode enforce-new-ripr",
        "cargo xtask ripr-pr --base origin/HEAD --head HEAD --check",
        "cargo xtask ripr-plus --receipt target/receipts/quality/ripr-plus.json --check",
        "cargo xtask ripr-review-comments --base origin/HEAD --head HEAD --check",
        "cargo xtask quality-gate --mode enforce-new-ripr",
    ] {
        assert!(
            commands.iter().any(|command| command.contains(required)),
            "RIPR local proof block must include `{required}`"
        );
    }

    Ok(())
}

fn fenced_block_after<'a>(content: &'a str, heading: &str) -> Option<&'a str> {
    let start = content.find(heading)?;
    let rest = &content[start..];
    let fence_start = rest.find("```bash")? + "```bash".len();
    let after_start = &rest[fence_start..];
    let body_start = after_start.strip_prefix('\n').unwrap_or(after_start);
    let fence_end = body_start.find("```")?;
    Some(&body_start[..fence_end])
}

fn section_block<'a>(content: &'a str, heading: &str) -> Option<&'a str> {
    let start = content.find(heading)?;
    let rest = &content[start..];
    let next = rest
        .lines()
        .skip(1)
        .scan(heading.len() + 1, |offset, line| {
            let current = *offset;
            *offset += line.len() + 1;
            Some((current, line))
        })
        .find_map(
            |(offset, line)| {
                if line.starts_with("## ") && line != heading { Some(offset) } else { None }
            },
        )
        .unwrap_or(rest.len());
    Some(&rest[..next])
}

fn workflow_step<'a>(content: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("- name: {name}");
    let start = content.find(&needle)?;
    let rest = &content[start..];
    let next = rest
        .lines()
        .skip(1)
        .scan(needle.len() + 1, |offset, line| {
            let current = *offset;
            *offset += line.len() + 1;
            Some((current, line))
        })
        .find_map(
            |(offset, line)| {
                if line.trim_start().starts_with("- name:") { Some(offset) } else { None }
            },
        )
        .unwrap_or(rest.len());
    Some(&rest[..next])
}
