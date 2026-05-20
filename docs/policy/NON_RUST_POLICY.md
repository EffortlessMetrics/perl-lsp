# Non-Rust File Policy

The doctrine is in [FILE_POLICY.md](FILE_POLICY.md). This document is the
operational detail for the **non-Rust allowlist** — the primary ledger that
declares which non-Rust files are permitted in `perl-lsp` and on what terms.

## Source of truth

```text
policy/non-rust-allowlist.toml   # the active allowlist
policy/non-rust-debt.toml        # uncertain entries pending classification
docs/policy/NON_RUST_INVENTORY.md  # generated inventory (PR 3)
```

Reader / writer:

```text
cargo xtask non-rust inventory        # emit Markdown + JSON inventory
cargo xtask non-rust propose          # propose entries for unallowlisted files
cargo xtask non-rust validate-policy  # validate allowlist/debt TOML schema
cargo xtask non-rust migration-candidates  # find tooling candidates to migrate into Rust-owned surfaces
cargo xtask check-file-policy         # enforce the allowlist
```

## Schema

Every entry under `[[allow]]` carries the following fields. The [file-policy
checker](POLICY_ALLOWLISTS.md) enforces these.

```toml
[[allow]]
id              = "non-rust-<short-stable-id>"
glob            = "<repo-relative glob>"   # or `path = "<exact-path>"`
kind            = "<category>"
language        = "<primary language>"
surface         = "<product/repo surface>"
classification  = "<production|test|tooling|config|documentation>"
owner           = "<team-or-area>"
reason          = "<why this must be non-Rust>"
covered_by      = ["<test/check/lint that catches regressions>", ...]
created         = "YYYY-MM-DD"
review_after    = "YYYY-MM-DD"
expires         = "YYYY-MM-DD"        # optional; required for temporary entries
broad_glob_reason = "..."             # required when the matcher is broad
retired         = false               # optional; true keeps the receipt for history
```

### Field semantics

- **`glob` vs `path`.** Use `path` for an exact file. Use `glob` for a tree
  or extension. A glob entry must include `**`, `*`, or `?` to be considered
  a glob; otherwise the checker treats it as a typo.
- **`kind`.** A short category. The current vocabulary:
  `documentation`, `language_fixture`, `editor_extension`,
  `native_parser_binding`, `ci_declarative`, `ci_policy_config`,
  `ci_tooling`, `release_metadata`, `asset`, `config`, `generated`. Add a
  new `kind` only when an existing one is misleading.
- **`language`.** The dominant language. `mixed` is allowed for trees that
  legitimately span languages (e.g., `scripts/**`).
- **`surface`.** Where this lives in product terms — `docs`, `parser`,
  `editor`, `ci`, `tooling`, `release`. Used to group the policy report.
- **`classification`.** The blast radius if this surface breaks.
  `production` (ships to users), `test` (parser/LSP fixtures),
  `tooling` (developer / CI helpers), `config` (declarative settings),
  `documentation` (prose).
- **`owner`.** Stable identifier — `parser`, `editor/vscode`, `release/ci`,
  `runtime/subprocess`, `docs`. Avoid individual usernames.
- **`reason`.** One sentence answering "why is this not Rust?"
- **`covered_by`.** Concrete commands or check names that catch regressions
  on this surface. The checker requires at least one entry for any
  `production`, `test`, or `tooling` classification.
- **`review_after`.** A date the next maintainer should re-justify the
  entry. Default cadence: 30 days for new entries, 90–180 days for stable
  entries.
- **`expires`.** Required when `kind` describes something temporary
  (compatibility wrapper, migration shim). Absence implies durable.
- **`broad_glob_reason`.** Required when the glob covers a broad tree —
  e.g., `docs/**`, `scripts/**`, `.ci/**`. Explains why the breadth is
  acceptable and points at the companion policies that cover risky drift.
- **`retired`.** When set `true`, the entry is kept as a historical
  receipt but no longer matches files. The strict-mode checker still
  flags retired entries that have any matching files.

## Worked examples

### Documentation tree

```toml
[[allow]]
id = "non-rust-docs"
glob = "docs/**"
kind = "documentation"
language = "markdown"
surface = "docs"
classification = "documentation"
owner = "docs"
reason = "Project documentation, release notes, implementation plans, status pages, and policy prose."
covered_by = [
  "cargo xtask check-file-policy",
  "cargo xtask markdown-links",
]
broad_glob_reason = "Docs are intentionally tree-shaped and non-executable; executable/process/network/generated checks cover risky drift."
created = "2026-05-07"
review_after = "2026-08-07"
```

This is acceptable because `docs/**` is broad but the companion policies
narrow the risk:

- The executable allowlist forbids `+x` files in `docs/**`.
- The process and network allowlists forbid `Command::new` / network calls
  outside of `crates/perl-subprocess-runtime` and `xtask`.
- The generated allowlist names the specific status pages.

### Perl fixture

```toml
[[allow]]
id = "non-rust-perl-fixtures"
glob = "**/*.pl"
kind = "language_fixture"
language = "perl"
surface = "fixtures"
classification = "test"
owner = "parser/lsp-fixtures"
reason = "Perl source files are first-class parser, semantic-analysis, LSP, DAP, and corpus fixtures."
covered_by = [
  "cargo test --workspace",
  "cargo xtask parser-corpus-sweep --enforce --receipt",
]
created = "2026-05-07"
review_after = "2026-11-07"
```

`**/*.pl` is broad, but Perl source files **are the corpus the LSP parses**.
The receipt makes that explicit.

### Editor extension

```toml
[[allow]]
id = "non-rust-vscode-extension"
glob = "vscode-extension/**"
kind = "editor_extension"
language = "typescript"
surface = "editor"
classification = "production"
owner = "editor/vscode"
reason = "VS Code extension client, packaging, and managed-binary behavior require the VS Code extension ecosystem."
covered_by = [
  "vscode-managed-binary-smoke",
  "vscode-published-extension-smoke",
  "cargo xtask check-file-policy",
]
created = "2026-05-07"
review_after = "2026-08-07"
```

Production surface, owned by `editor/vscode`, covered by two real CI smokes.

### Temporary helper

```toml
[[allow]]
id = "non-rust-release-compat-wrapper"
path = "scripts/release-compat-wrapper.sh"
kind = "release_tooling"
language = "shell"
surface = "release"
classification = "tooling"
owner = "release/ci"
reason = "Bridges CI to a release tool that does not yet have an xtask command."
covered_by = ["cargo xtask check-executable-files"]
created = "2026-05-07"
review_after = "2026-06-07"
expires = "2026-08-07"
```

`expires` is set because the answer here is "migrate to `xtask`," not "keep
forever." The checker rejects entries past their expiry date.

## Common rejections

Reviewers should reject:

- **Glob has no matchers.** `glob = "scripts/release.sh"` is a typo —
  use `path` for exact files.
- **`covered_by = []`** for a `production` / `test` / `tooling` entry.
  Documentation entries may legitimately have an empty cover list.
- **`broad_glob_reason` missing** for any glob whose first segment is `**`,
  whose second segment is `**`, or whose tree includes more than one
  language family.
- **`owner = "TODO"`** in the active allowlist. `TODO` is allowed in
  proposals (`cargo xtask non-rust propose`) and in
  `policy/non-rust-debt.toml`, but not in the active ledger.
- **`reason` that is a tautology.** "This is markdown because it is
  markdown" is not a reason. The reason should explain why this surface
  exists at all.
- **Adding the same path under two `id`s.** Duplicate matchers without an
  explicit override break the proposal command.

## Lifecycle

```text
proposed (target/policy/non-rust-proposed-allowlist.toml, generated)
   ↓ owner classifies, edits, fills TODOs
debt (policy/non-rust-debt.toml, time-boxed)
   ↓ owner agrees on permanent shape
active (policy/non-rust-allowlist.toml)
   ↓ surface goes away
retired (active, retired = true, kept as receipt)
   ↓ historical retention threshold
removed (deleted from active)
```

Most entries skip the `debt` step and go straight from `proposed` to
`active`. The debt ledger exists for cases where ownership is genuinely
ambiguous and a maintainer wants a tracked place for the question.

## Cadence

| Trigger                                          | Action                                        |
| ------------------------------------------------ | --------------------------------------------- |
| New non-Rust file in PR                          | `cargo xtask check-file-policy` fails; author runs `cargo xtask non-rust propose` and adds an entry. |
| `review_after` date passes                       | Strict mode flags; owner re-justifies, advances date, or removes the entry. |
| `expires` date passes                            | Hard fail in any blocking mode. Owner removes or replaces the entry. |
| Surface goes away                                | Owner sets `retired = true`. Strict mode flags any remaining matches. |
| Allowlist drift detected on master               | Tooling-debt scout files an issue; check-file-policy bumped to strict for that surface. |

## What this policy does *not* do

- It does not gate Clippy lint exceptions — that is `policy/clippy-debt.toml`.
- It does not gate workflow-trigger linting — that is the existing
  workflow-trigger policy.
- It does not gate `Cargo.lock` reproducibility — that is
  `cargo check --locked --workspace`.
- It does not enforce *content* — only *existence*. A `docs/foo.md`
  matching the docs allowlist is permitted; whether the prose is correct
  is a review concern, not a policy one.

## See also

- [FILE_POLICY.md](FILE_POLICY.md) — the overarching doctrine.
- [POLICY_ALLOWLISTS.md](POLICY_ALLOWLISTS.md) — every ledger.
- [Rollout plan](../ci/perl-lsp-ci-policy-rollout.md) — the 11-PR sequence.
- [NON_RUST_LADDER.md](NON_RUST_LADDER.md) — builder-ready remaining ladder
  with tracking issues for every row (rollout PRs 04-11, inventory
  classification rows A-I, tightening rows J-K).
