# perl-lsp File-Policy Rollout

This is the multi-PR rollout that builds the file-policy stack described in
[FILE_POLICY.md](../policy/FILE_POLICY.md), [NON_RUST_POLICY.md](../policy/NON_RUST_POLICY.md),
and [POLICY_ALLOWLISTS.md](../policy/POLICY_ALLOWLISTS.md).

This is a **separate sequence** from the [CI economics rollout](perl-lsp-rollout-plan.md).
The CI economics work answers "how much does verification cost?" The file
policy work answers "what files belong in this repo, and who owns them?"
The two stacks compose: the CI economics gate runs the file-policy gate
as one of its pr-fast checks.

## Order

| PR | Name | Purpose | Mode introduced |
|---:|---|---|---|
| 01 | File-policy doctrine | Document `FILE_POLICY.md`, `NON_RUST_POLICY.md`, `POLICY_ALLOWLISTS.md`, this rollout. | n/a |
| 02 | Non-Rust TOML ledger | Add `policy/non-rust-allowlist.toml` and `policy/non-rust-debt.toml`. | n/a |
| 03 | `non-rust inventory` | `cargo xtask non-rust inventory` emits Markdown + JSON. | n/a |
| 04 | `check-file-policy` | `cargo xtask check-file-policy` enforces the allowlist. | advisory |
| 05 | `non-rust propose` | `cargo xtask non-rust propose` generates entries for unallowlisted files. | n/a |
| 06 | Generated / executable / dependency policies | Companion ledgers and checkers. | advisory |
| 07 | Process / network policies | Risky-behavior ledgers and checkers. | advisory |
| 08 | Workflow surface policy | `policy/workflow-allowlist.toml` + `check-workflow-surfaces`. | advisory |
| 09 | Unified policy report | `cargo xtask policy-report` aggregates the seven ledgers. | n/a |
| 10 | Wire into `.ci/gate-policy.yaml` | Add a `file_policy` gate emitting receipts. | promote to `blocking-allowlist` for owned ledgers |
| 11 | Strict-mode promotion | Promote `check-file-policy`, `check-generated`, `check-executable-files`, `check-dependency-surfaces`, `check-workflow-surfaces` to `blocking-strict`. | `blocking-strict` |

## Sequencing constraints

- **PR 02 must land before PR 04.** The checker reads the ledger.
- **PR 03 may land in parallel with PR 04** — inventory does not require
  the checker.
- **PR 05 depends on PR 04.** The proposer needs the checker's "what's
  unallowlisted" output.
- **PR 06, 07, 08 may land in any order after PR 04**, but each adds its
  own ledger and checker.
- **PR 09 depends on PR 06, 07, 08.** The unified report aggregates
  them.
- **PR 10 depends on PR 09.** The gate runs the unified report.
- **PR 11 depends on PR 10** plus a calibration window of clean baseline
  receipts.

## What each PR includes

### PR 01 — File-policy doctrine (this PR)

```text
docs/policy/FILE_POLICY.md
docs/policy/NON_RUST_POLICY.md
docs/policy/POLICY_ALLOWLISTS.md
docs/ci/perl-lsp-ci-policy-rollout.md
```

No code, no policy TOML, no checker. Doctrine before mechanics.

### PR 02 — Non-Rust TOML ledger

```text
policy/non-rust-allowlist.toml
policy/non-rust-debt.toml
docs/policy/NON_RUST_INVENTORY.md   # initial seed; PR 03 generates the live one
```

Acceptance:

```bash
python3 - <<'PY'
import pathlib, tomllib
for p in pathlib.Path("policy").glob("*non-rust*.toml"):
    tomllib.loads(p.read_text())
PY
```

### PR 03 — `cargo xtask non-rust inventory`

```text
xtask/src/tasks/file_policy.rs
xtask/src/tasks/mod.rs
xtask/src/main.rs
docs/policy/NON_RUST_INVENTORY.md
```

Outputs:

```text
target/policy/non-rust-inventory.md
target/policy/non-rust-inventory.json
```

Acceptance:

```bash
cargo check -p xtask --locked
cargo xtask non-rust inventory
test -f target/policy/non-rust-inventory.md
```

### PR 04 — `cargo xtask check-file-policy`

```text
xtask/src/tasks/file_policy.rs        # extended
xtask/tests/file_policy.rs
docs/policy/POLICY_ALLOWLISTS.md      # mark advisory mode active
```

Modes: `advisory` (default), `blocking-allowlist`, `blocking-strict`.

Acceptance:

```bash
cargo check -p xtask --locked
cargo test -p xtask file_policy
cargo xtask check-file-policy --mode advisory
```

### PR 05 — `cargo xtask non-rust propose`

```text
xtask/src/tasks/file_policy.rs        # extended
xtask/tests/file_policy_propose.rs
```

Outputs:

```text
target/policy/non-rust-proposed-allowlist.toml
target/policy/non-rust-proposal.md
```

Critical guarantee: **never mutates `policy/non-rust-allowlist.toml`**.

### PR 06 — Generated / executable / dependency policies

```text
policy/generated-allowlist.toml
policy/executable-allowlist.toml
policy/dependency-surface-allowlist.toml
xtask/src/tasks/generated_policy.rs
xtask/src/tasks/executable_policy.rs
xtask/src/tasks/dependency_surface_policy.rs
xtask/tests/generated_policy.rs
xtask/tests/executable_policy.rs
xtask/tests/dependency_surface_policy.rs
```

Each checker introduces in `advisory` mode.

### PR 07 — Process / network policies

```text
policy/process-allowlist.toml
policy/network-allowlist.toml
xtask/src/tasks/process_policy.rs
xtask/src/tasks/network_policy.rs
xtask/tests/process_policy.rs
xtask/tests/network_policy.rs
```

Search patterns include both Rust forms (`Command`, `reqwest`, `TcpStream`)
and script/workflow forms (`subprocess`, `curl`, `npm install`).

### PR 08 — Workflow surface policy

```text
policy/workflow-allowlist.toml
xtask/src/tasks/workflow_surface_policy.rs
xtask/tests/workflow_surface_policy.rs
docs/ci/workflow-policy.md
```

Pairs with the existing `cargo xtask workflow-trigger-lint` and
`workflow-policy-lint`. The new checker covers ownership and required-or-not
status; the existing lints cover triggers and required-checks composition.

### PR 09 — Unified policy report

```text
xtask/src/tasks/policy_report.rs
xtask/tests/policy_report.rs
docs/policy/POLICY_REPORT.md
```

Outputs:

```text
target/policy/policy-report.md
target/policy/policy-report.json
```

Sections: non-Rust files / generated / executable / dependency surfaces /
workflow surfaces / process / network / expired / stale reviews / unused /
broad globs / debt.

### PR 10 — Wire into `.ci/gate-policy.yaml`

```text
.ci/gate-policy.yaml
.github/workflows/ci.yml
docs/project/CI.md
docs/policy/POLICY_ALLOWLISTS.md     # mode promotion
```

The new gate runs:

```bash
cargo xtask check-file-policy            --mode blocking-allowlist
cargo xtask check-generated              --mode blocking-allowlist
cargo xtask check-executable-files       --mode blocking-allowlist
cargo xtask check-dependency-surfaces    --mode blocking-allowlist
cargo xtask check-workflow-surfaces      --mode blocking-allowlist
cargo xtask check-process-policy         --mode advisory
cargo xtask check-network-policy         --mode advisory
cargo xtask policy-report
```

Receipt path: `target/receipts/file-policy.json`. Receipt schema follows
the existing `.ci/receipt.schema.json`.

### PR 11 — Strict-mode promotion

```text
.ci/gate-policy.yaml
docs/policy/POLICY_ALLOWLISTS.md
```

Promotes the well-baselined ledgers to `blocking-strict`. Process and
network are typically last (the noisiest) and may stay at
`blocking-allowlist` for longer.

## Definition of done

The rollout is complete when **all** of the following are true:

- `policy/non-rust-allowlist.toml` exists and covers every tracked
  non-Rust file. `cargo xtask check-file-policy --mode blocking-strict`
  passes on master.
- `policy/non-rust-debt.toml` is empty or every entry has an owner and a
  near-term `review_after`.
- `cargo xtask non-rust inventory` emits Markdown + JSON inventory.
- `cargo xtask non-rust propose` emits proposed TOML without mutating
  the active ledger.
- `policy/generated-allowlist.toml`,
  `policy/executable-allowlist.toml`,
  `policy/dependency-surface-allowlist.toml`, and
  `policy/workflow-allowlist.toml` exist and are at least
  `blocking-allowlist`.
- `policy/process-allowlist.toml` and `policy/network-allowlist.toml`
  exist and are at least `blocking-allowlist`.
- `cargo xtask policy-report` emits Markdown + JSON.
- `.ci/gate-policy.yaml` declares a `file_policy` gate that produces a
  receipt.
- CI uploads policy receipts.
- Broad globs are justified.
- Expired entries fail the gate.
- Unused entries fail strict mode.
- New non-Rust files cannot land anonymously.

## Out of scope

This rollout does **not**:

- Replace the strict panic-family Clippy lints.
- Replace the existing `xtask check-lint-policy`, workflow-trigger lint,
  or workflow-policy lint.
- Replace gate receipts or the `.ci/gate-policy.yaml` model.
- Migrate `scripts/**` to `xtask` automatically. Migration is
  per-script, owner-driven, and tracked through individual entries with
  `expires` dates.
- Cover content correctness — see `cargo xtask markdown-links`,
  `cargo xtask validate-receipts`, and per-crate test suites.

## Rollout principles

- **Doctrine before mechanics.** PR 01 lands the rule. PRs 02+ implement it.
- **Advisory before blocking.** Every checker introduces at `advisory`.
  Promotions go through their own PR.
- **Companions narrow broad globs.** `docs/**` is allowed because the
  executable, generated, process, and network ledgers cover the risky
  drift.
- **No silencing.** Allowlist entries are receipts, not waivers.
- **Owner over individual.** `owner = "release/ci"` not
  `owner = "@steven"`. Areas outlast people.
- **Receipt over rule.** The TOML ledger is the durable artifact;
  any command-line output is regenerable.
