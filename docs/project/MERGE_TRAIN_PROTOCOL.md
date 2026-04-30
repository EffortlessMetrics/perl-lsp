# Merge Train Protocol (Issue #7288)

This protocol is a **bounded operator workflow** to keep `master`/`main` green during batch or admin merge windows.

It is intentionally **not** a merge bot:

- no automatic merge execution
- no `--admin` requirement
- no CI policy weakening

## Goal

Validate a small, ordered PR train from the **current green master SHA** before any merge in the train occurs.

## Inputs

- Queue snapshot (recommended): `cargo xtask queue snapshot --out target/receipts/queue-state.json`
- Current master SHA (required truth): `git rev-parse origin/master`
- Candidate PR list + intended order (operator supplied)

## Candidate requirements

Every PR candidate must satisfy all of the following before entering a train:

1. **Current head SHA recorded**
   - Record the PR head SHA at train formation time.
   - Treat any later SHA drift as stale and stop the train.
2. **No active `needs-*` labels**
   - Examples: `needs-ci-fix`, `needs-builder-fix`, `needs-diff-fix`, `needs-deep-review`.
3. **CI green (or expected-skip-normalized green)**
   - Required checks passed, or intentionally path-conditioned skipped lanes that are expected by policy.
4. **Mergeability known**
   - Either mergeable now, or explicitly included in an intentional order where earlier PRs are expected to unlock later PRs.

## Train sizes

Use conservative train sizing by conflict risk:

1. **3 PR overlapping cluster**
   - Shared files, parser/lexer core, or likely rebase churn.
2. **5 PR normal code cluster**
   - Typical independent code work with moderate overlap risk.
3. **10 PR docs/leaf non-overlapping cluster**
   - Docs or low-risk leaf changes with minimal conflict surface.

If uncertain, downgrade to a smaller train size.

## Train check procedure

Run from a clean worktree synced to current green master:

1. **Anchor to green base**
   - Confirm latest green master run and capture exact base SHA.
2. **Build train plan**
   - Write candidate list in merge order with per-PR recorded head SHA.
3. **Apply/simulate in order (preferred locally in temp branch)**
   - Sequentially cherry-pick/merge PR heads in planned order to validate composability.
4. **Run required checks on composed train state**
   - Conflict markers:
     - `just check-conflict-markers`
   - Formatting:
     - `cargo xtask fmt --check`
   - Fast gate receipt:
     - `cargo xtask gates --tier pr-fast --base origin/master --receipt`
5. **Only after checks pass**
   - Merge PRs in the same tested order.

## Stop conditions (hard)

Stop the train immediately on any of the following:

1. **Conflict** while applying/simulating order
2. **Stale SHA** (candidate head changed since train formation)
3. **Failed check** (conflict marker, fmt, or pr-fast failure)
4. **Unexpected skip** (required lane skipped without policy justification)
5. **Red master** (base is no longer green)

On stop: publish receipt, remove unsafe candidates from active train, reform from latest green master.

## Receipt / output format

Each train run must produce a markdown or JSON receipt with:

- base branch + base SHA
- timestamp
- candidates (PR number, recorded head SHA)
- planned order
- checks executed (command + pass/fail/skip)
- final verdict (`PASS`/`STOP`)
- stop reason (if stopped)

Suggested artifact path:

- `target/receipts/merge-train-<timestamp>.md`
- or `target/receipts/merge-train-<timestamp>.json`

## Minimum operator command set

```bash
# 1) Snapshot queue and reconcile label state if needed
cargo xtask queue snapshot --out target/receipts/queue-state.json
cargo xtask reconcile-queue --dry-run

# 2) Record anchor SHA
git rev-parse origin/master

# 3) Required train checks on composed order
just check-conflict-markers
cargo xtask fmt --check
cargo xtask gates --tier pr-fast --base origin/master --receipt
```

## Notes

- This protocol complements existing CI and queue tooling (`queue snapshot`, `queue health`, `reconcile-queue`) rather than replacing them.
- It preserves current review/label gates and only adds a deterministic pre-merge batch discipline.
