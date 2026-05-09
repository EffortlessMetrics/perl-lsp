# Policy Allowlists

`perl-lsp` runs a layered policy stack. Each layer is a TOML ledger plus an
`xtask` checker plus a CI receipt. The doctrine is in
[FILE_POLICY.md](FILE_POLICY.md). This document is the catalog: what each
ledger covers, where the checker lives, and how the layers compose.

## Why layered

A single broad allowlist would force trade-offs that hurt safety. The
non-Rust allowlist may legitimately include `docs/**` and `scripts/**` —
broad globs in the file dimension. But a broad allowance for **subprocess
spawning** or **network access** is never legitimate, even in `scripts/**`.

The layers separate those concerns:

```text
non-rust       → which files may exist?
generated      → which generated artifacts belong?
executable     → which tracked files are +x?
dependency     → which dependency-manager manifests exist?
workflow       → which CI workflows exist, owned by whom?
process        → where may we spawn subprocesses?
network        → where may we contact the network?
```

A file in `scripts/**` may be allowed by the non-Rust ledger and **still
fail** the executable, process, or network ledger. That is the point: a
broad file-existence allowance is constrained by narrow risky-behavior
allowances.

## The seven ledgers

| Ledger                                   | xtask command                              | Question                                                  | Default mode |
| ---------------------------------------- | ------------------------------------------ | --------------------------------------------------------- | ------------ |
| `policy/non-rust-allowlist.toml`         | `cargo xtask check-file-policy`            | Which non-Rust files are permitted?                       | advisory → blocking-allowlist |
| `policy/generated-allowlist.toml`        | `cargo xtask check-generated`              | Which generated artifacts belong in the tree?             | advisory → blocking-allowlist |
| `policy/executable-allowlist.toml`       | `cargo xtask check-executable-files`       | Which tracked files may carry the `+x` bit?               | advisory → blocking-allowlist |
| `policy/dependency-surface-allowlist.toml` | `cargo xtask check-dependency-surfaces`    | Which dependency-manager manifests are permitted?         | advisory → blocking-allowlist |
| `policy/workflow-allowlist.toml`         | `cargo xtask check-workflow-surfaces`      | Which `.github/workflows/*.yml` exist, owned by whom?     | advisory → blocking-allowlist |
| `policy/process-allowlist.toml`          | `cargo xtask check-process-policy`         | Where in the codebase may we spawn subprocesses?          | advisory → blocking-allowlist |
| `policy/network-allowlist.toml`          | `cargo xtask check-network-policy`         | Where may we contact the network?                         | advisory → blocking-allowlist |

A unified report aggregates the seven:

```bash
cargo xtask policy-report
# emits target/policy/policy-report.md and policy-report.json
```

## Common schema

Every ledger shares this preamble:

```toml
schema_version = 1
policy = "<ledger-name>"
owner = "EffortlessMetrics"
status = "advisory"   # or "active"
updated = "YYYY-MM-DD"
```

Every entry under `[[allow]]` shares this minimum:

```toml
[[allow]]
id          = "<ledger-prefix>-<short-id>"
owner       = "<team-or-area>"
reason      = "<why this exception exists>"
created     = "YYYY-MM-DD"
review_after = "YYYY-MM-DD"
covered_by  = ["<command-or-check>", ...]
```

Layer-specific fields extend this base.

## Per-ledger detail

### `policy/non-rust-allowlist.toml`

Detailed in [NON_RUST_POLICY.md](NON_RUST_POLICY.md). The primary ledger:
which non-Rust files exist and on what terms. Adds `glob`/`path`, `kind`,
`language`, `surface`, `classification`, optional `expires` and
`broad_glob_reason`.

### `policy/generated-allowlist.toml`

Lists files that are **deliberately generated** and committed. Each entry
declares the generator and the regeneration command.

```toml
[[allow]]
id = "generated-cargo-lock"
path = "Cargo.lock"
kind = "lockfile"
generated_by = "cargo"
regenerate = "cargo update --workspace"
owner = "rust/dependencies"
reason = "Workspace lockfile pins dependency graph for reproducible CI and release builds."
covered_by = [
  "cargo check --locked --workspace",
  "cargo xtask check-generated",
]
created = "2026-05-07"
review_after = "2026-08-07"
```

The strict-mode checker fails when:

- A tracked file matches no entry but contains generator markers
  (`# AUTO-GENERATED`, `# Do not edit`, etc.).
- A `regenerate` command is missing for `kind = "lockfile"` /
  `kind = "status_page"` entries.

### `policy/executable-allowlist.toml`

Tracks files with the `+x` bit set in the index. Default state should be
**near-empty**. A `+x` shell script in `scripts/**` requires its own entry.

```toml
[[allow]]
id = "executable-release-helper"
path = "scripts/release-helper.sh"
kind = "release_tooling"
owner = "release/ci"
reason = "Temporary release compatibility wrapper; migrate to cargo xtask."
covered_by = ["cargo xtask check-executable-files"]
created = "2026-05-07"
review_after = "2026-06-07"
expires = "2026-08-07"
```

The checker walks `git ls-files --eol`-style output (or `git
ls-files --stage` and inspects mode bits) to find `+x` files and rejects
any not in the allowlist.

### `policy/dependency-surface-allowlist.toml`

Catches the appearance of new package-manager manifests. The default
allowed set is small: root `Cargo.toml`, per-crate `Cargo.toml`, the VS
Code `package.json`. Anything else (a stray `pyproject.toml`, `Gemfile`,
`go.mod`) fails until receipted.

```toml
[[allow]]
id = "dependency-vscode-package-json"
path = "vscode-extension/package.json"
kind = "node_manifest"
owner = "editor/vscode"
reason = "VS Code extension packaging requires Node package metadata."
covered_by = [
  "vscode-managed-binary-smoke",
  "cargo xtask check-dependency-surfaces",
]
created = "2026-05-07"
review_after = "2026-08-07"
```

Strict mode also flags entries whose corresponding manifest file is no
longer present (drift in the other direction).

### `policy/workflow-allowlist.toml`

Catalogs every `.github/workflows/*.yml` with intent, ownership, and
required-or-not status. Pairs with the existing workflow-trigger lint and
required-checks policy under `.ci/policies/`.

```toml
[[allow]]
id = "workflow-ci"
path = ".github/workflows/ci.yml"
kind = "required_pr_gate"
owner = "release/ci"
reason = "Primary merge-blocking CI workflow with PR-fast, merge-gate shards, UX, memory, Windows, and aggregate status."
required = true
covered_by = [
  "cargo xtask workflow-trigger-lint",
  "cargo xtask workflow-policy-lint",
]
created = "2026-05-07"
review_after = "2026-08-07"
```

Adds rules:

- Every workflow must have an entry.
- `required = true` workflows must appear in
  `.ci/policies/required-checks.toml`.
- Workflows with `pull_request:` triggers must declare an `intent`.
- Label-triggered workflows must name the labels.
- Broad `pull_request: types: [...]` triggers without scope need a
  `broad_trigger_reason`.

### `policy/process-allowlist.toml`

Constrains where the codebase may spawn subprocesses. The pattern set
includes `Command::new`, `tokio::process::Command`, `std::process::Command`,
`spawn`, and shell forms (`exec`, `system`, `subprocess.run`).

```toml
[[allow]]
id = "process-subprocess-runtime"
path = "crates/perl-subprocess-runtime/src/**"
pattern = "Command"
owner = "runtime/subprocess"
reason = "Subprocess runtime is the explicit boundary for external Perl/tool execution."
covered_by = [
  "cargo test -p perl-subprocess-runtime",
  "cargo xtask check-process-policy",
]
created = "2026-05-07"
review_after = "2026-08-07"
```

The checker greps the source tree (Rust, shell, Python, TypeScript) for
process-spawning patterns and fails when a match exists outside the
allowlisted path/pattern combinations.

### `policy/network-allowlist.toml`

Constrains where the codebase may contact the network. Patterns include
`reqwest`, `ureq`, `TcpStream`, `UdpSocket`, `curl`, `wget`,
`Invoke-WebRequest`, `npm install`, `pip install`, `cargo install`.

```toml
[[allow]]
id = "network-release-publish"
path = "xtask/src/**"
pattern = "crates.io"
owner = "release/ci"
reason = "Release automation may contact crates.io only during explicit release/publish flows."
allowed_in = ["release", "workflow_dispatch"]
covered_by = ["cargo xtask check-network-policy"]
created = "2026-05-07"
review_after = "2026-08-07"
```

`allowed_in` is a list of GitHub Actions event names where the call is
permitted. The CI receipt cross-references the workflow event when
deciding whether to fail.

## Modes and promotion

Every checker accepts `--mode advisory|blocking-allowlist|blocking-strict`.
The rollout strategy is:

1. **Land at `advisory`.** Inventory the surface, generate the report,
   let owners assign themselves.
2. **Promote to `blocking-allowlist`.** Unallowlisted, expired, or
   malformed entries fail the gate. Stale `review_after` and unused
   entries are still warnings.
3. **Promote to `blocking-strict`.** Stale reviews fail. Unused entries
   fail. Broad globs without `broad_glob_reason` fail.

Each promotion is its own PR with the commit message `policy(<ledger>):
promote to <mode>`.

## Composition rules

- **A file allowed by `non-rust` may still fail `executable`,
  `dependency`, `workflow`, `process`, or `network`.** That is intentional.
- **A file rejected by `non-rust` does not run the other ledgers.** The
  non-Rust ledger is the first gate; if a file shouldn't exist, no other
  policy needs to consider it.
- **`generated` and `non-rust` overlap.** A generated `Cargo.lock` is
  Rust by language but generated by `cargo`; both ledgers list it.
  Removing one entry without the other is an error.
- **Companion ledgers reference each other in `covered_by`.** A `docs/**`
  entry in `non-rust` lists `cargo xtask check-executable-files` in its
  `covered_by` to make the composition explicit in the receipt.

## Per-PR enforcement

The full policy gate is one CI step:

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

It writes `target/receipts/file-policy.json` and the unified policy report
under `target/policy/`. The CI lane is wired into `.ci/gate-policy.yaml`
in PR 10 of the [rollout](../ci/perl-lsp-ci-policy-rollout.md).

## Anti-corruption rules

- **Never edit `target/policy/non-rust-proposed-allowlist.toml` and rename
  it.** The proposal exists for review; the active ledger is curated.
  Direct moves bypass the review gate.
- **Never set `mode = "advisory"` on a checker that was previously
  `blocking-allowlist` without a PR titled `policy(<ledger>): demote`
  and a maintainer review.**
- **Never delete an entry to "fix" a failing check.** If the surface is
  no longer present, set `retired = true` so the receipt is preserved.
  If the surface is still present and policy is wrong, fix the policy
  with a separate PR.
- **Never silence the gate by adding `**/*` to a non-Rust allowlist
  glob.** That is the textbook anti-pattern this stack exists to prevent.

## See also

- [FILE_POLICY.md](FILE_POLICY.md) — the doctrine.
- [NON_RUST_POLICY.md](NON_RUST_POLICY.md) — the primary ledger in
  detail.
- [Rollout plan](../ci/perl-lsp-ci-policy-rollout.md) — the 11-PR
  sequence that implements this catalog.
