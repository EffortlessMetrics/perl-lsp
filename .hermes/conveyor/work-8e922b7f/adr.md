# ADR-042: cpanfile/META.yml Dependency Analysis

**Status**: Proposed

**Work Item**: work-8e922b7f

**GitHub Issue**: https://github.com/EffortlessMetrics/perl-lsp/issues/3428

---

## Context

The LSP server recognizes `cpanfile`, `META.yml`, `META.json`, `Build.PL`, and `Makefile.PL` as workspace root markers but does not parse them to extract dependency information. Users must manually configure `includePaths` despite these files existing.

The Technical Vision explicitly targets META.json parsing (v0.13.0) and cpanfile support (v0.14.0) — this work serves those roadmap items.

---

## Decision

### 1. New Crate: `perl-dependency-metadata`

Create `crates/perl-dependency-metadata/` (Tier 3) to parse Perl dependency files. This separates parsing from LSP concerns and enables reuse.

**Parsers**:
- **cpanfile**: Regex-based parser for `requires 'Module', 'version';` lines. Grammar is trivial.
- **META.json**: `serde_json` (already a dependency) — JSON structure is well-specified by CPAN.
- **META.yml**: `serde_yaml` (add as dependency) — YAML structure is well-specified; graceful fallback on parse failure.

**Output**: For each workspace root, return:
```rust
pub struct DependencyInfo {
    pub declared: Vec<ModuleRequirement>,  // module name + version constraint
    pub vendor_path: Option<PathBuf>,        // vendor/lib/perl5 or local/lib/perl5
}
```

**Vendor path detection**: Check for `vendor/lib/perl5` and `local/lib/perl5` directory existence. Do NOT use a `DependencyManager` enum — all tools (carton/carmel/cpm) use `vendor/lib/perl5`, manual local::lib uses `local/lib/perl5`.

---

### 2. Auto-includePaths Integration (Phase 2)

Extend `crates/perl-lsp-config/src/lib.rs`:

- After loading `.perl-lsp.toml` and user settings, scan workspace root for `cpanfile`, `META.json`, `META.yml`
- If found, parse via `perl-dependency-metadata` and augment `effective_include_paths()` with vendor paths
- Add `vendor_path: Option<PathBuf>` field to `WorkspaceConfig` for diagnostics use
- Auto-detection can be disabled via `perl.include_paths` in `.perl-lsp.toml`

**Caching**: Cache parsed `DependencyInfo` per workspace root; invalidate on file modification.

---

### 3. Missing Dependency Diagnostics Enhancement (Phase 3)

Enhance `crates/perl-lsp-diagnostics/src/lints/missing_module.rs`:

- `check_missing_modules()` already emits **PL701** (Warning) for modules not found
- Add a two-stage resolver:
  1. Check if the missing module is declared in cpanfile/META.json/META.yml
  2. If declared → suppress PL701 (it will be installed)
  3. If not declared → emit PL701
- **Scope**: `use` statements only. `require` statements are out of scope (no `NodeKind::Require` exists).

**Note**: Phase 3 enhances existing PL701; it does NOT create a new diagnostic code.

---

### 4. Code Action to Add Missing Dependency (Phase 4a + 4b)

**Phase 4a** (new): Build cpanfile editing utility in `perl-dependency-metadata`:
- Read cpanfile, preserve formatting and comments
- Insert `requires 'Module';` in alphabetical order
- Handle edge case: cpanfile doesn't exist → create it

**Phase 4b**: Wire into `crates/perl-lsp-code-actions/src/`:
- Register `CodeActionKind::QuickFix` handler for PL701
- Activate → call Phase 4a utility → write updated cpanfile

---

## Consequences

### Tradeoffs

| Benefit | Cost |
|---------|------|
| Clean separation: dedicated crate | Adds Tier-3 crate to workspace |
| Incremental delivery | Phase 4a must ship before 4b |
| Reuses PL701 infrastructure | Phase 3 suppressed for declared-but-not-installed only |
| Directory-based vendor detection (simple) | Cannot distinguish between carton/carmel/cpm |

### Risks

| Risk | Mitigation |
|------|-----------|
| YAML non-standard structure | Prioritize META.json; serde_yaml with lenient parsing; graceful fallback |
| Performance on large workspaces | Cache parsed results per workspace root; invalidate on file change |
| Phase 4a complexity underestimated | Split 4a/4b makes dependency explicit; 4a is scoped narrowly |
| require statement handling deferred | Documented as out-of-scope; future work item |

---

## Alternatives Considered

| Alternative | Why Rejected |
|-------------|-------------|
| Embed parsing in `perl-lsp-config` | Violates single-responsibility |
| `DependencyManager` enum | 6 variants collapse to 2 paths — unnecessary indirection |
| Single Phase 4 (no split) | Phase 4a must exist before 4b; sequencing makes dependency explicit |
| New `missing-dependency` diagnostic | Duplicates existing PL701 |
| Handle `require` in Phase 3 | `NodeKind::Require` doesn't exist |
| Parse Build.PL/Makefile.PL | Executable Perl scripts, not declarative — too complex |
