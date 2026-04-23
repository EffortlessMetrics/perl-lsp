# Receipt Surface Evolution Archaeology
## From PR-Body Proof Bundles To Typed Gate Contracts

This note tracks a specific change in the repo's proof surface: the project
started with PRs that carried human-readable receipt bundles, then added PR
template guidance, and eventually hardened the same idea into a machine-readable
gate contract, CI status plumbing, and forensics tooling.

The important distinction is between:

- receipt format: the PR body, checklist, comments, and attached artifacts
- receipt contract: the schema, gate runner, status checks, and audit surfaces

That distinction matters because the repo did not merely "add more receipts."
It moved from descriptive proof to enforceable proof.

All counts and PR examples below were verified from the GitHub archive and repo
files on `2026-03-19`.

---

## 1. PR Bodies Started As Human-Facing Receipt Bundles

PR `#209` is the clearest early example of a receipt-heavy PR body. Its review
surface carries:

- `6` reviews
- `29` comments
- labels such as `review:stage:intake`, `merge-ready`,
  `gate:docs (clean)`, `gate:perf (ok)`, `gate:tests (pass)`,
  `gate:security (clean)`, `gate:policy (clear)`, `state:in-progress`,
  `state:ready`, `ready-to-merge`, and `flow:integrative`

The body itself is a bundle of proof claims: test output, performance claims,
security claims, documentation claims, and a checklist-style readiness
statement. That is still a PR-body format, not a schema contract.

The historical point is that the PR body was already doing real governance
work. It was not just commentary. It was the visible place where the author
asserted what was verified.

PR `#533` shows the same lineage later in the year, but with a more explicit
verification posture:

- `2` reviews
- `3` comments
- the body includes a verification receipt section and a base-comparison note

That PR is useful because it still looks like a human-facing bundle, but it is
already talking in the language of receipts, verification receipts, and
reproducibility.

---

## 2. PR Templates Turned The Format Into A Habit

PR `#274`, `ci: add PR template with local gate requirement and label guidance`,
is the bridge between informal proof bundles and a more disciplined workflow.
Its body says the template should require local `nix develop -c just ci-gate`
receipts before merge, and it documents optional CI labels for expensive
validation.

That is not yet a machine contract. It is a behavioral scaffold.

The template does three things:

- it normalizes local proof before push
- it tells contributors which expensive gates exist
- it makes the cost of validation visible early

That is important historically because the repository is no longer relying on
each PR author to invent the receipt shape from scratch. The format is becoming
standardized, but still by convention.

---

## 3. Issue #210 Converts Receipt Thinking Into Governance

Issue `#210`, `Formalize Merge-Blocking Gates, Receipts, and Check-Run Lifecycle
for perl-lsp`, is the explicit policy request that turns the pattern into a
contract.

The issue asks for:

- one policy file for required gates and thresholds
- a deterministic LSP scenario harness
- a machine-readable `receipt.json` per run
- CI that blocks merges when thresholds fail
- a check-run lifecycle that reports gate state
- local commands to reproduce the same result before pushing

That is the key historical seam. The repo is no longer just asking for a PR
body that explains what happened. It is asking for a system that can re-run the
work, prove it, and publish the proof mechanically.

This is where receipt format becomes receipt contract.

---

## 4. The Contract Lives In Schema, Xtask, And Gate Policy

The machine-enforced surface is visible in three repo files.

[`.ci/receipt.schema.json`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.ci/receipt.schema.json)
defines the shape of the receipt. It requires:

- `schema_version`
- `metadata`
- `gates`
- `summary`

It also forces the receipt to carry execution context such as commit SHA,
branch, toolchain, platform, environment, and per-gate results.

[`xtask/src/tasks/gates.rs`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/xtask/src/tasks/gates.rs)
is the typed runner. It reads `.ci/gate-policy.yaml`, executes gate tiers,
captures timing and status, and serializes the receipt structure from code
rather than from ad hoc shell output.

[`scripts/run-gates.sh`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/scripts/run-gates.sh)
is the older bridge surface. It still emits a JSON receipt, but in shell form
and with a much thinner contract. The existence of both surfaces shows the
transition clearly: shell proof first, typed proof later.

The policy file and runner together make the receipt a contract, not just a
report.

---

## 5. CI And Forensics Make The Contract Public

The contract is not hidden in local tooling.

[`docs/forensics/prompts/measurement-auditor.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/forensics/prompts/measurement-auditor.md)
formalizes the rule that a metric without commands, receipts, and git context
is not trustworthy. It is explicitly about auditability and reproducibility.

[`docs/project/AGENTIC_DEV.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/AGENTIC_DEV.md)
turns that into the project philosophy: AI-native work is receipt-based, not
trust-based.

That means the repo has three layers now:

1. PR body receipts that explain what was run
2. typed gate receipts that enforce what was run
3. audit prompts that check whether the receipts themselves are honest

The surface keeps getting more explicit because each layer exposed a weakness in
the one before it.

---

## 6. Historical Meaning

The lineage is not just "more paperwork."

1. PR `#209` shows the original receipt bundle in a PR body.
2. PR `#274` standardizes the habit with local gate guidance and labels.
3. Issue `#210` turns the habit into a governance request.
4. `.ci/receipt.schema.json` and `xtask/src/tasks/gates.rs` make the receipt
   machine-checkable.
5. `scripts/run-gates.sh` preserves the older bridge, which makes the
   transition legible instead of pretending it was instantaneous.
6. The forensics prompts then audit the whole chain.

That is the real evolution: from narrative proof, to standardized proof, to
enforced proof, to audited proof.

---

## Evidence Pointers

- [PR_REVIEW_RECEIPT_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/PR_REVIEW_RECEIPT_ARCHAEOLOGY.md)
- [GATE_RECEIPT_FORENSICS_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/GATE_RECEIPT_FORENSICS_ARCHAEOLOGY.md)
- [RECEIPTS_LIE_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/RECEIPTS_LIE_ARCHAEOLOGY.md)
- [VALIDATOR_BLIND_SPOT_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/VALIDATOR_BLIND_SPOT_ARCHAEOLOGY.md)
- [PROVENANCE_RECEIPTS_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/PROVENANCE_RECEIPTS_ARCHAEOLOGY.md)
- [PR #209](https://github.com/EffortlessMetrics/perl-lsp/pull/209)
- [PR #274](https://github.com/EffortlessMetrics/perl-lsp/pull/274)
- [PR #533](https://github.com/EffortlessMetrics/perl-lsp/pull/533)
- [Issue #210](https://github.com/EffortlessMetrics/perl-lsp/issues/210)
- [receipt.schema.json](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.ci/receipt.schema.json)
- [gates.rs](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/xtask/src/tasks/gates.rs)
- [run-gates.sh](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/scripts/run-gates.sh)
