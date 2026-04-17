# ADR-0041: Microcrate Collapse — From 132 Published Crates to ~30

**Status**: Accepted
**Date**: 2026-04-14
**Decision Makers**: Perl LSP Architecture Team
**Related**: [SRP_MICROCRATES.md](../SRP_MICROCRATES.md), [PUBLISHING.md](../PUBLISHING.md), [Tracking issue #4410](https://github.com/EffortlessMetrics/perl-lsp/issues/4410)

## Context

The workspace currently has 135 members and a publish allowlist of 132 crates in `[workspace.metadata.publish].allow`. This is the end state of a deliberate microcrate decomposition campaign described in [`docs/SRP_MICROCRATES.md`](../SRP_MICROCRATES.md): single-responsibility crates, isolated tests, agent-friendly boundaries, and a topologically-ordered publish pipeline (`scripts/publish-topo.py`) with dev-dependency cycle handling via Tarjan SCC.

The original microcrate bet had four explicit goals:

1. **Decoupled versions** — each microcrate ships on its own cadence; consumers pin narrowly.
2. **Smaller per-update publish surface** — only the changed crates need releases, not the workspace.
3. **Faster compile times** — small crates rebuild fast; cargo's incremental + parallel pipeline benefits.
4. **Agent-sized work units** — clear SRP boundaries enable parallel automated work without merge conflicts.

After ~18 months of operating the microcrate workspace, only goal #4 (agent work) delivered as expected. The other three did not, for reasons that turned out to be inherent to crates.io's design.

### What didn't materialize

- **Decoupled versions never happened.** The 132 crates are too internally coupled; in practice they move together. A change in one crate ripples up the dep graph and forces re-publishes of intermediates to bump dep versions. Per-release publish surface ENLARGED, not shrank.
- **Per-update publish surface enlarged.** Same ripple effect. A leaf change in `perl-token` cascades through ~40 crates that re-publish to bump their dep version. The "only the changed crates need releases" outcome required true API independence that never materialized.
- **Compile times didn't improve.** cargo's parallelism is real, but per-crate link/codegen overhead dominates for a workspace this size. 132 crates means 132 separate codegen units even when most could share. Worse, the publish pipeline itself became a serial bottleneck (topo + rate limits + dev-dep SCC + partial-publish handling).
- **Public surface bloat is real.** 132 published crates is 132 permanent semver contracts, 132 docs.rs pages, 132 search results, 132 things downstream users have to reason about. The external story is really 4 products + a handful of reusable kernels — the rest is implementation plumbing that became permanent public artifacts.

### The crates.io constraint that drove all four failure modes

Cargo enforces that **path-only dependencies are forbidden in published crates**. The supported pattern is `path + version`: locally Cargo uses the path, when published it uses the registry version. crates.io itself does not allow published packages to depend on code outside crates.io, with the sole exception of dev-dependencies.

The consequence: any internal microcrate in the runtime/build graph of a published crate **must itself be published**. There is no "internal-only crate as architectural boundary" option for runtime code on the published path. Every architectural seam expressed as a crate boundary became a permanent public artifact and a permanent semver contract.

The only escape hatches are:

- `publish = false` for crates outside the runtime/build graph (tooling, test harnesses, internal apps)
- Module boundaries inside published crates (with `pub(crate)` + facade discipline)
- Published support crates kept deliberately small and stable

We had used the first escape only for tooling and test crates, and the second escape barely at all. The result was that nearly every architectural seam in the system became a published crate.

## Decision

**We will collapse the published surface from 132 crates to ~30, converting ~100 product-internal microcrates to subfolder modules inside their owning published crates.** The collapse runs immediately, before the next release ships; the next release will be the first to expose the new 30-crate surface as a clean break (no bridge crates).

This means:

- **~30 published crates** organized as: 4 products + 2 tree-sitter + 1 alternate parser + 5 foundation primitives + 1 diagnostic catalog (NEW) + 2 wire protocols + 3 semantic kernels + 1 symbol model + 1 tool integration + 4 test/corpus ecosystem + 6 standalone tooling kernels.
- **4 internal-only crates** (`publish = false`): xtask, perl-ci-hygiene, perl-lsp-ux-tests, perl-parser-bench.
- **~100 retired microcrates** become folder modules: each former crate gets a directory (`mod.rs` facade, `pub(crate)` default, optional `CLAUDE.md`, preserved tests).
- **Layer guards via xtask** (`cargo xtask layer-check`) replace the implicit guard that crate boundaries provided. A `xtask/layer-rules.toml` config enforces dependency direction at the import level inside crates.
- **Publish-closure guard via xtask** (`cargo xtask publish-closure`) prevents `publish = false` crates from re-entering the runtime graph.
- **Ratchet via xtask** (`cargo xtask published-crate-count`) prevents the published count from creeping back up.

The published ~30 are reserved for **durable external problems**:

```
Products (4):              perllsp, perl-lsp-rs, perl-dap, perl-parser
Tree-sitter (2):           tree-sitter-perl-c, tree-sitter-perl-rs
Alternate parser (1):      perl-parser-pest
Foundation primitives (5): perl-lexer, perl-token, perl-line-index, perl-uri, perl-pod
Diagnostic surface (1):    perl-diagnostic-catalog (NEW — absorbs 3)
Wire protocols (2):        perl-lsp-protocol, perl-content-length-framing
Semantic kernels (3):      perl-semantic-analyzer, perl-module-resolution (absorbs 13),
                           perl-workspace-index (absorbs 7)
Symbol model (1):          perl-symbol (absorbs 4)
Tool integrations (1):     perl-lsp-perltidy
Test/corpus ecosystem (4): perl-corpus, perl-tdd-support, perl-test-must, perl-test-generators
Standalone tooling (6):    perl-feature-catalog, perl-incremental-parsing, perl-refactoring,
                           perl-dead-code, perl-heredoc-anti-patterns, perl-path-security
```

## Decision Drivers

1. **crates.io fit**: publish runs minutes not hours; topological order trivial at ~30 nodes; dev-dep SCC handling becomes dead code; rate-limit retries unnecessary; per-release version bump touches a handful of crates not the workspace.
2. **Architectural honesty**: published crates become a real product surface, not an artifact of internal SRP decomposition. Each published crate represents a durable external problem.
3. **Compile economics**: larger crates have less per-change link overhead; module-shared codegen units optimize better; incremental rebuilds get less granular but cycle faster on average for typical edit/build cycles.
4. **Discoverability**: docs.rs and crates.io search become navigable. Downstream users can build a mental model of the project from the crate names without reading internal architecture docs.
5. **Semver discipline preserved where it matters**: the ~30 published crates keep semver contracts. Internal modules are free to refactor without semver cost.
6. **Architectural separation preserved**: subfolder modules with `pub(crate)` + facade discipline + xtask layer guards give the same separation we wanted from microcrates (clear ownership, isolated tests, bounded change scope, agent-friendly boundaries) without permanent public surface.

## Considered Options

### Option 1: Status quo — 132 published crates

Continue with the existing microcrate decomposition. Accept the publish complexity and public surface as the cost of separation.

**Pros**
- No migration cost.
- Existing tooling (publish-topo, allowlist) keeps working.
- External users depending on individual microcrates are unaffected.

**Cons**
- All four failure modes documented above persist indefinitely.
- Every new feature carries pressure to add yet another microcrate.
- Public surface continues growing; semver contracts accumulate.
- Publish runs remain a serial, multi-hour, partially-failing operation.

### Option 2: Bridge-and-freeze — keep all 132 names, ship empty re-export shims

For each retired microcrate, publish one final version that re-exports from its new home, then freeze. Existing dependents keep building.

**Pros**
- Minimizes downstream breakage.
- Old crate names continue to resolve.

**Cons**
- 132 crates remain published artifacts with active version histories indefinitely.
- Bridge crates rot — they become a permanent maintenance burden with no active development.
- Doesn't solve the publish-pipeline complexity (still 132 crates to manage in topo order).
- Encourages downstream users to keep depending on internal-shaped names that should never have been public.
- The "we don't really publish those anymore" social contract is unenforceable.

### Option 3: Clean break — collapse to ~30 published crates with migration guide

Stop publishing retired microcrates. Document a migration table from old crate paths to new module paths. Ship with a major version bump that signals the breaking change.

**Pros**
- Solves all four failure modes.
- Eliminates the publish-pipeline complexity and operational debt.
- Keeps the ~30 published crates as a real product surface that documents what the project is for external users.
- Incremental compile economics improve.
- Migration cost is one-time; ongoing maintenance is permanently cheaper.

**Cons**
- Downstream users depending on individual microcrates must update imports.
- Snapshot test paths drift on absorption (`INSTA_UPDATE` review per wave).
- Loss of crate-level cargo test scoping (`cargo test -p perl-lsp-folding` becomes a module path filter).
- Loss of crate-level incremental rebuild granularity (offset by the larger-crate savings above).
- Migration window has merge-conflict pressure from many waves touching the same Cargo.toml files.

## Decision Outcome

We choose **Option 3** (clean break to ~30 published crates).

The microcrate bet was deliberate and reasonable given what we expected from cargo and crates.io. After operating it at scale, the data is clear: three of the four anticipated benefits did not materialize, and the fourth (agent work) is preserved by subfolder modules with the same discipline. The publish-pipeline pain and public-surface bloat are real costs we no longer need to pay.

The ~30-crate target reflects "all the legitimate reusable kernels and products," not "as few as possible." Each published crate solves a durable external problem. Each retired crate was internal plumbing that became publicly visible as a side effect of using crates as the modular boundary.

## Consequences

### Positive

- **Publish runs become trivial**: ~30 crates in topo order, no SCCs, no rate-limit issues at scale.
- **Public surface tells a true story**: the ~30 crates document what the project is for external consumers.
- **Per-release version bumps shrink dramatically**: most releases touch a handful of crates.
- **Compile economics improve**: less per-crate link overhead, better codegen unit sharing.
- **Architectural separation preserved**: folder discipline + xtask layer guards do the work crate boundaries did.
- **Tooling debt retires**: dev-dep SCC logic, allowlist drift checks, publish-pipeline rate-limit retries become dead code or trivial.

### Negative

- **One-time migration cost**: 14 PRs across several weeks of focused work.
- **Downstream breakage for direct microcrate dependents**: addressed via [`docs/MIGRATION_v0.13.md`](../MIGRATION_v0.13.md) and a major-version bump.
- **Snapshot test path drift**: per-wave review and regeneration step.
- **Loss of crate-level test scoping**: `cargo test -p perl-X` becomes `cargo test -p perl-parser X::` for absorbed crates.
- **Reduced crate-level incremental granularity**: a change to a former microcrate now rebuilds the whole owning crate. Mitigated by larger crates having less per-change overhead.

### Mitigations

- **Layer guards** (`cargo xtask layer-check`) enforce dependency direction inside crates, replacing the implicit guard that crate boundaries provided.
- **Publish-closure guard** (`cargo xtask publish-closure`) prevents `publish = false` crates from re-entering the runtime graph.
- **Ratchet** (`cargo xtask published-crate-count`) prevents published count creep.
- **Migration guide** (`docs/MIGRATION_v0.13.md`) maps every retired crate to its new module path.
- **Folder discipline preserved**: each former microcrate is a folder with `mod.rs` facade, `pub(crate)` default, preserved tests, optional `CLAUDE.md`. Agents and humans navigating the codebase still see a microcrate-shaped layout.
- **Wave merge serialization**: same-Cargo.toml waves serialize (parser train: PR #2 → A → B → C → D; LSP train: F → G1 → G2 → G3); cross-train parallelism is allowed (DAP wave H parallel after D).

## Source-Grounded Evidence

The current state and the constraint that drives this ADR:

- `Cargo.toml` lists 135 workspace members and 132 entries in `[workspace.metadata.publish].allow`.
- `xtask`, `crates/perl-ci-hygiene`, and `crates/perl-lsp-ux-tests` are the only workspace members currently marked `publish = false`.
- `scripts/publish-topo.py` implements topological publish order with Tarjan SCC for dev-dep cycle breaking — operational evidence that the publish graph is non-trivial.
- `crates/perl-parser/Cargo.toml` currently depends on 8 `perl-lsp-*` crates (code-actions, completion, diagnostics, inlay-hints, navigation, rename, semantic-tokens, tooling) — a layering inversion that PR #0 of the collapse fixes as a prerequisite.
- Cargo packaging documentation: path-only deps rejected at package time unless paired with a registry version; dev-dependencies are exempt from this restriction.
- crates.io publishing rules: no dependencies on code outside crates.io, with dev-dependencies as the documented exception.

## When to Revisit

Review this ADR if any of the following become true:

1. A retired microcrate develops a real external consumer story (in which case promote it back to a published crate per the promotion checklist below).
2. crates.io itself adds first-class support for unpublished workspace dependencies in published runtime graphs (would relax the central constraint).
3. Compile-time profiling shows large-crate codegen overhead exceeds the per-crate link overhead we eliminated (would suggest selective re-splitting).
4. The ~30-crate count grows past ~40 through accumulated promotions (would suggest the initial classification was too conservative).

## Promotion checklist (for moving a module back to a published crate later)

Only promote when **all** of the following are true:

1. It is independently useful outside perl-lsp (you can name a plausible external consumer).
2. It has a stable, documentable API surface (you can keep it stable for a year).
3. Its dependency cone is reasonable (it doesn't drag the whole server runtime).
4. It has its own tests/fixtures that don't require booting the entire workspace.
5. You are willing to publish and support it (because crates.io makes it permanent once it's in the published dependency graph).

When promoting, also update these operational artifacts:
- Add the new crate name to `[workspace.metadata.publish].allow` in `Cargo.toml`.
- Add a row to `docs/project/PUBLISHING_AFTER_COLLAPSE.md` in the appropriate category.
- Verify `cargo xtask published-crate-count` still passes (or raise the ceiling intentionally with a comment).

## Amendments

### Amendment 1 — 2026-04-15: Target count frozen at 30; pilot target corrected; guardrails added

**Source:** Orchestrator refinement following ledger construction in `.spec/microcrate-collapse/ledger.md`.

#### Target count: 30 published (not 31)

The original Decision section listed "~30" informally. After constructing the full per-crate ledger, the precise count is **30 published crates**. The earlier draft that circulated as "~30/31" double-counted `perl-diagnostic-catalog`, which appeared both under the diagnostic surface bullet and as an annotation in the "1 NEW" note. Corrected category breakdown:

| Category | Count | Crates |
|----------|------:|--------|
| Products | 4 | perllsp, perl-lsp-rs, perl-dap, perl-parser |
| Tree-sitter | 2 | tree-sitter-perl-c, tree-sitter-perl-rs |
| Alternate parser | 1 | perl-parser-pest |
| Foundation primitives | 5 | perl-lexer, perl-token, perl-line-index, perl-uri, perl-pod |
| Diagnostic catalog (NEW) | 1 | perl-diagnostic-catalog |
| Wire protocols | 2 | perl-lsp-protocol, perl-content-length-framing |
| Semantic kernels | 3 | perl-semantic-analyzer, perl-module, perl-workspace (see Amendment 2) |
| Symbol model | 1 | perl-symbol |
| Tool integrations | 1 | perl-lsp-perltidy |
| Test/corpus ecosystem | 4 | perl-corpus, perl-tdd-support, perl-test-must, perl-test-generators |
| Standalone tooling | 6 | perl-feature-catalog, perl-incremental-parsing, perl-refactoring, perl-dead-code, perl-heredoc-anti-patterns, perl-path-security |
| **Total** | **30** | |

#### Pilot target: `perl-module` facade (not `perl-module-resolution`)

Wave 1 PILOT absorbs 13 `perl-module-*` crates. The Decision section above named `perl-module-resolution` as the absorption target; that name is retired. The surviving published crate is **`perl-module`** — a better external noun that owns names, imports, references, resolution, rename, and boundary as internal folder families. Facade-first design: the new `perl-module` crate presents a clean `pub use` surface; all 13 absorbed crates become internal `mod` folders.

#### No public `perl-syntax` umbrella crate

An earlier draft discussion proposed a `perl-syntax` umbrella crate. That proposal is rejected. Syntax primitives (AST, quote, heredoc, error) are absorbed into `perl-parser` as a `syntax/` internal folder family. There is no publicly-published `perl-syntax` crate.

#### Wave order locked

The migration wave sequence is fixed as:

```
1. perl-module-* → perl-module         (PILOT)
2. perl-workspace-* → perl-workspace   (perl-workspace-index renamed; see Amendment 2)
3. lexer satellites → perl-lexer
4. parser/AST satellites → perl-parser
5. semantic shards → perl-semantic-analyzer
E. diagnostic catalog (NEW) → perl-diagnostic-catalog
F. perl-lsp-feature-* → perl-lsp-rs::features
G1. LSP providers → perl-lsp-rs::providers
G2. LSP runtime → perl-lsp-rs::runtime
G3. LSP governance → perl-lsp-rs
H. perl-dap-* → perl-dap              (last, after DAP is stable)
FINAL. Shrink allowlist to 30
```

Do **not** start with the `perl-lsp-*` forest (waves F/G) before the module/workspace/lexer/parser/semantic trains (waves 1-5) are complete. The LSP forest is the largest and most snapshot-heavy; doing it last reduces rebase pressure.

#### Guardrails added

Two additional CI gates join the three listed in the Decision section:

- **Packaging dry-runs**: `cargo package -p <crate>` and `cargo publish --dry-run -p <crate>` run in CI for each surviving published crate. Catches path-only dep drift before a real publish attempt.
- **Public API ratchet**: `cargo public-api diff` runs per-published-crate. Fails on unintended public surface expansion between PRs.

#### Migration ledger is authoritative

The per-crate workboard at [`.spec/microcrate-collapse/ledger.md`](../../.spec/microcrate-collapse/ledger.md) is the authoritative source for crate disposition, wave assignment, and progress tracking. It supersedes any comments, draft notes, or memory entries that reference "~30/31" or name `perl-module-resolution` as the Wave 1 target. Agents and humans executing the collapse must read the ledger, not reconstruct from this ADR alone.

### Amendment 2 — 2026-04-16: Target release is v0.13.0 (not v0.14.x); Wave 2 owner renamed perl-workspace

**Source:** User correction 2026-04-16.

#### Target release: v0.13.0 clean-break (not v0.14.x)

The Decision section and Amendment 1 referenced "v0.14.x" as the release that ships the new
30-crate surface. This was incorrect. The correct target is **v0.13.0**.

Current workspace version is **0.12.4**. The microcrate collapse ships as **v0.13.0** — a clean
break that moves directly from the 0.12.x confidence track to the collapsed surface. There is no
interim pre-collapse v0.13.0 release; v0.13.0 *is* the collapse release.

Any document, spec file, or comment that says "v0.14.x", "v0.14.0", or implies a pre-collapse
v0.13.0 ship gate is incorrect. The correction is: **v0.13.0 is the clean-break collapse release**.

The migration guide will be published as `docs/MIGRATION_v0.13.md` (not `MIGRATION_v0.14.md`).

#### Wave 2 owner renamed: perl-workspace-index → perl-workspace

Wave 2 (the `perl-workspace-*` collapse) was slotted to absorb 6 satellites into the existing
`perl-workspace-index` crate. The 6 satellites span two functional families:

- **Enumeration**: `perl-workspace-discovery`, `perl-workspace-folder`, `perl-workspace-ignore`
- **Observability**: `perl-workspace-index-monitoring`, `perl-workspace-index-slo`, `perl-workspace-index-state-machine`

The absorbed scope is broader than "indexing." The existing `perl-workspace-index` crate will
be **renamed to `perl-workspace`** during Wave 2 execution. `perl-workspace` is a more accurate
external noun for a crate that owns workspace enumeration, discovery, and observability alongside
the index itself.

The Amendment 1 table in this document listed `perl-workspace-index` under "Semantic kernels."
That entry should now read **`perl-workspace`**:

| Category | Count | Crates |
|----------|------:|--------|
| Semantic kernels | 3 | perl-semantic-analyzer, perl-module, **perl-workspace** |

The migration ledger at [`.spec/microcrate-collapse/ledger.md`](../../.spec/microcrate-collapse/ledger.md)
is the authoritative source and has been updated accordingly.

### Amendment 3 — 2026-04-16: Ledger corrected — perl-symbol is its own published crate (Wave B)

**Source:** Accuracy-scout finding on issue #4428; user ruling: ADR wins.

The migration ledger at `.spec/microcrate-collapse/ledger.md` previously listed all four
`perl-symbol-*` satellites as Wave 5 modules absorbing into `perl-semantic-analyzer`. This
contradicted the Decision section above ("Symbol model (1): `perl-symbol`").

**The correction:** The four satellites (`perl-symbol-types`, `perl-symbol-cursor`,
`perl-symbol-index`, `perl-symbol-surface`) absorb into **`perl-symbol`** — a standalone
small published crate — not into `perl-semantic-analyzer`. The ledger now has a dedicated
**Wave B** section for this family and Wave 5 is annotated to reflect it holds no symbol crates.

**Rationale for keeping perl-symbol as its own published crate:**
`perl-symbol-types` is consumed directly by `perl-workspace-index`, `perl-semantic-analyzer`,
and `perl-lsp`. If it folded into `perl-semantic-analyzer`, then `perl-workspace-index` and
`perl-lsp` would need to depend on the whole analyzer just to get symbol types — a dependency
inversion. Keeping `perl-symbol` as a small, focused, published crate preserves clean layering.
This is the same reasoning that kept `perl-workspace` as its own crate (Amendment 2).

The 30-crate published target and all other ADR content remain unchanged.

### Amendment 4 — 2026-04-17: Ledger corrected — perl-token stays published (Wave C)

**Source:** Same pattern as Amendment 3 (perl-symbol, Wave B). Ledger/ADR conflict; ADR wins.

The migration ledger at `.spec/microcrate-collapse/ledger.md` previously listed `perl-token` in
the Wave 3 lexer-satellites table as a module absorbing into `perl-lexer`. This contradicted the
Decision section above ("Foundation primitives (5): `perl-lexer`, `perl-token`, `perl-line-index`,
`perl-uri`, `perl-pod`") and Amendment 1's confirmation of `perl-token` as one of the 5 foundation
primitives.

**The correction:** Wave C (the lexer collapse) absorbs only the 4 satellites
(`perl-tokenizer`, `perl-keywords`, `perl-builtins`, `perl-builtins-phf`) into `perl-lexer`.
`perl-token` remains a separately published foundation primitive per ADR-0041's original design.

**Rationale for keeping perl-token as its own published crate:**
`perl-token` is a minimal, durable foundation primitive — the token type is consumed across the
full analysis stack (lexer, parser, semantic analyzer, LSP, DAP). Folding it into `perl-lexer`
would force every downstream consumer of the token type to depend on the full lexer implementation
just to get the type. Keeping `perl-token` as a small, focused, published crate preserves clean
layering and matches the same reasoning that kept `perl-symbol` (Amendment 3) and `perl-workspace`
(Amendment 2) as their own published crates.

The 30-crate published target and all other ADR content remain unchanged.

## References

- [Tracking issue #4410: Microcrate collapse to ~30 published crates](https://github.com/EffortlessMetrics/perl-lsp/issues/4410)
- [Migration ledger](../../.spec/microcrate-collapse/ledger.md) — authoritative per-crate workboard (Amendment 1, updated Amendment 2)
- [docs/SRP_MICROCRATES.md](../SRP_MICROCRATES.md) — historical record of the microcrate decomposition campaign
- [docs/PUBLISHING.md](../PUBLISHING.md) — current publishing pipeline; will simplify post-collapse
- [scripts/publish-topo.py](../../scripts/publish-topo.py) — current topological publish ordering with dev-dep SCC handling
- [Cargo manifest: publish field](https://doc.rust-lang.org/cargo/reference/manifest.html#the-publish-field)
- [Cargo: specifying dependencies (path + version pattern)](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html)
- [crates.io publishing rules](https://doc.rust-lang.org/cargo/reference/publishing.html)
