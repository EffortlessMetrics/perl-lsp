---
description: Start the Issue Discovery / Bug Scout Desk — fan out discovery scouts, collect candidate packets, triage
argument-hint: "[wave] e.g. 'all', 'dap', 'lsp', 'parser', 'ci-ops', 'robustness', 'docs' (default: first wave)"
---

# Issue Discovery / Bug Scout Desk

Run the swarm's radar. Fan out **read-only** discovery scouts to find
evidence-backed candidate issues, then triage. This lane is **upstream of
plan review** — it discovers high-signal findings; it does not build fixes
or mark issues `builder-ready`.

Doctrine: `docs/reference/ISSUE_DISCOVERY_DOCTRINE.md`.

Target: **$ARGUMENTS** (default: first wave — dap, lsp, parser, ci-ops,
robustness, docs)

## Core rule

> **Discovery can batch. Filing cannot.** Read-only sweeps run wide and in
> parallel. Mutations (file / label / dedupe a candidate issue) are
> issue-by-issue.

## Steps

1. **Fan out the first wave** — spawn these scouts in parallel (one message,
   multiple `Agent()` calls), each worktree-isolated:

   ```
   Agent(subagent_type: "scout-find-dap-gaps",        prompt: "Sweep DAP stack/scopes/variables/lifecycle/transport. Follow your todo list.", name: "find-dap-gaps")
   Agent(subagent_type: "scout-find-lsp-gaps",        prompt: "Sweep LSP document-state/URI/completion/hover/code-action/semantic-token. Follow your todo list.", name: "find-lsp-gaps")
   Agent(subagent_type: "scout-find-parser-gaps",     prompt: "Sweep parser/AST/NodeKind/recovery/fixtures. Follow your todo list.", name: "find-parser-gaps")
   Agent(subagent_type: "scout-find-ci-ops-gaps",     prompt: "Sweep workflow routing/path-filters/labels/cleanup/runner-capacity. Follow your todo list.", name: "find-ci-ops-gaps")
   Agent(subagent_type: "scout-find-robustness-gaps", prompt: "Sweep parser/lexer/LSP/DAP/transport for panic/DoS/unsafe-indexing. Follow your todo list.", name: "find-robustness-gaps")
   Agent(subagent_type: "scout-find-docs-receipt-drift", prompt: "Compare status docs against receipts for drift/basis conflicts. Follow your todo list.", name: "find-docs-drift")
   ```

   Wave two (spare capacity only): `scout-find-workspace-facts-gaps`,
   `scout-find-editor-ux-gaps`, and a test-quality cross-cut.

2. **Collect candidate packets.** Each scout returns ≤5 packets and files
   ≤2 high-confidence candidate issues via the Candidate Issue template
   (`.github/ISSUE_TEMPLATE/candidate_issue.yml`, labels `candidate-issue` +
   `swarm-discovered`). Read their final summaries — not their worktrees.

3. **Run the triage pass.** Do not build from findings. For each candidate,
   pick exactly one next lane: `keep` · `merge into existing issue` · `send
   to plan-review` · `send to architecture review` · `send to repro-lab` ·
   `discard as noise`. Dedupe by **failure mode**, never by file/theme/base
   overlap.

4. **Produce one triage table** — candidate · confidence · duplicate? · next
   lane — and recommend routing for the plan-review desk.

## Guardrails

- Scouts are read-only except filing/updating a candidate issue, one at a
  time. No builds, no PRs, no closing/retitling, no `builder-ready`.
- Max 2 filed issues per scout unless clearly high-confidence. The lane wins
  by signal quality, not volume.
- No high-frequency GitHub polling — use point-in-time snapshots.

## Output

A short report: the triage table, the filed candidate issue numbers, and the
handoff list for the Issue Research / Plan Review Desk. The headline metric
is not volume — it is the share of filed findings that survive plan review.
