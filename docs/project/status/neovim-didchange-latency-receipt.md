# Neovim Ranged-Typing Latency Receipt — Off-Lock Async Parse Worker

**Regenerated**: 2026-07-11
**Branch base**: `origin/main` @ `d1b5222e6` (post-#3618 off-lock async parse worker)
**Crate**: `perl-lsp-rs`
**Test**: `crates/perl-lsp-rs/tests/ux_neovim_ranged_typing_latency_receipt.rs::ux_neovim_ranged_typing_medium_file_receipt`
**Lane**: "Fresh Facts Fast" — Neovim live-edit latency (issue #3396)
**Basis PRs**: #3618/#3396 (off-lock parse worker), #3765 (generation-owned lazy analyzer), #3811 (hover migration to generation-owned facts)

---

## What this is (and is not)

This is an **informational, debug-build, hardware-dependent instrumentation
receipt** — not a product latency budget or SLA. It exists to make the current
merged edit-to-answer *shape* visible and reproducible on demand.

The invariant this receipt proves is a **shape** invariant, not a millisecond
threshold: on the current async path (`incremental_eager = false`, the
production default since #3412, with the off-lock parse worker installed since
#3618), `didChange` performs **zero** full parses and **zero** parent-map
builds before returning. It applies the text edit, bumps the generation,
enqueues a coalescing parse job, and returns. The worker parses off-lock and
publishes only the final, freshness-current generation.

CI asserts **SHAPE only**: a receipt is emitted per scenario, every provider
returns, and — on the async scenario — `did_change_full_parse_count == 0`,
`did_change_parent_map_count == 0`, `worker_jobs_published == 1`, and at least
one job is discarded-or-coalesced. It never asserts the millisecond payloads as
pass/fail thresholds. See #1373 for why: stable receipts first, budgets later.

> **This doc supersedes the pre-#3618 revision.** The earlier version of this
> file documented the *synchronous* `didChange` shape (one full parse per
> ranged edit, inline, before the handler returned) measured against base
> `d6409a100`. That is the **opposite** of the current merged invariant. The
> synchronous shape now survives only as the receipt's BEFORE differential
> control (`incremental_eager = true`), which reproduces the pre-#3412 cost
> model on the same build/hardware for comparison.

## Reproducible command

```bash
cargo test -p perl-lsp-rs --features expose_lsp_test_api \
    --test ux_neovim_ranged_typing_latency_receipt -- --nocapture
```

A bare `cargo test --test ux_neovim_ranged_typing_latency_receipt` (without
`--features expose_lsp_test_api`) compiles **0 tests** and reports a false
green — the whole file is behind that feature gate (`#![cfg(feature =
"expose_lsp_test_api")]`, file line 53). `--nocapture` is required to see the
`PERL_LSP_TIMING_RECEIPT {...}` JSON payloads on stdout.

The test emits **two** labeled payloads:
`after_async_parse_worker_phase3` and `before_eager_on_pre_3412_baseline`.

## Scenario shape

- **Fixture**: deterministic ~78 KB synthetic Perl file (`package
  Medium::Fixture;` + 600 small `sub helper_NNNNN { ... }` blocks). Generated
  by `medium_fixture()`; the test asserts `file_bytes >= 50_000`.
- **Edit pattern**: 20 **ranged** edits (not full-document replacements),
  reproducing the realistic Neovim keystroke-by-keystroke edit shape rather
  than the lean-mode full-replace receipts used elsewhere in the corpus.
- **Providers measured after the burst settles** (first-response wall time +
  whether each returned): `textDocument/completion`, `textDocument/hover`,
  `textDocument/semanticTokens/full`, `textDocument/references`. The burst is
  drained deterministically via `test_wait_for_parse_worker_settled` (condvar,
  never a sleep) before providers are called.
- **Build profile**: debug (`cargo test`, no `--release`).
- **Two scenarios, one test function**, run back-to-back against fresh
  `LspServer` instances:
  - **AFTER** (`incremental_eager = false`): the current production default,
    **with the real off-lock async parse worker installed**
    (`LspServer::install_default_parse_worker`). This is the durable
    current-main artifact and the scenario the Phase-3 closure claim is proven
    against.
  - **BEFORE** (`incremental_eager = true`): reproduces the pre-#3412 cost
    model (eager `incremental_doc_update` maintenance on every keystroke),
    which still requires the parse to run synchronously under the mutation lock
    and so is **not** eligible for the async worker path. This is the
    differential control, not a supported production mode.

## The shape invariant CI enforces

These are the assertions in
`ux_neovim_ranged_typing_medium_file_receipt` (test function at file line 366).
They are the durable, machine-checked receipt — not the timings.

| Assertion (AFTER, async) | Location | Meaning |
|---|---|---|
| `after.is_async` | line 408 | The AFTER scenario ran on the installed async worker |
| `did_change_full_parse_count == 0` | lines 409–412 | `didChange` performed NO full parse before returning |
| `did_change_parent_map_count == 0` | lines 413–416 | `didChange` performed NO parent-map build before returning |
| `worker_jobs_started <= ranged_edits` | lines 423–426 | Coalescing starts no more jobs than edits enqueued |
| `worker_jobs_published == 1` | lines 427–430 | Exactly one (the final) generation from the burst publishes |
| `worker_jobs_discarded_or_coalesced > 0` | lines 431–434 | At least one job from the 20-edit burst was discarded or coalesced |

| Assertion (BEFORE, sync control) | Location | Meaning |
|---|---|---|
| `!before.is_async` | line 437 | The BEFORE scenario stayed on the synchronous fallback |
| `did_change_full_parse_count == ranged_edits` | lines 438–441 | Each ranged edit triggered exactly one synchronous full parse (pre-#3412 shape) |
| `full_parse_max_ms > 0.0` | lines 442–445 | The `full_parse` span recorded a real (non-zero) duration |

Line numbers above are a convenience for this regeneration snapshot; the
assertions are keyed on the stable field names (`did_change_full_parse_count`,
`worker_jobs_published`, …) and the `PERL_LSP_TIMING_RECEIPT` payload keys —
grep those, not the line numbers, which drift.

## On the millisecond numbers

The receipt payload carries per-scenario timings
(`did_change_handler_*` external wall-times, `internal_spans_ms`,
`*_first_response_ms`, `incremental_doc_update_max`, `worker_jobs_*`). **These
are receipts from a specific environment, not guarantees.** They are debug-build,
single-machine, hardware-dependent measurements that vary across machines and
profiles; do not compare across machines, and do not promote any observed
number into a universal budget. To read current numbers for your environment,
run the reproducible command above — the JSON is the artifact. The AFTER
payload's `did_change_handler_*` wall-times collapse toward the text-apply-only
cost (no parse happens before the handler returns); the BEFORE payload retains
the synchronous full-parse-per-edit cost as the differential control.

## Provider coverage

| Provider | Test entrypoint | Status |
|---|---|---|
| `textDocument/completion` | `test_handle_completion` | Measured after settle |
| `textDocument/hover` | `test_handle_hover` | Measured after settle |
| `textDocument/semanticTokens/full` | `test_handle_semantic_tokens` | Measured after settle |
| `textDocument/references` | `test_handle_references` | Measured after settle |

All four provider entrypoints already existed behind `expose_lsp_test_api`; the
receipt drives them, it does not add them.

## Scope note

This artifact is **docs-only**. The receipt test and the
`set_incremental_eager` / `install_default_parse_worker` production APIs it
exercises are already merged on `origin/main` (via #3618/#3396); this document
only reconciles the status doc to that merged shape.
