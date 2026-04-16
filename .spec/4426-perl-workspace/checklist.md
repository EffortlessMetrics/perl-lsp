# Implementation Checklist: Wave A Microcrate Collapse

**Branch**: `impl/4426-perl-workspace`
**Target**: Rename `perl-workspace-index` → `perl-workspace`; absorb 6 satellites into flat module structure
**Test counts**: 15 test files total (discovery=6, folder=4, monitoring=1, slo=3, state_machine=1, existing index=8)
**Crate touch**: perl-workspace-index (core), 8 consumers (perl-lsp, perl-module, perl-parser, perl-semantic-analyzer, perl-refactoring, perl-dead-code, perl-lsp-completion, perl-lsp-diagnostics), perl-ci-hygiene (1 line)
**Imports to update**: ~131 occurrences across 8+ source files

---

## Phase 1: Crate Rename & Initial Structure

### Step 1.1: Rename package in `crates/perl-workspace-index/Cargo.toml`

**File**: `/h/Code/Rust/perl-lsp/crates/perl-workspace-index/Cargo.toml`

**Changes**:
- Line 2: `name = "perl-workspace-index"` → `name = "perl-workspace"`
- Line 7: `description = "Workspace indexing and refactoring orchestration for Perl"` → `description = "Workspace file discovery, indexing, and observability for Perl"`
- Line 12: `documentation = "https://docs.rs/perl-workspace-index"` → `documentation = "https://docs.rs/perl-workspace"`

**Why first**: Cargo needs the package name updated before any crate references will resolve correctly.

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo build -p perl-workspace 2>&1 | head -20
```

---

## Phase 2: Absorb Satellite Content into Folder-Modules

### Step 2.1: Create `src/discovery/mod.rs` from `perl-workspace-discovery/src/lib.rs`

**File**: `/h/Code/Rust/perl-lsp/crates/perl-workspace-index/src/discovery/mod.rs`

**Action**: Copy complete content from `crates/perl-workspace-discovery/src/lib.rs` to new file.

**Dependencies**: 
- `perl-workspace-ignore` becomes `crate::ignore::` (internal)
- All other deps (walkdir, perl-source-file, tracing) are added to perl-workspace-index/Cargo.toml in Step 3.1

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo check -p perl-workspace 2>&1 | grep -E "error|warning.*discovery" | head -10
```

---

### Step 2.2: Create `src/folder/mod.rs` from `perl-workspace-folder/src/lib.rs`

**File**: `/h/Code/Rust/perl-lsp/crates/perl-workspace-index/src/folder/mod.rs`

**Action**: Copy complete content from `crates/perl-workspace-folder/src/lib.rs` to new file.

**Dependencies**: All external deps already in perl-workspace-index (check Step 3.1)

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo check -p perl-workspace 2>&1 | grep -E "error|warning.*folder" | head -10
```

---

### Step 2.3: Create `src/ignore/mod.rs` from `perl-workspace-ignore/src/lib.rs`

**File**: `/h/Code/Rust/perl-lsp/crates/perl-workspace-index/src/ignore/mod.rs`

**Action**: Copy complete content from `crates/perl-workspace-ignore/src/lib.rs` to new file.

**Dependencies**: None (standalone)

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo check -p perl-workspace 2>&1 | grep -E "error|warning.*ignore" | head -10
```

---

### Step 2.4: Create `src/monitoring/mod.rs` from `perl-workspace-index-monitoring/src/lib.rs`

**File**: `/h/Code/Rust/perl-lsp/crates/perl-workspace-index/src/monitoring/mod.rs`

**Action**: Copy complete content from `crates/perl-workspace-index-monitoring/src/lib.rs` to new file.

**Remove from perl-workspace-index**: Delete the old thin wrapper at `src/workspace/monitoring.rs` (but keep a re-export in `src/workspace/mod.rs` per Step 4.2)

**Dependencies**: Check during Step 3.1

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo check -p perl-workspace 2>&1 | grep -E "error|warning.*monitoring" | head -10
```

---

### Step 2.5: Create `src/slo/mod.rs` from `perl-workspace-index-slo/src/lib.rs`

**File**: `/h/Code/Rust/perl-lsp/crates/perl-workspace-index/src/slo/mod.rs`

**Action**: Copy complete content from `crates/perl-workspace-index-slo/src/lib.rs` to new file.

**Remove from perl-workspace-index**: Delete the old thin wrapper at `src/workspace/slo.rs` (but keep a re-export in `src/workspace/mod.rs` per Step 4.2)

**Dependencies**: Check during Step 3.1

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo check -p perl-workspace 2>&1 | grep -E "error|warning.*slo" | head -10
```

---

### Step 2.6: Create `src/state_machine/mod.rs` from `perl-workspace-index-state-machine/src/lib.rs`

**File**: `/h/Code/Rust/perl-lsp/crates/perl-workspace-index/src/state_machine/mod.rs`

**Action**: Copy complete content from `crates/perl-workspace-index-state-machine/src/lib.rs` to new file.

**Remove from perl-workspace-index**: Delete the old thin wrapper at `src/workspace/state_machine.rs` (but keep a re-export in `src/workspace/mod.rs` per Step 4.2)

**Dependencies**: Check during Step 3.1

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo check -p perl-workspace 2>&1 | grep -E "error|warning.*state_machine" | head -10
```

---

## Phase 3: Update Core perl-workspace-index Structure

### Step 3.1: Update `crates/perl-workspace-index/Cargo.toml` dependencies

**File**: `/h/Code/Rust/perl-lsp/crates/perl-workspace-index/Cargo.toml`

**Changes**:

1. **Remove satellite dependencies** (lines 30-32):
   ```toml
   perl-workspace-index-monitoring = { workspace = true }
   perl-workspace-index-slo = { workspace = true }
   perl-workspace-index-state-machine = { workspace = true }
   ```
   Delete these 3 lines.

2. **Add runtime dependencies** (if not already present):
   - `perl-source-file = { workspace = true }` (used by discovery)
   - `walkdir = { workspace = true }` (used by discovery)
   - `tracing = { workspace = true }` (used by discovery)
   - `serde_json = { workspace = true }` (used by folder, for LSP event parsing)

   Check current state: `grep -E "^(perl-source-file|walkdir|tracing|serde_json)" crates/perl-workspace-index/Cargo.toml`

   If missing, add to `[dependencies]` section.

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo build -p perl-workspace 2>&1 | head -20
```

---

### Step 3.2: Update `crates/perl-workspace-index/src/lib.rs` module declarations

**File**: `/h/Code/Rust/perl-lsp/crates/perl-workspace-index/src/lib.rs`

**Action**: Add module declarations for the 6 new folders (in addition to existing modules like `pub mod workspace;`):

```rust
pub mod discovery;
pub mod folder;
pub mod ignore;
pub mod monitoring;
pub mod slo;
pub mod state_machine;
```

Insert these near the top of the crate root, after any `#![...]` attributes and before existing `pub mod workspace;`.

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo check -p perl-workspace 2>&1 | head -20
```

---

### Step 3.3: Create `crates/perl-workspace-index/src/api.rs` with explicit re-exports

**File**: `/h/Code/Rust/perl-lsp/crates/perl-workspace-index/src/api.rs`

**Action**: Create new file with explicit (non-wildcard) re-exports:

```rust
//! Public API re-exports for perl-workspace modules.
//! 
//! This module uses explicit named re-exports (no wildcards) to avoid
//! type name conflicts between observability satellites. Enumeration
//! module public APIs are re-exported here; observability modules
//! are accessed via qualified paths (e.g., `perl_workspace::monitoring::IndexStateKind`).

// Enumeration satellite public APIs
pub use crate::discovery::{
    DiscoveryMethod, DiscoveryResult, discover_perl_files, is_perl_discovery_path,
};
pub use crate::folder::{
    WorkspaceFolderChange, workspace_folder_to_path, extract_workspace_folder_uris,
    extract_workspace_folder_change, root_path_to_file_uri,
};
pub use crate::ignore::{
    is_skipped_dir_name, path_contains_skipped_component,
};

// NOTE: Observability satellites (monitoring, slo, state_machine) are NOT
// re-exported here due to type name conflicts (IndexStateKind, etc. defined
// in both monitoring and state_machine). Use qualified paths instead:
//   use perl_workspace::monitoring::IndexStateKind;
//   use perl_workspace::slo::SloTracker;
//   use perl_workspace::state_machine::IndexStateMachine;
```

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo check -p perl-workspace 2>&1 | head -10
```

---

### Step 3.4: Update `crates/perl-workspace-index/src/workspace/mod.rs` for backward compatibility

**File**: `/h/Code/Rust/perl-lsp/crates/perl-workspace-index/src/workspace/mod.rs`

**Action**: Update the workspace module to re-export observability satellites with explicit named re-exports (not wildcards):

Current state likely has:
```rust
pub use perl_workspace_index_monitoring::*;
pub use perl_workspace_index_slo::*;
pub use perl_workspace_index_state_machine::*;
```

Replace with explicit re-exports to preserve paths like `perl_workspace::workspace::monitoring::IndexPhase`:

```rust
// Re-export observability satellites for backward compatibility.
// Existing code using perl_workspace::workspace::monitoring::* paths must continue to work.

pub use crate::monitoring::{
    IndexPhase, IndexStateTransition, DegradationReason, ResourceKind,
    // ... other types from monitoring module
};

pub use crate::slo::{
    SloTracker, OperationType,
    // ... other types from slo module
};

pub use crate::state_machine::{
    IndexStateMachine, IndexStateKind,
    // ... other types from state_machine module
};
```

**Important**: Check the actual public API of each observability satellite by reading their lib.rs files first. Only re-export public types/functions (those marked `pub` at module level).

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo test -p perl-workspace --lib 2>&1 | grep -E "test result|error" | head -10
```

---

## Phase 4: Migrate Test Files

### Step 4.1: Migrate discovery tests (6 files)

**Source files** → **Destination files**:
- `crates/perl-workspace-discovery/tests/comprehensive_unit_tests.rs` → `/h/Code/Rust/perl-lsp/crates/perl-workspace-index/tests/discovery_comprehensive_unit_tests.rs` (RENAME)
- `crates/perl-workspace-discovery/tests/discovery_bdd_tests.rs` → `/h/Code/Rust/perl-lsp/crates/perl-workspace-index/tests/discovery_bdd_tests.rs`
- `crates/perl-workspace-discovery/tests/discovery_coverage_tests.rs` → `/h/Code/Rust/perl-lsp/crates/perl-workspace-index/tests/discovery_coverage_tests.rs`
- `crates/perl-workspace-discovery/tests/discovery_fuzz_tests.rs` → `/h/Code/Rust/perl-lsp/crates/perl-workspace-index/tests/discovery_fuzz_tests.rs`
- `crates/perl-workspace-discovery/tests/discovery_integration_tests.rs` → `/h/Code/Rust/perl-lsp/crates/perl-workspace-index/tests/discovery_integration_tests.rs`
- `crates/perl-workspace-discovery/tests/discovery_property_tests.rs` → `/h/Code/Rust/perl-lsp/crates/perl-workspace-index/tests/discovery_property_tests.rs`

**For each file**:
1. Copy content to new destination
2. Update import: `use perl_workspace_discovery::` → `use perl_workspace::discovery::`

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo test -p perl-workspace --test 'discovery_*' 2>&1 | grep -E "test result|error" | head -10
```

---

### Step 4.2: Migrate folder tests (4 files)

**Source files** → **Destination files**:
- `crates/perl-workspace-folder/tests/workspace_folder_bdd.rs` → `/h/Code/Rust/perl-lsp/crates/perl-workspace-index/tests/workspace_folder_bdd.rs`
- `crates/perl-workspace-folder/tests/workspace_folder_fuzz.rs` → `/h/Code/Rust/perl-lsp/crates/perl-workspace-index/tests/workspace_folder_fuzz.rs`
- `crates/perl-workspace-folder/tests/workspace_folder_integration.rs` → `/h/Code/Rust/perl-lsp/crates/perl-workspace-index/tests/workspace_folder_integration.rs`
- `crates/perl-workspace-folder/tests/workspace_folder_prop.rs` → `/h/Code/Rust/perl-lsp/crates/perl-workspace-index/tests/workspace_folder_prop.rs`

**For each file**:
1. Copy content to new destination
2. Update import: `use perl_workspace_folder::` → `use perl_workspace::folder::`

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo test -p perl-workspace --test 'workspace_folder_*' 2>&1 | grep -E "test result|error" | head -10
```

---

### Step 4.3: Migrate monitoring tests (1 file)

**Source files** → **Destination files**:
- `crates/perl-workspace-index-monitoring/tests/monitoring_behaviour.rs` → `/h/Code/Rust/perl-lsp/crates/perl-workspace-index/tests/monitoring_behaviour.rs`

**For the file**:
1. Copy content to new destination
2. Update import: `use perl_workspace_index_monitoring::` → `use perl_workspace::monitoring::`

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo test -p perl-workspace --test 'monitoring_behaviour' 2>&1 | grep -E "test result|error" | head -10
```

---

### Step 4.4: Migrate slo tests (3 files)

**Source files** → **Destination files**:
- `crates/perl-workspace-index-slo/tests/bdd.rs` → `/h/Code/Rust/perl-lsp/crates/perl-workspace-index/tests/slo_bdd.rs` (RENAME)
- `crates/perl-workspace-index-slo/tests/fuzz.rs` → `/h/Code/Rust/perl-lsp/crates/perl-workspace-index/tests/slo_fuzz.rs` (RENAME)
- `crates/perl-workspace-index-slo/tests/property.rs` → `/h/Code/Rust/perl-lsp/crates/perl-workspace-index/tests/slo_property.rs` (RENAME)

**For each file**:
1. Copy content to new destination with prefix
2. Update import: `use perl_workspace_index_slo::` → `use perl_workspace::slo::`

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo test -p perl-workspace --test 'slo_*' 2>&1 | grep -E "test result|error" | head -10
```

---

### Step 4.5: Migrate state_machine tests (1 file)

**Source files** → **Destination files**:
- `crates/perl-workspace-index-state-machine/tests/predicates.rs` → `/h/Code/Rust/perl-lsp/crates/perl-workspace-index/tests/state_machine_predicates.rs` (RENAME)

**For the file**:
1. Copy content to new destination with prefix
2. Update import: `use perl_workspace_index_state_machine::` → `use perl_workspace::state_machine::`

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo test -p perl-workspace --test 'state_machine_*' 2>&1 | grep -E "test result|error" | head -10
```

---

## Phase 5: Update Consumer Crates

### Step 5.1: Update `crates/perl-lsp/Cargo.toml`

**File**: `/h/Code/Rust/perl-lsp/crates/perl-lsp/Cargo.toml`

**Changes**:
- Remove: `perl-workspace-discovery = { workspace = true }`
- Remove: `perl-workspace-folder = { workspace = true }`
- Remove: `perl-workspace-ignore = { workspace = true }`
- Rename: `perl-workspace-index` → `perl-workspace`

**Before**:
```toml
perl-workspace-discovery = { workspace = true }
perl-workspace-folder = { workspace = true }
perl-workspace-ignore = { workspace = true }
perl-workspace-index = { workspace = true }
```

**After**:
```toml
perl-workspace = { workspace = true }
```

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo build -p perl-lsp-rs 2>&1 | head -20
```

---

### Step 5.2: Update `crates/perl-module/Cargo.toml`

**File**: `/h/Code/Rust/perl-lsp/crates/perl-module/Cargo.toml`

**Changes**:
- Remove: `perl-workspace-folder = { workspace = true }`
- Add (if not present): `perl-workspace = { workspace = true }`

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo build -p perl-module 2>&1 | head -20
```

---

### Step 5.3: Update `crates/perl-parser/Cargo.toml`

**File**: `/h/Code/Rust/perl-lsp/crates/perl-parser/Cargo.toml`

**Changes**:
- Rename: `perl-workspace-index` → `perl-workspace`
- If features include `perl-workspace-index/lsp-compat`, update to `perl-workspace/lsp-compat`
- If features include `perl-workspace-index/workspace`, update to `perl-workspace/workspace`

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo build -p perl-parser 2>&1 | head -20
```

---

### Step 5.4: Update `crates/perl-semantic-analyzer/Cargo.toml`

**File**: `/h/Code/Rust/perl-lsp/crates/perl-semantic-analyzer/Cargo.toml`

**Changes**:
- Rename: `perl-workspace-index` → `perl-workspace`

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo build -p perl-semantic-analyzer 2>&1 | head -20
```

---

### Step 5.5: Update `crates/perl-refactoring/Cargo.toml`

**File**: `/h/Code/Rust/perl-lsp/crates/perl-refactoring/Cargo.toml`

**Changes**:
- Rename: `perl-workspace-index` → `perl-workspace`

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo build -p perl-refactoring 2>&1 | head -20
```

---

### Step 5.6: Update `crates/perl-dead-code/Cargo.toml`

**File**: `/h/Code/Rust/perl-lsp/crates/perl-dead-code/Cargo.toml`

**Changes**:
- Rename: `perl-workspace-index` → `perl-workspace`

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo build -p perl-dead-code 2>&1 | head -20
```

---

### Step 5.7: Update `crates/perl-lsp-completion/Cargo.toml`

**File**: `/h/Code/Rust/perl-lsp/crates/perl-lsp-completion/Cargo.toml`

**Changes**:
- If `perl-workspace-index` appears twice, keep only one renamed instance
- Rename: `perl-workspace-index` → `perl-workspace` (all instances)

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo build -p perl-lsp-completion 2>&1 | head -20
```

---

### Step 5.8: Update `crates/perl-lsp-diagnostics/Cargo.toml`

**File**: `/h/Code/Rust/perl-lsp/crates/perl-lsp-diagnostics/Cargo.toml`

**Changes**:
- Rename: `perl-workspace-index` → `perl-workspace`

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo build -p perl-lsp-diagnostics 2>&1 | head -20
```

---

## Phase 6: Update Workspace Root

### Step 6.1: Update `Cargo.toml` workspace dependencies

**File**: `/h/Code/Rust/perl-lsp/Cargo.toml`

**Location**: `[workspace.dependencies]` section

**Changes**:
1. Rename key `perl-workspace-index` → `perl-workspace` (path stays `crates/perl-workspace-index`)
2. Delete 6 satellite keys: `perl-workspace-discovery`, `perl-workspace-folder`, `perl-workspace-ignore`, `perl-workspace-index-monitoring`, `perl-workspace-index-slo`, `perl-workspace-index-state-machine`

**Before**:
```toml
perl-workspace-discovery = { path = "crates/perl-workspace-discovery" }
perl-workspace-folder = { path = "crates/perl-workspace-folder" }
perl-workspace-ignore = { path = "crates/perl-workspace-ignore" }
perl-workspace-index = { path = "crates/perl-workspace-index" }
perl-workspace-index-monitoring = { path = "crates/perl-workspace-index-monitoring" }
perl-workspace-index-slo = { path = "crates/perl-workspace-index-slo" }
perl-workspace-index-state-machine = { path = "crates/perl-workspace-index-state-machine" }
```

**After**:
```toml
perl-workspace = { path = "crates/perl-workspace-index" }
```

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo metadata --no-deps --format-version 1 | head -20
```

---

### Step 6.2: Update workspace publish allowlist

**File**: `/h/Code/Rust/perl-lsp/Cargo.toml`

**Location**: `[workspace.metadata.publish].allow` section (array of strings)

**Changes**:
1. Rename entry: `"perl-workspace-index"` → `"perl-workspace"`
2. Remove 6 entries: `"perl-workspace-discovery"`, `"perl-workspace-folder"`, `"perl-workspace-ignore"`, `"perl-workspace-index-monitoring"`, `"perl-workspace-index-slo"`, `"perl-workspace-index-state-machine"`

**Count check**: Before = 120 entries; After = 114 entries (120 - 6 + 1 = 115... wait, let me recalculate: 120 items, remove 6 satellites = 114, rename 1 = still 114. But the spec says 120 - 6 - 1 + 1 = 114. Let me verify builder checks this with `cargo xtask publish-closure`.)

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo metadata --no-deps --format-version 1 | jq '.workspace_metadata.publish.allow | length'
```

---

## Phase 7: Update Source Imports (131 occurrences)

### Step 7.1: Update perl-lsp source files

**Files to update**:
- `/h/Code/Rust/perl-lsp/crates/perl-lsp/src/runtime/file_discovery.rs`
- `/h/Code/Rust/perl-lsp/crates/perl-lsp/src/runtime/lifecycle/capabilities.rs`
- `/h/Code/Rust/perl-lsp/crates/perl-lsp/src/runtime/workspace.rs`

**Changes** (use find-and-replace or manual edit):
- `use perl_workspace_index::` → `use perl_workspace::`
- `use perl_workspace_discovery::` → `use perl_workspace::discovery::`
- `use perl_workspace_folder::` → `use perl_workspace::folder::`
- `use perl_workspace_ignore::` → `use perl_workspace::ignore::`

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo build -p perl-lsp-rs 2>&1 | grep "error.*workspace" | head -10
```

---

### Step 7.2: Update perl-module source files

**Files to update**:
- `/h/Code/Rust/perl-lsp/crates/perl-module/src/resolution/uri.rs`

**Changes**:
- `use perl_workspace_folder::` → `use perl_workspace::folder::`

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo build -p perl-module 2>&1 | grep "error.*workspace" | head -10
```

---

### Step 7.3: Update perl-parser source files

**Files to update**:
- `/h/Code/Rust/perl-lsp/crates/perl-parser/src/workspace.rs`

**Changes**:
- `use perl_workspace_index::` → `use perl_workspace::`

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo build -p perl-parser 2>&1 | grep "error.*workspace" | head -10
```

---

### Step 7.4: Update perl-semantic-analyzer source files

**Files to update**:
- `/h/Code/Rust/perl-lsp/crates/perl-semantic-analyzer/src/lib.rs`

**Changes**:
- `use perl_workspace_index::` → `use perl_workspace::`

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo build -p perl-semantic-analyzer 2>&1 | grep "error.*workspace" | head -10
```

---

### Step 7.5: Update perl-refactoring source files

**Files to update**:
- `/h/Code/Rust/perl-lsp/crates/perl-refactoring/src/lib.rs`
- `/h/Code/Rust/perl-lsp/crates/perl-refactoring/src/refactor/workspace_rename.rs`

**Changes**:
- `use perl_workspace_index::` → `use perl_workspace::`

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo build -p perl-refactoring 2>&1 | grep "error.*workspace" | head -10
```

---

### Step 7.6: Update perl-dead-code source files

**Files to update**:
- `/h/Code/Rust/perl-lsp/crates/perl-dead-code/src/lib.rs`
- `/h/Code/Rust/perl-lsp/crates/perl-dead-code/tests/*.rs` (if any import satellite crates)

**Changes**:
- `use perl_workspace_index::` → `use perl_workspace::`

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo build -p perl-dead-code 2>&1 | grep "error.*workspace" | head -10
```

---

### Step 7.7: Update perl-lsp-completion source files

**Files to update**:
- `/h/Code/Rust/perl-lsp/crates/perl-lsp-completion/src/**/*.rs` (all files)
- `/h/Code/Rust/perl-lsp/crates/perl-lsp-completion/benches/*.rs` (if any)
- `/h/Code/Rust/perl-lsp/crates/perl-lsp-completion/tests/*.rs` (if any)

**Changes**:
- `use perl_workspace_index::` → `use perl_workspace::`

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo build -p perl-lsp-completion 2>&1 | grep "error.*workspace" | head -10
```

---

### Step 7.8: Update perl-lsp-diagnostics source files

**Files to update**:
- `/h/Code/Rust/perl-lsp/crates/perl-lsp-diagnostics/src/**/*.rs` (all files)
- `/h/Code/Rust/perl-lsp/crates/perl-lsp-diagnostics/tests/*.rs` (if any)

**Changes**:
- `use perl_workspace_index::` → `use perl_workspace::`

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo build -p perl-lsp-diagnostics 2>&1 | grep "error.*workspace" | head -10
```

---

## Phase 8: Update Hardcoded Crate Name Strings

### Step 8.1: Update `crates/perl-ci-hygiene/src/main.rs`

**File**: `/h/Code/Rust/perl-lsp/crates/perl-ci-hygiene/src/main.rs`

**Location**: Line 4505 (verify with grep first)

**Change**: Hardcoded string `"perl-workspace-index"` → `"perl-workspace"`

**Command to find**:
```bash
grep -n 'perl-workspace-index' /h/Code/Rust/perl-lsp/crates/perl-ci-hygiene/src/main.rs
```

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo build -p perl-ci-hygiene 2>&1 | head -10
```

---

### Step 8.2: Update `crates/perl-parser/tests/missing_docs_ac_tests.rs`

**File**: `/h/Code/Rust/perl-lsp/crates/perl-parser/tests/missing_docs_ac_tests.rs`

**Location**: Lines 607-608 (verify with grep first)

**Change**: Hardcoded strings `name: "perl-workspace-index"` → `name: "perl-workspace"`

**Command to find**:
```bash
grep -n 'perl-workspace-index' /h/Code/Rust/perl-lsp/crates/perl-parser/tests/missing_docs_ac_tests.rs
```

**Verify**:
```bash
cd /h/Code/Rust/perl-lsp && cargo test -p perl-parser --test missing_docs_ac_tests 2>&1 | grep -E "test result|error" | head -10
```

---

## Phase 9: Delete Old Crate Directories

### Step 9.1: Delete 6 satellite crate directories

**Directories to remove**:
1. `/h/Code/Rust/perl-lsp/crates/perl-workspace-discovery/`
2. `/h/Code/Rust/perl-lsp/crates/perl-workspace-folder/`
3. `/h/Code/Rust/perl-lsp/crates/perl-workspace-ignore/`
4. `/h/Code/Rust/perl-lsp/crates/perl-workspace-index-monitoring/`
5. `/h/Code/Rust/perl-lsp/crates/perl-workspace-index-slo/`
6. `/h/Code/Rust/perl-lsp/crates/perl-workspace-index-state-machine/`

**Command**:
```bash
rm -rf /h/Code/Rust/perl-lsp/crates/perl-workspace-discovery \
       /h/Code/Rust/perl-lsp/crates/perl-workspace-folder \
       /h/Code/Rust/perl-lsp/crates/perl-workspace-ignore \
       /h/Code/Rust/perl-lsp/crates/perl-workspace-index-monitoring \
       /h/Code/Rust/perl-lsp/crates/perl-workspace-index-slo \
       /h/Code/Rust/perl-lsp/crates/perl-workspace-index-state-machine
```

**Verify**:
```bash
ls -d /h/Code/Rust/perl-lsp/crates/perl-workspace-* 2>&1 | grep -v "perl-workspace-index"
```
(Should return only `/h/Code/Rust/perl-lsp/crates/perl-workspace-index`)

---

## Phase 10: Final Verification

### Step 10.1: Verify no old crate names in source

**Command**:
```bash
grep -r 'perl_workspace_discovery\|perl_workspace_folder\|perl_workspace_ignore\|perl_workspace_index_monitoring\|perl_workspace_index_slo\|perl_workspace_index_state_machine' crates/ --include='*.rs' 2>&1 | grep -v "^Binary" || echo "OK: No old satellite names found"
```

**Expected**: "OK: No old satellite names found"

---

### Step 10.2: Verify workspace member count

**Command**:
```bash
cd /h/Code/Rust/perl-lsp && cargo metadata --no-deps --format-version 1 | python3 -c "import sys,json; d=json.load(sys.stdin); print('Members:', len(d['workspace_members']))"
```

**Expected**: Members: 117 (was 123, now -6)

---

### Step 10.3: Verify publish allowlist count

**Command**:
```bash
cd /h/Code/Rust/perl-lsp && cargo xtask publish-closure 2>&1 | tail -20
```

**Expected**: 114 allowed packages (old count was 120; removed 6, renamed 1 = 114)

---

### Step 10.4: Full test suite

**Command**:
```bash
cd /h/Code/Rust/perl-lsp && cargo test --workspace 2>&1 | tail -30
```

**Expected**: All tests pass (or only expected failures if any pre-exist)

---

### Step 10.5: Format and lint

**Command**:
```bash
cd /h/Code/Rust/perl-lsp && cargo xtask fmt 2>&1 | tail -10
```

**Command**:
```bash
cd /h/Code/Rust/perl-lsp && cargo clippy --workspace --lib 2>&1 | tail -20
```

**Expected**: No errors (warnings OK if pre-existing)

---

### Step 10.6: Build all 8 consumer crates

**Command**:
```bash
cd /h/Code/Rust/perl-lsp && \
  cargo build -p perl-lsp-rs && \
  cargo build -p perl-module && \
  cargo build -p perl-parser && \
  cargo build -p perl-semantic-analyzer && \
  cargo build -p perl-refactoring && \
  cargo build -p perl-dead-code && \
  cargo build -p perl-lsp-completion && \
  cargo build -p perl-lsp-diagnostics && \
  echo "All consumer crates built successfully"
```

**Expected**: "All consumer crates built successfully"

---

## Compilation Checkpoints

The builder should verify compilation after each phase:

- **After Phase 1**: `cargo build -p perl-workspace` (may have missing imports, that's OK)
- **After Phase 2**: `cargo check -p perl-workspace` (folder-modules exist, types resolved)
- **After Phase 3**: `cargo build -p perl-workspace` (pub mod declarations in place)
- **After Phase 4**: `cargo test -p perl-workspace` (tests migrated and imports fixed)
- **After Phase 5**: `cargo build -p <each consumer>` (one at a time; spot-check 2-3)
- **After Phase 6**: `cargo check` (workspace deps resolved)
- **After Phase 7**: `cargo build --workspace --lib` (all source imports updated)
- **After Phase 8**: `cargo build -p perl-ci-hygiene && cargo test -p perl-parser` (hardcoded strings fixed)
- **After Phase 9**: `cargo metadata --no-deps` (6 directories gone)
- **After Phase 10**: Full test suite and lint pass

---

## Notes for Builder

1. **Test file migration**: When copying test files, preserve comment structure and module documentation.
2. **Import replacement**: Use `sed` or find-and-replace in editor; verify each change compiles.
3. **Backward compatibility**: The `src/workspace/mod.rs` re-exports are critical for existing consumers. Test with a consumer that uses `perl_workspace::workspace::*` paths.
4. **api.rs explicitness**: Do not use wildcards in `api.rs`. If a type name appears in multiple modules (e.g., `IndexStateKind`), the compiler will reject wildcard re-export.
5. **Hardcoded strings**: Double-check all string matches; some may be in comments (OK to skip) or test snapshots (must update).
6. **Commit strategy**: Consider intermediate commits (one per phase) so history is clear; but final PR should be a single squashed commit with the spec summary.

---

## Change Order Summary

1. Rename package (Cargo.toml line 2)
2. Create 6 folder-modules (src/discovery, folder, ignore, monitoring, slo, state_machine)
3. Declare modules in lib.rs
4. Create api.rs with explicit re-exports
5. Update workspace/mod.rs for backward compatibility
6. Update perl-workspace-index Cargo.toml (remove satellites, add runtime deps)
7. Migrate 15 test files (6+4+1+3+1) with import updates
8. Update 8 consumer Cargo.toml files
9. Update workspace root Cargo.toml (dependencies and allowlist)
10. Update ~131 import statements across consumer source files
11. Update hardcoded crate name strings (2 locations)
12. Delete 6 old crate directories
13. Verify (no old names, member count, publish count, tests, lint)

**Estimated effort**: 4-6 hours (bulk is mechanical find-and-replace; validation is thorough)
