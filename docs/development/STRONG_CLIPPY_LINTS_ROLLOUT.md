# Strong Clippy Lints Rollout

> **Companion to** [`RUST_1_95_ROLLOUT.md`](RUST_1_95_ROLLOUT.md).
> That doc tracks the **Rust 1.95 toolchain + workspace-allow burn-down** rail.
> This doc tracks the **next workstream** — promote additional clippy lints
> from `allow` / unspecified to `warn` / `deny` across the workspace, beyond
> the 1.95-deweighting rail.
>
> Umbrella: **#8590**. Each row below is a builder-ready GitHub issue with
> the `strong-clippy-lints` label.

## Doctrine

- **One lint per PR.** Adjacent-line conflicts on `[workspace.lints.clippy]`
  cause rebase storms (see allow-burndown PRs #8511, #8520-#8523, #8538).
- **Move planned ledger entries to active in the same PR.** The
  `[[planned]]` entries in `policy/clippy-lints.toml` carry the rationale;
  promotion to `[[lint]]` keeps the rationale visible.
- **No bundled cleanup.** If a lint has more than ~10 call sites, file a
  dedicated burn-down sub-rail (see `str_to_string` row) before flipping
  the workspace `Cargo.toml`.
- **No bare `#[allow]`.** Use `#[expect(..., reason = "...")]` per
  `docs/CLIPPY_POLICY.md` § Suppression style. If a lint needs scoped
  debt, add an entry to `policy/clippy-debt.toml` with `expires`.

## Ladder

Each row is one PR. Branch names follow `chore/clippy-activate-<lint_name>`.

### Tier 1 — Zero-site activations (cheap, low risk)

| Issue | Lint | Tier | Target level | Sites |
| ----- | ---- | ---- | ------------ | ----- |
| #8601 | `clippy::manual_take` | 1.95 stable | warn | 0 |
| #8602 | `clippy::manual_pop_if` | 1.95 stable | warn | 0 |
| #8603 | `clippy::same_length_and_capacity` | 1.94 stable | deny | 0 |
| #8604 | `clippy::manual_ilog2` | 1.94 stable | warn | 0 |
| #8605 | `clippy::decimal_bitwise_operands` | 1.94 stable | warn | 0 |
| #8606 | `clippy::needless_type_cast` | 1.94 stable | warn | 0 |

These six can land in any order. Each is a two-line `Cargo.toml` edit plus
a `policy/clippy-lints.toml` planned -> active move. Zero call sites today.

### Tier 2 — Small activations (≤10 call sites)

| Issue | Lint | Sites | Notes |
| ----- | ---- | ----- | ----- |
| #8607 | `clippy::duration_suboptimal_units` | 5 lib | Refactor `from_<small>` -> larger unit at known sites in `perl-tdd-support`, `perl-lsp-rs-core`. |
| #8608 | `clippy::unnecessary_trailing_comma` | 3 lib + 2 test | Mechanical syntax cleanup in `perl-lsp-rs-core` navigation shadows and `perl-diagnostics` tests. |

### Tier 3 — Workspace-wide promotion of existing per-crate denies

| Issue | Lint | Status today |
| ----- | ---- | ------------ |
| #8609 | `clippy::print_stderr` + `clippy::print_stdout` | Per-crate `#![deny]` in 7 crates: `tree-sitter-perl-rs`, `perl-corpus`, `perl-semantic-analyzer`, `perl-lsp-rs-core`, `perl-dap`, `perl-lsp-rs`. New crates inherit no protection. |

Promotion requires:

1. Workspace `[lints.clippy]` deny + matching `policy/clippy-lints.toml` active entry.
2. Audit non-bin/non-test code for legitimate prints; add scoped
   `#[expect(..., reason = "...")]` receipts.
3. Remove now-redundant per-crate `#![deny(...)]` lines.
4. Update `crates/perl-lsp-rs-core/tests/lint_enforcement_test.rs` to
   recognise workspace-level enforcement instead of the per-crate marker.

### Tier 4 — Opt-in policy lint (empty config)

| Issue | Lint | Sites |
| ----- | ---- | ----- |
| #8610 | `clippy::disallowed_fields` | 0 today (empty `clippy.toml` policy) |

Phase 1 (#8610): activate the lint with an empty `disallowed-fields = []`
in `clippy.toml`. Zero behavior change.

Phase 2 (follow-ups, file when seams identified): pick architectural
seams worth banning direct field access on. Candidate seams:

- Parser context internals (encourage accessors over field access).
- LSP runtime state (provider crates should reach via traits).
- DAP session state (avoid cross-component reaches into internals).

### Tier 5 — Restriction lints (medium / large surface)

| Issue | Lint | Sites | Notes |
| ----- | ---- | ----- | ----- |
| #4914 | `clippy::wildcard_imports` | ~60 lib, ~150 all-targets | Pre-existing issue. Crate-by-crate burn-down then workspace deny on libs (tests retain `use super::*;` carveout). |
| #4923 | `clippy::ptr_arg` | low (mostly already clean) | Pre-existing issue. Promote to workspace warn -> deny on facade crates. |
| #8611 | `clippy::str_to_string` | **~3030 lib**, even more test | Planning row. Sub-rail required — per-crate cleanup PRs (highest density first: `perl-semantic-analyzer` 224, `perl-parser-core` 213, `perl-parser` 173, `perl-refactoring` 121, `perl-workspace` 78) then workspace flip. |

### Removed in Rust 1.95

`clippy::string_to_string` was **removed in Rust 1.95** (its checks are
now covered by `clippy::implicit_clone`). Do **not** add a planned entry
for `string_to_string` — verify the ledger does not list it, and route
the equivalent restriction work through `str_to_string` (#8611) instead.

## What this rollout does not cover

The following are tracked elsewhere — do not bundle:

- **Workspace `allow` burn-down** (`collapsible_match`, `manual_range_contains`,
  `useless_vec`, `vec_init_then_push`, `assertions_on_constants`): tracked
  under the `clippy-cleanup` label and #8508 umbrella. See
  `RUST_1_95_ROLLOUT.md` rows C-1, C-2, C-3.
- **Test carveout removal** (`allow-unwrap-in-tests`): row T-1 of the
  Rust 1.95 rollout.
- **No-panic baseline infra**: rows N-1, N-2, N-3 of the Rust 1.95 rollout.
- **Workspace-wide rustc lint floor tightening**: row R-1 of the Rust 1.95
  rollout (covers `unused_must_use` if not yet denied, etc.).
- **MSRV bump past 1.95**: file a new ladder issue when the MSRV moves.

## Acceptance gates (every PR)

```bash
cargo fmt --all -- --check
cargo clippy --workspace --lib --no-deps -- -D warnings
cargo clippy --workspace --all-targets --no-deps -- -D warnings -A missing_docs
cargo xtask check-lint-policy
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --lib -- --test-threads=2
git diff --check
```

PR titles follow `chore(clippy): activate <lint_name> (#8590)` or
`chore(clippy): promote <lint_name> to workspace-deny (#8590)`.

## Self-review checklist (every PR)

```markdown
## Self-review

- Scope matches PR title (one lint activation, period):
- `Cargo.toml` edit is two-line (lint line + alphabetical sort):
- `policy/clippy-lints.toml` `[[planned]]` -> `[[lint]]` move done:
- `xtask check-lint-policy` passes:
- No bundled cleanup outside the lint surface:
- No bare `#[allow(...)]` added (use `#[expect(..., reason = "...")]`):
- Any new test code paths re-verified:
- CI status:
- Bot comments addressed:
- Follow-ups (sub-rail issues for large-surface lints):
```

## Do not (per cross-repo rollout doctrine)

- Combine two lint activations in one PR.
- Combine activation with unrelated cleanup (formatting, refactoring,
  CI changes).
- Add `#[allow(clippy::...)]` without a `policy/clippy-debt.toml` entry.
- Flip a workspace lint level before its per-crate sub-rail hits zero.
- Re-activate a removed lint (`string_to_string` — covered by
  `implicit_clone` since 1.95).

## References

- Umbrella: #8590
- Ladder issues: #8601 - #8611, plus pre-existing #4914 and #4923.
- Sibling rollout: `RUST_1_95_ROLLOUT.md` (toolchain + allow-burndown rail).
- Policy ledger: `policy/clippy-lints.toml`, `policy/clippy-debt.toml`.
- High-level policy: `docs/CLIPPY_POLICY.md`.
- Active workspace lints: `Cargo.toml` `[workspace.lints.clippy]`.
- Previous allow-burndown PRs (pattern): #8511, #8520, #8521, #8522, #8523, #8526, #8538, #8559.
