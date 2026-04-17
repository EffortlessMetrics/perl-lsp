# Context: Wave A Microcrate Collapse (perl-workspace-* 6 satellites)

## Decision Log

### Target owner name: `perl-workspace` (amended)
- **Previous**: crate was named `perl-workspace-index`
- **Decision**: Rename during collapse, per ledger Amendment 2 (issue #4427 merged)
- **Trade-off**: Directory stays as `crates/perl-workspace-index/` to avoid deep path issues; package name differs from directory, which is valid in Cargo
- **Rationale**: "perl-workspace" is the user-facing name; "index" was an implementation detail

### Layout: flat module folders
- **Decision**: 6 new folders inside `crates/perl-workspace-index/src/` — one per absorbed crate
- **Rationale**: Proven pattern from Wave 1 (#4422, perl-module-* → perl-module); flat is simpler than nested
- **Alternative rejected**: nested folders like `src/observability/monitoring/`, `src/observability/slo/` — adds unnecessary hierarchy

### Type name conflicts → explicit re-exports in `api.rs`
- **Context**: `IndexStateKind`, `IndexStateTransition`, `DegradationReason`, `ResourceKind` defined in BOTH `perl-workspace-index-monitoring` AND `perl-workspace-index-state-machine`
- **Decision**: Use wildcard re-exports only for enumeration satellites (discovery, folder, ignore); explicit named re-exports in `api.rs` for observability (monitoring, slo, state_machine)
- **Trade-off**: More verbose in `api.rs`, but avoids public API ambiguity (users won't see conflicting names exported from the same crate)
- **Rationale**: Consumer code uses observability types via qualified paths (`perl_workspace::monitoring::IndexStateKind`), not wildcard re-exports

### Backward compatibility: existing consumer paths
- **Context**: Existing code calls `perl_workspace::workspace::monitoring::IndexPhase`, `::slo::*`, `::state_machine::*`
- **Decision**: Preserve dual paths — consumers can use both `perl_workspace::monitoring::X` (new) and `perl_workspace::workspace::monitoring::X` (old)
- **Implementation**: `src/workspace/mod.rs` contains explicit named re-exports, not wildcards
- **Trade-off**: Slightly larger `api.rs`, but zero breakage for existing code

### Test file naming
- **Context**: `perl-workspace-index/tests/` already has `comprehensive_unit_tests.rs`; incoming discovery also has `comprehensive_unit_tests.rs`
- **Decision**: Prefix incoming files to avoid collision (discovery → `discovery_comprehensive_unit_tests.rs`)
- **Pattern**: Wave 1 used this convention (perl-module/tests/)
- **Implication**: 15 test files total; 1 needs prefix (discovery), others keep original names

### Why not delete `perl-workspace-index` directory outright?
- **Problem**: Deep nested paths like `/h/Code/Rust/perl-lsp/.claude/worktrees/agent-XXXXX/crates/perl-workspace-index/` already hit Windows MAX_PATH limits
- **Decision**: Keep directory, rename package only
- **Precedent**: Wave 1 learned this (feedback_wave1_collapse_gotchas.md)
- **Impact**: CI/build behavior unchanged; only `Cargo.toml` name field updates

### Why 8 consumer crates, not fewer?
- **Finding**: architecture-reviewer identified all paths to workspace module usage across the codebase
- **List**: perl-lsp, perl-module, perl-parser, perl-semantic-analyzer, perl-refactoring, perl-dead-code, perl-lsp-completion, perl-lsp-diagnostics
- **Verification**: Each dependency is declared in their Cargo.toml (verified before spec planning)

### Publish allowlist impact: 120 → 114
- **Logic**: Remove 6 satellite names + rename 1 old name = 7 entries removed, 1 added = 114 total
- **Why NOT 30?**: 30 is the end-state of the entire collapse program (#4410); this wave is just Wave A
- **Implication**: Allowlist remains hand-maintained; builder must verify `cargo xtask publish-closure` shows 114

## Alternatives Considered

1. **Nested observability structure** (`src/observability/{monitoring,slo,state_machine}`) — rejected as over-engineered
2. **Wildcard re-export in `api.rs` for observability** — rejected; creates public API ambiguity
3. **Move directory to `crates/perl-workspace/`** — rejected; Windows MAX_PATH risk
4. **Delete and recreate the crate** — rejected; adds friction, directory identity is not critical

## Edge Cases & Mitigations

### Edge case: Consumer code with glob imports
- **Risk**: `use perl_workspace_index::*;` becomes invalid after crate rename
- **Mitigation**: Grep for `use perl_workspace_index::*` before builder starts; update any found
- **Likelihood**: Low (not idiomatic Rust); spot-checked at spec-planning

### Edge case: hardcoded crate name strings in tests/tooling
- **Risk**: `perl-workspace-index` string references in CI/hygiene tools or test snapshots
- **Mitigation**: Known locations identified (perl-ci-hygiene, perl-parser tests); checklist includes updates
- **Likelihood**: Medium (tooling is rigid)

### Edge case: Cargo.lock or lock-file changes
- **Risk**: Renaming a workspace member may cause lock-file churn
- **Mitigation**: Builder should commit the lock-file result as-is (no manual tweaks)
- **Likelihood**: Low (workspace members don't change ordering in lock)

### Edge case: Documentation/README references
- **Risk**: Project docs or README still say `perl-workspace-index`
- **Mitigation**: Not in scope for this spec; leave as-is for a separate doc-refresh issue
- **Likelihood**: Low priority (docs/README are not blockers)

## Verification Strategy

1. **Post-merge verification** (builder responsibility):
   - Workspace member count: 123 → 117 (6 crates deleted)
   - Publish allowlist count: 120 → 114 (6 removed, 1 renamed)
   - No imports of old crate names in source
   - All 8 consumer crates compile

2. **Test verification**:
   - 15 test files migrated, no name collisions
   - All tests pass in perl-workspace and consumers
   - No dangling references to old modules

3. **Cargo verification**:
   - `cargo metadata --no-deps` shows 117 members
   - `cargo xtask publish-closure` shows 114 allowed packages
   - `cargo build -p perl-workspace` succeeds
   - All 8 consumers build without errors

## Key Risk Flags

- **High**: Hardcoded crate name strings in tooling (perl-ci-hygiene line 4505, perl-parser test lines 607-608)
- **Medium**: 131 import occurrences to update across 8+ crates
- **Medium**: Type name conflicts in api.rs require careful explicit re-exports
- **Low**: Test file name collision (discovery/comprehensive_unit_tests.rs → discovery_comprehensive_unit_tests.rs)
