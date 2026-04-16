# Specification Context for #4430 — Wave H Microcrate Collapse (perl-dap-* → perl-dap)

## Orchestrator-Locked Decisions

These are FINAL and do not re-debate. From plan-review comment on issue #4430.

### Layout & Module Organization
- **Target owner**: `perl-dap` (existing, published crate)
- **Layout**: FLAT (not hierarchical grouping) — one `src/<module>/mod.rs` per absorbed satellite
- **Module names** (canonical, match satellite names without `perl-dap-` prefix):
  - `breakpoint`, `eval`, `config`, `platform`, `command_args`, `shell`, `stack`, `types`, `value`, `security`, `variables`
- **`api.rs`**: explicit named re-exports only, NO wildcard `pub use module::*;`
- **Existing modules**: `configuration.rs` (LaunchConfiguration/AttachConfiguration) is NOT the same as new `config/` (from perl-dap-config); both coexist

### Test File Strategy
- **Collision prevention**: prefix each test file with its source module name (Wave 1 pattern)
- **Explicit registration**: `perl-dap/Cargo.toml` has 23 existing `[[test]]` sections; add 28 new ones (total: 51)
- **Location**: all 28 files go to `crates/perl-dap/tests/` (no collisions with existing 46 files confirmed)

### Dependency DAG (becomes module import order in lib.rs)
```
command_args  (no deps)
platform      <- command_args
shell         <- platform + command_args
value         (no deps)
variables     <- value
```
Module declarations in `lib.rs` must respect this order.

---

## The Five Traps (Builder Hazards)

### Trap 1: File-vs-Folder Conflict on `platform.rs` and `security.rs`

**Location**: `crates/perl-dap/src/platform.rs` and `crates/perl-dap/src/security.rs` exist as public module files.

**Problem**: Rust doesn't allow both a file `platform.rs` and folder `platform/` at the same level. Current structure:
```
lib.rs: pub mod platform;  // resolves to src/platform.rs
lib.rs: pub mod security;  // resolves to src/security.rs
```

**Remediation**: Convert files to folder modules BEFORE merging satellite content:
```bash
mv crates/perl-dap/src/platform.rs  crates/perl-dap/src/platform/mod.rs
mv crates/perl-dap/src/security.rs  crates/perl-dap/src/security/mod.rs
```

The `pub mod platform;` and `pub mod security;` declarations in `lib.rs` don't change — Rust auto-resolves folder form. Replace the `pub use perl_dap_platform::*` and `pub use perl_dap_security::*` re-exports inside each module with the actual satellite source code.

**Verification**: `cargo check -p perl-dap` after this step.

---

### Trap 2: Type Name Collision (`StackFrame`/`Source`)

**Location**: 
- `perl-dap/src/protocol.rs` defines `StackFrame`, `Source`, and `Variable` — these are aliased in lib.rs re-exports as `ProtocolStackFrame`, etc.
- `perl-dap-types/src/lib.rs` also defines `StackFrame`, `Source`, `Variable`
- `perl-dap/src/debug_adapter/mod.rs` imports `perl_dap_types::{Source, StackFrame, Variable}`

**Problem**: After collapse, both sets of definitions live in the same crate. Ambiguous import paths without qualification.

**Remediation**:
1. In incoming `src/types/mod.rs` (from `perl-dap-types`), keep type names as-is
2. Update `src/debug_adapter/mod.rs` to use fully-qualified paths: `use crate::types::{Source, StackFrame, Variable};`
3. Existing `protocol.rs` types keep their names; they're already aliased as `ProtocolStackFrame` in re-exports
4. Run `cargo check -p perl-dap` immediately to catch any remaining ambiguity

**Key**: The types do NOT collide if module-qualified. Verify import paths are explicit.

---

### Trap 3: Explicit `[[test]]` Sections Required

**Location**: `crates/perl-dap/Cargo.toml` lines 90-140 (approx.)

**Problem**: When `[[test]]` sections are explicitly declared in Cargo.toml, Cargo ONLY runs tests registered in those sections. Auto-discovery is disabled. Current `perl-dap` has 23 explicit sections; 28 new tests will be added.

**Remediation**: Add one `[[test]]` entry per test file from the mapping table in `checklist.md`. Format:
```toml
[[test]]
name = "breakpoint_breakpoint_tests"
path = "tests/breakpoint_breakpoint_tests.rs"
```

**Verification**: `cargo test -p perl-dap --list` shows 51 registered test targets after this step.

---

### Trap 4: Ten Internal Use-Path Updates

**Location**: Files inside `crates/perl-dap/src/` that import satellites directly.

**Files affected** (from plan-review Trap 4 table):
1. `src/breakpoints.rs`
2. `src/configuration.rs`
3. `src/debug_adapter/mod.rs` (5 imports)
4. `src/platform/mod.rs` (2 imports)
5. `src/security/mod.rs` (1 import)

**Remediation**: For each file, replace `use perl_dap_*::` with `use crate::<module>::`. Exact changes documented in the checklist.

**Verification**: `cargo build -p perl-dap` succeeds with no unresolved imports.

---

### Trap 5: Three Workspace Sections Require Editing

**Location**: Root `Cargo.toml`

**Sections**:
1. `[workspace] members` (lines 109-121): remove 11 `"crates/perl-dap-*"` entries
2. Publish allowlist (lines 237-250): remove 11 `"perl-dap-*"` entries (keep `"perl-dap"`)
3. `[workspace.dependencies]` (lines 344-356): remove 11 `perl-dap-* = { path = "...", version = "0.12.4" }` entries

**Remediation**: Bulk remove the lines per section.

**Verification**: `cargo xtask publish-closure` reports 112 workspace members (down from 123).

---

## External Consumer Migration

Two crates import `perl_dap_platform` directly and must be updated:

### crates/perl-lsp

**File**: `Cargo.toml`
- Remove: `perl-dap-platform = { workspace = true }`
- Add: `perl-dap = { workspace = true }` (if not already present)

**File**: `crates/perl-lsp/src/runtime/lifecycle/workspace.rs`
- Current: `use perl_dap_platform::{PerlInterpreterResult, find_perl_interpreter};`
- New: `use perl_dap::platform::{PerlInterpreterResult, find_perl_interpreter};`

### crates/perl-lsp-config

**File**: `Cargo.toml`
- Remove: `perl-dap-platform = { workspace = true }`
- Add: `perl-dap = { workspace = true }` (if not already present)

**File**: `crates/perl-lsp-config/src/lib.rs`
- Current: `use perl_dap_platform::resolve_perl_path_with_toolchain;`
- New: `use perl_dap::platform::resolve_perl_path_with_toolchain;`

---

## Crate Removal

After all code is migrated, remove the 11 satellite crate directories:

```bash
rm -rf crates/perl-dap-breakpoint/
rm -rf crates/perl-dap-eval/
rm -rf crates/perl-dap-config/
rm -rf crates/perl-dap-platform/
rm -rf crates/perl-dap-command-args/
rm -rf crates/perl-dap-shell/
rm -rf crates/perl-dap-stack/
rm -rf crates/perl-dap-types/
rm -rf crates/perl-dap-value/
rm -rf crates/perl-dap-security/
rm -rf crates/perl-dap-variables/
```

---

## Test File Mapping (28 files, prefixed to prevent collisions)

| Source crate | Original filename | New filename |
|---|---|---|
| perl-dap-breakpoint | `breakpoint_tests.rs` | `breakpoint_breakpoint_tests.rs` |
| perl-dap-breakpoint | `edge_case_tests.rs` | `breakpoint_edge_case_tests.rs` |
| perl-dap-breakpoint | `extended_unit_tests.rs` | `breakpoint_extended_unit_tests.rs` |
| perl-dap-eval | `extended_unit_tests.rs` | `eval_extended_unit_tests.rs` |
| perl-dap-eval | `safe_evaluator.rs` | `eval_safe_evaluator.rs` |
| perl-dap-eval | `timeout_and_exception_tests.rs` | `eval_timeout_and_exception_tests.rs` |
| perl-dap-config | `attach_config_tests.rs` | `config_attach_config_tests.rs` |
| perl-dap-config | `launch_config_tests.rs` | `config_launch_config_tests.rs` |
| perl-dap-config | `serde_edge_case_tests.rs` | `config_serde_edge_case_tests.rs` |
| perl-dap-platform | `comprehensive_unit_tests.rs` | `platform_comprehensive_unit_tests.rs` |
| perl-dap-platform | `perl_path_edge_cases.rs` | `platform_perl_path_edge_cases.rs` |
| perl-dap-command-args | `integration_tests.rs` | `command_args_integration_tests.rs` |
| perl-dap-shell | `integration_tests.rs` | `shell_integration_tests.rs` |
| perl-dap-stack | `comprehensive_unit_tests.rs` | `stack_comprehensive_unit_tests.rs` |
| perl-dap-stack | `extended_unit_tests.rs` | `stack_extended_unit_tests.rs` |
| perl-dap-stack | `malformed_debugger_output_tests.rs` | `stack_malformed_debugger_output_tests.rs` |
| perl-dap-types | `edge_case_tests.rs` | `types_edge_case_tests.rs` |
| perl-dap-types | `shared_types.rs` | `types_shared_types.rs` |
| perl-dap-value | `integration_tests.rs` | `value_integration_tests.rs` |
| perl-dap-value | `serde_round_trip_tests.rs` | `value_serde_round_trip_tests.rs` |
| perl-dap-security | `dap_path_traversal_hardened_tests.rs` | `security_dap_path_traversal_hardened_tests.rs` |
| perl-dap-security | `dap_security_ac16_tests.rs` | `security_dap_security_ac16_tests.rs` |
| perl-dap-security | `path_traversal_tests.rs` | `security_path_traversal_tests.rs` |
| perl-dap-variables | `comprehensive.rs` | `variables_comprehensive.rs` |
| perl-dap-variables | `dap_deep_structure_truncation.rs` | `variables_dap_deep_structure_truncation.rs` |
| perl-dap-variables | `deep_truncation.rs` | `variables_deep_truncation.rs` |
| perl-dap-variables | `extended_unit_tests.rs` | `variables_extended_unit_tests.rs` |
| perl-dap-variables | `variable_inspection.rs` | `variables_variable_inspection.rs` |

---

## Key Decisions & Rationale

1. **Flat layout over hierarchy**: Matches Wave 1 precedent; simpler to navigate; avoids deep nesting.
2. **Explicit re-exports in api.rs**: Forces visibility boundaries; prevents accidental public API surface from internal re-exports.
3. **Prefix test names**: Wave 1 pattern; avoids collision surprises when tests are auto-discovered in parent.
4. **File-to-folder conversion first**: Unblocks satellite integration; prevents compilation errors from overlapping paths.
5. **DAG-respecting module order**: Matches dependency graph; prevents forward dependencies.

---

## Scope Notes

- **No internal refactoring**: Copy satellite source as-is; only update cross-crate imports
- **11 crate removal**: Happens at end after all tests pass
- **Backwards-compatible public API**: Consumer code imports from `perl_dap::platform::*` instead of `perl_dap_platform::*`
- **No feature flag changes**: All 11 satellites are published; DAP phases (dap-phase1, dap-phase2) are in `perl-dap` only
