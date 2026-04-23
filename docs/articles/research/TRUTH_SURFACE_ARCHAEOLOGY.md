# Truth Surface Archaeology
## How The Repo Externalized Anti-Drift Into Catalogs, Receipts, And Checks

This repository does not rely on authors remembering the truth.

Over time it externalized truth into:

- canonical catalogs
- generated evidence surfaces
- typed receipts
- fail-closed checks
- lessons that record how drift happened before

That is why the repo's documentation discipline feels unusually mechanical for
an AI-native codebase. Truth maintenance became an engineering subsystem.

---

## 1. Why The System Exists

The repo names the original failure modes directly.

[`docs/project/LESSONS.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/LESSONS.md)
records:

- overstated LSP coverage
- performance claims without published receipts
- superlatives with no evidence
- issue numbers being mistaken for PR numbers

The pattern is explicit: wrong claim, evidence, fix, prevention.

That matters because the anti-drift system is not theoretical. It is scar-story
driven.

---

## 2. The Repo Separates Truth From Planning On Purpose

[`docs/project/CURRENT_STATUS.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/CURRENT_STATUS.md)
defines itself as the evidence document and gives a formal truth contract:

- `Cargo.toml` for release-line truth
- `just ci-gate` output for merge-gate truth
- ignored-test receipts for debt truth
- `features.toml`, capability snapshots, or targeted tests for capability truth

The same file is explicit that:

- generated fenced regions are machine-updated
- `ROADMAP.md` is the planning document
- `status-update` and `status-check` are the anti-drift workflow

That distinction is not editorial style. It is a structural split between
evidence surfaces and planning surfaces.

---

## 3. The Earliest Strong Truth Substrate Is The Feature Catalog

The anti-drift story starts before `status-check`.

[`docs/project/FEATURE_GOVERNANCE.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/FEATURE_GOVERNANCE.md)
explains why a hand-maintained feature inventory would drift by design:

- runtime behavior
- advertised capabilities
- tests
- docs
- CI

would all diverge unless they were anchored to one catalog.

So the repo made `features.toml` the single source of truth and pushed it
through thin governance crates:

- `perl-feature-catalog`
- `perl-lsp-feature-contracts`
- `perl-lsp-feature-ids`
- `perl-lsp-capability-map`
- `perl-lsp-feature-policy`
- related facade and reporting crates

This is anti-drift as architecture, not just as documentation hygiene.

Local git history makes that timing visible. The deeper source-of-truth
substrate appears in late August 2025 with feature-catalog and compliance work,
well before the repo uses the later `status-check` language.

---

## 4. October 2025 Makes Documentation Truth Explicit

[`docs/project/DOCUMENTATION_TRUTH_SYSTEM.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/DOCUMENTATION_TRUTH_SYSTEM.md)
documents the first clearly named docs-truth system.

Commit history shows:

- `206c3fdde` on `2025-10-22` introduces the self-healing documentation system
- `e26d45d79` on `2026-03-07` later moves that document into the Diataxis tree

Its model is already recognizably modern:

- generate canonical receipts
- render docs from receipts
- guard with CI

The tooling later changed, but the model stayed.

---

## 5. January 2026 Is The Explicit Anti-Drift Turn

The clearest turn happens on `2026-01-07`.

Commit `25f0b29a5` adds the lessons ledger and broader forensics material.
That same period is when the repo starts naming:

- claim drift
- evidence-backed metrics
- no performance claims without receipts
- no numeric claims outside computed sources

This is the point where the repo stops merely generating truth artifacts and
starts treating drift as a recurring systems problem with named prevention.

The language in
[`docs/project/CLAUDE_MD_EVOLUTION.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/CLAUDE_MD_EVOLUTION.md)
matches that reading closely:

- metrics are computed, not hand-edited
- no volatile metrics in promotional or instruction surfaces
- operational docs should link to computed truth rather than restating it

---

## 6. Truth Maintenance Becomes Executable

By early 2026 the repo is no longer depending on policy prose alone.

The `justfile` wires truth maintenance directly into local workflows:

- `status-update`
- `status-check`
- `ci-policy`
- receipt-oriented gate execution

The enforcement layer is executable:

- [`xtask/src/tasks/update_status.rs`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/xtask/src/tasks/update_status.rs)
  regenerates status sections and roadmap metrics
- [`xtask/src/tasks/features.rs`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/xtask/src/tasks/features.rs)
  checks catalog invariants and compliance drift
- [`xtask/src/tasks/gates.rs`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/xtask/src/tasks/gates.rs)
  emits, diffs, and blocks on gate receipts
- [`xtask/src/tasks/receipts.rs`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/xtask/src/tasks/receipts.rs)
  replaces the earlier script-based receipt generation

The repo is progressively moving from "remember the rules" to "run the rule
engine."

---

## 7. The Receipt Layer Is Typed And Fail-Closed

The receipt system is not just a pile of JSON files.

[`.ci/receipt.schema.json`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.ci/receipt.schema.json)
defines the structure of gate evidence. `xtask gates` uses that structure for:

- execution metadata
- individual gate results
- summary status
- diffing current receipts against baselines
- blocking on failures

That means the repo's proof surfaces are not only narratively explained. They
are formalized and machine-consumable.

This is one reason the project can keep tightening trust without relying on a
human to manually inspect every claim surface.

---

## 8. March 2026 Consolidates The Current Form

The March 2026 xtask wave is the clearest operating-system moment for the
current truth-maintenance stack.

By that point:

- script-era helpers are being ported into `xtask`
- local status and receipts are repeatable through one tool surface
- computed sections are normal
- truth checks are part of ordinary local gating

The architecture is not fully new in March. But March is when the repo's
anti-drift system becomes a more native, cohesive runtime surface.

---

## 9. Why This Matters Historically

The distinctive thing here is not that the repo has documentation discipline.
Many repos have style guidance.

The distinctive thing is that this repo progressively externalized truth into a
cooperating stack:

- source catalogs
- evidence docs
- generated sections
- receipt schemas
- gate runners
- lessons ledgers
- provenance schemas

That is closer to an operating system for truthful change than a normal docs
folder.

It also explains why so many later launch-article and historical notes keep
returning to the same theme: trusted change is not just about tests passing. It
is about whether the repo has made drift mechanically difficult.

---

## Evidence Pointers

- [CURRENT_STATUS.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/CURRENT_STATUS.md)
- [LESSONS.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/LESSONS.md)
- [FEATURE_GOVERNANCE.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/FEATURE_GOVERNANCE.md)
- [DOCUMENTATION_TRUTH_SYSTEM.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/DOCUMENTATION_TRUTH_SYSTEM.md)
- [CLAUDE_MD_EVOLUTION.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/CLAUDE_MD_EVOLUTION.md)
- [METRICS_PROVENANCE.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/METRICS_PROVENANCE.md)
- [justfile](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/justfile)
- [xtask/src/tasks/update_status.rs](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/xtask/src/tasks/update_status.rs)
- [xtask/src/tasks/features.rs](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/xtask/src/tasks/features.rs)
- [xtask/src/tasks/gates.rs](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/xtask/src/tasks/gates.rs)
- [xtask/src/tasks/receipts.rs](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/xtask/src/tasks/receipts.rs)
- [receipt.schema.json](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.ci/receipt.schema.json)
- commits `206c3fdde`, `25f0b29a5`, `a1981069b`
