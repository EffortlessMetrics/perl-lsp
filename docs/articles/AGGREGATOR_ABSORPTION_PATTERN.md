# The Aggregator Absorption Pattern: Collapsing a ~1,600-LOC Crate Into a Module

**Date**: 2026-04-19
**Session**: Wave G1b collapse on perl-lsp (PR #4510)
**Cross-references**: [SRP_MICROCRATES.md](../SRP_MICROCRATES.md), memory: `project_wave_d_facade_pattern.md`

---

## TL;DR

During the 2026-04-19 Wave G1b collapse on perl-lsp, one of the ten crates absorbed was not a leaf — it was an *aggregator*. `perl-lsp-providers` existed to re-export and wire together nine other provider crates, and itself contained ~1,600 lines of original code in `ide/lsp_compat/` (signature_help, linked_editing, selection_range) that was used by both internal and external consumers.

Absorbing an aggregator into a module is harder than absorbing a leaf crate. Leaves have no dependent crates by assumption. Aggregators have the most dependent crates — often every other crate in the family, plus external consumers. This article names the aggregator absorption pattern that made the G1b absorption clean and documents the two decisions that made the difference: the `ide::lsp_compat` target module (not `registry`) and the deprecated alias preservation.

---

## The Two Subtly-Different Absorption Cases

**Leaf absorption.** An absorbed crate is a leaf — no other crate in the workspace depends on it. Collapse is mechanical: move source, update re-export in `mod.rs`, delete directory, remove from Cargo.toml workspace members, done. This was most of Wave G1a (15 crates): 12 leaves and 3 that depended only on other G1a leaves.

**Aggregator absorption.** An absorbed crate is depended on by many other crates — it's a hub. Collapse has to preserve:
1. The aggregator's own code (re-exports + any original implementations).
2. External consumers' import paths.
3. Public API surface so external Cargo consumers don't break.

Wave G1b had exactly one aggregator: `perl-lsp-providers`. Its absorption needed all three.

## What Was Inside `perl-lsp-providers`

Initial read: a pure aggregator that re-exported the nine other G1b provider crates. The spec-planner's first draft spec proposed collapsing it to `perl_lsp_rs_core::providers::registry` — essentially "the thing that registers all providers with the LSP server."

Plan-reviewer reread the crate directly and found this was wrong. `crates/perl-lsp-providers/src/ide/lsp_compat/` contained ~1,600 LOC of original implementations:

- `signature_help.rs` (~548 LOC) — LSP signature-help computation against AST
- `linked_editing.rs` (~407 LOC) — LSP linked-editing ranges
- `selection_range.rs` (~232 LOC) — LSP selection-range expansion

Plus ~400 LOC of other original code across smaller files. The re-export surface was real but so was the internal implementation. Collapsing to `providers::registry` would have left the 1,600 LOC homeless.

**Correction:** target module is `providers::lsp_compat`, not `providers::registry`. The name reflects what the code actually does (LSP-compat-layer implementations) rather than what the original crate name suggested (a grouping/aggregation).

This is the first lesson: **the absorbed crate's name is a label for humans, not a structural fact**. The target module should reflect what the code does at the module's granularity, which is often not what the source crate's name suggests. Plan-reviewer's direct-read saved a 1,600-LOC misplacement.

## The Deprecated Alias

External consumers of `perl-lsp-providers` (before the collapse) imported items via paths like `perl_lsp_providers::tooling_export::*`. After the collapse, the paths change to `perl_lsp_rs_core::providers::lsp_compat::*`. External Cargo consumers — anyone who had `perl-lsp-providers = "0.12.x"` in their `Cargo.toml` — would have their imports break.

The options:

1. **Hard break.** Publish a new major version. Consumers upgrade at their own pace. Clean but disruptive.
2. **Keep the old crate as a thin re-export facade.** Publish `perl-lsp-providers 0.13.0` that re-exports from `perl-lsp-rs-core`. Small ongoing maintenance.
3. **Deprecated alias.** Preserve the old path as a deprecated alias for six months, then remove in a future version. Consumers get a migration window.

Option 3 was chosen. `perl_lsp_providers::tooling_export` remains reachable via an alias marked `#[deprecated(since = "0.12.4", note = "use perl_lsp_rs_core::providers::lsp_compat instead")]`. External consumers see a warning at compile time but their code still works. Six months from now (0.14.x), the alias will be removed.

This is the second lesson: **aggregator absorption's external cost is largely the migration window**. A deprecated alias costs little (6 months of maintaining a thin shim) and saves external consumers from scrambling. It also creates a documented migration path — the `#[deprecated]` attribute tells compilers and IDEs exactly where to go.

## The Concrete Migration

The `providers::lsp_compat` module was organized as a submodule hierarchy mirroring the original crate's structure:

```rust
// crates/perl-lsp-rs-core/src/providers/lsp_compat/mod.rs
pub mod ide;

// crates/perl-lsp-rs-core/src/providers/lsp_compat/ide/mod.rs
pub mod signature_help;  // 548 LOC original
pub mod linked_editing;  // 407 LOC original
pub mod selection_range; // 232 LOC original
// ... and others

// crates/perl-lsp-rs-core/src/providers/mod.rs
pub mod lsp_compat;
// Plus 9 other G1b submodules and 15 from G1a
```

The deprecated alias lives as a module-level attribute at `crates/perl-lsp-rs-core/src/providers/mod.rs`:

```rust
/// DEPRECATED: use `providers::lsp_compat` instead.
/// Will be removed in 0.14.x.
#[deprecated(since = "0.12.4", note = "use perl_lsp_rs_core::providers::lsp_compat instead")]
pub use lsp_compat as tooling_export;
```

External consumers' import paths that referenced the old `perl_lsp_providers::tooling_export::signature_help::*` now resolve through the alias with a deprecation warning. The compiler points them at the new path. No API break.

## What The Pattern Saved

Absent this approach:
- If we'd collapsed to `providers::registry` (the first draft), ~1,600 LOC would have landed in the wrong semantic location. Every future contributor reading `providers::registry` would have been confused — it's not a registry, it's an LSP-compat-layer implementation.
- If we'd skipped the deprecated alias, external consumers (including downstream users of perl-lsp as a Cargo crate) would have broken on v0.13.0 without a transition window.

Both mistakes are recoverable, but the cost of recovery compounds. Silently bad module naming accumulates as technical debt; API-breaking changes force consumers to choose between "upgrade right now" and "fork." Neither is what we want for a v0.13.0 public alpha.

## Generalizable Rules

From this absorption:

1. **When absorbing a crate, read its source to determine what's actually inside**. Don't trust the crate name, don't trust the issue body's description, don't trust the spec. Read `src/lib.rs` and enumerate the public items. The target module name should reflect what the code does.

2. **For aggregators (crates with many external consumers), always preserve a deprecated alias across at least one minor version boundary**. This is cheap insurance against downstream pain.

3. **Sequence the absorption: leaves first, consumers second, aggregator LAST**. Wave G1b's checklist had four phases: pure leaves (rename, diagnostics, inline_completion, semantic_tokens), near-leaves (formatting, ai), consumers (completion, navigation, code_actions), and the aggregator (providers). Running in this order means at each step, every crate the in-progress crate depends on has already been absorbed. No mid-PR compile breaks.

4. **Publish crate names are user-facing; internal module names are contributor-facing**. The collapsed module name doesn't have to match the absorbed crate name. In fact it often shouldn't — the module name should describe the code's role in the post-collapse architecture, not its historical origin.

## Related

- [SRP_MICROCRATES.md](../SRP_MICROCRATES.md) — single-responsibility microcrate architecture (the pre-collapse state)
- Facade/core split pattern — see Wave D (#4486) and Wave F (#4493) merge history for the established pattern this collapse fits into
- [forensics/2026-04-19-wave-g1-collapse-retrospective.md](../forensics/2026-04-19-wave-g1-collapse-retrospective.md) — full session, Wave G1b details
- Pull request #4510 — the actual aggregator absorption diff
