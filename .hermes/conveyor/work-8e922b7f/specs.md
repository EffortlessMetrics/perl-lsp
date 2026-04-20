# Specification: cpanfile/META.yml Dependency Analysis

**Work Item**: work-8e922b7f

**GitHub Issue**: https://github.com/EffortlessMetrics/perl-lsp/issues/3428

---

## Feature Summary

Auto-detect Perl dependencies from `cpanfile`, `META.json`, and `META.yml` files to:
1. Augment `includePaths` with vendor paths (carton/carmel/cpm → `vendor/lib/perl5`, local::lib → `local/lib/perl5`)
2. Suppress PL701 (missing module) warnings for modules declared in dependency files
3. Provide a quick-fix code action to add missing dependencies to cpanfile

---

## Acceptance Criteria

### AC-1: Dependency Parsing
**Given** a workspace root containing `cpanfile`, `META.json`, or `META.yml`

**When** `perl-dependency-metadata` is queried for that root

**Then** it returns a `DependencyInfo` containing:
- A list of declared `ModuleRequirement { name: String, version: Option<String> }` parsed from the file
- The detected vendor path (`vendor/lib/perl5` or `local/lib/perl5`) based on directory existence, or `None` if neither exists

**And** parsing failures for one format (e.g., malformed YAML) do not cause failures for other formats

---

### AC-2: Auto-includePaths Augmentation
**Given** a workspace with a `cpanfile` in its root that declares `requires 'Moo';`

**When** the LSP computes `effective_include_paths()` without any user-configured `include_paths`

**Then** the result includes `vendor/lib/perl5` or `local/lib/perl5` (whichever exists) in addition to the defaults (`["lib", ".", "local/lib/perl5"]`)

**And** if the user has manually configured `include_paths` in `.perl-lsp.toml`, their paths are not overridden — only augmented

---

### AC-3: PL701 Suppression for Declared Dependencies
**Given** a workspace with a `cpanfile` declaring `requires 'Moo';` but `Moo` is not yet installed

**When** a file in the workspace uses `use Moo;`

**Then** PL701 is **not** emitted (suppressed because Moo is declared in cpanfile)

**And** if the same file uses `use NonExistent::Module;` which is NOT in cpanfile, PL701 **is** emitted

---

### AC-4: Quick-Fix Code Action for Missing Dependency
**Given** a file with `use NonExistent::Module;` where `NonExistent::Module` is not in cpanfile and not a core Perl module

**When** the LSP server receives a `textDocument/codeAction` request for that diagnostic

**Then** a `CodeActionKind::QuickFix` is returned with label "Add 'NonExistent::Module' to cpanfile"

**And** applying the code action inserts `requires 'NonExistent::Module';` into the cpanfile in alphabetical order, preserving existing formatting and comments

---

## Non-Goals (Out of Scope)

- **Build.PL / Makefile.PL parsing** — these are executable Perl scripts, not declarative dependency files
- **Running package managers** — no `cpanm install` or similar invocations
- **Lockfile-aware resolution** — cpanfile.snapshot and cpanfile.lock are not consulted
- **Hover information** — showing dependency version requirements on hover is a separate feature
- **IntelliSense for cpanfile editing** — cpanfile DSL completion/hints are a separate feature
- **`require` statements** — only `use` statements are handled in Phase 3; `require` requires a `NodeKind::Require` parser addition
- **Carton/Carmel/CPM differentiation** — all tools using `vendor/lib/perl5` are treated identically; tool-specific behavior is out of scope

---

## Dependencies

| Dependency | Purpose |
|------------|---------|
| `serde_yaml` | META.yml parsing (new) — add to `perl-dependency-metadata/Cargo.toml` |
| `serde_json` | META.json parsing (already in workspace) |
| `regex` | cpanfile parsing (already in workspace) |
| `perl-dependency-metadata` | New crate: `crates/perl-dependency-metadata/` |
| `perl-lsp-diagnostics` | PL701 enhancement (existing crate) |
| `perl-lsp-code-actions` | Quick-fix registration (existing crate) |

---

## Phase Deliverables

| Phase | Deliverable |
|-------|-------------|
| Phase 1 | `crates/perl-dependency-metadata/` crate with cpanfile, META.json, META.yml parsers |
| Phase 2 | `perl-lsp-config` augmented to call Phase 1 and add vendor paths to `effective_include_paths()` |
| Phase 3 | `perl-lsp-diagnostics` PL701 enhanced with two-stage cpanfile-aware resolver |
| Phase 4a | Cpanfile editing utility (read, insert in alphabetical order, preserve formatting) |
| Phase 4b | Code action wired to Phase 4a for PL701 |

---

## File Changes

| File | Change |
|------|--------|
| `crates/perl-dependency-metadata/Cargo.toml` | New file |
| `crates/perl-dependency-metadata/src/lib.rs` | New file — parsers and `DependencyInfo` |
| `crates/perl-lsp-config/src/lib.rs` | Add vendor path detection via Phase 1 |
| `crates/perl-lsp-diagnostics/src/lints/missing_module.rs` | Two-stage resolver for PL701 |
| `crates/perl-lsp-code-actions/src/code_actions.rs` | Register quick-fix for PL701 |
| `crates/perl-lsp-code-actions/src/quick_fixes.rs` | Implement add-to-cpanfile quick-fix |
| `Cargo.toml` | Add `perl-dependency-metadata` to workspace members |
| `docs/reference/CONFIG.md` | Document auto-detection behavior |
