# Wave G2 Runtime Crate Absorption — Implementation Checklist

Base: 44bda08d1. Scope: 6 crates (perl-lsp-performance deferred to G3 per plan-reviewer).

## Step 1: Create runtime module structure

**File:** `crates/perl-lsp-rs-core/src/runtime/` (CREATE directory)

**Action:** Create the directory structure:
```
crates/perl-lsp-rs-core/src/runtime/
├── cancellation/
├── input_validation/
├── launcher/
├── limits/
├── text_utils/
└── transport/
```

**Verify:**
```bash
mkdir -p /h/Code/Rust/perl-lsp/crates/perl-lsp-rs-core/src/runtime/{cancellation,input_validation,launcher,limits,text_utils,transport}
```

---

## Step 2: Copy source files to runtime modules

**File:** `crates/perl-lsp-rs-core/src/runtime/*/mod.rs` + sibling files (CREATE)

**Action:** For each of the 6 crates, copy source files into the runtime module structure:

1. **cancellation:**
   - Copy: `crates/perl-lsp-cancellation/src/lib.rs` → `crates/perl-lsp-rs-core/src/runtime/cancellation/mod.rs`

2. **limits:**
   - Copy: `crates/perl-lsp-limits/src/lib.rs` → `crates/perl-lsp-rs-core/src/runtime/limits/mod.rs`

3. **input_validation:**
   - Copy: `crates/perl-lsp-input-validation/src/lib.rs` → `crates/perl-lsp-rs-core/src/runtime/input_validation/mod.rs`

4. **launcher** (includes sibling):
   - Copy: `crates/perl-lsp-launcher/src/lib.rs` → `crates/perl-lsp-rs-core/src/runtime/launcher/mod.rs`
   - Copy: `crates/perl-lsp-launcher/src/timing.rs` → `crates/perl-lsp-rs-core/src/runtime/launcher/timing.rs`

5. **transport** (includes sibling):
   - Copy: `crates/perl-lsp-transport/src/framing.rs` → `crates/perl-lsp-rs-core/src/runtime/transport/framing.rs`
   - Copy: `crates/perl-lsp-transport/src/lib.rs` → `crates/perl-lsp-rs-core/src/runtime/transport/mod.rs`

6. **text_utils:**
   - Copy: `crates/perl-lsp-text-utils/src/lib.rs` → `crates/perl-lsp-rs-core/src/runtime/text_utils/mod.rs`

**Verify:** Each file exists and compiles:
```bash
cargo check -p perl-lsp-rs-core 2>&1 | head -20
```

---

## Step 3: Add `pub mod runtime;` to rs-core lib.rs

**File:** `crates/perl-lsp-rs-core/src/lib.rs` (MODIFY)

**Change:** Add the following line after the `pub mod providers;` declaration:
```rust
pub mod runtime;
```

**Verify:**
```bash
grep "pub mod runtime;" /h/Code/Rust/perl-lsp/crates/perl-lsp-rs-core/src/lib.rs
```

---

## Step 4: Create runtime/mod.rs with module declarations

**File:** `crates/perl-lsp-rs-core/src/runtime/mod.rs` (CREATE)

**Action:** Create the module-level file with the required doc comment and module declarations. The doc comment MUST explain the grouping rationale and note that text_utils is providers-adjacent:

```rust
//! LSP runtime infrastructure (Wave G2: 6 runtime crates absorbed).
//!
//! This module contains the implementation of LSP runtime support previously
//! distributed across 6 separate crates: cancellation (request/thread lifecycle),
//! limits (resource constraints), input_validation (security/hygiene),
//! launcher (process control), transport (protocol framing), and text_utils
//! (text editing utilities).
//!
//! **Note:** text_utils is semantically providers-adjacent (used by code_actions)
//! but grouped here as runtime infrastructure for organizational coherence with
//! other protocol support utilities. Verify re-exports in rs-core::providers
//! if adding new consumers.
//!
//! ## Module structure
//!
//! - **cancellation**: Request-scoped cancellation tokens with atomic operations
//! - **limits**: Memory/resource budgets and deadline constraints
//! - **input_validation**: Security validation (file paths, content, LSP requests)
//! - **launcher**: CLI parsing, logging initialization, startup coordination
//! - **transport**: LSP message framing and protocol-level utilities
//! - **text_utils**: Text editing helpers (TextEditHelpers, edit composition)

pub mod cancellation;
pub mod input_validation;
pub mod launcher;
pub mod limits;
pub mod text_utils;
pub mod transport;

// Re-exports for ergonomic access
pub use cancellation::*;
pub use input_validation::*;
pub use launcher::*;
pub use limits::*;
pub use text_utils::*;
pub use transport::*;
```

**Verify:**
```bash
head -30 /h/Code/Rust/perl-lsp/crates/perl-lsp-rs-core/src/runtime/mod.rs
```

---

## Step 5: Update import in crates/perl-lsp/src/cancellation.rs

**File:** `crates/perl-lsp/src/cancellation.rs` (MODIFY)

**Change:** Replace:
```rust
pub use perl_lsp_cancellation::*;
```

With:
```rust
pub use perl_lsp_rs_core::runtime::cancellation::*;
```

**Verify:**
```bash
grep "pub use perl_lsp_rs_core::runtime::cancellation::" /h/Code/Rust/perl-lsp/crates/perl-lsp/src/cancellation.rs
```

---

## Step 6: Update import in crates/perl-lsp/src/cli.rs (two sites)

**File:** `crates/perl-lsp/src/cli.rs` (MODIFY)

**Change 1:** Replace:
```rust
use perl_lsp_launcher::{
```

With:
```rust
use perl_lsp_rs_core::runtime::launcher::{
```

**Change 2:** Replace the tracing filter string at line ~326:
```rust
"perl_lsp=info,perl_lsp_launcher=info,info",
```

With:
```rust
"perl_lsp=info,perl_lsp_rs_core=info,info",
```

**Important Note:** The tracing filter string must use `perl_lsp_rs_core` not `perl_lsp_rs_core::runtime` because tracing filters match crate names, not module paths.

**Verify:**
```bash
grep -n "perl_lsp_rs_core::runtime::launcher" /h/Code/Rust/perl-lsp/crates/perl-lsp/src/cli.rs
grep -n "perl_lsp_rs_core=info" /h/Code/Rust/perl-lsp/crates/perl-lsp/src/cli.rs
```

---

## Step 7: Update import in crates/perl-lsp/src/security/validation.rs

**File:** `crates/perl-lsp/src/security/validation.rs` (MODIFY)

**Change:** Replace:
```rust
pub use perl_lsp_input_validation::{
```

With:
```rust
pub use perl_lsp_rs_core::runtime::input_validation::{
```

**Verify:**
```bash
grep "pub use perl_lsp_rs_core::runtime::input_validation::" /h/Code/Rust/perl-lsp/crates/perl-lsp/src/security/validation.rs
```

---

## Step 8: Update import in crates/perl-lsp/src/state/mod.rs

**File:** `crates/perl-lsp/src/state/mod.rs` (MODIFY)

**Change:** Replace:
```rust
pub use perl_lsp_limits::*;
```

With:
```rust
pub use perl_lsp_rs_core::runtime::limits::*;
```

**Verify:**
```bash
grep "pub use perl_lsp_rs_core::runtime::limits::" /h/Code/Rust/perl-lsp/crates/perl-lsp/src/state/mod.rs
```

---

## Step 9: Update import in crates/perl-lsp/src/transport/mod.rs

**File:** `crates/perl-lsp/src/transport/mod.rs` (MODIFY)

**Change:** Replace:
```rust
pub use perl_lsp_transport::*;
```

With:
```rust
pub use perl_lsp_rs_core::runtime::transport::*;
```

**Verify:**
```bash
grep "pub use perl_lsp_rs_core::runtime::transport::" /h/Code/Rust/perl-lsp/crates/perl-lsp/src/transport/mod.rs
```

---

## Step 10: Update crates/perl-lsp/Cargo.toml

**File:** `crates/perl-lsp/Cargo.toml` (MODIFY)

**Action:** Remove these 6 dependency lines from the `[dependencies]` section:
```toml
perl-lsp-cancellation = { workspace = true }
perl-lsp-input-validation = { workspace = true }
perl-lsp-limits = { workspace = true }
perl-lsp-transport = { workspace = true }
perl-lsp-launcher = { workspace = true }
```

**Note:** Keep any feature flags like `"perl-lsp-launcher/lsp-ga-lock"` but map them to `perl-lsp-rs-core` if they apply post-absorption (rarely needed, usually safe to drop).

**Verify:**
```bash
grep -E "perl-lsp-(cancellation|input-validation|limits|transport|launcher)" /h/Code/Rust/perl-lsp/crates/perl-lsp/Cargo.toml || echo "OK: All 6 deps removed"
```

---

## Step 11: Update workspace Cargo.toml [workspace.dependencies]

**File:** `Cargo.toml` (workspace root, MODIFY)

**Action:** Remove these 6 lines from the `[workspace.dependencies]` section:
```toml
perl-lsp-input-validation = { path = "crates/perl-lsp-input-validation", version = "0.12.4" }
perl-lsp-text-utils = { path = "crates/perl-lsp-text-utils", version = "0.12.4" }
perl-lsp-transport = { path = "crates/perl-lsp-transport", version = "0.12.4" }
perl-lsp-cancellation = { path = "crates/perl-lsp-cancellation", version = "0.12.4" }
perl-lsp-limits = { path = "crates/perl-lsp-limits", version = "0.12.4" }
perl-lsp-launcher = { path = "crates/perl-lsp-launcher", version = "0.12.4" }
```

**Verify:**
```bash
grep -E "perl-lsp-(cancellation|input-validation|limits|transport|launcher|text-utils)" /h/Code/Rust/perl-lsp/Cargo.toml || echo "OK: All 6 deps removed"
```

---

## Step 12: Update xtask/published-crate-baseline.txt

**File:** `xtask/published-crate-baseline.txt` (MODIFY)

**Change:** Replace:
```
49
```

With:
```
43
```

**Rationale:** 49 - 6 = 43 (remove cancellation, limits, input-validation, launcher, transport, text-utils)

**Verify:**
```bash
cat /h/Code/Rust/perl-lsp/xtask/published-crate-baseline.txt
```

---

## Step 13: Migrate 11 test files to crates/perl-lsp-rs-core/tests/

**File:** `crates/perl-lsp-rs-core/tests/runtime_*.rs` (CREATE 11 files)

**Action:** Copy all 11 integration test files to the new location:

From `crates/perl-lsp-cancellation/tests/`:
- `cancellation_bdd.rs` → `crates/perl-lsp-rs-core/tests/runtime_cancellation_bdd.rs`
- `cancellation_property.rs` → `crates/perl-lsp-rs-core/tests/runtime_cancellation_property.rs`
- `comprehensive_unit_tests.rs` → `crates/perl-lsp-rs-core/tests/runtime_cancellation_comprehensive.rs`

From `crates/perl-lsp-limits/tests/`:
- `comprehensive_unit_tests.rs` → `crates/perl-lsp-rs-core/tests/runtime_limits_comprehensive.rs`
- `extended_unit_tests.rs` → `crates/perl-lsp-rs-core/tests/runtime_limits_extended.rs`
- `memory_budget_tests.rs` → `crates/perl-lsp-rs-core/tests/runtime_limits_memory.rs`

From `crates/perl-lsp-input-validation/tests/`:
- `boundary_cases.rs` → `crates/perl-lsp-rs-core/tests/runtime_validation_boundary.rs`

From `crates/perl-lsp-launcher/tests/`:
- `comprehensive_unit_tests.rs` → `crates/perl-lsp-rs-core/tests/runtime_launcher_comprehensive.rs`

From `crates/perl-lsp-transport/tests/`:
- `comprehensive_unit_tests.rs` → `crates/perl-lsp-rs-core/tests/runtime_transport_comprehensive.rs`

From `crates/perl-lsp-text-utils/tests/`:
- `edge_cases.rs` → `crates/perl-lsp-rs-core/tests/runtime_text_utils_edge.rs`
- `text_edit_helpers_tests.rs` → `crates/perl-lsp-rs-core/tests/runtime_text_utils_helpers.rs`

**Update imports in each test file:** Replace crate-level imports with module paths:
- `use perl_lsp_cancellation::*` → `use perl_lsp_rs_core::runtime::cancellation::*`
- `use perl_lsp_limits::*` → `use perl_lsp_rs_core::runtime::limits::*`
- `use perl_lsp_launcher::*` → `use perl_lsp_rs_core::runtime::launcher::*`
- (and so on for other modules)

**Verify:** Each test file compiles:
```bash
cargo test -p perl-lsp-rs-core --test runtime_cancellation_bdd --no-run 2>&1 | head -10
```

---

## Step 14: Delete source crate directories

**File:** `crates/perl-lsp-{cancellation,limits,input-validation,launcher,transport,text-utils}/` (DELETE)

**Action:** Remove the 6 source crate directories:
```bash
rm -rf \
  /h/Code/Rust/perl-lsp/crates/perl-lsp-cancellation \
  /h/Code/Rust/perl-lsp/crates/perl-lsp-limits \
  /h/Code/Rust/perl-lsp/crates/perl-lsp-input-validation \
  /h/Code/Rust/perl-lsp/crates/perl-lsp-launcher \
  /h/Code/Rust/perl-lsp/crates/perl-lsp-transport \
  /h/Code/Rust/perl-lsp/crates/perl-lsp-text-utils
```

**Verify:**
```bash
ls -d /h/Code/Rust/perl-lsp/crates/perl-lsp-{cancellation,limits,input-validation,launcher,transport,text-utils} 2>&1 | wc -l
# Should output: 0 (all deleted)
```

---

## Step 15: Verify compilation and tests

**Action:** Run the complete verification suite:

```bash
# 15a: Format and lint
cargo xtask fmt
cargo clippy -p perl-lsp-rs-core --all-targets

# 15b: Compile check
cargo check -p perl-lsp-rs-core
cargo check -p perl-lsp
cargo check -p perl-dap

# 15c: Run unit tests
cargo test -p perl-lsp-rs-core --lib
cargo test -p perl-lsp --lib

# 15d: Run integration tests
cargo test -p perl-lsp-rs-core --test 'runtime_*' -- --test-threads=1

# 15e: LSP-specific threading test
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2

# 15f: Full workspace gate
cargo xtask ci-gate

# 15g: Verify published crate count
cargo xtask published-crate-count-check

# 15h: DAP regression gate (perl-dap compile time ≤5% vs master)
cargo build -p perl-dap --release
```

**Verify:**
```bash
# All commands should succeed with no errors
# If any fail, check the specific failure and fix forward
```

---

## Summary of Changes

**Crates absorbed:** cancellation, limits, input-validation, launcher, transport, text-utils (6 total)

**Crates deferred:** perl-lsp-performance (remains in crates/, absorbed in G3)

**Modules created:** `perl-lsp-rs-core/src/runtime/{cancellation,input-validation,launcher,limits,text_utils,transport}/`

**Import sites updated:** 5 (cancellation.rs, cli.rs ×2, validation.rs, state/mod.rs, transport/mod.rs)

**Dependencies removed:** 6 from crates/perl-lsp/Cargo.toml + 6 from workspace Cargo.toml

**Baseline update:** 49 → 43

**Test files migrated:** 11 (with naming scheme `runtime_<module>_<test>.rs`)

**Sibling files preserved:** launcher/timing.rs, transport/framing.rs
