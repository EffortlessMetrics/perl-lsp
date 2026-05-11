# Non-Rust File Policy — Remaining Ladder

> Companion to [FILE_POLICY.md](FILE_POLICY.md), [NON_RUST_POLICY.md](NON_RUST_POLICY.md),
> [POLICY_ALLOWLISTS.md](POLICY_ALLOWLISTS.md), and the
> [perl-lsp CI policy rollout](../ci/perl-lsp-ci-policy-rollout.md).

This document is the **builder-ready remaining ladder** for the non-Rust file
policy rollout. Each row maps to one or more open GitHub issues that any
coworker agent (factory-droid, codex, claude-burst) can pick up directly.

## Status snapshot

Done on master:

| Stage | Status | Reference |
|-------|--------|-----------|
| PR 01 — File-policy doctrine | merged | #8158 |
| PR 02 — `policy/non-rust-allowlist.toml` + `policy/non-rust-debt.toml` | merged | #8159 |
| PR 03 — `cargo xtask non-rust inventory` | merged | #8512 |

Inventory current state (regenerable via `cargo xtask non-rust inventory`):

- 8466 tracked files
- 2375 Rust-family
- 6091 non-Rust (3974 allowlisted, **2117 unclassified**)

The remaining ladder is split into three streams:

1. **Rollout stream** — the remaining PRs 04 → 11 from the rollout plan.
2. **Inventory stream** — classification PRs for the 2117 unclassified files.
3. **Tightening stream** — narrowing broad globs and adding maintainer affordances.

## Rollout stream (sequential dependencies)

| Row | PR | Title | Tracking issue | Depends on |
|----:|---:|-------|---------------:|-----------:|
| R-04 | PR 04 | `cargo xtask check-file-policy` (advisory) | [#8566](https://github.com/EffortlessMetrics/perl-lsp/issues/8566) | PR 03 (merged) |
| R-05 | PR 05 | `cargo xtask non-rust propose` | [#8568](https://github.com/EffortlessMetrics/perl-lsp/issues/8568) | R-04 |
| R-06a | PR 06a | `policy/generated-allowlist.toml` + `check-generated` | [#8570](https://github.com/EffortlessMetrics/perl-lsp/issues/8570) | R-04 |
| R-06b | PR 06b | `policy/executable-allowlist.toml` + `check-executable-files` | [#8572](https://github.com/EffortlessMetrics/perl-lsp/issues/8572) | R-04 |
| R-06c | PR 06c | `policy/dependency-surface-allowlist.toml` + `check-dependency-surfaces` | [#8575](https://github.com/EffortlessMetrics/perl-lsp/issues/8575) | R-04 |
| R-07a | PR 07a | `policy/process-allowlist.toml` + `check-process-policy` | [#8577](https://github.com/EffortlessMetrics/perl-lsp/issues/8577) | R-04 |
| R-07b | PR 07b | `policy/network-allowlist.toml` + `check-network-policy` | [#8580](https://github.com/EffortlessMetrics/perl-lsp/issues/8580) | R-04 |
| R-08 | PR 08 | `policy/workflow-allowlist.toml` + `check-workflow-surfaces` | [#8583](https://github.com/EffortlessMetrics/perl-lsp/issues/8583) | R-04 |
| R-09 | PR 09 | `cargo xtask policy-report` (unified report) | [#8587](https://github.com/EffortlessMetrics/perl-lsp/issues/8587) | R-06{a,b,c}, R-07{a,b}, R-08 |
| R-10 | PR 10 | Wire `file_policy` gate into `.ci/gate-policy.yaml`, promote to `blocking-allowlist` | [#8589](https://github.com/EffortlessMetrics/perl-lsp/issues/8589) | R-09 + Inventory stream clean |
| R-11 | PR 11 | Promote five ledgers to `blocking-strict` | [#8592](https://github.com/EffortlessMetrics/perl-lsp/issues/8592) | R-10 + 1-2 week clean baseline |

R-05, R-06{a,b,c}, R-07{a,b}, R-08 are **independent siblings** once R-04 is
merged — open in parallel.

## Inventory stream (independent, parallel-safe)

Rows that classify the 2117 unclassified files. Each is a separate small PR
that adds explicit `[[allow]]` entries to `policy/non-rust-allowlist.toml`.
Land these before R-10 so blocking-allowlist mode is clean.

| Row | Title | Tracking issue | Coverage |
|----:|-------|---------------:|----------|
| I-A | `fuzz/**` corpus and crash artifacts | [#8596](https://github.com/EffortlessMetrics/perl-lsp/issues/8596) | ~2000 files |
| I-B | `archive/**` legacy tree (with `expires`) | [#8597](https://github.com/EffortlessMetrics/perl-lsp/issues/8597) | ~20 files |
| I-C | `distribution/**` packaging tree | [#8599](https://github.com/EffortlessMetrics/perl-lsp/issues/8599) | ~14 files |
| I-D | `crates/*/LICENSE-{APACHE,MIT}` (widen existing glob to `**/LICENSE-*`) | [#8649](https://github.com/EffortlessMetrics/perl-lsp/issues/8649) | ~50 files |
| I-E | `crates/*/features_sot.toml` + corpus `concepts/*.toml` + `*.meta.toml` | [#8650](https://github.com/EffortlessMetrics/perl-lsp/issues/8650) | ~30 files |
| I-F | `benchmarks/scripts/**` + `ci/**` legacy (with `expires`) | [#8651](https://github.com/EffortlessMetrics/perl-lsp/issues/8651) | ~40 files |
| I-G | Templates, `.rst` docs, `.pest` grammar, `.perltidyrc`, `.disabled` files | [#8652](https://github.com/EffortlessMetrics/perl-lsp/issues/8652) | ~10 files |
| I-H | Triage `crates/perl-parser-core/libcheck_unwrap.rlib` (binary in git) | [#8653](https://github.com/EffortlessMetrics/perl-lsp/issues/8653) | 1 file (anomalous) |
| I-I | `book/book.toml` + `.kiro/specs/**/.config.kiro` | [#8654](https://github.com/EffortlessMetrics/perl-lsp/issues/8654) | 4 files |

After all I-rows land, `cargo xtask non-rust inventory` should report **zero
unclassified files** and `cargo xtask check-file-policy --mode blocking-allowlist`
(landing in R-04) should pass cleanly.

## Tightening stream (post-inventory, pre-strict)

| Row | Title | Tracking issue | Purpose |
|----:|-------|---------------:|---------|
| T-J | Tighten `.github/**` broad glob into specific subtrees | [#8655](https://github.com/EffortlessMetrics/perl-lsp/issues/8655) | Remove silent waiver |
| T-K | Add `cargo xtask non-rust audit-cadence` for `review_after`/`expires` visibility | [#8656](https://github.com/EffortlessMetrics/perl-lsp/issues/8656) | Maintainer affordance ahead of strict mode |

T-J should land before R-11 (strict mode flags broad globs without
`broad_glob_reason`). T-K is independent.

## Dependency graph

```text
PR 03 (merged: #8512)
   └─ R-04 (#8566 — check-file-policy)
         ├─ R-05 (#8568 — propose)
         ├─ R-06a (#8570 — generated)
         ├─ R-06b (#8572 — executable)
         ├─ R-06c (#8575 — dependency-surface)
         ├─ R-07a (#8577 — process)
         ├─ R-07b (#8580 — network)
         ├─ R-08 (#8583 — workflow-surface)
         └─ R-09 (#8587 — policy-report)
               └─ R-10 (#8589 — CI gate + blocking-allowlist)
                     └─ R-11 (#8592 — blocking-strict)

Inventory stream (parallel to R-04..R-09):
   I-A (#8596), I-B (#8597), I-C (#8599),
   I-D (#8649), I-E (#8650), I-F (#8651),
   I-G (#8652), I-H (#8653), I-I (#8654)

Tightening stream:
   T-J (#8655)  — before R-11
   T-K (#8656)  — independent
```

## How agents should pick work from this ladder

1. **Sort by depth:** R-04 has no dependencies (other than the merged PR 03)
   and unblocks the most downstream work — start there.
2. **Pick siblings in parallel:** R-05 through R-08 are independent once R-04
   lands. Inventory rows I-A through I-I are all independent.
3. **Respect "Do not combine":** Each issue has a `## Do not combine` section.
   Honor it — one ledger / one row / one PR.
4. **Validate locally:**
   - For inventory rows: run `cargo xtask non-rust inventory` and verify the
     Unclassified table shrinks by the expected count.
   - For rollout rows: run the `## Acceptance` block.
5. **Reference the umbrella:** Every PR description should cite `Closes #<row>`
   AND reference umbrella #8174 so the rollout is traceable.

## Cross-references

- Umbrella tracking: [#8174](https://github.com/EffortlessMetrics/perl-lsp/issues/8174)
- Rollout plan: [perl-lsp-ci-policy-rollout.md](../ci/perl-lsp-ci-policy-rollout.md)
- Doctrine: [FILE_POLICY.md](FILE_POLICY.md)
- Schema: [NON_RUST_POLICY.md](NON_RUST_POLICY.md)
- Catalog: [POLICY_ALLOWLISTS.md](POLICY_ALLOWLISTS.md)
- Live inventory: [NON_RUST_INVENTORY.md](NON_RUST_INVENTORY.md) (regenerable)
