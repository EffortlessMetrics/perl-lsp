# @INC / Module Resolution Conformance

This page tracks live LSP module-resolution behavior for provider consumers.
It is distinct from HIR compiler-substrate module-request facts, which are
tracked in [compiler_facts.md](compiler_facts.md) and [#8242](https://github.com/EffortlessMetrics/perl-lsp/issues/8242).

Consumer-consistency matrix — verified end-to-end through all four LSP consumers
(PL701 diagnostic, completion, goto-definition, hover) for each `@INC` resolution mode.

**Test**: `cargo test -p perl-lsp-ux-tests --test ux_scenario_14_inc_conformance -- --nocapture`

## Consumer Consistency Matrix

Each cell indicates whether the consumer agrees on module resolution for the given mode.
A `+` means the consumer produced the expected answer (resolved or not-resolved consistently).
A `-` means the consumer diverges or the feature is not yet fully enforced.

**Fixture semantics**: completion uses prefix fixtures (`use Gre<cursor>`);
PL701, goto-definition, and hover use exact-module fixtures (`use GreetModule;`).

| Resolution Mode | PL701 diagnostic | completion | goto-definition | hover | Notes |
|---|---|---|---|---|---|
| Workspace `includePaths` | + | + | + | + | Config-driven: `includePaths: ["lib"]` |
| Absolute `includePaths` | + | + | + | + | Config-driven: absolute path entry |
| Lexical `use lib` | + | + | + | + | In-source pragma extraction |
| `no lib` cancellation | + | + | + | + | Position-aware negative; all four consumers enforce #8516 |
| FindBin-relative | + | + | + | + | `$FindBin::Bin/lib` pattern |
| PERL5LIB env | + | + | + | + | `usePerl5lib=true` gates PERL5LIB |
| interpreter startup `@INC` | + | + | + | + | `useSystemInc=true` gates interpreter startup paths |

**Key**: Consumer cells are `+` (consistent) or `-` (divergent / unimplemented).
Conformance means all consumers agree — not necessarily that every mode resolves.

## Rail Status — @INC integration complete (2026-05-11)

The cross-consumer `@INC` rail landed across `#8493 → #8506`:

- `PERL5LIB` is gated by `usePerl5lib`; the startup-`@INC` probe also strips `PERL5LIB` from its subprocess environment when `usePerl5lib=false` so the two flags stay independent. (#8493)
- Interpreter startup `@INC` is gated by `useSystemInc`; the probe is bounded by `SYSTEM_INC_PROBE_TIMEOUT = 1000 ms` and cached. (#8497)
- Completion, PL701, goto-definition, and hover share `EffectiveIncContext` for include-root assembly. (#8504, #8505, #8506)
- PL701 displays labeled search roots via `ModuleSearchPathDisplay`. (#8502)
- Nested multi-root workspaces resolve folder, config, include paths, and completion-cache write-back against the most-specific (deepest) matching folder. (#8496)
- Module completion uses prefix-directed scan for namespaced prefixes. (#8498)
- Startup-`@INC` probe failures and timeouts emit targeted warnings while preserving the cached-empty fail-closed behavior. (#8518)
- Docs and JSON schema document `usePerl5lib`, `perl5libPrecedence`, and the three sources of search paths. (#8494)
- Scenario 14 conformance harness has a completion column and prefix-vs-exact fixture semantics. (#8495)

Known follow-ups (each tracked as its own issue, not blocking rail closure):

- Pull-diagnostics path (`features/diagnostics/pull.rs`) and workspace-index-backed consumers now honor per-use-statement `no lib` cancellation — resolved in the follow-up commit to #8516.
- Runtime-owned short TTL cache for prefix module scans — split out of #8491 after PR 7a (scan-only) landed in #8498.

## Resolution Mode Details

### Workspace `includePaths`

Configured via `workspace/didChangeConfiguration`:

```json
{ "settings": { "perl": { "workspace": { "includePaths": ["lib"] } } } }
```

Module lives at `lib/Module.pm` relative to the workspace root.

### Absolute `includePaths`

Configured via `workspace/didChangeConfiguration` with an absolute folder path:

```json
{ "settings": { "perl": { "workspace": { "includePaths": ["/abs/path/to/lib"] } } } }
```

Module lives at `/abs/path/to/lib/AbsoluteModule.pm`.

### Lexical `use lib`

Source-level pragma: `use lib 'lib';` before `use Module;`. The LSP extracts
`use lib` and `no lib` operations in lexical order via
`resolve_use_lib_paths_from_source()` in
`crates/perl-module/src/resolution/use_lib.rs`.

### `no lib` Cancellation

Position-aware negative test: `use lib 'lib'; no lib 'lib'; use GoneModule;`.
The module file exists on disk but must NOT resolve because `no lib` cancelled
the earlier `use lib` before the `use GoneModule` line.

### FindBin-Relative

Pattern: `use FindBin; use lib "$FindBin::Bin/lib";`. `$FindBin::Bin` resolves
to the directory containing the script being analyzed. The module must be at
`<script_dir>/lib/Module.pm`.

### PERL5LIB env (`usePerl5lib`)

Module lives outside the workspace in a directory injected via the `PERL5LIB`
environment variable. Controlled by `usePerl5lib: true` in workspace config.
This flag is independent of `useSystemInc`.

### Interpreter startup `@INC` (`useSystemInc`)

Modules reachable via the interpreter's startup `@INC` (the result of
`perl -e 'print join("\n", @INC)'`). Controlled by `useSystemInc: true` in
workspace config. The startup-@INC probe strips `PERL5LIB` from the spawned
environment when `usePerl5lib=false` to prevent cross-flag leakage.

The two flags are independent: `usePerl5lib` controls PERL5LIB; `useSystemInc`
controls interpreter startup roots. Setting one does not imply the other.

## Include-Root Classification

When filtering workspace-symbol candidates through `EffectiveIncContext`,
include roots are NOT equivalent paths — they have semantically distinct
sources that affect cancellation, reachability, and filter behavior.

| Kind | Source | Subject to `no lib` cancellation? | Notes |
|---|---|---|---|
| `WorkspaceDefaultDot` | `.` from the workspace folder | No | **Wildcard-like** — matches almost any workspace file; do NOT treat as an ordinary library root for reachability filters |
| `WorkspaceConfiguredRelative` | `includePaths: ["lib", "t/lib"]` config | No | Explicit operator intent; persists for the workspace lifetime |
| `WorkspaceConfiguredAbsolute` | `includePaths: ["/abs/path"]` config | No | Same as Relative but absolute |
| `LexicalUseLib` | `use lib '...'` in the source under analysis | **Yes** (position-scoped) | Cancelled by a downstream `no lib '...'` at the cancel-point offset |
| `LexicalNoLibCancellation` | `no lib '...'` cancellation marker | n/a — the cancel itself | Removes a `LexicalUseLib` entry from the position-scoped active set |
| `Perl5LibEnv` | `PERL5LIB`, gated by `usePerl5lib` | No | Inherited from the LSP process environment; stripped from subprocess oracles per #8551 |
| `InterpreterStartup` | `perl -e 'print @INC'`, gated by `useSystemInc` | No | Output of the subprocess seam — see [perl-subprocess-seams.md](../../architecture/perl-subprocess-seams.md) (#8555) |
| `FindBinDerived` | `use FindBin; use lib "$FindBin::Bin/..."` | Yes (position-scoped) | `$FindBin::Bin` derived per analyzed file |
| `RuntimeDerived` | Other lexical paths derived at runtime | Yes (position-scoped) | Currently rare; reserved for future use |

**Why this matters**: if `.` (the workspace default) is treated like any
other configured include root, reachability filters incorrectly conclude
that nearly every workspace file is reachable, which defeats checks like
"after `no lib 'lib'`, `lib/GoneModule.pm` should NOT resolve." Filters
must branch on the kind, not the path.

See `crates/perl-lsp-rs/src/runtime/lifecycle/inc_context.rs` for the
runtime implementation and `crates/perl-lsp-rs/CLAUDE.md` for the per-crate
rule.

## Implementation Notes

- Position-aware resolution is implemented in `crates/perl-module/src/resolution/use_lib.rs`.
- Effective include-root assembly is shared through
  `build_effective_inc_roots()` in `crates/perl-module/src/resolution/uri.rs`;
  it preserves source labels for configured paths, lexical `use lib`, PERL5LIB,
  and interpreter startup paths.
- The four LSP consumers all call either `resolve_module_to_path_with_doc()` or
  `resolve_module_path_with_uri()` from
  `crates/perl-lsp-rs/src/runtime/lifecycle/module_resolution.rs`.
- Consumer call sites:
  - PL701 diagnostic: runtime diagnostics and pull-diagnostics paths
  - completion: module completion path (uses `perl5lib_paths_for_completion()`)
  - goto-definition: runtime navigation path
  - hover: runtime hover path

## Compiler-Substrate Boundary

HIR `CompileEnvironment` already records module requests and include-root facts.
Static module requests now produce HIR candidate facts through
[#8242](https://github.com/EffortlessMetrics/perl-lsp/issues/8242). That HIR
lane does not read ambient environment or spawn Perl from parser lowering;
callers provide configured, lexical, PERL5LIB-labeled, and system-labeled roots
explicitly. Runtime consumers still own filesystem-backed resolution and LSP
provider behavior.

## Follow-up Scope

Tracked follow-ups from the @INC rail completion (each its own issue):

- **Position-aware `no lib` cancellation** — landed in [#8516](https://github.com/EffortlessMetrics/perl-lsp/issues/8516). PL701, pull diagnostics, completion, goto-definition, and hover now reject modules whose path was cancelled by `no lib`; workspace-index-backed consumers are filtered so they cannot bypass active `@INC` state.
- **Runtime-owned TTL cache for module-completion scans** — see [#8514](https://github.com/EffortlessMetrics/perl-lsp/issues/8514). Builds on the prefix-directed scan in #8498.

Backlog (pre-existing, not part of the @INC rail closure):

- `inc_nested_use_lib` — `use lib` inside `BEGIN` block
- `inc_qw_use_lib` — `use lib qw(lib t/lib)` multi-path form
- Cross-scorecard: add `expected.json` diagnostic sidecars to all `inc_*` fixtures

## Literal `require` / `import`

Spec lane tracked under [#4280](https://github.com/EffortlessMetrics/perl-lsp/issues/4280)
(umbrella) and [#8616](https://github.com/EffortlessMetrics/perl-lsp/issues/8616)
(this spec). All literal-form resolution **must** flow through
`EffectiveIncContext` (the same filter introduced by [#8544](https://github.com/EffortlessMetrics/perl-lsp/pull/8544)
for workspace-symbol lookups). No consumer may bypass active `@INC` state to
resolve a literal `require` or `import` form.

### Consumer × form matrix

| Form | PL701 diagnostic | completion | goto-definition | hover |
|---|---|---|---|---|
| `require Foo;` (bareword) | resolve via `EffectiveIncContext` | suggest from prefix scan, filtered by context | jump to module file | show module summary |
| `require "Foo.pm";` (literal, single-segment) | resolve same as bareword `Foo` | suggest matching `.pm` paths under context roots | jump to module file | show module summary |
| `require "Foo/Bar.pm";` (literal, multi-segment) | resolve same as bareword `Foo::Bar` | suggest matching nested `.pm` paths | jump to module file | show module summary |
| `import Foo;` (static bareword) | treat as `use Foo;` for resolution purposes | suggest from prefix scan | jump to module file | show module summary |
| `Foo->import;` (static method call on bareword) | resolve `Foo`, do not interpret import list | suggest the bareword target | jump to `Foo` | show `Foo` summary |

### Boundary table

| In scope | Out of scope |
|---|---|
| `require Foo;` (bareword) | `eval STRING` where STRING contains `use`/`require` |
| `require "Foo.pm";` (string literal, single segment) | `require $module;` (variable holds the path) |
| `require "Foo/Bar.pm";` (string literal, multi segment) | `require "${prefix}::Foo";` (string interpolation) |
| `import Foo;` (static, no list) | Runtime `import` with computed module names |
| `Foo->import;` (static method call) | Plugin frameworks that synthesize module names at runtime |

Out-of-scope forms remain explicitly unresolved — diagnostics may flag them
as "dynamic; cannot statically resolve" but must not guess.

### Acceptance test path (when impl lands)

- New fixtures under `crates/perl-lsp-ux-tests/tests/fixtures/literal_require/`.
- Consumer harness reused from Scenario 14 (`@INC` conformance).
- Boundary table out-of-scope rows MUST produce a documented "unresolved (dynamic)" outcome — not a panic, not a stale hit, not a false positive.

Receipts:

- [#4280](https://github.com/EffortlessMetrics/perl-lsp/issues/4280) — umbrella ux-journey
- [#8616](https://github.com/EffortlessMetrics/perl-lsp/issues/8616) — this spec
- [#8544](https://github.com/EffortlessMetrics/perl-lsp/pull/8544) — `EffectiveIncContext` filter that literal forms must reuse
