# @INC / Module Resolution Conformance

This page tracks live LSP module-resolution behavior for provider consumers.
It is distinct from HIR compiler-substrate module-request facts, which are
tracked in [compiler_facts.md](compiler_facts.md) and [#8242](https://github.com/EffortlessMetrics/perl-lsp/issues/8242).

Consumer-consistency matrix — verified end-to-end through all three LSP consumers
(PL701 diagnostic, goto-definition, hover) for each `@INC` resolution mode.

**Test**: `cargo test -p perl-lsp-ux-tests --test ux_scenario_14_inc_conformance -- --nocapture`

## Consumer Consistency Matrix

Each cell indicates whether the consumer agrees on module resolution for the given mode.
A `+` means the consumer produced the expected answer (resolved or not-resolved consistently).

| Resolution Mode | PL701 diagnostic | goto-definition | hover | Notes |
|---|---|---|---|---|
| Workspace `includePaths` | + | + | + | Config-driven: `includePaths: ["lib"]` |
| Absolute `includePaths` | + | + | + | Config-driven: absolute path entry |
| Lexical `use lib` | + | + | + | In-source pragma extraction |
| `no lib` cancellation | + | + | + | Position-aware negative case |
| FindBin-relative | + | + | + | `$FindBin::Bin/lib` pattern |
| System `@INC` (PERL5LIB) | + | + | + | PERL5LIB injection |

**Key**: Consumer cells are `+` (consistent) or `-` (divergent / unimplemented).
Conformance means all consumers agree — not necessarily that every mode resolves.

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

### System `@INC` (PERL5LIB)

Module lives outside the workspace in a directory injected via the `PERL5LIB`
environment variable. Requires `usePerl5lib: true` in workspace config.

## Implementation Notes

- Position-aware resolution is implemented in `crates/perl-module/src/resolution/use_lib.rs`.
- Effective include-root assembly is shared through
  `build_effective_inc_roots()` in `crates/perl-module/src/resolution/uri.rs`;
  it preserves source labels for configured paths, lexical `use lib`, PERL5LIB,
  and interpreter startup paths.
- The three LSP consumers all call either `resolve_module_to_path_with_doc()` or
  `resolve_module_path_with_uri()` from
  `crates/perl-lsp-rs/src/runtime/lifecycle/module_resolution.rs`.
- Consumer call sites:
  - PL701 diagnostic: runtime diagnostics and pull-diagnostics paths
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

## Follow-up Scope (PR 2)

- `inc_perl5lib_env` — PERL5LIB separate from system @INC flag
- `inc_nested_use_lib` — `use lib` inside `BEGIN` block
- `inc_qw_use_lib` — `use lib qw(lib t/lib)` multi-path form
- Cross-scorecard: add `expected.json` diagnostic sidecars to all `inc_*` fixtures
