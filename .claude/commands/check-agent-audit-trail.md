---
description: Triage agent audit-trail directory additions on a PR — distinguish legitimate self-attributed trails from cross-PR leaks
---

# Check Agent Audit-Trail Additions

Diff-auditors and reviewers use this when a PR adds content under an agent's dot-prefixed directory (`.hermes/`, `.jules/`, `.spec/`, `.run/`, `.codex/`, etc.). These directories hold agent planning / spec / audit trails — they are **normal and kept by default** when they represent the PR's own work, and **cleaned as scope drift** when they're leaks from another agent's parallel work.

See `memory/feedback_agent_audit_trail_directories.md` for the underlying pattern.

## When to use this skill

- You are diff-auditing a PR and see additions under `.hermes/`, `.spec/`, `.jules/`, `.run/`, or a similar agent directory.
- You are tempted to flag those additions as "scope drift" or "needs-diff-fix" — run this first.

**Don't use it for existing directories in the repo.** Only content that THIS PR ADDS matters. Pre-existing agent trails from prior PRs are always keep.

## Procedure

### 1. List additions under agent dot-directories

```bash
gh pr diff <PR> --name-only | grep -E '^\.(hermes|jules|spec|run|codex)/' || echo "NONE"
```

If output is `NONE`, there's nothing to triage under this skill.

### 2. Identify the PR's own issue/slug

```bash
# PR number is in the title suffix like (#NNNN) or (#0000)
# Branch name may be issue-ref-bearing, e.g. impl/5499-completion-scope-distance
gh pr view <PR> --json title,headRefName,number
```

Note the issue number or slug (e.g., `5499-completion-scope-distance`).

### 3. For each added path, check for issue match

For an added path like `.hermes/5714-perf/<files>`:

- **Match**: the path segment after the dot-dir includes THIS PR's issue number or slug → KEEP
- **Match (looser)**: the path segment includes a feature term clearly matching THIS PR's title → KEEP
- **Mismatch**: the segment is a different issue number, a different feature, or clearly unrelated → SCOPE DRIFT, flag for cleanup

### 4. Verdict

- **All additions match this PR's issue**: diff is CLEAN for agent-trail considerations. Set `diff-audited`. Do not flag.
- **One or more additions are unrelated**: flag `needs-diff-fix` and post a PR comment naming the specific paths to remove.
- **Genuinely unclear**: leave alone with a comment asking the PR author; default to KEEP rather than over-clean.

## Example decisions

| PR | Addition | Verdict |
|---|---|---|
| #5714 (Hermes perf feature) | `.hermes/5714-perf/plan.md` | KEEP (matches PR issue) |
| #5714 | `.hermes/5700-random-docs/spec.md` | SCOPE DRIFT (different issue) |
| Human-authored #5691 | `.hermes/5691/notes.md` | CHECK author. If co-authored with Hermes, keep. Otherwise clean. |
| Any PR | `.spec/4000-old/`  (pre-existing on master) | KEEP (not this PR's concern) |
| Docs-only PR | `.hermes/docs-x/` adding spec content | SCOPE DRIFT (docs PR shouldn't produce Hermes specs) |

## What NOT to do

- **Don't** add a blanket rule excluding agent dot-directories via `.gitignore`. They are valuable project memory when attributed correctly.
- **Don't** strip `.hermes/` / `.spec/` / etc. from a PR that legitimately produced them. The audit trail accompanies the work.
- **Don't** remove pre-existing agent directories on unrelated PRs. They belong to prior work.

## Outputs

When leak is detected, post a PR comment like:

> Scope drift found under `.hermes/`: this PR adds `.hermes/<other-issue>/<files>` which appear to be audit-trail content for a different PR/issue. Please remove these paths — your own `.hermes/<this-issue>/` content should stay.

When clean, no comment needed. Just set `diff-audited`.

## Related

- `memory/feedback_agent_audit_trail_directories.md` — full pattern description
- `memory/feedback_spec_folders_are_history.md` — `.spec/` original principle
- `diff-audit-check` skill — called from diff-auditor step 1
