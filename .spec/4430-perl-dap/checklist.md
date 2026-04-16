# Implementation Checklist for #4430 — Wave H Microcrate Collapse

## Overview

- **Total steps**: 48
- **Critical gates**: Trap 1 (file-to-folder), Trap 3 (test registration), Trap 4 (use-path updates), Trap 5 (workspace cleanup)
- **Test targets**: 51 registered (23 existing + 28 new)
- **External consumers**: 2 (perl-lsp, perl-lsp-config)

---

## Phase 1: Setup & File Structure Conversion (Steps 1-5)

### Step 1: Convert `platform.rs` to `platform/mod.rs`

**File**: `crates/perl-dap/src/platform.rs` → `crates/perl-dap/src/platform/mod.rs`

**Action**: 
- Read current `platform.rs` content (441 bytes, contains re-exports from `perl_dap_platform`)
- Create folder `crates/perl-dap/src/platform/`
- Move file to `crates/perl-dap/src/platform/mod.rs`
- Keep the `pub mod platform;` declaration in `lib.rs` unchanged (Rust auto-resolves)

**Verification**: `cargo check -p perl-dap` succeeds

---

### Step 2: Convert `security.rs` to `security/mod.rs`

**File**: `crates/perl-dap/src/security.rs` → `crates/perl-dap/src/security/mod.rs`

**Action**:
- Read current `security.rs` content (236 bytes, contains re-exports from `perl_dap_security`)
- Create folder `crates/perl-dap/src/security/`
- Move file to `crates/perl-dap/src/security/mod.rs`
- Keep the `pub mod security;` declaration in `lib.rs` unchanged

**Verification**: `cargo check -p perl-dap` succeeds

---

### Step 3: Create 11 module folders and add module declarations to lib.rs

**Files affected**: 
- `crates/perl-dap/src/lib.rs`
- Create: `crates/perl-dap/src/breakpoint/`
- Create: `crates/perl-dap/src/eval/`
- Create: `crates/perl-dap/src/config/`
- Create: `crates/perl-dap/src/command_args/`
- Create: `crates/perl-dap/src/shell/`
- Create: `crates/perl-dap/src/stack/`
- Create: `crates/perl-dap/src/types/`
- Create: `crates/perl-dap/src/value/`
- Create: `crates/perl-dap/src/variables/`

**Action**:
- Create 9 empty folders (platform and security already exist after Steps 1-2)
- Add module declarations to `lib.rs` in dependency DAG order. Insert after line 398 (after `pub mod security;`):

```rust
pub mod breakpoint;
pub mod eval;
pub mod config;
pub mod command_args;
pub mod platform;    // already exists, but re-order here if needed
pub mod shell;
pub mod stack;
pub mod types;
pub mod value;
pub mod security;    // already exists, but re-order here if needed
pub mod variables;
```

The exact order matters for compilation — must respect the inter-satellite DAG documented in context.md. **Current lib.rs has these scattered; consolidate them.**

**Verification**: `cargo check -p perl-dap` succeeds (may still fail on imports, which follow next)

---

### Step 4: Copy satellite source into module folders

**Detailed action for each satellite**:

For each satellite crate `perl-dap-{MODULE}`:
1. Read `crates/perl-dap-{MODULE}/src/lib.rs`
2. Copy all public items (functions, structs, modules, etc.) into `crates/perl-dap/src/{MODULE}/mod.rs`
3. Update any `use` statements inside the copied code that reference the old crate name

**Satellites and their module folders**:
1. **perl-dap-breakpoint** → `src/breakpoint/mod.rs`
2. **perl-dap-eval** → `src/eval/mod.rs`
3. **perl-dap-config** → `src/config/mod.rs`
4. **perl-dap-platform** → merge into `src/platform/mod.rs` (replacing the old re-exports)
5. **perl-dap-command-args** → `src/command_args/mod.rs`
6. **perl-dap-shell** → `src/shell/mod.rs`
7. **perl-dap-stack** → `src/stack/mod.rs`
8. **perl-dap-types** → `src/types/mod.rs`
9. **perl-dap-value** → `src/value/mod.rs`
10. **perl-dap-security** → merge into `src/security/mod.rs` (replacing the old re-exports)
11. **perl-dap-variables** → `src/variables/mod.rs`

**Special handling for platform and security** (Trap 1 aftermath):
- `src/platform/mod.rs`: Already contains re-exports from `perl_dap_platform`; replace the entire file with the content from `perl-dap-platform/src/lib.rs`
- `src/security/mod.rs`: Already contains re-exports from `perl_dap_security`; replace the entire file with the content from `perl-dap-security/src/lib.rs`

**Internal dependencies within copied code**: If a satellite's lib.rs imports from another satellite, it must be updated at copy time. Example:
- `perl-dap-shell/src/lib.rs` contains `use perl_dap_platform::{...};` → becomes `use crate::platform::{...};`
- `perl-dap-variables/src/lib.rs` contains `use perl_dap_value::{...};` → becomes `use crate::value::{...};`

**Verification**: After each copy, `cargo check -p perl-dap` (will show unresolved imports from perl-dap itself, fixed in next phase)

---

### Step 5: Create `api.rs` with explicit re-exports

**File**: Create `crates/perl-dap/src/api.rs`

**Content**: Re-export key public items from all 11 modules using explicit named re-exports (no wildcards). Example structure:

```rust
// Re-exports from breakpoint module
pub use crate::breakpoint::{
    AstBreakpointValidator,
    BreakpointValidator,
    // ... other public types/functions
};

// Re-exports from eval module
pub use crate::eval::{
    SafeEvaluator,
    // ... other public items
};

// ... (repeat for all 11 modules)
```

**Add to lib.rs**: Include `pub mod api;` and optionally `pub use api::*;` if the API re-exports should be at the top level.

**Verification**: `cargo check -p perl-dap` — no new errors at this stage (just fixing existing ones)

---

## Phase 2: Use-Path Updates (Steps 6-15) — TRAP 4

Update all internal imports that reference satellites directly.

### Step 6: Update `src/breakpoints.rs` — 1 import

**File**: `crates/perl-dap/src/breakpoints.rs`

**Current line** (grep result confirms):
```rust
use perl_dap_breakpoint::{AstBreakpointValidator, BreakpointValidator};
```

**Change to**:
```rust
use crate::breakpoint::{AstBreakpointValidator, BreakpointValidator};
```

**Verification**: `cargo check -p perl-dap`

---

### Step 7: Update `src/configuration.rs` — 1 import

**File**: `crates/perl-dap/src/configuration.rs`

**Current** (find the exact line with grep):
```rust
pub use perl_dap_config::{...};
```

**Change to**:
```rust
pub use crate::config::{...};
```

The spread `{...}` should remain unchanged; only the path changes.

**Verification**: `cargo check -p perl-dap`

---

### Step 8: Update `src/debug_adapter/mod.rs` — 5 imports

**File**: `crates/perl-dap/src/debug_adapter/mod.rs`

**Imports to update**:

1. `use perl_dap_breakpoint::{AstBreakpointValidator, BreakpointValidator};`
   → `use crate::breakpoint::{AstBreakpointValidator, BreakpointValidator};`

2. `use perl_dap_eval::SafeEvaluator;`
   → `use crate::eval::SafeEvaluator;`

3. `use perl_dap_stack::{PerlStackParser, is_internal_frame_name_and_path};`
   → `use crate::stack::{PerlStackParser, is_internal_frame_name_and_path};`

4. `use perl_dap_types::{Source, StackFrame, Variable};`
   → `use crate::types::{Source, StackFrame, Variable};`
   
   **TRAP 2 NOTE**: Verify that this import doesn't collide with any existing `protocol.rs` types. If ambiguity remains, qualify as `crate::types::StackFrame` at call sites.

5. `use perl_dap_variables::{...};`
   → `use crate::variables::{...};`

**Verification**: `cargo check -p perl-dap` — should now show far fewer unresolved import errors

---

### Step 9: Update `src/platform/mod.rs` — 2 imports

**File**: `crates/perl-dap/src/platform/mod.rs` (converted to folder in Step 1)

**Current imports** (internal to platform module):
- Look for `use perl_dap_platform::...` or references to the old platform crate

After copying platform satellite source in Step 4, this module will contain the actual platform code. Check for:

1. `use perl_dap_command_args::format_command_args;`
   → `use crate::command_args::format_command_args;`

2. Any remaining cross-satellite imports should be updated to `use crate::<module>::...;`

**Verification**: `cargo check -p perl-dap`

---

### Step 10: Update `src/security/mod.rs` — 1 import

**File**: `crates/perl-dap/src/security/mod.rs` (converted to folder in Step 2)

**Current imports** (internal to security module):
- Check for any satellite imports and replace with `use crate::<module>::...;`

After copying security satellite source in Step 4, verify there are no remaining external satellite references.

**Verification**: `cargo check -p perl-dap`

---

### Step 11: Update `src/shell/mod.rs` — internal update during Step 4

**Note**: When copying `perl-dap-shell/src/lib.rs` into `src/shell/mod.rs` (Step 4), ensure the following imports are updated DURING the copy:

1. `use perl_dap_platform::{...};` → `use crate::platform::{...};`
2. `use perl_dap_command_args::...;` → `use crate::command_args::...;`

This is done in Step 4, but listed here for completeness.

**Verification**: `cargo check -p perl-dap`

---

### Step 12: Update `src/variables/mod.rs` — internal update during Step 4

**Note**: When copying `perl-dap-variables/src/lib.rs` into `src/variables/mod.rs` (Step 4), ensure:

1. `use perl_dap_value::{...};` → `use crate::value::{...};`

This is done in Step 4, but listed here for completeness.

**Verification**: `cargo check -p perl-dap`

---

### Step 13: Build perl-dap with no unresolved imports

**Verification**: `cargo build -p perl-dap --lib` succeeds

At this point, all internal perl-dap imports should resolve.

---

## Phase 3: External Consumer Migration (Steps 14-17) — TRAP 5 (partial)

Update the two external crates that import `perl_dap_platform` directly.

### Step 14: Update `crates/perl-lsp/Cargo.toml`

**File**: `crates/perl-lsp/Cargo.toml`

**Action**:
1. Find the line: `perl-dap-platform = { workspace = true }`
2. Remove it
3. If `perl-dap` is not already in the dependencies, add: `perl-dap = { workspace = true }`

**Verification**: `cargo tree -p perl-lsp | grep perl-dap` should show `perl-dap` only

---

### Step 15: Update `crates/perl-lsp/src/runtime/lifecycle/workspace.rs`

**File**: `crates/perl-lsp/src/runtime/lifecycle/workspace.rs`

**Current import**:
```rust
use perl_dap_platform::{PerlInterpreterResult, find_perl_interpreter};
```

**Change to**:
```rust
use perl_dap::platform::{PerlInterpreterResult, find_perl_interpreter};
```

**Verification**: `cargo check -p perl-lsp` succeeds

---

### Step 16: Update `crates/perl-lsp-config/Cargo.toml`

**File**: `crates/perl-lsp-config/Cargo.toml`

**Action**:
1. Find the line: `perl-dap-platform = { workspace = true }`
2. Remove it
3. If `perl-dap` is not already in the dependencies, add: `perl-dap = { workspace = true }`

**Verification**: `cargo tree -p perl-lsp-config | grep perl-dap` should show `perl-dap` only

---

### Step 17: Update `crates/perl-lsp-config/src/lib.rs`

**File**: `crates/perl-lsp-config/src/lib.rs`

**Current import**:
```rust
use perl_dap_platform::resolve_perl_path_with_toolchain;
```

**Change to**:
```rust
use perl_dap::platform::resolve_perl_path_with_toolchain;
```

**Verification**: `cargo check -p perl-lsp-config` succeeds

---

## Phase 4: Test File Migration (Steps 18-45) — TRAP 3

Copy and register all 28 test files with prefixed names.

### Step 18-45: Copy test files (28 files, one per step)

For each test file in the mapping table (from context.md):

**Action template** (apply to each):
1. Read `crates/perl-dap-{SOURCE}/tests/{ORIGINAL_FILENAME}`
2. Write to `crates/perl-dap/tests/{PREFIXED_FILENAME}` with no modifications
3. Add `[[test]]` entry to `crates/perl-dap/Cargo.toml`

**Test file list** (28 total):

1. `breakpoint/breakpoint_tests.rs` → `tests/breakpoint_breakpoint_tests.rs`
2. `breakpoint/edge_case_tests.rs` → `tests/breakpoint_edge_case_tests.rs`
3. `breakpoint/extended_unit_tests.rs` → `tests/breakpoint_extended_unit_tests.rs`
4. `eval/extended_unit_tests.rs` → `tests/eval_extended_unit_tests.rs`
5. `eval/safe_evaluator.rs` → `tests/eval_safe_evaluator.rs`
6. `eval/timeout_and_exception_tests.rs` → `tests/eval_timeout_and_exception_tests.rs`
7. `config/attach_config_tests.rs` → `tests/config_attach_config_tests.rs`
8. `config/launch_config_tests.rs` → `tests/config_launch_config_tests.rs`
9. `config/serde_edge_case_tests.rs` → `tests/config_serde_edge_case_tests.rs`
10. `platform/comprehensive_unit_tests.rs` → `tests/platform_comprehensive_unit_tests.rs`
11. `platform/perl_path_edge_cases.rs` → `tests/platform_perl_path_edge_cases.rs`
12. `command-args/integration_tests.rs` → `tests/command_args_integration_tests.rs`
13. `shell/integration_tests.rs` → `tests/shell_integration_tests.rs`
14. `stack/comprehensive_unit_tests.rs` → `tests/stack_comprehensive_unit_tests.rs`
15. `stack/extended_unit_tests.rs` → `tests/stack_extended_unit_tests.rs`
16. `stack/malformed_debugger_output_tests.rs` → `tests/stack_malformed_debugger_output_tests.rs`
17. `types/edge_case_tests.rs` → `tests/types_edge_case_tests.rs`
18. `types/shared_types.rs` → `tests/types_shared_types.rs`
19. `value/integration_tests.rs` → `tests/value_integration_tests.rs`
20. `value/serde_round_trip_tests.rs` → `tests/value_serde_round_trip_tests.rs`
21. `security/dap_path_traversal_hardened_tests.rs` → `tests/security_dap_path_traversal_hardened_tests.rs`
22. `security/dap_security_ac16_tests.rs` → `tests/security_dap_security_ac16_tests.rs`
23. `security/path_traversal_tests.rs` → `tests/security_path_traversal_tests.rs`
24. `variables/comprehensive.rs` → `tests/variables_comprehensive.rs`
25. `variables/dap_deep_structure_truncation.rs` → `tests/variables_dap_deep_structure_truncation.rs`
26. `variables/deep_truncation.rs` → `tests/variables_deep_truncation.rs`
27. `variables/extended_unit_tests.rs` → `tests/variables_extended_unit_tests.rs`
28. `variables/variable_inspection.rs` → `tests/variables_variable_inspection.rs`

**Verification**: All files copied, zero collision conflicts with existing 46 test files

---

### Step 46: Register all 28 test files in Cargo.toml

**File**: `crates/perl-dap/Cargo.toml`

**Action**: Add the following `[[test]]` sections (after the existing 23, which end around line 140):

```toml
[[test]]
name = "breakpoint_breakpoint_tests"
path = "tests/breakpoint_breakpoint_tests.rs"

[[test]]
name = "breakpoint_edge_case_tests"
path = "tests/breakpoint_edge_case_tests.rs"

[[test]]
name = "breakpoint_extended_unit_tests"
path = "tests/breakpoint_extended_unit_tests.rs"

[[test]]
name = "eval_extended_unit_tests"
path = "tests/eval_extended_unit_tests.rs"

[[test]]
name = "eval_safe_evaluator"
path = "tests/eval_safe_evaluator.rs"

[[test]]
name = "eval_timeout_and_exception_tests"
path = "tests/eval_timeout_and_exception_tests.rs"

[[test]]
name = "config_attach_config_tests"
path = "tests/config_attach_config_tests.rs"

[[test]]
name = "config_launch_config_tests"
path = "tests/config_launch_config_tests.rs"

[[test]]
name = "config_serde_edge_case_tests"
path = "tests/config_serde_edge_case_tests.rs"

[[test]]
name = "platform_comprehensive_unit_tests"
path = "tests/platform_comprehensive_unit_tests.rs"

[[test]]
name = "platform_perl_path_edge_cases"
path = "tests/platform_perl_path_edge_cases.rs"

[[test]]
name = "command_args_integration_tests"
path = "tests/command_args_integration_tests.rs"

[[test]]
name = "shell_integration_tests"
path = "tests/shell_integration_tests.rs"

[[test]]
name = "stack_comprehensive_unit_tests"
path = "tests/stack_comprehensive_unit_tests.rs"

[[test]]
name = "stack_extended_unit_tests"
path = "tests/stack_extended_unit_tests.rs"

[[test]]
name = "stack_malformed_debugger_output_tests"
path = "tests/stack_malformed_debugger_output_tests.rs"

[[test]]
name = "types_edge_case_tests"
path = "tests/types_edge_case_tests.rs"

[[test]]
name = "types_shared_types"
path = "tests/types_shared_types.rs"

[[test]]
name = "value_integration_tests"
path = "tests/value_integration_tests.rs"

[[test]]
name = "value_serde_round_trip_tests"
path = "tests/value_serde_round_trip_tests.rs"

[[test]]
name = "security_dap_path_traversal_hardened_tests"
path = "tests/security_dap_path_traversal_hardened_tests.rs"

[[test]]
name = "security_dap_security_ac16_tests"
path = "tests/security_dap_security_ac16_tests.rs"

[[test]]
name = "security_path_traversal_tests"
path = "tests/security_path_traversal_tests.rs"

[[test]]
name = "variables_comprehensive"
path = "tests/variables_comprehensive.rs"

[[test]]
name = "variables_dap_deep_structure_truncation"
path = "tests/variables_dap_deep_structure_truncation.rs"

[[test]]
name = "variables_deep_truncation"
path = "tests/variables_deep_truncation.rs"

[[test]]
name = "variables_extended_unit_tests"
path = "tests/variables_extended_unit_tests.rs"

[[test]]
name = "variables_variable_inspection"
path = "tests/variables_variable_inspection.rs"
```

**Verification**: `cargo test -p perl-dap --list` shows 51 test targets (23 existing + 28 new)

---

## Phase 5: Workspace Cleanup (Steps 47-48) — TRAP 5 (final)

### Step 47: Clean workspace `Cargo.toml` — three sections

**File**: Root `Cargo.toml`

**Section 1: `[workspace] members`** (lines ~109-121)

**Current** (11 lines to remove):
```toml
    "crates/perl-dap-breakpoint",
    "crates/perl-dap-eval",
    "crates/perl-dap-config",
    "crates/perl-dap-platform",
    "crates/perl-dap-command-args",
    "crates/perl-dap-shell",
    "crates/perl-dap-stack",
    "crates/perl-dap-types",
    "crates/perl-dap-value",
    "crates/perl-dap-security",
    "crates/perl-dap-variables",
```

**Action**: Remove all 11 lines. Keep `"crates/perl-dap"`.

**Section 2: Publish allowlist** (lines ~237-250)

**Current** (11 lines to remove):
```toml
    "perl-dap-breakpoint",
    "perl-dap-eval",
    "perl-dap-config",
    "perl-dap-platform",
    "perl-dap-command-args",
    "perl-dap-shell",
    "perl-dap-stack",
    "perl-dap-types",
    "perl-dap-value",
    "perl-dap-security",
    "perl-dap-variables",
```

**Action**: Remove all 11 lines. Keep `"perl-dap"`.

**Section 3: `[workspace.dependencies]`** (lines ~344-356)

**Current** (11 lines to remove):
```toml
perl-dap-breakpoint = { path = "crates/perl-dap-breakpoint", version = "0.12.4" }
perl-dap-eval = { path = "crates/perl-dap-eval", version = "0.12.4" }
perl-dap-config = { path = "crates/perl-dap-config", version = "0.12.4" }
perl-dap-platform = { path = "crates/perl-dap-platform", version = "0.12.4" }
perl-dap-command-args = { path = "crates/perl-dap-command-args", version = "0.12.4" }
perl-dap-shell = { path = "crates/perl-dap-shell", version = "0.12.4" }
perl-dap-stack = { path = "crates/perl-dap-stack", version = "0.12.4" }
perl-dap-types = { path = "crates/perl-dap-types", version = "0.12.4" }
perl-dap-value = { path = "crates/perl-dap-value", version = "0.12.4" }
perl-dap-security = { path = "crates/perl-dap-security", version = "0.12.4" }
perl-dap-variables = { path = "crates/perl-dap-variables", version = "0.12.4" }
```

**Action**: Remove all 11 lines.

**Verification**: `cargo build -p perl-dap --lib` succeeds

---

### Step 48: Remove satellite crate directories

**Action**:
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

**Verification**: 
- `cargo metadata --no-deps | grep "\"name\":\"perl-dap" | wc -l` shows 1 (only perl-dap, no satellites)
- `cargo xtask publish-closure` reports 112 workspace members (down from 123)

---

## Final Verification (post-implementation)

Run these in order to confirm the collapse is complete:

```bash
cargo build -p perl-dap --lib
cargo test -p perl-dap
cargo build -p perl-lsp --release
cargo build -p perl-lsp-config --release
cargo xtask publish-closure
cargo clippy -p perl-dap
cargo xtask fmt
```

**Expected outcomes**:
- `cargo test -p perl-dap` runs 51 tests (51 pass)
- `cargo xtask publish-closure` reports 112 published crates (down from 123)
- `cargo clippy -p perl-dap` returns zero warnings
- All binaries build successfully

---

## Notes for TDD Builder

1. **Don't skip Trap 1**: Converting `platform.rs` and `security.rs` to folders MUST happen before copying satellite content. Failing this creates file-vs-folder conflicts.

2. **Module order matters (Trap 4 part 2)**: lib.rs declarations must respect the DAG:
   - `command_args` first (no deps)
   - `platform` after (depends on command_args)
   - `shell` after (depends on platform + command_args)
   - `value` before `variables`

3. **Test registration is mandatory (Trap 3)**: Without explicit `[[test]]` entries in Cargo.toml, the 28 new test files won't run. This is a hard requirement, not auto-discovery.

4. **Type qualification (Trap 2)**: If `cargo check` shows type ambiguity errors between `protocol.rs` and `types/mod.rs`, qualify with `crate::types::` at call sites, not in imports.

5. **External consumer migration**: Don't forget `perl-lsp-config`. Both `perl-lsp` and `perl-lsp-config` depend on `perl-dap-platform` and must be updated.
