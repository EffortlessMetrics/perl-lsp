# Native Stack Product Policy

This policy defines the product line agents should preserve while cleaning up
legacy external-tool references.

## Product rule

`perl-lsp` ships the native Rust stack by default:

- `perllsp` for LSP;
- `perl-dap` for DAP;
- native formatter;
- native critic diagnostics;
- native parser, workspace, semantic, and indexing crates.

External Perl tools such as `perltidy`, `perlcritic`, and
`Perl::LanguageServer` are not bundled and are not required for normal
operation. They may appear only as explicit compatibility, migration, or
conformance-comparison adapters.

## Public-surface rule

First-mile product surfaces must describe the native path only. This includes:

- root/product README sections;
- `docs/tutorials/` user guides;
- editor extension marketplace copy and settings descriptions;
- crate READMEs when they are the primary package landing page;
- status/readiness tables that describe distribution readiness;
- CLI help for default shipped binaries.

Legacy adapters must not be introduced in those surfaces as prerequisites,
recommended setup, normal run modes, or default behavior.

## Allowed legacy surfaces

Legacy external tooling references are allowed when the surrounding document is
explicit about compatibility or migration scope. Preferred locations are:

- `docs/reference/*LEGACY*`;
- `docs/reference/archive/`;
- migration or compatibility how-to pages;
- conformance reports and receipts;
- tests that prove legacy references are isolated from native-first surfaces.

## Default-behavior rule

Installed external tools must not change default behavior merely by being on
`PATH`.

- Formatting defaults to the native formatter. `perltidy` may run only when an
  explicit external/compatibility engine is selected.
- Critic diagnostics default to the native critic engine. `perlcritic` may run
  only when an explicit external/compatibility engine is selected.
- Native DAP launch/attach requires a local Perl interpreter and the shipped
  `perl-dap` binary; `Perl::LanguageServer` must not be required for the native
  DAP path.

## Packaging rule

Release archives should contain product binaries and runtime assets owned by
this workspace. They must not bundle external Perl tooling payloads such as
`perltidy`, `perlcritic`, `Perl::LanguageServer`, or
`Devel::TSPerlDAP`.

## Regression search

Agents working on this cleanup should start with a targeted search like:

```bash
rg -n "requires perltidy|requires perlcritic|Perl::LanguageServer.*required|BridgeAdapter|cpanm Perl::LanguageServer|cpan Perl::LanguageServer" \
  docs/tutorials docs/reference crates/perl-dap vscode-extension/package.json book/src
```

Allowed hits must either be in a legacy/compatibility document or in tests that
assert legacy wording stays out of first-mile native docs.

## Facts-substrate layering rule

**Since 2026-07-03 · ADR:** [PLSP-ADR-0006](../adr/PLSP-ADR-0006-perl-workspace-core-facts-substrate.md)

The "native ships" product rule above says *what* runs by default. This rule
says *how the analysis crates are layered* so that the same project facts feed
every product surface without dragging the editor runtime into batch tools.

**Core facts are LSP-free.** Batch fact production lives in one shared
substrate, `perl-workspace-core`, that sits *below* the editor/LSP runtime and
*above* the raw parser. It owns the deterministic project model — files,
packages, symbols, imports/exports, POD, tests, dist metadata, compile effects,
dynamic boundaries — with **stable IDs, byte-and-line source ranges,
provenance, confidence, and explicit limitations** on every fact.

```
leaf facts crates
  perl-lexer · perl-parser-core · perl-position-tracking
  perl-semantic-facts · perl-symbol · perl-module · perl-uri
        ↓  produce raw syntax/semantic facts
perl-workspace-core   (LSP-FREE project-facts substrate)
        ↓  consumed by / exported by
perl-ripr-facts (RIPR)   perl-kwalitee (dist quality)   perl-tree-sitter-compat (later)
        ↓
product runtimes: perl-workspace → perl-lsp-rs-core → perl-lsp-rs
                  perl-dap (own runtime; consumes the substrate for facts)
```

The rule, stated tersely:

```
Core facts crates produce facts.
Product/runtime crates consume facts.
Export crates project facts into external schemas.
```

- **Do not** put batch fact production in `perl-lsp-rs`.
- **Do not** put product schemas (RIPR packet, Kwalitee receipt) in
  `perl-parser-core`.
- **Do not** put LSP types (`lsp-types`, UTF-16 positions) in core facts
  crates. UTF-16 LSP positions are computed only at the LSP boundary, from the
  substrate's byte + UTF-8 `SourceRange`.

### Forbidden dependencies for `perl-workspace-core`

Enforced by `crates/perl-workspace-core/tests/dependency_contract.rs`:

```
perl-lsp-rs · perl-lsp-rs-core · perllsp · perl-dap
lsp-types · tokio · tower-lsp
perl-workspace   (transitively pulls lsp-types via perl-position-tracking lsp-compat)
```

Allowed dependencies:

```
perl-parser-core · perl-position-tracking · perl-semantic-facts
perl-symbol · perl-uri · serde · serde_json (receipts only) · walkdir
```

### Crate naming decisions

| Crate | Role | Decision |
|-------|------|----------|
| `perl-workspace-core` | LSP-free project-facts substrate | **create** |
| `perl-kwalitee` | native distribution-quality scoring | **create** (real crate, not just an xtask script) |
| `perl-ripr-facts` | RIPR packet exporter | **keep** (migrate onto substrate) |
| `perl-tree-sitter-compat` | tree-sitter-compatible output adapter | **later**, only if it grows |
| `perl-test-facts` | reusable Test2/Test::More reader | **later**, only after test facts stabilize |
| `perl-intelligence` / `perl-brain` / `perl-lsp-facts` | — | **rejected** (vague, or wrongly ties facts to the editor runtime) |
| `perl-test2` | — | **rejected** (we *read* Test2, not reimplement it) |

The crate name describes what it **owns**, not the product slogan.
