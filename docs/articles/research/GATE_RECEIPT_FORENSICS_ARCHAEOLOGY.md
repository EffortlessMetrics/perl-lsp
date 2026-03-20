# Gate Receipt Forensics Archaeology
## How Issue #210 Made Proof Governance Executable And Inspectable

Issue `#210` is the point where the repo stops treating merge proof as a nice-to-have artifact and starts treating it as governance. The historical record is not a single commit; it is a chain that moves from planning language, to a shell receipt emitter, to a typed Rust gate harness, to CI check-run plumbing, and finally into the forensics prompt pack that audits the quality of those same proof surfaces.

---

## 1. The Issue Was Framed As A Trust-Surface Problem Before It Became Code

The repo's planning docs already place `#210` at the center of the trust surface.

[docs/forensics/IMPLEMENTATION_PHASES.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/worktrees/gate-receipt-forensics/docs/forensics/IMPLEMENTATION_PHASES.md) puts `#210` in Phase A under "Trust Surface Stabilization" and defines the goal as "one authoritative merge gate posture" with "predictable receipts" at lines 9-23. That is the first clear sign that proof governance had become a structural concern rather than a one-off review preference.

The same file later makes the dependency explicit in the batch plan: `#210` is grouped with `#211` as "Gate consolidation" at lines 134-135. A separate ops commit, `c55d292d5` on `2026-01-08`, tightens the sequencing again by adding a milestone verification recipe and a blockers section that orders `#211 -> #210 -> #143`. That commit matters because it shows the repo turning the issue into a release-order dependency, not just a backlog item.

---

## 2. The First Implementation Wave Turned The Issue Into A Real Harness

The main execution step lands in `21ec9bd54` on `2026-01-25`: `feat: implement standardized CI gate harness (#533)`. The commit message is explicit about the new surfaces:

- `xtask`-based gate runner with tiering and receipt generation
- gate policy definitions in `.ci/gate-policy.yaml`
- receipt schema in `.ci/receipt.schema.json`
- local CI documentation and workflow consolidation

Those surfaces still exist, and the current code shows what they became:

- [xtask/src/tasks/gates.rs](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/worktrees/gate-receipt-forensics/xtask/src/tasks/gates.rs) says it reads `.ci/gate-policy.yaml`, executes gates, captures timing/output/status, and generates receipts matching the schema at lines 1-8.
- The same file defines the structured gate policy and receipt types at lines 74-213.
- [.ci/gate-policy.yaml](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/worktrees/gate-receipt-forensics/.ci/gate-policy.yaml) declares itself the "single source of truth" for gate configuration at lines 5-7.
- [.ci/receipt.schema.json](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/worktrees/gate-receipt-forensics/.ci/receipt.schema.json) defines the machine-readable receipt contract, with required `metadata`, `gates`, and `summary` sections at lines 4-8.
- [justfile](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/worktrees/gate-receipt-forensics/justfile) exposes the issue-labeled `gates` recipe and points it at `cargo xtask gates` at lines 446-454.

There is also a historical bridge rather than a clean rewrite: [scripts/run-gates.sh](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/worktrees/gate-receipt-forensics/scripts/run-gates.sh) is the older shell emitter that writes `target/receipts/receipt.json` and records gate name, command, status, exit code, and duration at lines 1-18 and 104-158. The repo's arc is therefore not "shell then nothing"; it is "shell proof, then typed proof, then policy-aware proof."

---

## 3. CI Made The Receipt Inspectable, Not Just Present

The next step is the CI surface, where receipts stop being an opaque local artifact and become part of the hosted check-run lifecycle.

[.github/workflows/ci.yml](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/worktrees/gate-receipt-forensics/.github/workflows/ci.yml) runs `just gates`, uploads the receipt and logs as artifacts, prints a failure tail, renders a step summary, and publishes a commit status named `ci/merge-gate` at lines 69-173. That is the inspectability layer: the gate is no longer only "pass/fail"; the receipt is visible in workflow artifacts, summary text, and commit status.

The workflow hardening was not static. `1951d3878` on `2026-02-20` fixes the CI parser to match the receipt schema, specifically `gate_name`, `duration_ms`, and `skip` status semantics. That commit is important because it shows the repo debugging its own receipt reader, not just the gate runner.

`ece49f915` on `2026-02-28` adds the final check-run piece by publishing the merge-gate commit status. Taken together, the history says the repo was not satisfied with logs alone; it wanted a receipt that could drive both humans and GitHub's status API.

---

## 4. Status Updates Became A Second Governance Loop

`#210` is about gates, but the same proof governance later reaches project-status drift.

[xtask/src/tasks/update_status.rs](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/worktrees/gate-receipt-forensics/xtask/src/tasks/update_status.rs) is the Rust port of `scripts/update-current-status.py`. Its header at lines 1-5 says it updates `docs/project/CURRENT_STATUS.md` and `docs/project/ROADMAP.md`, computing metrics and patching the markdown between markers. The entry point at lines 22-73 makes the anti-drift behavior explicit: `--write` updates the docs, `--check` fails if they are stale, and the default mode is check.

The current project docs carry that contract forward. [docs/project/CURRENT_STATUS.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/worktrees/gate-receipt-forensics/docs/project/CURRENT_STATUS.md) says generated sections are machine-updated by `just status-update` at lines 11-16 and that `just status-update` plus `just status-check` are the anti-drift workflow at lines 74-79 and 148-153. [docs/project/ROADMAP.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/worktrees/gate-receipt-forensics/docs/project/ROADMAP.md) mirrors that separation by saying evidence belongs in `CURRENT_STATUS.md`, planning belongs in `ROADMAP.md`, and live capability posture can be checked with `just status-check` at lines 3-5 and 122-123.

This is the same governance move as `#210`, but applied to documentation truth: claims are not accepted because they are written down; they are accepted because the check path can re-derive them.

---

## 5. The Forensics Prompt Pack Generalized The Same Rule

The later forensics surfaces make the lineage obvious. They do not replace the gate/receipt model; they audit it.

[docs/forensics/README.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/worktrees/gate-receipt-forensics/docs/forensics/README.md) identifies `measurement-auditor.md` as the measurement-integrity analyzer and `policy-auditor.md` as the governance analyzer at lines 14-22. [docs/forensics/IMPLEMENTATION_PHASES.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/worktrees/gate-receipt-forensics/docs/forensics/IMPLEMENTATION_PHASES.md) then turns that into a shared contract, saying Phase C is a "Single semantics layer for all measurement tools" with a "Common receipt format" at lines 49-74.

The prompt files themselves show the inheritance:

- [docs/forensics/prompts/measurement-auditor.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/worktrees/gate-receipt-forensics/docs/forensics/prompts/measurement-auditor.md) audits whether numbers in dossiers, status updates, and receipts actually match what was measured, and requires commands, receipts, and git context at lines 3-60.
- That same file makes `not_comparable` the hard stop when the measurement contract is unstable at lines 202-216.
- [docs/forensics/prompts/policy-auditor.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/worktrees/gate-receipt-forensics/docs/forensics/prompts/policy-auditor.md) audits governance integrity, with required inputs for `features.toml`, `CURRENT_STATUS.md`, `just status-check`, and schema validation at lines 3-57.
- Its output schema explicitly checks catalog drift, metrics drift, schema compliance, and guardrail effectiveness at lines 59-167.

That is the long tail of `#210`: the repo did not just build a gate runner. It built a vocabulary for checking whether the gate runner, the docs, and the receipts still agree.

---

## 6. Historical Meaning

The lineage is:

1. `#209` exposes the danger of proof that is technically true but operationally weak.
2. `#210` converts that lesson into a merge-gate governance request.
3. `21ec9bd54` makes the request executable through policy, schema, and a structured gate runner.
4. `1951d3878` and `ece49f915` make CI consume and publish those receipts correctly.
5. `65c169835` makes status drift fail as a checked computation.
6. `measurement-auditor` and `policy-auditor` turn the whole pattern into audit surfaces.

That is the durable shift: proof governance becomes code, then CI behavior, then audit tooling. The repo ends up not merely asking for evidence, but requiring that evidence be executable, inspectable, and re-checkable from the same surfaces that produced it.

---

## Evidence Pointers

- [docs/forensics/IMPLEMENTATION_PHASES.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/worktrees/gate-receipt-forensics/docs/forensics/IMPLEMENTATION_PHASES.md)
- [xtask/src/tasks/gates.rs](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/worktrees/gate-receipt-forensics/xtask/src/tasks/gates.rs)
- [.ci/gate-policy.yaml](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/worktrees/gate-receipt-forensics/.ci/gate-policy.yaml)
- [.ci/receipt.schema.json](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/worktrees/gate-receipt-forensics/.ci/receipt.schema.json)
- [scripts/run-gates.sh](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/worktrees/gate-receipt-forensics/scripts/run-gates.sh)
- [justfile](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/worktrees/gate-receipt-forensics/justfile)
- [.github/workflows/ci.yml](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/worktrees/gate-receipt-forensics/.github/workflows/ci.yml)
- [xtask/src/tasks/update_status.rs](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/worktrees/gate-receipt-forensics/xtask/src/tasks/update_status.rs)
- [docs/project/CURRENT_STATUS.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/worktrees/gate-receipt-forensics/docs/project/CURRENT_STATUS.md)
- [docs/project/ROADMAP.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/worktrees/gate-receipt-forensics/docs/project/ROADMAP.md)
- [docs/forensics/README.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/worktrees/gate-receipt-forensics/docs/forensics/README.md)
- [docs/forensics/prompts/measurement-auditor.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/worktrees/gate-receipt-forensics/docs/forensics/prompts/measurement-auditor.md)
- [docs/forensics/prompts/policy-auditor.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/worktrees/gate-receipt-forensics/docs/forensics/prompts/policy-auditor.md)
- `c55d292d5`, `21ec9bd54`, `1951d3878`, `ece49f915`, `65c169835`, `b78a1de57`
