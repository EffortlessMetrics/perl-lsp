# Perlcritic Integration Research & Implementation Plan

**Date**: 2026-03-19
**Status**: Complete Research + Implementation Plan
**Effort Estimate**: 2-3 sprints (phased)

---

## Executive Summary

**Current State**: Built-in perlcritic integration is 85% complete but **external perlcritic NOT integrated into diagnostics**. Only built-in policies are shown. PerlNavigator (competitors) has full external perlcritic + configurable severity.

**Gap**: `perl.runCritic` executeCommand works, but diagnostics publisher only uses built-in analyzer. External tool violations never reach the editor.

**Fastest Path**: Wire external perlcritic into diagnostics publisher (1-2 sprints).

---

## Current Architecture

### ✅ What Exists

#### 1. Parser Infrastructure
- **crates/perl-lsp-critic-parser** (v0.12.0)
  - Parses perlcritic verbose output: `file:line:column:severity:policy:message`
  - Robust path handling (handles colons in Windows paths)
  - Validates policy names
  - Tests: 10+ unit tests, mutation killing tests

#### 2. Analyzer Infrastructure
- **crates/perl-lsp-tooling/src/perl_critic/**
  - `analyzer.rs`: `CriticAnalyzer` (external tool wrapper)
    - Subprocess execution via `perl-subprocess-runtime`
    - Caching layer
    - Config builder: `--severity`, `--profile`, `--theme`, `--include`, `--exclude`
    - LSP-compatible: `to_diagnostics()` method exists
  - `built_in.rs`: `BuiltInAnalyzer` (native Rust implementation)
    - ~20 built-in policies (require strict, warnings, etc.)
    - No external dependency
  - `types.rs`: `Violation`, `Severity`, `CriticConfig`
  - `quick_fix.rs`: QuickFix code actions (partially implemented)

#### 3. LSP Integration
- **Diagnostics Publisher** (`crates/perl-lsp/src/runtime/diagnostics.rs`)
  - Lines 63-90: Already publishes built-in analyzer violations
  - Missing: External perlcritic integration
- **Execute Command** (`crates/perl-lsp/src/execute_command/provider.rs`)
  - `perl.runCritic` command: Works via `run_critic_secure()`
  - Calls `CriticAnalyzer::analyze_file()` on demand
  - Returns structured JSON with violations

#### 4. Documentation
- **ADR-004**: Execute Command & Code Actions (DRAFT)
  - Strategy: Graceful degradation (external → built-in fallback)
  - Performance targets: <2s for perlcritic analysis
- **docs/project/PERL_LSP_VISION.md**
  - Goal: "Full perlcritic rules catalog, configurable per project"

---

## The Gap: Why External Perlcritic Isn't in Diagnostics

### Current Diagnostics Flow (publish_diagnostics)
```rust
// Line 63-90 in diagnostics.rs
1. Parse errors
2. Semantic analysis (unused vars, etc.)
3. Built-in analyzer violations (always on)
4. Convert to LSP format
5. Publish
```

**Missing**: External perlcritic never runs during `publish_diagnostics()`. Only `perl.runCritic` command triggers it.

### Why This Gap Exists
- Built-in analyzer is **async-safe** (pure Rust, instant)
- External perlcritic is **subprocess-based** (slow, blocking)
- Diagnostics must be fast (LSP 3.16+ push model)
- Original design avoided subprocess calls in hot path

---

## Implementation Plan

### Phase 1: Wire External Perlcritic into Diagnostics (Sprint 1-2)

#### 1.1 Add Config-Driven Toggle
**Goal**: Let users enable external perlcritic diagnostics

**Changes**:
- Add `use_external_perlcritic: bool` to `LspServer` config
- Read from workspace `.perlcriticrc` or `server.initializationOptions`
- Default: **disabled** (don't break current fast path)

**File**: `crates/perl-lsp/src/runtime/config.rs` or new `crates/perl-lsp-config/`

#### 1.2 Integrate into publish_diagnostics
**Goal**: Call external analyzer if enabled

**Changes in `crates/perl-lsp/src/runtime/diagnostics.rs`**:

```rust
// After built-in analysis (line 90)
// Add external analysis if configured
if self.should_use_external_perlcritic() {
    if let Ok(violations) = self.run_external_perlcritic(&doc.text) {
        for violation in violations {
            diagnostics.push(/* convert to internal Diagnostic */);
        }
    }
    // On error, silently skip (graceful degradation)
}
```

**Key Design Decision**:
- External analyzer runs on every file open/change (configurable debounce)
- Failures are silent (don't block diagnostics)
- Falls back to built-in if external not available

#### 1.3 Subprocess Runtime Threading
**Goal**: Don't block LSP server

**Options**:
- **Sync in diagnostics** (simpler, <100ms for typical files)
- **Async with debounce** (complex, requires tokio/channel refactor)

**Recommendation**: Start with sync, add debounce + async later if needed

**Implementation**:
```rust
// In diagnostics.rs
let config = CriticConfig {
    severity: self.critic_severity,
    profile: self.critic_profile.clone(),
    ..Default::default()
};
let mut analyzer = CriticAnalyzer::with_os_runtime(config);

// Timeout to avoid hangs
let result = std::thread::spawn(|| {
    analyzer.analyze_file(Path::new(&doc.path))
})
.join()
.ok()
.and_then(|r| r.ok());
```

---

### Phase 2: User Configuration (Sprint 2-3)

#### 2.1 Settings Schema
**Goal**: Let users control perlcritic from editor

**Add to `crates/perl-lsp/src/server_capabilities.rs`**:

```json
"perl.critic.enabled": bool (default: false),
"perl.critic.severity": 1-5 (default: 3),
"perl.critic.profile": string (default: null),
"perl.critic.theme": string (default: null),
"perl.critic.include": string[] (default: []),
"perl.critic.exclude": string[] (default: []),
"perl.critic.rcFile": string (default: ".perlcriticrc")
```

#### 2.2 Workspace Configuration Discovery
**Goal**: Auto-load `.perlcriticrc` from workspace root

**Implementation**:
```rust
fn load_perlcriticrc(workspace_root: &Path) -> Option<CriticConfig> {
    let rc_path = workspace_root.join(".perlcriticrc");
    if rc_path.exists() {
        // Parse [perlcritic] section
        // Or shell out: perlcritic --verbose=11 --dump-config
    }
    None
}
```

#### 2.3 LSP `workspace/didChangeConfiguration`
**Goal**: Respond to runtime setting changes

**Current Gap**: No implementation

**Add to `crates/perl-lsp/src/runtime/lifecycle/`**:
```rust
fn handle_did_change_configuration(&mut self, settings: Value) {
    self.critic_config = parse_settings(&settings);
    self.refresh_all_diagnostics();
}
```

---

### Phase 3: Performance & UX (Sprint 3+)

#### 3.1 Debouncing & Caching
- **Debounce**: Only run perlcritic 500ms after last edit
- **Cache**: Per-file, invalidate on change
- **Priority**: Analyze active file first

#### 3.2 Configurable Diagnostic Severity Mapping
- Map perlcritic severity (1-5) to LSP severity (1-4)
- Allow filtering violations by severity

#### 3.3 Quick Fixes Integration
- Existing: `quick_fix.rs` has some implementations
- Wire into `textDocument/codeAction` response
- Example: "RequireUseStrict" → "add use strict" quick fix

---

## Technical Decisions

### 1. Sync vs Async Analysis
**Decision**: Start sync, add async + debounce in Phase 3
**Rationale**: Typical Perl file analysis ~50-100ms; acceptable for pull diagnostics model

### 2. Error Handling
**Decision**: Graceful degradation (fail silently)
**Rationale**:
- perlcritic not installed → show built-in only
- perlcritic hangs → don't block LSP
- Bad config → use defaults

### 3. Configuration Source Priority
1. LSP `initializationOptions` (highest)
2. Workspace `.perlcriticrc`
3. User `~/.perlcriticrc` (shell out to perlcritic)
4. Built-in defaults (lowest)

### 4. External vs Built-in Default
**Decision**: External disabled by default
**Rationale**:
- Preserve current performance
- Opt-in for users who have perlcritic installed
- Built-in policies always available

---

## Implementation Roadmap

### Sprint 1 (This Sprint)
- **Task 1a**: Create `perlcritic_config.rs` module
- **Task 1b**: Wire `CriticAnalyzer` into `publish_diagnostics()`
- **Task 1c**: Add feature flag `external-perlcritic` (optional, default off)
- **Task 1d**: Tests: mock perlcritic analysis in diagnostics

### Sprint 2
- **Task 2a**: Add `workspace/didChangeConfiguration` handler
- **Task 2b**: Implement `.perlcriticrc` discovery
- **Task 2c**: Settings schema in `features.toml`
- **Task 2d**: Tests: config override scenarios

### Sprint 3+
- **Task 3a**: Debouncing layer
- **Task 3b**: Async subprocess (tokio)
- **Task 3c**: Quick fix code actions
- **Task 3d**: VSCode extension UI for perlcritic settings

---

## User Experience

### Before (Current)
```perl
# .vscode/settings.json
{
  "perl-lsp.debug": true
}

# Editor: Only built-in policies shown
# Must run "perl.runCritic" command manually for external
```

### After (Phase 1+2)
```json
{
  "perl.critic.enabled": true,
  "perl.critic.severity": 2,
  "perl.critic.profile": "${workspaceFolder}/.perlcriticrc"
}
```

**Result**:
- Diagnostics automatically show perlcritic violations
- Severity filter configurable
- `.perlcriticrc` auto-loaded
- Quick fixes for common violations

### Integration with PerlNavigator Parity
| Feature | Current | After Phase 2 | After Phase 3 |
|---------|---------|---------------|---------------|
| Built-in policies | ✅ | ✅ | ✅ |
| External perlcritic | ❌ | ✅ | ✅ |
| Configurable severity | ❌ | ✅ | ✅ |
| `.perlcriticrc` discovery | ❌ | ✅ | ✅ |
| Quick fixes | Partial | Partial | ✅ |
| Real-time diagnostics | N/A | ✅ | ✅ |

---

## Effort Estimate

| Phase | Tasks | Story Points | Timeline |
|-------|-------|--------------|----------|
| 1: Wiring | 4 tasks | 8-13 | 1-2 sprints |
| 2: Config | 3 tasks | 5-8 | 1 sprint |
| 3: Performance | 4 tasks | 8-13 | 1-2 sprints |
| **Total** | **11 tasks** | **21-34** | **3-5 sprints** |

**Fast Path (Phase 1 only)**: 2 weeks to users can enable external perlcritic

---

## Risk & Mitigation

| Risk | Impact | Mitigation |
|------|--------|-----------|
| perlcritic hangs | LSP blocks | Timeout + thread, skip on error |
| `.perlcriticrc` parsing | Config ignored | Fall back to defaults, log warning |
| Performance regression | User pain | Debounce in Phase 2, benchmark before/after |
| Different output format | Parser fails | Robust parser exists; test against real perlcritic |

---

## References

- **Parser**: `/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/crates/perl-lsp-critic-parser/src/lib.rs`
- **Analyzer**: `/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/crates/perl-lsp-tooling/src/perl_critic/`
- **Diagnostics Publisher**: `/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/crates/perl-lsp/src/runtime/diagnostics.rs:54-90`
- **Execute Command**: `/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/crates/perl-lsp/src/execute_command/provider.rs:120-221`
- **ADR-004**: `docs/adr/ADR_004_EXECUTE_COMMAND_CODE_ACTIONS.md`

---

## Next Steps for Builders

1. **Phase 1 Builder**: Take Task 1a-d (4 PRs or 1 bundled)
   - Prerequisite: Read diagnostics.rs and analyzer.rs
   - Acceptance: Perlcritic violations appear in diagnostics when external perlcritic installed

2. **Phase 2 Builder**: Take Task 2a-d (config + discovery)

3. **Phase 3 Builder**: Take Task 3a-d (performance + UX)

---

## Questions for Steering

1. **Sync vs Async**: Is 50-100ms per-file analysis acceptable, or should we go async from the start?
2. **Default Enabled**: Should external perlcritic be on by default (if installed) or opt-in?
3. **Priority**: Is this higher priority than other parser error fixes (corpus ratchet)?
