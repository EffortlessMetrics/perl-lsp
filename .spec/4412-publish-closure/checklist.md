# Implementation Checklist: publish-closure gate

**Issue**: #4412 — feat(xtask): add publish-closure gate (PR #1 of microcrate collapse, #4410)
**Branch**: `impl/4412-publish-closure`
**Priority**: Small (~2 hours, single file)

This checklist removes all ambiguity for the red TDD builder and implementation builder. Each step is executable and results in a compiling workspace.

---

## Phase 1: Module skeleton and CLI wiring (10 minutes)

### Step 1a: Create new module file
**File**: `xtask/src/tasks/publish_closure.rs` (NEW)

**What to write**:
- Public function signature: `pub fn run(crate_filter: Option<String>) -> Result<()>`
- Placeholder body: `Ok(())` (will be replaced in Phase 3)
- Module docstring: `//! Verify transitive normal-dep closure of published crates`

**Why first**: Rust requires the module to exist before it can be registered and imported.

**Verify**: 
```bash
cd xtask && cargo check -p xtask
```
Expected: Will fail on import in mod.rs (next step).

---

### Step 1b: Register module in mod.rs
**File**: `xtask/src/tasks/mod.rs` (MODIFY)

**What changes**:
- Find line 56: `pub mod publish_receipts;`
- Add after it: `pub mod publish_closure;`

**Verify**:
```bash
cd xtask && cargo check -p xtask
```
Expected: Still fails (main.rs doesn't import yet).

---

### Step 1c: Add CLI enum variant
**File**: `xtask/src/main.rs` (MODIFY)

**Where**: After line 717 (`PublishVscode` command, around line 717)

**What changes**:
```rust
    /// Verify transitive normal-dep closure of published crates contains only publishable deps
    PublishClosure {
        /// Check only this crate (default: all allowlisted crates)
        #[arg(long)]
        crate_name: Option<String>,
    },
```

**Verify**:
```bash
cd xtask && cargo check -p xtask
```
Expected: Still fails (dispatch not wired yet).

---

### Step 1d: Wire dispatch in main.rs
**File**: `xtask/src/main.rs` (MODIFY)

**Where**: Find line 1366-1368 (PublishVscode dispatch), around there add:
```rust
        Commands::PublishClosure { crate_name } => publish_closure::run(crate_name),
```

**Before or after**: Add right after PublishVscode dispatch (line ~1367)

**Verify**:
```bash
cargo xtask publish-closure --help
```
Expected: Shows help text with `--crate-name` option.

```bash
cargo check -p xtask
```
Expected: Success (module compiles, just returns Ok()).

---

## Phase 2: CLI recipe wiring (5 minutes)

### Step 2a: Add justfile recipe
**File**: `justfile` (MODIFY)

**Where**: After line 859 (`ci-forbid-fatal:` recipe), add:
```just
ci-publish-closure:
    @echo "🔐 Checking publish-closure transitive deps..."
    @cargo xtask publish-closure
    @echo "✅ Publish-closure check passed"
```

**Verify**:
```bash
just ci-publish-closure
```
Expected: Runs, prints "Publish-closure check passed", exits 0.

---

### Step 2b: Wire into pr-fast
**File**: `justfile` (MODIFY)

**Where**: Line 52 (after `just _timed "test-core" "just test-core"`)

**What changes** (current):
```just
pr-fast: _check-tools-basic
    #!/usr/bin/env bash
    ...
    just _timed "test-core" "just test-core"
    RC=$?
```

**Change to**:
```just
pr-fast: _check-tools-basic
    #!/usr/bin/env bash
    ...
    just _timed "test-core" "just test-core" && \
    just _timed "publish-closure" "just ci-publish-closure"
    RC=$?
```

**Verify**:
```bash
just pr-fast
```
Expected: Runs, test-core passes, then publish-closure passes.

---

### Step 2c: Wire into ci-gate
**File**: `justfile` (MODIFY)

**Where**: Line 763 (after `just hook-tests`)

**What changes** (current line 763):
```just
    just hook-tests
    # @START=$$(date +%s); \
```

**Change to**:
```just
    just hook-tests && \
    just ci-publish-closure
    # @START=$$(date +%s); \
```

**Verify**:
```bash
just ci-gate
```
Expected: Long gate runs and passes (includes publish-closure).

---

## Phase 3: Core implementation (60 minutes)

### Step 3a: Define metadata structs
**File**: `xtask/src/tasks/publish_closure.rs` (MODIFY)

**What to add** (before `pub fn run()`):

```rust
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

#[derive(Deserialize)]
struct FullMetadata {
    packages: Vec<FullPackage>,
    workspace_members: Vec<String>,
    metadata: Option<WorkspacePublishMeta>,
    resolve: Option<ResolveGraph>,
}

#[derive(Deserialize)]
struct FullPackage {
    name: String,
    id: String,
    publish: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct WorkspacePublishMeta {
    publish: Option<AllowList>,
}

#[derive(Deserialize)]
struct AllowList {
    allow: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct ResolveGraph {
    nodes: Vec<ResolveNode>,
}

#[derive(Deserialize)]
struct ResolveNode {
    id: String,
    deps: Vec<ResolveDep>,
}

#[derive(Deserialize)]
struct ResolveDep {
    pkg: String,       // CRITICAL: "pkg" not "id"
    dep_kinds: Vec<DepKind>,
}

#[derive(Deserialize)]
struct DepKind {
    kind: Option<String>,   // null = normal, "dev" = dev, "build" = build
    target: Option<String>,
}
```

**Why these structs**: Unlike `publish.rs`, we need the full `resolve` graph to walk transitive deps. Structs are intentionally flat (not nested) to simplify deserialization.

**Verify**:
```bash
cargo check -p xtask
```
Expected: Success.

---

### Step 3b: Implement metadata loading
**File**: `xtask/src/tasks/publish_closure.rs` (MODIFY)

**Replace the `pub fn run()` placeholder** with:

```rust
pub fn run(crate_filter: Option<String>) -> Result<()> {
    let metadata = load_metadata()?;
    
    // Load allowlist from workspace.metadata.publish.allow
    let allowlist = metadata
        .metadata
        .as_ref()
        .and_then(|m| m.publish.as_ref())
        .and_then(|p| p.allow.as_ref())
        .ok_or_else(|| eyre!("No [workspace.metadata.publish.allow] found in Cargo.toml"))?;
    
    // Build set of workspace-member names
    let workspace_members: HashSet<String> = metadata
        .workspace_members
        .iter()
        .filter_map(|member| {
            // "cargo_manifest:///path/to/Cargo.toml" -> extract name
            member.split("cargo_manifest:///").nth(1).and_then(|path| {
                std::path::Path::new(path)
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
            })
        })
        .collect();
    
    // Build set of publish=false workspace members
    let no_publish: HashSet<String> = metadata
        .packages
        .iter()
        .filter(|pkg| {
            workspace_members.contains(&pkg.name) && 
            pkg.publish == Some(vec![])
        })
        .map(|pkg| pkg.name.clone())
        .collect();
    
    // Filter crates to check (all allowlist or single crate_name)
    let crates_to_check: Vec<&String> = if let Some(ref filter) = crate_filter {
        if !allowlist.contains(filter) {
            bail!("Crate '{}' not found in publish allowlist", filter);
        }
        vec![filter]
    } else {
        allowlist.iter().collect()
    };
    
    // Build package_id -> name mapping
    let id_to_name: HashMap<String, String> = metadata
        .packages
        .iter()
        .map(|pkg| (pkg.id.clone(), pkg.name.clone()))
        .collect();
    
    // Build resolve graph for BFS: pkg_id -> [dep_ids (normal only)]
    let mut resolve_graph: HashMap<String, Vec<String>> = HashMap::new();
    if let Some(resolve) = metadata.resolve.as_ref() {
        for node in &resolve.nodes {
            let normal_deps: Vec<String> = node
                .deps
                .iter()
                .filter_map(|dep| {
                    // Include if kind is None (normal dep)
                    if dep.dep_kinds.iter().all(|dk| dk.kind.is_none()) {
                        Some(dep.pkg.clone())
                    } else {
                        None
                    }
                })
                .collect();
            resolve_graph.insert(node.id.clone(), normal_deps);
        }
    }
    
    // Check each crate in allowlist
    let mut violations = Vec::new();
    for crate_name in crates_to_check {
        // Find package ID for this crate
        let pkg_id = metadata
            .packages
            .iter()
            .find(|p| p.name == *crate_name)
            .map(|p| p.id.clone());
        
        if let Some(start_id) = pkg_id {
            // BFS the resolve graph following normal deps only
            let bad_deps = check_transitive_closure(&start_id, &resolve_graph, &no_publish, &id_to_name);
            for bad_dep in bad_deps {
                violations.push((crate_name.clone(), bad_dep));
            }
        }
    }
    
    // Report violations
    if !violations.is_empty() {
        for (published, forbidden) in violations {
            eprintln!("ERROR: publish-closure violation");
            eprintln!("  Published crate `{}` has transitive normal dep on `{}` (publish = false)", published, forbidden);
        }
        bail!("publish-closure check failed");
    }
    
    // Success message
    let count = if crate_filter.is_some() { 1 } else { allowlist.len() };
    println!("publish-closure: OK ({} crate{} checked, 0 violations)", 
             count, 
             if count == 1 { "" } else { "s" });
    
    Ok(())
}

fn load_metadata() -> Result<FullMetadata> {
    let output = std::process::Command::new("cargo")
        .args(["metadata", "--format-version", "1"])
        .output()?;
    if !output.status.success() {
        bail!("cargo metadata failed");
    }
    let metadata: FullMetadata = serde_json::from_slice(&output.stdout)?;
    Ok(metadata)
}

fn check_transitive_closure(
    start_id: &str,
    graph: &HashMap<String, Vec<String>>,
    no_publish: &HashSet<String>,
    id_to_name: &HashMap<String, String>,
) -> Vec<String> {
    use std::collections::VecDeque;
    
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut bad_deps = Vec::new();
    
    queue.push_back(start_id.to_string());
    visited.insert(start_id.to_string());
    
    while let Some(current_id) = queue.pop_front() {
        if let Some(deps) = graph.get(&current_id) {
            for dep_id in deps {
                if !visited.contains(dep_id) {
                    visited.insert(dep_id.clone());
                    
                    // Check if this dependency is a no_publish crate
                    if let Some(dep_name) = id_to_name.get(dep_id) {
                        if no_publish.contains(dep_name) {
                            bad_deps.push(dep_name.clone());
                        }
                    }
                    
                    queue.push_back(dep_id.clone());
                }
            }
        }
    }
    
    bad_deps
}
```

**Why this approach**:
- `cargo metadata --format-version 1` WITHOUT `--no-deps` includes the `resolve` section (critical difference from `publish.rs`)
- BFS walks transitive closure; we collect package IDs at each level
- We skip dev and build deps by filtering `dep_kinds` for `kind: None`
- We only report crates in the `no_publish` set (workspace members with `publish = []`)

**Verify**:
```bash
cargo xtask publish-closure
```
Expected: "publish-closure: OK (132 crates checked, 0 violations)"

```bash
cargo xtask publish-closure --crate-name perl-token
```
Expected: "publish-closure: OK (1 crate checked, 0 violations)"

```bash
cargo xtask publish-closure --crate-name nonexistent-xyz
```
Expected: Error: "Crate 'nonexistent-xyz' not found in publish allowlist"

```bash
cargo clippy -p xtask
```
Expected: Clean (no warnings).

```bash
cargo xtask fmt --check
```
Expected: No formatting changes needed.

---

## Phase 4: Tests (15 minutes)

### Step 4a: Create test file
**File**: `xtask/tests/publish_closure.rs` (NEW)

**Write**:
```rust
use assert_cmd::Command;
use color_eyre::eyre::Result;

#[test]
fn publish_closure_passes_on_master() -> Result<()> {
    Command::cargo_bin("xtask")?
        .args(["publish-closure"])
        .assert()
        .success();
    Ok(())
}

#[test]
fn publish_closure_single_crate_flag() -> Result<()> {
    Command::cargo_bin("xtask")?
        .args(["publish-closure", "--crate-name", "perl-token"])
        .assert()
        .success();
    Ok(())
}

#[test]
fn publish_closure_unknown_crate_exits_nonzero() -> Result<()> {
    Command::cargo_bin("xtask")?
        .args(["publish-closure", "--crate-name", "nonexistent-crate-xyz"])
        .assert()
        .failure();
    Ok(())
}
```

**Verify**:
```bash
cargo test -p xtask -- publish_closure
```
Expected: All 3 tests pass.

---

## Phase 5: Final verification (5 minutes)

### Step 5a: Full clippy check
```bash
cargo clippy -p xtask --all-targets -- -D warnings
```
Expected: Clean.

---

### Step 5b: Full format check
```bash
cargo xtask fmt --check
```
Expected: No changes needed.

---

### Step 5c: pr-fast gate
```bash
just pr-fast
```
Expected: All checks pass, including new ci-publish-closure recipe.

---

### Step 5d: ci-gate (optional but recommended)
```bash
just ci-gate
```
Expected: All checks pass (long, ~5-10 min).

---

## Acceptance Criteria Checklist

- [ ] `cargo xtask publish-closure` runs and exits 0 on master
- [ ] `cargo xtask publish-closure --crate-name perl-token` exits 0
- [ ] `cargo xtask publish-closure --crate-name nonexistent-xyz` exits 1 with clear error
- [ ] `cargo xtask publish-closure --help` shows usage
- [ ] Violation output matches: `ERROR: publish-closure violation\n  Published crate '...' has transitive normal dep on '...' (publish = false)`
- [ ] Success output matches: `publish-closure: OK (N crates checked, 0 violations)`
- [ ] All 3 tests in `xtask/tests/publish_closure.rs` pass
- [ ] `just ci-publish-closure` recipe exists and runs successfully
- [ ] `pr-fast` gate includes `just ci-publish-closure` step
- [ ] `ci-gate` includes `just ci-publish-closure` step
- [ ] `cargo clippy -p xtask` clean
- [ ] `cargo xtask fmt --check` clean
- [ ] No `unwrap()`, `expect()`, `panic!()`, `todo!()`, `dbg!()` in production code
- [ ] All deps already exist in xtask/Cargo.toml (serde, serde_json, color_eyre)

---

## Files Changed Summary

| File | Type | Lines | Change |
|------|------|-------|--------|
| `xtask/src/tasks/publish_closure.rs` | NEW | ~150 | Full implementation |
| `xtask/src/tasks/mod.rs` | MODIFY | +1 | Register module |
| `xtask/src/main.rs` | MODIFY | +10 | Add command + dispatch |
| `justfile` | MODIFY | +10 | Add recipe + wire into pr-fast/ci-gate |
| `xtask/tests/publish_closure.rs` | NEW | ~30 | Integration tests |

---

## Key Implementation Details

1. **No --no-deps flag**: Critical difference from `publish.rs`. We need `resolve` section for transitive dep walking.

2. **Field name: `pkg` not `id`**: In cargo metadata JSON, `resolve.nodes[].deps[].pkg` is the dependency package ID string. NOT `id`.

3. **Normal-dep filtering**: Only follow edges where `dep_kinds` contains `DepKind { kind: None, .. }`. Skip `kind == Some("dev")` and `kind == Some("build")`.

4. **External crates never violations**: Only workspace members with `publish = false` are flagged. Registry deps are safe.

5. **Empty publish array means no_publish**: `publish: []` (empty JSON array) means `publish = false` in Cargo.toml. Matches Python script.

6. **Allowlist source**: Read from `[workspace.metadata.publish.allow]` section of root `Cargo.toml`.

7. **Workspace member detection**: `workspace_members` is a list of `cargo_manifest://` URIs. Extract crate name from path.

---

## Deferred Work

- `cargo xtask layer-check` — follow-up issue #4415
- `cargo xtask published-crate-count` — follow-up issue #4416
- Integration with Wave B/C of #4410 collapse

---

## Related Issues

- Parent: #4410 — Microcrate collapse (v0.13.0)
- ADR: PR #4413 (ADR-0041)
- Design pattern source: `scripts/publish-topo.py` (publish order, not closure)
