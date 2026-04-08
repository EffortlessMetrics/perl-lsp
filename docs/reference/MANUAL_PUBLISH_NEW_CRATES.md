# Manual New-Crate Publish Procedure

This document explains how to manually publish new (first-ever) crates to
crates.io when the automated publish workflow is blocked by crates.io's
account-level new-crate creation rate limit.

## Background: Two Separate Rate Limits

crates.io enforces **two distinct rate limits** on publishing:

| Limit type | Burst | Steady-state refill | Who it affects |
|---|---|---|---|
| **Update existing crate** | 30 | 1 per 12 seconds | Publishing a new version of an existing crate |
| **Create new crate** | **5** | **1 per 10 minutes** | First publish of a brand-new crate name |

The automated `publish-crates.yml` workflow handles the update limit fine
(it sleeps 13s between publishes). The **new-crate limit** is the one that
bites during a release that introduces new crate names.

### Error message

When the new-crate burst of 5 is exhausted, crates.io returns HTTP 429 with:

> You have published too many new crates in a short period of time

The token bucket refills at **1 per 10 minutes**. This means:
- You can publish 5 new crates immediately (burst).
- After that, you must wait 10 minutes per additional new crate.
- If you keep retrying immediately after each 429, the window resets and
  you may wait much longer.

Source: crates.io rate limiter source (`PublishNew` action):
burst=5, default_rate_seconds=600 (10 minutes).

### Why the automated workflow hits this limit

The `publish-crates.yml` workflow publishes all crates in a single sequential
loop with only a 13-second throttle. That throttle is designed for the update
rate limit (1 per 12s), but does nothing for the new-crate limit. When a
release includes more than 5 new crate names, the 6th attempt immediately
hits the 429.

PR #3316 (`continue-on-failure`) mitigates this by letting updates proceed
even when new-crate publishes fail, but the new crates still need manual
intervention.

## The 4 New Crates for v0.12.2

These 4 crates are making their first-ever appearance on crates.io:

| # | Crate | Tier | Runtime deps (workspace) |
|---|---|---|---|
| 1 | `perl-test-generators` | Tier 1 (leaf) | None (only `proptest`) |
| 2 | `tree-sitter-perl-c` | Tier 1 (leaf) | None (only `tree-sitter` C crate) |
| 3 | `tree-sitter-perl-rs` | Tier 2+ | `perl-parser-core`, `perl-ast` (existing crates) |
| 4 | `perl-workspace-index-monitoring` | Tier 2+ | None new (`parking_lot` only; dev-dep on `perl-tdd-support` is existing) |

None of the 4 new crates depend on another new crate, so this order is
flexible. The order above matches the workspace topological allowlist.

## Automated Helper Script

A helper script is provided:

```bash
scripts/publish-new-crates-manually.sh
```

And a justfile recipe:

```bash
just publish-new-crates
```

### Dry run (safe, no publishing)

```bash
DRY_RUN=true bash scripts/publish-new-crates-manually.sh
# or via just:
DRY_RUN=true just publish-new-crates
```

Expected output:

```
[HH:MM:SS] INFO  === Manual New-Crate Publisher ===
[HH:MM:SS] INFO  Mode: true
[HH:MM:SS] INFO  Crates to publish: 4
[HH:MM:SS] INFO  Inter-publish sleep: 600s
...
[HH:MM:SS] INFO  --- Publishing 1/4: perl-test-generators ---
[HH:MM:SS] INFO  Manifest: /path/to/crates/perl-test-generators/Cargo.toml
[HH:MM:SS] INFO  [DRY RUN] Would strip [dev-dependencies] from ...
[HH:MM:SS] INFO  [DRY RUN] Would run: cargo publish -p perl-test-generators --no-verify --allow-dirty
[HH:MM:SS] INFO  [DRY RUN] Would wait for sparse index visibility
[HH:MM:SS] INFO  [DRY RUN] Would restore ...
[HH:MM:SS] INFO  [DRY RUN] Would sleep 600s before next crate
...
[HH:MM:SS] INFO  === Done: published=4, skipped=0, total=4 ===
[HH:MM:SS] INFO  Dry run complete. Remove DRY_RUN=true to actually publish.
```

### Live run

Requires a crates.io API token with publish permission:

```bash
export CARGO_REGISTRY_TOKEN="your-crates-io-token"
bash scripts/publish-new-crates-manually.sh
```

The script will:
1. Check the sparse index — if a crate is already visible, it skips it
   (safe to re-run after a partial failure).
2. Strip `[dev-dependencies]` from the Cargo.toml before publishing
   (same technique as `publish-crates.yml`).
3. Run `cargo publish -p <crate> --no-verify --allow-dirty`.
4. Poll the sparse index until the crate appears (up to 90 seconds).
5. Restore the original Cargo.toml.
6. Sleep **600 seconds (10 minutes)** before the next crate.

Total wall-clock time: approximately **40 minutes** for 4 crates
(3 × 10-minute gaps + index wait time per crate).

### On 429 / rate limit hit

If the script exits with code 4, it means a 429 was returned. At that point:

1. **Do not retry immediately** — repeating attempts extends the back-off.
2. Wait at least 10 minutes (one full refill cycle).
3. Re-run the script — it will skip already-published crates automatically.

If you continue to hit 429 after waiting, the account-level bucket may be
partially depleted. Wait longer (30–60 minutes) or contact crates.io support
at <https://github.com/rust-lang/crates.io/issues>.

## Manual Invocation (without the script)

If you prefer to run each crate by hand:

### Prerequisites

```bash
export CARGO_REGISTRY_TOKEN="your-crates-io-token"
cd /path/to/perl-lsp  # workspace root
```

### Step-by-step per crate

For each crate in the order above:

1. **Back up the Cargo.toml**:
   ```bash
   cp crates/<crate>/Cargo.toml crates/<crate>/Cargo.toml.bak
   ```

2. **Strip dev-dependencies** (prevents publish failures from workspace
   dev-dep cycles when sibling crates are not yet on crates.io):
   ```bash
   python3 - crates/<crate>/Cargo.toml << 'EOF'
   import re, sys, pathlib
   path = pathlib.Path(sys.argv[1])
   text = path.read_text(encoding="utf-8")
   new_text = re.sub(
       r"^\[dev-dependencies\].*?(?=^\[|\Z)",
       "",
       text,
       flags=re.MULTILINE | re.DOTALL,
   )
   path.write_text(new_text, encoding="utf-8")
   EOF
   ```

3. **Publish**:
   ```bash
   cargo publish -p <crate> --no-verify --allow-dirty
   ```

4. **Restore the original Cargo.toml**:
   ```bash
   cp crates/<crate>/Cargo.toml.bak crates/<crate>/Cargo.toml
   ```

5. **Verify the crate appeared in the sparse index** (replace `<path>`
   using the layout rules below):
   ```bash
   # For crates with 4+ character names: first-two / next-two / name
   # e.g. perl-test-generators -> pe/rl/perl-test-generators
   curl -s https://index.crates.io/pe/rl/perl-test-generators | tail -1 | python3 -m json.tool
   ```

6. **Wait 10 minutes** before publishing the next crate.

### Sparse index URL reference for these 4 crates

| Crate | Sparse index URL |
|---|---|
| `perl-test-generators` | `https://index.crates.io/pe/rl/perl-test-generators` |
| `tree-sitter-perl-c` | `https://index.crates.io/tr/ee/tree-sitter-perl-c` |
| `tree-sitter-perl-rs` | `https://index.crates.io/tr/ee/tree-sitter-perl-rs` |
| `perl-workspace-index-monitoring` | `https://index.crates.io/pe/rl/perl-workspace-index-monitoring` |

## Rate Limit Diagnostic (2026-04-08 incident)

During the v0.12.2 release:

- The `publish-crates.yml` workflow published ~20+ updates successfully.
- On the first attempt to publish a new crate, it received HTTP 429 with
  message: "You have published too many new crates in a short period of time"
- Retrying immediately continued to produce 429 responses because the token
  bucket was exhausted and attempts reset the wait.
- 19 update publishes (`perl-parser`, `perl-dap`, `perl-lsp-rs`, `perllsp`,
  etc.) were blocked because the workflow exits on the first hard failure.
  (PR #3316 addresses the exit-on-failure issue.)

**Root cause**: The automated workflow did not account for the new-crate rate
limit (burst=5, refill=1/10 min), which is separate from the update limit
(burst=30, refill=1/12 s).

**Lesson**: Any release adding more than 5 new crate names requires either
(a) pre-publishing the new crates manually with 10-minute gaps, or (b)
splitting the release into two waves — new crates first (manually), then
all updates via the workflow.

## Related Issues and PRs

- PR #3307 — fixed the update rate limit (added 13s inter-publish sleep)
- PR #3316 — `continue-on-failure` in publish workflow (prevents update
  publishes from being blocked by a new-crate failure)
- Issue #3311 — context for the v0.12.2 publish incident
