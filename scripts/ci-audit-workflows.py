#!/usr/bin/env python3
"""
CI Workflow Spend Audit

Enforces that expensive jobs in GitHub Actions workflows are gated behind labels
or workflow_dispatch, preventing silent spend regression on PRs.

Exit codes:
  0 - All jobs are properly gated or allowlisted
  1 - Found ungated expensive jobs (potential spend risk)
"""

import sys
import yaml
from pathlib import Path

# Workflows that are allowed to have ungated jobs (cheap/essential)
ALLOWED_WORKFLOWS = {
    "ci.yml",           # Core fast gate (fmt → clippy → test → docs)
    "check-ignored.yml", # Cheap check
}

# Jobs that are allowed to run ungated (very cheap, ~seconds)
ALLOWED_UNGATED_JOBS = {
    "tautology-check",  # ~1s grep-based check
    "test-metrics",     # ~2s metric counting
    "fmt",              # Fast rustfmt check
    "clippy",           # Fast lint (when cached)
}


def parse_workflow(path: Path) -> dict:
    """Parse a workflow YAML file."""
    with open(path) as f:
        return yaml.safe_load(f)


def has_pr_trigger(workflow: dict) -> bool:
    """Check if workflow runs on pull_request events."""
    on_field = workflow.get("on", {})
    if isinstance(on_field, list):
        return "pull_request" in on_field
    if isinstance(on_field, dict):
        return "pull_request" in on_field
    if isinstance(on_field, str):
        return on_field == "pull_request"
    return False


def is_gated(job: dict) -> bool:
    """Check if a job has an if: condition (gated)."""
    return "if" in job


def audit_workflows(workflows_dir: Path) -> list[str]:
    """Audit all workflows for ungated expensive jobs."""
    violations = []

    for wf_path in workflows_dir.glob("*.yml"):
        wf_name = wf_path.name

        # Skip allowlisted workflows
        if wf_name in ALLOWED_WORKFLOWS:
            continue

        try:
            workflow = parse_workflow(wf_path)
        except yaml.YAMLError as e:
            violations.append(f"{wf_name}: YAML parse error: {e}")
            continue

        if not workflow:
            continue

        # Only check workflows with PR triggers
        if not has_pr_trigger(workflow):
            continue

        jobs = workflow.get("jobs", {})
        for job_name, job_config in jobs.items():
            # Skip allowlisted jobs
            if job_name in ALLOWED_UNGATED_JOBS:
                continue

            if not isinstance(job_config, dict):
                continue

            if not is_gated(job_config):
                violations.append(
                    f"{wf_name}:{job_name} - runs on PRs without if: condition"
                )

    return violations


def main():
    workflows_dir = Path(".github/workflows")

    if not workflows_dir.exists():
        print("✓ No .github/workflows directory found")
        return 0

    violations = audit_workflows(workflows_dir)

    if violations:
        print("❌ CI Spend Audit Failed")
        print()
        print("The following jobs run on every PR without gating:")
        for v in violations:
            print(f"  - {v}")
        print()
        print("Fix by adding one of:")
        print("  1. if: contains(github.event.pull_request.labels.*.name, 'ci:<label>')")
        print("  2. Add job to ALLOWED_UNGATED_JOBS in scripts/ci-audit-workflows.py")
        print("  3. Add workflow to ALLOWED_WORKFLOWS (if entire workflow is cheap)")
        return 1

    print("✓ CI workflow spend audit passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
