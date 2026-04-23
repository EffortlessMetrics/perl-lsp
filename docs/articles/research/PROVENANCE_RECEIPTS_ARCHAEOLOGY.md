# Provenance Receipts Archaeology
## How Proof Became A First-Class Artifact In The Repo

This note traces a specific repository habit: when the project learned to trust machines for generation, it also learned to demand machine-checkable proof.

The result is a chain of structured surfaces rather than a pile of ad hoc claims:

- `AGENTIC_DEV.md` defines the AI-native operating model
- `CURRENT_STATUS.md` turns the live repo state into an evidence document
- `METRICS_PROVENANCE.md` requires every metric to carry provenance
- `FORENSICS_SCHEMA.md` turns PR archaeology into a dossier format
- xtask and `just` commands generate receipts instead of letting claims float in prose

That is the core pattern: proof is not an afterthought. It is a product surface.

---

## 1. The Repo Defines Trust As Mechanical, Not Social

`docs/project/AGENTIC_DEV.md` makes the baseline distinction explicit:

- AI-assisted work is human-limited and trust-based
- AI-native work is machine-limited and receipt-based

That is a stronger claim than "we use agents." It says the repo changed the unit of trust.

The same document then names the operating consequences:

- `nix develop -c just ci-gate` is the canonical local gate
- `just status-check` is the anti-drift check for docs and computed values
- receipts, not vibes, prove claims
- wrongness should be logged and prevented, not hidden

The important archaeology point is that this is not just policy text. It is the conceptual root of the repo's later proof surfaces.

The repo is deciding that trustworthy change must be mechanically inspectable.

---

## 2. `CURRENT_STATUS.md` Is The Evidence Document

`docs/project/CURRENT_STATUS.md` is not a dashboard in the casual sense. It is a truth contract.

The file says:

- claims must be backed by `Cargo.toml`, `ci-gate`, `ignored-test-count.sh`, `features.toml`, or capability snapshots
- generated sections are machine-updated by `just status-update`
- `just status-check` exists to catch drift
- the file is the evidence document, while `ROADMAP.md` is the planning document

That division matters because it prevents a common failure mode in AI-heavy repos: narrative docs drifting away from the real system.

`CURRENT_STATUS.md` solves that by separating:

- narrative claims
- computed metrics
- generation discipline

The archaeology signal is clear: the repo started turning status into a reproducible artifact, not a manually curated summary.

---

## 3. Provenance Becomes A Schema, Not A Convention

`docs/project/METRICS_PROVENANCE.md` pushes the discipline one level deeper.

It says every metric must declare:

- `value`
- `kind`
- `basis`
- `coverage`
- `confidence`
- `method` when derived
- `assumptions` when estimated

That is important because it distinguishes measurement from interpretation.

For this repo, a metric without provenance is not merely incomplete. It is malformed.

The schema also classifies the source of the claim:

- `receipts_included`
- `github_plus_agent_logs`
- `github_only`
- `self_attested`

That gives readers a direct signal about how much trust to place in the number. The repo is not pretending all evidence is equal.

The archaeological significance is that provenance itself became a structured artifact. The repo stopped treating proof as free-form justification and started treating it as data.

---

## 4. Forensics Turns Claims Into Dossiers

`docs/reference/FORENSICS_SCHEMA.md` makes the strongest move of all: it turns PR archaeology into a repeatable dossier format.

Its core principle is blunt:

> The product isn't code. It's decisions + proof.

The schema then asks the same questions every time:

- what changed
- what was actually verified
- whether truth surfaces stayed honest
- how the change converged over time
- what prevention actions should follow

That structure matters because it forces the repo to publish not just outcomes, but the reasoning chain behind them.

The four measured panels are especially telling:

- change surface
- verification depth
- governance integrity
- temporal topology

Then come budget estimates, quality deltas, factory delta, exhibit score, and next prevention actions.

This is a maturity jump. The repo is no longer content with "PR merged" or "test passed." It wants:

- evidence
- provenance
- drift checks
- prevention

That is what makes the archive useful for launch articles. It gives a stable template for saying why a change mattered, not just that it landed.

---

## 5. Receipts Are Generated, Not Merely Attested

The xtask migration docs show how the repo operationalizes the whole model.

`docs/project/XTASK_MIGRATION.md` describes receipt-generating subcommands such as:

- `cargo xtask gates`
- `cargo xtask features verify`
- `cargo xtask release`
- `cargo xtask publish-crates`
- `cargo xtask doc`

It also says the migration is replacing shell scripts that used to scatter proof generation across many ad hoc entrypoints.

That matters because receipts are only as trustworthy as the process that emits them.

The repository is moving proof generation into Rust-native, testable, workspace-aware commands instead of leaving it in shell glue. That makes the receipt itself more reproducible and easier to validate.

The same pattern appears in the CI docs:

- `just ci-gate` is the merge receipt
- `just ci-full` is the deeper confidence receipt
- `just status-update` regenerates computed metrics
- `just status-check` verifies they are current

This is a pipeline for proof, not just a pipeline for code.

---

## 6. Wrongness Gets A Record Too

`docs/project/LESSONS.md` makes the repo's proof culture even more explicit.

Each entry follows:

- wrong
- evidence
- fix
- prevention

That is the same structure as the forensics schema, but for mistakes.

The lesson log matters archaeologically because it shows the repo treating failure as an input to future proof. When a claim drifted or a measurement was stale, the fix was not only to correct the text. The repo also changed the guardrail.

That is the same trust model as the rest of the system:

- wrongness should be made visible
- the evidence should be retained
- prevention should be mechanized

This is a practical definition of industrialized trust.

---

## 7. The System Is Self-Describing

Taken together, these surfaces show a project that learned to describe itself in operational terms:

- `AGENTIC_DEV.md` defines the trust model
- `CURRENT_STATUS.md` publishes the evidence-backed state
- `METRICS_PROVENANCE.md` tags claims with provenance
- `FORENSICS_SCHEMA.md` standardizes PR archaeology
- `LESSONS.md` records wrongness and prevention
- xtask generates receipts and validation artifacts

That is the important historical shift.

Earlier development could rely on memory, chat, or narrative summaries. This repo increasingly relies on versioned proof artifacts that can be checked, recomputed, and audited by future agents.

In other words, the repository did not just become AI-native. It became evidence-native.

---

## Evidence Pointers

- [AGENTIC_DEV.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/AGENTIC_DEV.md)
- [CURRENT_STATUS.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/CURRENT_STATUS.md)
- [METRICS_PROVENANCE.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/METRICS_PROVENANCE.md)
- [FORENSICS_SCHEMA.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/reference/FORENSICS_SCHEMA.md)
- [XTASK_MIGRATION.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/XTASK_MIGRATION.md)
- [LESSONS.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/LESSONS.md)
