# Specs: Published Crate Count Ratchet Gate

## Feature Description

A CI gate that monitors the count of entries in `[workspace.metadata.publish.allow]` and prevents accidental regression (allowlist growth) after the microcrate collapse. The gate uses a file-based baseline with auto-tightening behavior.

## Feature Behavior

### Core Behavior

Given `cargo xtask published-crate-count`:

1. Reads current published crate count from `[workspace.metadata.publish.allow]` via `cargo metadata --no-deps`
2. Reads baseline from `xtask/published-crate-baseline.txt` (single integer)
3. Compares current count against baseline:
   - `current > baseline` → exit 1 (fail with remediation message)
   - `current < baseline` → exit 0, auto-write new baseline (ratchet tightens)
   - `current == baseline` → exit 0 (pass silently)

### CLI Interface

```
cargo xtask published-crate-count
```

No flags needed — the ratchet auto-tightens on every run where count has decreased.

## Acceptance Criteria

### AC-1: Gate fails when count exceeds baseline

```
# With baseline = 81 and allowlist containing 82 entries
cargo xtask published-crate-count
# => Exit 1, error message:
# "published-crate-count: FAIL — 82 crates published, baseline is 81.
#  The published crate count increased. Either remove crates from
#  [workspace.metadata.publish.allow] in Cargo.toml, or if the increase is
#  intentional, update xtask/published-crate-baseline.txt explicitly in a reviewed commit."
```

### AC-2: Gate auto-tightens when count decreases

```
# With baseline = 98 and allowlist containing 81 entries
cargo xtask published-crate-count
# => Exit 0, message:
# "published-crate-count: RATCHET — count dropped from 98 to 81, updating xtask/published-crate-baseline.txt"
# Baseline file is updated to 81
```

### AC-3: Gate passes silently when count equals baseline

```
# With baseline = 81 and allowlist containing 81 entries
cargo xtask published-crate-count
# => Exit 0, message:
# "published-crate-count: OK (81 crates, baseline 81)"
```

### AC-4: Baseline file format is correct

```
# xtask/published-crate-baseline.txt contains:
81\n

# Must parse as integer 81, with or without trailing newline
```

### AC-5: Counts all allowlist entries (including non-perl- prefixed)

```
# Allowlist: ['perl-position-tracking', 'perl-token', 'tree-sitter-perl-c', ...]
# Count = 81 (includes tree-sitter-perl-c and tree-sitter-perl-rs)
```

## Non-Goals

- This gate does NOT validate which specific crates are in the allowlist (that is `publish-closure`'s job)
- This gate does NOT run during the collapse transition in a blocking manner (quarantine: true until collapse completes)
- This gate does NOT require `--update` flag — auto-tightening handles baseline updates

## Dependencies

- `cargo metadata --no-deps` (via `run_cargo_metadata(true)` in utils.rs)
- `serde_json` for parsing metadata JSON
- `serde::{Deserialize}` for metadata types
- No external crate additions needed (already uses existing dependencies)

## Implementation Location

- `xtask/src/tasks/count_ratchet.rs` (already exists, 167 lines)
- `xtask/src/tasks/mod.rs` — `pub mod count_ratchet;` (already present)
- `xtask/src/main.rs` — `Commands::PublishedCrateCount` variant + match arm (already present)
- `xtask/published-crate-baseline.txt` — baseline file (already exists, value: 81)
- `justfile` — `ci-published-crate-count` recipe (already exists)

## CI Integration (pending)

The gate needs to be formally added to `.ci/gate-policy.yaml`:

```yaml
- name: published_crate_count
  tier: merge_gate
  description: "Ratchet: published crate count must not exceed baseline"
  required: true
  command: cargo xtask published-crate-count
  timeout_seconds: 30
  retry_count: 0
  budgets:
    max_duration_ms: 5000
  quarantine: true  # Until collapse completes (~30-31 crates)
  tags:
    - ratchet
    - microcrate
    - collapse
```

Note: `quarantine: true` because the count is still 81 and target is ~30. When collapse completes, a follow-up PR will change `quarantine: false`.

## Verification

1. Run `cargo xtask published-crate-count` — should exit 0 with "RATCHET" or "OK"
2. Temporarily add a crate to allowlist, run again — should exit 1 with clear error
3. Verify baseline file updates correctly when count decreases
4. After adding to gate-policy.yaml, verify gate runs in CI (in quarantine mode initially)
