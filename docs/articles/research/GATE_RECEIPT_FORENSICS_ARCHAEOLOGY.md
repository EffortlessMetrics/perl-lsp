# Gate Receipt Forensics Archaeology
## How Issue #210 Made Proof Governance Executable And Inspectable

Issue `#210` is the point where the repo stops treating merge proof as a
nice-to-have artifact and starts treating it as governance. The historical
record is not a single commit. It is a chain that moves from planning language,
to a shell receipt emitter, to a typed Rust gate harness, to CI status
plumbing, and finally into the forensics prompt pack that audits those same
proof surfaces.

---

## 1. The Issue Was Framed As A Trust-Surface Problem Before It Became Code

[docs/forensics/IMPLEMENTATION_PHASES.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/forensics/IMPLEMENTATION_PHASES.md)
puts `#210` in Phase A under trust-surface stabilization and defines the goal
as one authoritative merge-gate posture with predictable receipts. The same
file later groups `#210` with `#211` as gate consolidation work.

That matters because the repo is treating proof governance as release-order
infrastructure, not as a documentation cleanup. The later ops commit
`c55d292d5` on `2026-01-08` reinforces the same sequencing by adding a
milestone-verification recipe and blockers section that orders `#211 -> #210 ->
#143`.

---

## 2. The First Implementation Wave Turned The Issue Into A Real Harness

The main execution step lands in `21ec9bd54` on `2026-01-25`:
`feat: implement standardized CI gate harness (#533)`.

The current code still shows the main surfaces from that move:

- [xtask/src/tasks/gates.rs](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/xtask/src/tasks/gates.rs)
  reads `.ci/gate-policy.yaml`, executes gates, captures timing and status, and
  emits structured receipts.
- [.ci/gate-policy.yaml](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.ci/gate-policy.yaml)
  declares itself the single source of truth for gate configuration.
- [.ci/receipt.schema.json](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.ci/receipt.schema.json)
  defines the machine-readable contract for the receipt surface.
- [justfile](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/justfile)
  routes local gate execution through `cargo xtask gates`.

There is also a historical bridge instead of a clean rewrite.
[scripts/run-gates.sh](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/scripts/run-gates.sh)
is the older shell emitter that writes `target/receipts/receipt.json` and
records gate name, command, status, exit code, and duration. The arc is not
"shell then nothing." It is "shell proof, then typed proof, then policy-aware
proof."

---

## 3. CI Made The Receipt Inspectable, Not Just Present

[.github/workflows/ci.yml](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.github/workflows/ci.yml)
runs `just gates`, uploads the receipt and logs as artifacts, prints a failure
tail, renders a step summary, and publishes a `ci/merge-gate` commit status.
That is the inspectability layer: the gate is no longer only pass/fail. The
receipt becomes a visible workflow artifact and status surface.

The history then shows the repo debugging the receipt reader itself:

- `1951d3878` on `2026-02-20` aligns CI receipt parsing with schema fields such
  as `gate_name`, `duration_ms`, and `skip`.
- `ece49f915` on `2026-02-28` publishes the merge-gate commit status.
- `b78a1de57` makes required timeout and error states fail closed instead of
  passing ambiguously.

That is the same pattern as the later validator notes: the repo keeps checking
whether the measuring instrument itself is trustworthy.

---

## 4. Status Drift Became A Second Governance Loop

`#210` is about gates, but the same proof-governance instinct later reaches
project-status drift.

[xtask/src/tasks/update_status.rs](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/xtask/src/tasks/update_status.rs)
ports the old Python updater into Rust and makes `--check` fail when
[docs/project/CURRENT_STATUS.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/CURRENT_STATUS.md)
or [docs/project/ROADMAP.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/ROADMAP.md)
drift from computed truth. The relevant transition lands in `65c169835` on
`2026-03-17`:
`feat(xtask): add update-status subcommand replacing Python script (#1526)`.

That is the same governance move as `#210`, but on documentation surfaces:

- claims are not accepted because they are written down
- claims are accepted because the repo can re-derive and check them

---

## 5. The Forensics Prompt Pack Generalized The Same Rule

The later forensics surfaces do not replace the gate/receipt model. They audit
it.

[docs/forensics/README.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/forensics/README.md)
identifies `measurement-auditor.md` as the measurement-integrity analyzer and
`policy-auditor.md` as the governance analyzer.
[docs/forensics/prompts/measurement-auditor.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/forensics/prompts/measurement-auditor.md)
demands commands, receipts, and git context, and it treats `not_comparable` as
a hard stop when the measurement contract is unstable.
[docs/forensics/prompts/policy-auditor.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/forensics/prompts/policy-auditor.md)
audits governance drift across `features.toml`, `CURRENT_STATUS.md`, receipt
surfaces, and `just status-check`.

That is the long tail of `#210`: the repo ends up not merely asking for
evidence, but requiring evidence that is executable, inspectable, and itself
auditable.

---

## 6. Historical Meaning

The lineage is:

1. PR `#209` exposes the danger of proof that is technically true but
   operationally weak.
2. Issue `#210` converts that lesson into a merge-gate governance request.
3. `21ec9bd54` makes the request executable through policy, schema, and a
   structured gate runner.
4. `1951d3878`, `b78a1de57`, and `ece49f915` make CI consume and publish those
   receipts correctly.
5. `65c169835` extends the same rule to status-drift checks.
6. The forensics prompt pack turns the whole pattern into audit surfaces.

That is the durable shift: proof governance becomes code, then CI behavior,
then audit tooling.

---

## Evidence Pointers

- [docs/forensics/IMPLEMENTATION_PHASES.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/forensics/IMPLEMENTATION_PHASES.md)
- [xtask/src/tasks/gates.rs](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/xtask/src/tasks/gates.rs)
- [.ci/gate-policy.yaml](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.ci/gate-policy.yaml)
- [.ci/receipt.schema.json](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.ci/receipt.schema.json)
- [scripts/run-gates.sh](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/scripts/run-gates.sh)
- [.github/workflows/ci.yml](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.github/workflows/ci.yml)
- [xtask/src/tasks/update_status.rs](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/xtask/src/tasks/update_status.rs)
- [docs/forensics/prompts/measurement-auditor.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/forensics/prompts/measurement-auditor.md)
- [docs/forensics/prompts/policy-auditor.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/forensics/prompts/policy-auditor.md)
- `c55d292d5`, `21ec9bd54`, `b78a1de57`, `1951d3878`, `ece49f915`, `65c169835`
