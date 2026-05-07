# Non-Rust File Inventory (seed)

> This is the **seed** inventory written by hand at the time the
> non-Rust allowlist was introduced. Once `cargo xtask non-rust inventory`
> lands (PR 3 of the file-policy rollout), this document is regenerated
> automatically and the hand-written seed should be replaced by the
> generator's output.

**Source counts** (at seed time, from `git ls-files`):

```text
8298  total tracked files
1842  *.md (documentation)
1451  Perl fixtures (*.pl + *.pm + *.t)
 239  *.json (CI metrics, schemas, fixtures)
 128  *.sh
 111  *.snap (cargo-insta snapshots)
  79  *.yml + 12 *.yaml (CI workflows, configs)
  69  *.toml (policy, manifests, configs)
  46  *.ts (VS Code extension)
  41  *.py (CI scripts)
  14  *.h
  14  *.svg
  13  *.js
   6  *.c
```

## Coverage by allowlist entry

The seed allowlist (`policy/non-rust-allowlist.toml`) declares **55
entries** covering the surface above. The intent is to reach >95% coverage
at seed time so the eventual blocking-allowlist mode (PR 4) is tractable.

### Documentation (`surface = "docs"`)

| Entry id                          | Matcher                                  | Notes                                         |
| --------------------------------- | ---------------------------------------- | --------------------------------------------- |
| `non-rust-docs-tree`              | `docs/**`                                | All project documentation.                    |
| `non-rust-root-governance-docs`   | `*.md`                                   | README, CHANGELOG, ROADMAP, etc.              |
| `non-rust-license-files`          | `LICENSE-*`                              | Apache-2.0 + MIT.                             |
| `non-rust-agent-config-files`     | `{AGENTS,AIDER,CLAUDE,CRUSH,GEMINI}.md`  | Agent-runtime project prompts.                |
| `non-rust-perl-lsp-example-config`| `.perl-lsp.toml.example`                 | User-facing config example.                   |

### CI / GitHub (`surface = "ci"`)

| Entry id                    | Matcher                       | Notes                          |
| --------------------------- | ----------------------------- | ------------------------------ |
| `non-rust-github-workflows` | `.github/workflows/*.yml`     | Required CI conveyor.          |
| `non-rust-github-actions`   | `.github/actions/**`          | Composite Actions.             |
| `non-rust-github-policy`    | `.github/**`                  | Issue/PR templates, dependabot.|
| `non-rust-ci-config`        | `.ci/**`                      | Gate policy, blockers, etc.    |
| `non-rust-policy-ledgers`   | `policy/**`                   | TOML policy stack.             |
| `non-rust-receipts-tree`    | `.receipts/**`                | Persisted CI receipts.         |
| `non-rust-ops-tree`         | `.ops-perl-lsp/**`            | Swarm orchestration state.     |
| `non-rust-trivyignore`      | `.trivyignore`                | Trivy security suppressions.   |

### Perl fixtures (`surface = "fixtures"`)

| Entry id                | Matcher        | Notes                                      |
| ----------------------- | -------------- | ------------------------------------------ |
| `non-rust-perl-fixtures`| `**/*.pl`      | 331 files; the corpus the LSP parses.      |
| `non-rust-perl-modules` | `**/*.pm`      | 1115 files; module-resolution fixtures.    |
| `non-rust-perl-tests`   | `**/*.t`       | Perl test files used as parser fixtures.   |

### Test snapshots and regression seeds

| Entry id                          | Matcher                       | Notes                          |
| --------------------------------- | ----------------------------- | ------------------------------ |
| `non-rust-insta-snapshots`        | `**/*.snap`                   | cargo-insta snapshots.         |
| `non-rust-insta-pending-snapshots`| `**/*.snap.new`               | Pending review.                |
| `non-rust-proptest-regressions`   | `**/*.proptest-regressions`   | proptest regression seeds.     |

### Native parser

| Entry id                       | Matcher                              | Notes                              |
| ------------------------------ | ------------------------------------ | ---------------------------------- |
| `non-rust-tree-sitter-c`       | `crates/tree-sitter-perl-c/**`       | C tree-sitter parser source.       |
| `non-rust-tree-sitter-grammar` | `crates/tree-sitter-perl/**`         | Grammar / queries / corpus shared. |

### Editor extension

| Entry id                    | Matcher              | Notes                                 |
| --------------------------- | -------------------- | ------------------------------------- |
| `non-rust-vscode-extension` | `vscode-extension/**`| TypeScript client + packaging.        |
| `non-rust-vscode-config`    | `.vscode/**`         | Workspace VS Code settings.           |

### Build / release tooling

| Entry id                       | Matcher / path           | Notes                                |
| ------------------------------ | ------------------------ | ------------------------------------ |
| `non-rust-justfile`            | `justfile`               | Repo task runner.                    |
| `non-rust-justfiles-tree`      | `.justfiles/**`          | Modular fragments.                   |
| `non-rust-flake-nix`           | `flake.nix`              | Nix flake for CI/dev shell.          |
| `non-rust-flake-lock`          | `flake.lock`             | Pinned flake inputs (generated).     |
| `non-rust-dist-workspace`      | `dist-workspace.toml`    | cargo-dist config.                   |
| `non-rust-cliff-toml`          | `cliff.toml`             | git-cliff changelog config.          |
| `non-rust-deny-toml`           | `deny.toml`              | cargo-deny rules.                    |
| `non-rust-codecov-yml`         | `codecov.yml`            | Codecov reporter config.             |
| `non-rust-ripr-toml`           | `ripr.toml`              | RIPR static-analysis config.         |
| `non-rust-features-toml`       | `features.toml`          | LSP feature catalog.                 |
| `non-rust-bacon-toml`          | `bacon.toml`             | Bacon developer-loop config.         |
| `non-rust-rust-analyzer-toml`  | `rust-analyzer.toml`     | rust-analyzer workspace config.      |
| `non-rust-clippy-toml`         | `clippy.toml`            | Workspace Clippy config.             |
| `non-rust-cargo-config`        | `.cargo/**`              | Cargo configuration tree.            |
| `non-rust-cargo-semver-checks` | `.cargo-semver-checks.toml` | semver-checks config.             |

### Container / packaging

| Entry id                    | Matcher / path        | Notes                                |
| --------------------------- | --------------------- | ------------------------------------ |
| `non-rust-docker-tree`      | `.docker/**`          | Dockerfiles + container scripts.     |
| `non-rust-docker-compose`   | `docker-compose.yml`  | Compose definition.                  |
| `non-rust-homebrew-formula` | `Formula/**`          | Homebrew formula.                    |

### Scripts / installers

| Entry id                     | Matcher / path | Notes                                 |
| ---------------------------- | -------------- | ------------------------------------- |
| `non-rust-ci-scripts-tree`   | `scripts/**`   | 128 .sh + 41 .py legacy/compat helpers. |
| `non-rust-install-shell`     | `install.sh`   | One-line installer (Linux/macOS).     |
| `non-rust-install-powershell`| `install.ps1`  | One-line installer (Windows).         |

### Editor / IDE / agent

| Entry id                   | Matcher                            | Notes                                |
| -------------------------- | ---------------------------------- | ------------------------------------ |
| `non-rust-claude-config`   | `.claude/**`                       | Claude Code agent config.            |
| `non-rust-spec-tree`       | `.spec/**`                         | Spec-planner artifacts.              |
| `non-rust-other-agents`    | `.{aider,roo,kiro,hermes,jules}/**`| Other-agent configs.                 |
| `non-rust-aider-conf`      | `.aider.conf.yml`                  | Aider config.                        |

### Repo metadata

| Entry id                  | Matcher / path        | Notes                                  |
| ------------------------- | --------------------- | -------------------------------------- |
| `non-rust-gitignore-family` | `**/.gitignore`     | Per-tree git ignore files.             |
| `non-rust-gitattributes`  | `.gitattributes`      | Git attributes (EOL).                  |
| `non-rust-markdownlint`   | `.markdownlint.json`  | markdownlint config.                   |
| `non-rust-cspell`         | `cspell.json`         | cspell dictionary.                     |
| `non-rust-tokeignore`     | `.tokeignore`         | tokei ignore.                          |
| `non-rust-pre-commit-hooks` | `.pre-commit-hooks.yaml` | pre-commit hook declarations.    |
| `non-rust-config-tree`    | `.config/**`          | XDG-style per-tool config.             |

## Validation

```bash
python3 scripts/policy/validate_non_rust_allowlist.py
```

Expected output:

```text
OK: validated 55 allow entries and 0 debt entries.
```

## Coverage caveats

The seed allowlist is wide but not exhaustive. The Rust checker
(`cargo xtask check-file-policy`, PR 4) will surface uncovered files via
the `non-rust propose` command (PR 5). When that runs against the
current repo, expect proposals for:

- Top-level files not yet listed (any `.gitignore`-style metadata that
  does not match `**/.gitignore`).
- New crates' fixture trees if they differ from the existing patterns.
- Any future test corpus addition (`test_corpus/<new-tree>/`).

## Update cadence

| Trigger                               | Action                                              |
| ------------------------------------- | --------------------------------------------------- |
| New crate added                       | Verify its non-Rust files are covered.              |
| New top-level file added              | Add an allowlist entry or set the file's owner expectation. |
| `review_after` date passes for a tree | Re-justify the entry; advance the date.             |
| Surface goes away                     | Set `retired = true` rather than deleting the entry.|
| `cargo xtask non-rust propose` finds  | Triage the proposal: classify into the active       |
| unallowlisted files                   | ledger, debt ledger, or remove the file.            |

## See also

- [FILE_POLICY.md](FILE_POLICY.md) — the doctrine.
- [NON_RUST_POLICY.md](NON_RUST_POLICY.md) — the schema in detail.
- [POLICY_ALLOWLISTS.md](POLICY_ALLOWLISTS.md) — the catalog of all seven ledgers.
- [Rollout plan](../ci/perl-lsp-ci-policy-rollout.md) — the 11-PR sequence.
