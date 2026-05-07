# File Policy

> Rust by default. Non-Rust by receipt. Risky behavior by companion policy.

This is the doctrine that governs which files are allowed in `perl-lsp` and how.
The mechanics — TOML ledgers, `xtask` checkers, CI receipts — come in later
PRs of [the file-policy rollout](../ci/perl-lsp-ci-policy-rollout.md). This
document is the rule the mechanics enforce.

## The rule

Three lines:

1. **Rust and `cargo xtask` are the default construction material** for repo
   automation, policy checks, test orchestration, release checks, fixture
   runners, and production logic.
2. **Non-Rust files are allowed when they are a real product, platform,
   fixture, documentation, generated, or integration surface.** Markdown docs,
   GitHub Actions YAML, Perl source fixtures, the VS Code extension, the
   tree-sitter C bindings, and `Cargo.lock` are all legitimate.
3. **They are not allowed anonymously.** Every non-Rust surface needs an
   explicit allowlist entry with owner, reason, surface classification, and
   coverage.

## Why

`perl-lsp` already has the right substrate — Rust 2024 with strict
panic-family lints, an `xtask` crate that owns CI receipts and policy lints,
and a gate-receipt model where every CI step produces a structured artifact.
What it has been missing is the layer that says *which* file kinds may exist
in the tree and *who* is on the hook when they regress.

Three failure modes this policy prevents:

- **Anonymous shell scripts** that nobody owns, nobody tests, and that quietly
  carry credentials, network access, or process spawning logic.
- **Drift from the receipt model** — a Python helper added "temporarily" and
  never migrated to `xtask`, so its behavior is invisible to gate receipts.
- **Broad globs without justification** — `**/*.sh` covered by nothing,
  reviewed by no one, expiring never.

The rule does not say "no scripts" or "no Python." It says: if a non-Rust
file exists, the repo holds a receipt explaining who owns it, why it must be
non-Rust, and what test or check covers it.

## What counts as "non-Rust"

For policy purposes, a file is **Rust** if it is one of:

- `*.rs` source files,
- `Cargo.toml` / `Cargo.lock` / `rust-toolchain*` manifests,
- `target/**` build artifacts (gitignored, never tracked).

Everything else is non-Rust and needs an allowlist receipt:

| Class                 | Examples                                                             |
| --------------------- | -------------------------------------------------------------------- |
| Documentation         | `docs/**`, `*.md`, `README*`, `CHANGELOG*`, `RELEASE_HISTORY*`       |
| CI / config           | `.github/workflows/*.yml`, `.ci/**`, `policy/*.toml`, `deny.toml`    |
| Perl fixtures         | `**/*.pl`, `**/*.pm`, `**/*.t`, `test_corpus/**`                     |
| Editor extensions     | `vscode-extension/**`, `editors/vscode/**`                           |
| Native parser bindings| `crates/tree-sitter-perl-c/**`                                       |
| CI / release scripts  | `scripts/**` (legacy compatibility helpers; migrating to `xtask`)    |
| Generated artifacts   | `Cargo.lock`, status pages under `docs/project/status/**`            |
| Assets                | images, icons, web assets                                            |

Each class lands in `policy/non-rust-allowlist.toml` with its own entry. The
[non-Rust policy doc](NON_RUST_POLICY.md) details the schema and review
cadence.

## What "allowed by receipt" means

Every entry in `policy/non-rust-allowlist.toml` is a **receipt**, not an
escape hatch. A receipt has:

| Field            | Required? | Meaning                                                                  |
| ---------------- | :-------: | ------------------------------------------------------------------------ |
| `id`             |    yes    | Stable identifier for the entry. Cited by the policy report and CI logs. |
| `glob` / `path`  |    yes    | The matcher. Repo-relative, no leading `./`, no Windows backslashes.     |
| `kind`           |    yes    | Category (`documentation`, `language_fixture`, `editor_extension`, …).   |
| `language`       |    yes    | Primary language (`markdown`, `yaml`, `perl`, `typescript`, …).          |
| `surface`        |    yes    | Where this lives in product terms (`docs`, `parser`, `editor`, `ci`, …). |
| `classification` |    yes    | `production` / `test` / `tooling` / `config` / `documentation`.          |
| `owner`          |    yes    | Team / area on the hook for regressions.                                 |
| `reason`         |    yes    | Why this surface must be non-Rust.                                       |
| `covered_by`     |    yes    | Tests / checks / lints that catch regressions.                           |
| `created`        |    yes    | Date the entry was added (`YYYY-MM-DD`).                                 |
| `review_after`   |    yes    | When the entry should be re-justified.                                   |
| `expires`        |    no     | Required for **temporary** exceptions; absent means durable.             |
| `broad_glob_reason` | conditional | Required when the matcher covers a broad tree (e.g., `docs/**`). |
| `retired`        |    no     | Set to `true` when the entry is intentionally unused but kept for history. |

The checker fails when any required field is missing, when a broad glob has
no `broad_glob_reason`, or when an entry has expired.

## Companion policies

The non-Rust allowlist answers **where non-Rust may exist**. It does not
answer:

- whether a generated file is intentional (`Cargo.lock` is; an accidental
  `report.json` is not),
- whether a tracked file is executable on disk (a stray `+x` on a fixture
  is a smell),
- whether a new package manager appeared (`package.json`, `pyproject.toml`,
  `Gemfile`),
- whether a workflow surface is intentional and required,
- whether code spawns subprocesses,
- whether code reaches the network.

Those are tracked by **companion policies**:

| Policy                  | Ledger                                       | Question it answers              |
| ----------------------- | -------------------------------------------- | -------------------------------- |
| Generated allowlist     | `policy/generated-allowlist.toml`            | Which generated files belong?    |
| Executable allowlist    | `policy/executable-allowlist.toml`           | Which tracked files are `+x`?    |
| Dependency surface      | `policy/dependency-surface-allowlist.toml`   | Which dep manifests exist?       |
| Workflow surface        | `policy/workflow-allowlist.toml`             | Which CI workflows exist, who owns them, are they required? |
| Process allowlist       | `policy/process-allowlist.toml`              | Where may we spawn subprocesses? |
| Network allowlist       | `policy/network-allowlist.toml`              | Where may we contact the network? |

Companion policies are *narrower* than the file allowlist on purpose. A
broad `docs/**` allowlist entry is fine. A broad "subprocesses anywhere"
allowance is not.

See [POLICY_ALLOWLISTS.md](POLICY_ALLOWLISTS.md) for the full set.

## Three modes

Every checker has three modes. Promotion only happens after the baseline is
clean.

| Mode                 | Behavior                                              |
| -------------------- | ----------------------------------------------------- |
| `advisory`           | Write the report; never fail.                         |
| `blocking-allowlist` | Fail unallowlisted files, expired entries, malformed entries, missing required fields. |
| `blocking-strict`    | Also fail stale `review_after` dates, unused entries, and unjustified broad globs. |

The rollout introduces each policy at `advisory`, lets the inventory settle,
then promotes once owners are assigned and noise is gone. Strict mode is
reserved for policies whose baseline is fully owned.

## Anti-patterns

Reviewers and agents should reject these patterns:

- **Adding `**/*` globs to silence the checker.** A broad glob without a
  `broad_glob_reason` and a narrow companion policy is a silent waiver.
- **Setting `owner = "TODO"` and merging.** Proposed entries from
  `cargo xtask non-rust propose` ship with `TODO` markers as a **prompt**,
  not an answer. Reviewers must replace them.
- **Adding a script "for now."** If `scripts/<helper>.sh` is the right
  answer for the next 90 days, write the entry with `expires` set 90 days
  out. If it is the right answer forever, migrate it to `xtask` first.
- **Disabling the gate.** Mode promotions go through PR review;
  demotions do too. The gate is not a knob individual contributors flip.

## Relationship to existing infrastructure

This policy does **not** replace:

- Strict panic-family Clippy lints in the workspace `Cargo.toml`.
- The existing `xtask check-lint-policy` for Clippy ledger drift.
- The existing workflow-trigger lint and required-checks policy.
- Gate receipts and the `.ci/gate-policy.yaml` model.

It **extends** the existing policy stack with a layer that asks "is this
file even supposed to be in this repo, and who is on the hook for it?"
before any Clippy lint or workflow lint runs.

## Roadmap

- [Rollout plan](../ci/perl-lsp-ci-policy-rollout.md) — the 11 PRs that
  build out this policy.
- [POLICY_ALLOWLISTS.md](POLICY_ALLOWLISTS.md) — every ledger and what it
  covers.
- [NON_RUST_POLICY.md](NON_RUST_POLICY.md) — the non-Rust allowlist in
  detail: schema, examples, common rejections.
