# Fresh Facts Fast — Done-Condition Proof Suite

**Status:** living spec · **Program:** Fresh Facts Fast (off-lock async parse worker, #3396) · **Executes AFTER:** #3618 merges (off-lock parse worker + `Weak` Arc-cycle / self-join fix)

This document is the durable, turnkey definition of "done" for the Fresh Facts
Fast program. It names the deterministic tests, real-Perl editor canaries, and
receipts that — taken together — *prove* the program complete, binds each to the
substrate seam it exercises (by stable name), inventories what already exists
versus what is still a gap, and gives a builder a turnkey design for every gap
so it can be closed without re-researching the substrate.

> **What "done" means here.** The program is done when the invariants below are
> each proven by a *deterministic* (barrier/channel/shutdown-signal, never
> sleep-based) test that actually **runs** (not a false-green 0-test behind an
> unset feature gate), plus real-Perl provider honesty canaries and the Neovim
> latency receipt, all as durable repository artifacts. Completion is tracked on
> the per-condition Implemented/Covered/Gap axis in the checklist at the end —
> not a single "done" flag.

> **Reference discipline.** This document cites tests and seams by **stable
> name** (test function name, symbol name, file path) — never by line number.
> Line numbers drift with every commit that touches a file and go stale within
> hours; names survive refactors and are what you'd grep for anyway. **§0–§6
> and the worker-shape half of §8** describe the `ParseWorker`/`parse_worker.rs`
> substrate, which lands via the Fresh Facts Fast program (#3396) through PR
> #3618. Rows/sections tagged "tracks #3618" describe that PR's content, not a
> point-in-time snapshot of `origin/main` — **check #3618 directly on GitHub
> for its current merge state**; this document does not attempt to track that
> live. **§7/§7a/§7b** (provider honesty canaries) are this PR's (#3649) own
> contribution, tagged "this PR" below. Do not infer either PR's merge status
> from prose in this document — check GitHub.

---

## 0. Substrate seam map

Every proof below binds to one of these production seams, cited by symbol and
file — re-grep the symbol for its current location rather than trusting a line
number (none are given here; see "Reference discipline" above). Rows citing
`parse_worker.rs` or the `text_sync.rs` oracle functions track via #3618; the
`document.rs` row is production code today, independent of #3618.

| Seam | File | What it guarantees |
|------|------|--------------------|
| `Coordinator::enqueue` (coalesce-replace, single lock) | `crates/perl-lsp-rs/src/runtime/parse_worker.rs` (tracks #3618) | At most one pending job per URI; `ParseWorkerMetrics::jobs_coalesced` bumped on replace |
| `Coordinator::take_next` (atomic pop from `ready`+`pending`) | `parse_worker.rs` (tracks #3618) | No TOCTOU orphan between the ready queue and the pending map |
| `Coordinator::finish` (re-queue latest / release URI) | `parse_worker.rs` (tracks #3618) | Newer edit that landed mid-parse is re-queued; URI released otherwise |
| `catch_unwind` + `FinishGuard` in the worker loop | `parse_worker.rs` (tracks #3618) | A panicking job never orphans its URI or shrinks the pool |
| `Weak<LspServer>` downgrade in `on_published` (breaks the Arc cycle) | `crates/perl-lsp-rs/src/runtime/mod.rs`, `install_default_parse_worker` (tracks #3618) | The callback only ever holds a transient strong ref via `cb_server.upgrade()`, so the server's strong count can reach zero without a worker thread being forced to join itself — narrows, but per §3b does not structurally prove closed, the self-join-from-callback-thread window. `ParseWorker::drop` itself (`parse_worker.rs`) is an unconditional join-all loop with no self-join guard of its own — see §3b. |
| `process_job` publish transaction (`Arc::ptr_eq` + `publish_parsed_if_current`) | `parse_worker.rs` (tracks #3618) | Document-instance identity + generation freshness gate, single lock acquisition |
| `DocumentState::publish_parsed_if_current` | `crates/perl-lsp-rs/src/state/document.rs` (production today) | A stale-generation snapshot publishes nothing |
| `DocumentState::current_parsed` (freshness-correct read) | `document.rs` (production today) | Returns `None` when the last published snapshot is older than the text generation (the pending-parse gap) |
| `LspServer::run_post_parse_side_effects` | `crates/perl-lsp-rs/src/runtime/text_sync.rs` (tracks #3618) | Deferred side effects (symbols, index, diagnostics) route through the freshness oracle |
| `commit_parse_effect_if_current` (the oracle) + `document_generation_still_current` | `text_sync.rs` (tracks #3618) | Re-validates `(document_instance, generation)` **at the moment of commit** |
| Test barriers / panic injector (test-only) | `parse_worker.rs`, `ParseWorkerTestBarrier` / `ParseWorkerPanicInjector` (tracks #3618) | Zero-sleep deterministic pause/release + panic injection |
| Worker metrics (test-API read) | `parse_worker.rs`, `ParseWorkerMetrics` (tracks #3618) | `jobs_started/coalesced/rejected_stale/published/panicked` — the assertion surface |

---

## 1. Coalescing — latest-only per URI

**Tracks #3618.**

**Invariant.** A rapid burst of N edits to one URI parses far fewer than N
times, and exactly one generation (the final one) publishes. An older,
not-yet-started job is silently replaced by a newer edit's job.

**Seam.** `Coordinator::enqueue` coalesce-replace (`parse_worker.rs`); final
publish gated by `process_job`.

**Existing coverage — COVERED (deterministic).**
- `rapid_burst_coalesces_to_far_fewer_jobs_than_edits` (`parse_worker.rs` test
  module). Enqueues 20 edits, waits via `Coordinator::wait_until_settled`
  (condvar-based — **not** a sleep), asserts `jobs_published == 1`,
  `jobs_started < 20`, `jobs_coalesced > 0`, and the published generation
  equals the final edit.
- Receipt-level corroboration: the worker-shape variant of
  `ux_neovim_ranged_typing_medium_file_receipt` (see §8) asserts
  `worker_jobs_discarded_or_coalesced > 0` and `worker_jobs_published == 1`
  across a 20-ranged-edit burst.

**Feature-gate / run.** Inline `#[cfg(test)]` module — runs under
`cargo test -p perl-lsp-rs --lib` (no feature flag; the test-only barrier types
are `#[cfg(any(test, feature = "expose_lsp_test_api"))]` and the `test` cfg is
active in a `--lib` test build). Verify it actually runs:
`cargo test -p perl-lsp-rs --lib rapid_burst_coalesces -- --nocapture` must
report `1 passed`, not `0 filtered out`.

**Receipt.** `jobs_started` / `jobs_coalesced` / `jobs_published` counters and
the `worker_jobs_discarded_or_coalesced` receipt field.

---

## 2. Close/reopen — document-instance identity across generation reset

**Tracks #3618.**

**Invariant.** A parse job queued against document instance A must never publish
into a *different* instance B that a `didClose`+`didOpen` cycle installed at the
same URI — **even when B's fresh generation counter has coincidentally reached
the same numeric value** A's job is trying to publish. Identity is `Arc::ptr_eq`
on the generation handle, not a `u32` compare.

**Seam.** `process_job`'s `Arc::ptr_eq(&doc.generation, &job.generation_handle)`
check (`parse_worker.rs`); the same identity check in the deferred-side-effect
oracle `document_generation_still_current` (`text_sync.rs`).

**Existing coverage — COVERED (deterministic).**
- `stale_job_cannot_publish_into_a_reopened_document_instance` (`parse_worker.rs`
  test module). Pauses A's gen-1 job at the barrier, replaces the map entry
  with a brand-new `DocumentState` (fresh `Arc<AtomicU32>`, bumped to the
  **same** numeric generation 1), releases A, asserts `jobs_rejected_stale >= 1`,
  zero side-effect callbacks, and that instance B is untouched (`Arc::ptr_eq`
  still B, `current_parsed()` still `None`).

**Feature-gate / run.** Inline `#[cfg(test)]` — `cargo test -p perl-lsp-rs --lib
stale_job_cannot_publish_into_a_reopened`.

**Receipt.** `jobs_rejected_stale` counter; the empty side-effect call log.

---

## 3. Panic / shutdown — worker survives panic + drains and joins on shutdown

**Tracks #3618.** Two distinct invariants; the program needs **both**.

### 3a. Panic survival — COVERED (deterministic)

**Invariant.** A job that panics inside `process_job` (pathological parser input,
or injected) must (1) be recorded, (2) never publish, (3) release its URI so a
later edit to the same URI still parses, and (4) keep the worker thread alive.

**Seam.** `catch_unwind` + `FinishGuard` (`parse_worker.rs`); the one-shot
`ParseWorkerPanicInjector`.

**Existing coverage.** `panicking_job_still_releases_its_uri_and_the_worker_keeps_processing`
(`parse_worker.rs` test module). Arms the panic injector for gen 1, asserts
`jobs_panicked >= 1` and `jobs_published == 0`, then enqueues gen 2 to the *same*
URI and asserts it publishes — proving the URI was released and the pool survived.

### 3b. Shutdown drain + self-join safety — **GAP**

**Invariant.** (i) Dropping the `ParseWorker` requests shutdown and joins every
worker thread, draining any jobs still in `ready` before exit (`take_next`
returns `None` only when `ready` is empty *and* shutdown was requested). (ii)
Dropping the **last** `Arc<LspServer>` from *inside* a worker thread's
`on_published` callback (the `Weak::upgrade()` temp going out of scope) must
NOT self-join-deadlock.

**Correction (deep review, #3649).** This section previously described the (ii)
fix as "a `Drop` guard that skips joining the current thread's own handle."
That is not what the code does: `impl Drop for ParseWorker` (`parse_worker.rs`)
is an unconditional `for handle in handles.drain(..) { let _ = handle.join(); }`
loop with no thread-identity check at all. The actual (ii) fix lives one layer
up, in `LspServer::install_default_parse_worker`
(`crates/perl-lsp-rs/src/runtime/mod.rs`): `on_published`'s closure captures
`cb_server: Weak<LspServer>` via `Arc::downgrade(self)` rather than a strong
`Arc`, so the callback only ever holds a transient strong ref
(`cb_server.upgrade()`) for the duration of `run_post_parse_side_effects`,
breaking the `LspServer -> ParseWorker -> worker threads -> on_published ->
Arc<LspServer>` reference cycle at its root — `ParseWorker::drop` itself is
unchanged and still has no self-join protection if a worker thread ever
*does* end up dropping the last strong `Arc<ParseWorker>`.

**Existing coverage.** Implicit only — every worker test drops the `ParseWorker`
at scope end, so a broken join would hang the suite. There is **no dedicated
regression test** that (a) asserts queued-but-unstarted jobs are drained on
shutdown, or (b) reproduces the self-join-from-callback-thread path described
in (ii) above. The guard is currently protected by code inspection + the fact
that the suite doesn't hang — a classic "the instrument is the only witness"
gap. #3618 separately added `dropping_the_server_joins_the_installed_parse_worker_threads`
(`parse_worker.rs` test module) — a bounded-timeout test that drops the
server's last strong `Arc<LspServer>` **on a dedicated thread it spawns for
the purpose**, proving the cycle-break generally, but it does **not**
reproduce scenario (ii): the drop happens from an external thread, not from
inside a worker thread's own `on_published` callback holding the last strong
ref. Do not read that test as closing (ii) — the self-join-from-callback-thread
path remains unproven either way.

**Two deterministic regression tests are still needed** (drain-on-shutdown;
self-join-from-callback-thread) to close this gap — tracked for design and
implementation via #3618, not sketched here. A prior revision of this section
proposed a specific self-join test design; it was removed after review found
the sketch could not actually reproduce the callback-thread-holds-the-last-ref
scenario as written (the side-effect barrier pauses *before* `on_published`
runs, i.e. before `Weak::upgrade()` — a real witness needs the barrier/handshake
placed *inside* the callback, after a successful `upgrade()`, so the external
strong reference can be dropped while that callback is still holding its own).
This is exactly the kind of prescriptive detail a done-condition **contract**
doc shouldn't carry speculatively — the actual test belongs in #3618, reviewed
against the real code at the time it's written, not pre-designed here against
a moving target.

**Receipt.** `jobs_panicked` (3a, existing); for 3b, once written: a
bounded-timeout "drop returned" boolean the test asserts — the absence of a
hang IS the receipt.

---

## 4. Cross-document progress — one doc's pending parse never blocks another

**Tracks #3618.**

**Invariant.** While one URI's parse is stalled (paused mid-publish, or a slow
large-file parse), edits to a *different* URI still parse and publish. Distinct
URIs dispatch to distinct pool threads; only same-URI edits serialize.

**Seam.** The bounded pool + per-URI `active` set (`QueueState::active`,
`parse_worker.rs`); the `PARSE_WORKERS` pool-size constant.

**Existing coverage — COVERED (deterministic).**
- `one_document_paused_does_not_block_another_documents_publish`
  (`parse_worker.rs` test module). Pauses doc A at the barrier (never released
  until cleanup), enqueues doc B, asserts B publishes and fires side effects
  while A is still paused/unpublished. Explicitly the test that would catch a
  single-global-worker regression.

**Feature-gate / run.** Inline `#[cfg(test)]` — `cargo test -p perl-lsp-rs --lib
one_document_paused_does_not_block`.

**Receipt.** Per-URI side-effect call log records B's `(uri, generation)` while
A's is absent.

---

## 5. Edit-during-analysis — an edit mid-analysis rejects the stale effect

**Tracks #3618.** `run_post_parse_side_effects`, `commit_parse_effect_if_current`,
and `document_generation_still_current` land in `text_sync.rs`; the
worker-level test lives in `parse_worker.rs`.

**Invariant.** If a newer edit (gen N+1) lands after gen N's parse published but
before gen N's *deferred side effects* (symbol reindex, workspace index,
diagnostics) commit, those side effects must be dropped — publication validity ≠
side-effect validity. The race window is real and reachable; the oracle closes it.

**Seam.** `run_post_parse_side_effects` (`text_sync.rs`) →
`commit_parse_effect_if_current`; the worker's separate `side_effect_barrier`
pause point (`parse_worker.rs`).

**Existing coverage — COVERED (deterministic, two layers).**
- Worker-level race reachability:
  `side_effect_barrier_pauses_after_publish_and_a_newer_edit_can_commit_while_paused`
  (`parse_worker.rs` test module). Proves publish lands (gen 1 current), the
  callback is withheld, a real gen-2 edit commits, and the document is already
  at gen 2 when gen 1's side effect fires — i.e. the window the oracle must
  guard is real.
- Direct oracle proof: `stale_generation_side_effects_never_reindex_symbols`
  (`text_sync/tests.rs`). Calls `run_post_parse_side_effects` with a ticket for
  a superseded generation; asserts the stale symbol never enters the index and
  the kept symbol survives.
- Real-worker end-to-end: `stale_side_effects_never_commit_through_the_real_worker_after_a_newer_edit`
  (`text_sync/tests.rs`). Installs the real worker, uses the side-effect
  barrier, lands a real N+1 edit while N is paused, releases N, asserts N's
  symbol reindex never reached the index.

**Feature-gate / run.** Worker-level test: `--lib`. Oracle + real-worker tests
are in `text_sync/tests.rs` (module tests): `cargo test -p perl-lsp-rs --lib
stale_generation_side_effects`, `... stale_side_effects_never_commit`. The
real-worker one needs `workspace` (default) for `symbol_index`.

**Receipt.** The `symbol_index` search-prefix assertions (stale absent, kept
present) are the durable receipt.

---

## 6. Stale-effect rejection — gen N never commits over gen N+1 (publish gate)

**Partial.** The `document.rs` unit-level gate is production code today,
independent of #3618. The worker-level tests are in `parse_worker.rs` and
track #3618.

**Invariant.** A completed parse for generation N must never overwrite a document
that has already advanced to N+1. The publish gate rejects it; a rejected publish
fires zero side effects.

**Seam.** `DocumentState::publish_parsed_if_current` (`document.rs`) — checks
both `current_generation() == expected_generation` and `snapshot.generation ==
expected_generation`; wired at `process_job` (`parse_worker.rs`).

**Existing coverage.**
- `stale_generation_is_rejected_latest_generation_publishes` (`parse_worker.rs`
  test module, tracks #3618). Pauses N (gen 1), lands N+1 (gen 2) as a
  *separately dequeued* job (not coalesced — exercises the "already started,
  rejected at publish" path), releases N, asserts `jobs_rejected_stale >= 1`,
  `jobs_published == 1`, final generation 2, side effects only for gen 2.
- `rejected_publish_never_invokes_the_side_effect_callback` (`parse_worker.rs`
  test module, tracks #3618). Isolated assertion: supersede gen 1 while
  paused, release, assert `jobs_rejected_stale >= 1` and the callback count
  is exactly 0.
- Unit-level gate (production today): `publish_parsed_if_current_rejects_stale_expected_generation`,
  `..._accepts_matching_generation`, `..._rejects_mismatched_snapshot_generation`
  (`document.rs` test module).

**Feature-gate / run.** All inline `#[cfg(test)]` — `cargo test -p perl-lsp-rs
--lib publish_parsed_if_current`, `... stale_generation_is_rejected`,
`... rejected_publish_never_invokes`.

**Receipt.** `jobs_rejected_stale` / `jobs_published` counters; the zero-call
side-effect log.

---

## 7. Provider honesty canaries — every user-facing provider stays honest through the gap

**This PR (#3649).** The tests in this section are this PR's own contribution.
The substrate they exercise — `DocumentState::current_parsed` / `latest_parsed`,
the `test_apply_text_change_without_reparse` / `test_publish_parse_for_current_generation`
test-API helpers — is production/test-API code that predates this PR.

**Invariant.** While `current_parsed()` is `None` (text generation ran ahead of
the last published snapshot), NO provider may present a **stale** fact (the
superseded identifier) or an **unearned fresh** fact (a claim about the
not-yet-parsed new text). Fail-closed providers produce nothing; the two declared
exceptions (completion, symbols) may answer only from a bounded text/regex
fallback over the *current* text, never the stale AST.

**Seam.** Every provider reads `DocumentState::current_parsed()` (`document.rs`)
rather than `latest_parsed()`; the gap is forced by the #3589 helpers
`test_apply_text_change_without_reparse` / `test_publish_parse_for_current_generation`,
and driven for real by the worker's pre-publish `test_barrier`.

**Existing coverage — COVERED for 10 providers (deterministic), synthetic + real.**
File: `crates/perl-lsp-rs/tests/pending_parse_provider_freshness_tests.rs`.

| Provider | Gap policy | Test |
|----------|-----------|------|
| Semantic tokens (full + range) | emit nothing (no gen-N claim from N-1 AST) | `semantic_tokens_emit_nothing_during_pending_parse_gap` |
| Hover | no stale AST claim | `hover_degrades_during_pending_parse_gap` |
| Signature help | no stale AST claim (falls back to the name-only builtin table, never the AST) | `signature_help_no_stale_claim_during_pending_parse_gap`, `signature_help_never_answers_from_stale_ast_with_matching_name_during_pending_parse_gap` |
| Definition (navigation) | fail closed | `definition_fails_closed_during_pending_parse_gap` |
| References | fail closed, never leak `foo` | `references_fail_closed_during_pending_parse_gap` |
| Rename | fail closed (zero edits) | `rename_fails_closed_during_pending_parse_gap` |
| Safe-delete | fail closed (fallback decision) | `safe_delete_fails_closed_during_pending_parse_gap` |
| Document symbols | current-text regex only, never stale AST | `document_symbols_never_leak_stale_identifier_during_pending_parse_gap` |
| Call hierarchy | fail closed (null) | `prepare_call_hierarchy_fails_closed_during_pending_parse_gap` |
| Completion | bounded fallback may answer, never `foo` | `completion_uses_bounded_fallback_during_pending_parse_gap` |

Plus the two headline cross-provider canaries on one shared `sub foo -> bar` edit:
- `sub_foo_to_bar_cross_provider_freshness_canary` (synthetic gap). Now also
  walks signature help (gap assertion + post-publish resolve-to-`bar`
  assertion), closing §7a.
- `sub_foo_to_bar_cross_provider_freshness_canary_real_async_worker` (real
  installed worker + barrier; tracks #3618 — re-grep if adding signature-help
  there too is desired as a follow-up).
- No-gap regression guard: `providers_answer_normally_with_no_pending_parse_gap`
  (ensures the gap logic doesn't misfire when there is no gap).

**Signature-help test API.** `LspServer::test_handle_signature_help` was added to
`crates/perl-lsp-rs/src/runtime/test_api.rs` (mirroring `test_handle_hover`) to
expose `handle_signature_help` for these canaries — it did not previously exist.

**7a. Signature-help — COVERED (closed).** `handle_signature_help`'s
user-defined-function branch (`get_user_function_signature`) requires
`doc.current_parsed()`'s AST, so it is skipped entirely during the gap; the
name-only builtin-function-table fallback does not recognize arbitrary
identifiers (`foo` is not a Perl builtin), so no stale claim surfaces. Proven by
`signature_help_no_stale_claim_during_pending_parse_gap` (standalone, mirrors
`hover_degrades_during_pending_parse_gap`) and by the two new assertion blocks
added to `sub_foo_to_bar_cross_provider_freshness_canary` (gap: `!json_contains(&sig_gap, "foo")`;
post-publish: `json_contains(&sig1, "bar")` and `!json_contains(&sig1, "foo")`).

*Revert-proves-red note (deep review, #3649).* The `foo` -> `bar` rename
fixture shared by the two tests above cannot by itself distinguish "gap
handled honestly" from "gap handled by silently reading `latest_parsed()`
instead of `current_parsed()`": the mutated code would look up `bar` in the
stale AST that only defines `foo`, miss by name regardless of which
snapshot is consulted, and the assertion (`!json_contains(&sig, "foo")`)
would pass either way — verified experimentally by reverting
`current_parsed()` -> `latest_parsed()` in `signature_help.rs` and observing
the original two assertions stay green. `signature_help_never_answers_from_stale_ast_with_matching_name_during_pending_parse_gap`
closes that hole with a same-named, signature-changing fixture (`sub calc {}`
-> `sub calc($x, $y) {}`) where a stale-AST answer is name-matchable but
produces a distinguishable wrong label (`"sub calc"`, 0 params); this test
was confirmed to fail under the same mutation and pass on the real
`current_parsed()`-gated implementation. It also asserts a **precise**
post-publish positive resolve: `parameters.len() == 2` on the fresh `calc`
signature, not merely that the response mentions `"calc"` (a stale
0-parameter match would also satisfy a bare substring check, which would
have silently defeated the very asymmetry this assertion exists to prove) —
mirroring the headline canary's gap/post-publish asymmetry.

**7b. Diagnostics honesty during the gap — COVERED (closed for document-pull).**
The internal side-effect staleness was already proven (§5), and
`pull_diagnostics_freshness_tests.rs` already had a "pending-parse gap tests"
section (`pull_document_diagnostic_stays_fresh_during_pending_parse_gap`,
`pull_workspace_diagnostic_omits_gapped_doc_from_items`,
`pull_workspace_diagnostic_resumes_after_pending_parse_gap_closes`) covering the
error-introduction direction and the post-gap-closure direction. The specific
gap this PR closes is the **error-fixing** direction *during* the gap for
single-document pull: `pull_document_diagnostic_does_not_report_a_fixed_syntax_error_as_current_during_pending_parse_gap`
opens a document with a syntax error (asserts `PL001` is reported), applies a
fix via `test_apply_text_change_without_reparse` (forcing the gap open), and
asserts the pull-diagnostic response taken *during* the gap does not carry the
now-fixed `PL001` diagnostic — i.e. the pull path reflects the current text
honestly rather than resurrecting a stale AST-derived diagnostic. Push
diagnostics (`textDocument/publishDiagnostics` notifications) have no
comparable test-only capture API in `test_api.rs` today; that surface is not
covered by this PR and is noted as residual scope, not silently assumed closed.

**Feature-gate / run.** `#![cfg(all(feature = "workspace",
feature = "expose_lsp_test_api"))]`. A bare `--test
pending_parse_provider_freshness_tests` compiles **0 tests** and false-greens.
Run: `RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --features
expose_lsp_test_api --test pending_parse_provider_freshness_tests -- --nocapture`
and confirm a non-zero passed count. Diagnostics honesty test:
`#![cfg(feature = "expose_lsp_test_api")]` in `pull_diagnostics_freshness_tests.rs`
— same run pattern, `--test pull_diagnostics_freshness_tests`.

**Receipt.** The `json_contains(&result, "foo") == false` assertions across every
provider, plus the post-publish resolve-to-`bar` assertions, are the durable
receipt for §7a. For §7b, the `code == "PL001"` presence/absence assertions
before, during, and after the gap are the durable receipt.

---

## 8. Neovim latency receipts — no full-parse / no parent-map rebuild in didChange

**Invariant.** On the async path (`incremental_eager = false`, production default
since #3412, worker installed), `didChange` performs **zero** full parses and
**zero** parent-map builds before returning — it applies the text edit, bumps the
generation, enqueues, and returns. The worker starts ≤ N jobs for N edits,
publishes exactly one (the final) generation, and discards/coalesces ≥ 1.

**Seam.** `didChange` enqueue-and-return path (`text_sync.rs`, gated by
`install_default_parse_worker` eligibility); the `didChange.full_parse` /
`didChange.parent_map` timing spans that must be absent on the async path.

**Two tests share one function name — do not conflate them.**
`crates/perl-lsp-rs/tests/ux_neovim_ranged_typing_latency_receipt.rs` exists in
production today and already has a test named
`ux_neovim_ranged_typing_medium_file_receipt`, but that version's AFTER/BEFORE
split is `incremental_eager` off vs. on (both still fully **synchronous**, no
`ParseWorker` installed at all), and it asserts one synchronous full parse per
edit — the opposite of the "zero full parses" claim below. The
`did_change_full_parse_count` / `did_change_parent_map_count` / `worker_jobs_*`
fields and the AFTER-is-the-real-worker semantics described here are added by
#3618, which changes this same test's AFTER scenario to install the real
`ParseWorker` instead of just toggling `incremental_eager`. Before trusting a
run of `ux_neovim_ranged_typing_medium_file_receipt` as proof of the
worker-path invariant this section claims, confirm which shape of the test is
actually present — check whether `install_default_parse_worker` and
`did_change_full_parse_count` appear in the file (mechanical, name-based check,
not a line-number lookup).

**Existing coverage — COVERED (shape-deterministic); worker-shape variant tracks #3618.**
File: `crates/perl-lsp-rs/tests/ux_neovim_ranged_typing_latency_receipt.rs`.
- `ux_neovim_ranged_typing_medium_file_receipt` (worker-shape variant, tracks
  #3618). Drives 20 ranged edits on a realistic ~78 KB fixture through the
  real installed worker (AFTER) and the sync fallback (BEFORE). Asserts on
  AFTER: `did_change_full_parse_count == 0`, `did_change_parent_map_count == 0`,
  `jobs_started <= 20`, `jobs_published == 1`, `jobs_discarded_or_coalesced > 0`.
  Asserts on BEFORE the Phase-2 invariant (one sync full parse per edit) as a
  differential control. Every provider (completion/hover/semantic-tokens/references)
  must return after the burst settles (`test_wait_for_parse_worker_settled`,
  condvar — no sleep). CI asserts SHAPE only; millisecond timings are
  informational (#1373).

**Feature-gate / run.** `#![cfg(feature = "expose_lsp_test_api")]`. A bare
`--test ux_neovim_ranged_typing_latency_receipt` compiles **0 tests** and
false-greens (documented in the file header). Run:
`cargo test -p perl-lsp-rs --features expose_lsp_test_api --test
ux_neovim_ranged_typing_latency_receipt -- --nocapture` — `--nocapture` is
required to emit the `PERL_LSP_TIMING_RECEIPT {...}` payloads.

**Receipt.** The printed `PERL_LSP_TIMING_RECEIPT` JSON (two payloads:
`after_async_parse_worker_phase3`, `before_eager_on_pre_3412_baseline`) with
`did_change_full_parse_count`, `did_change_parent_map_count`, and `worker_jobs_*`
fields — a durable, greppable artifact.

---

## 9. Completion checklist

Each done-condition tracked on the Implemented (production seam exists) · Covered
(deterministic proof exists that runs) · Gap axis. Rows tagged "tracks #3618"
land with that PR — check it directly on GitHub for current status; rows
tagged "this PR" are #3649's own contribution.

| # | Done-condition | Implemented | Covered (deterministic, runs) | Gap |
|---|----------------|:-----------:|:-----------------------------:|-----|
| 1 | Coalescing (latest-only per URI) | ✅ tracks #3618 | ✅ `rapid_burst_coalesces…` + receipt | — |
| 2 | Close/reopen instance identity | ✅ tracks #3618 | ✅ `stale_job_cannot_publish_into_a_reopened…` | — |
| 3a | Panic survival | ✅ tracks #3618 | ✅ `panicking_job_still_releases_its_uri…` | — |
| 3b | Shutdown drain + self-join | ⚠️ tracks #3618; Weak-downgrade breaks the cycle, `ParseWorker::drop` itself has no self-join guard | ⚠️ implicit only + one external-thread-drop test that doesn't reproduce the callback-thread case | **GAP** — no dedicated drain / self-join-from-callback-thread regression test; see §3b Correction |
| 4 | Cross-document progress | ✅ tracks #3618 | ✅ `one_document_paused_does_not_block_another…` | — |
| 5 | Edit-during-analysis (stale side-effect drop) | ✅ tracks #3618 | ✅ 3 layers (worker barrier + oracle + real-worker) | — |
| 6 | Stale-effect rejection (publish gate) | ✅ partial — unit gate is production today, worker-level tests track #3618 | ✅ `stale_generation_is_rejected…`, `rejected_publish_never_invokes…` (track #3618); unit gate `publish_parsed_if_current_*` (production today) | — |
| 7 | Provider honesty canaries (10 providers) | ✅ this PR | ✅ per-provider + synthetic cross-provider canary | — |
| 7a | — signature-help canary | ✅ this PR (reads `current_parsed`) | ✅ `signature_help_no_stale_claim_during_pending_parse_gap` + `signature_help_never_answers_from_stale_ast_with_matching_name_during_pending_parse_gap` + headline canary assertions | — |
| 7b | — diagnostics honesty-through-gap canary | ✅ this PR | ✅ `pull_document_diagnostic_does_not_report_a_fixed_syntax_error_as_current_during_pending_parse_gap` (document-pull); workspace-pull already covered | — |
| — | — real-async cross-provider canary (`…_real_async_worker`) | ✅ tracks #3618 (needs installed `ParseWorker`) | ✅ | — |
| 8 | Neovim receipts (no full-parse/parent-map in didChange) | ✅ worker-shape variant tracks #3618 | ✅ `ux_neovim_ranged_typing_medium_file_receipt` (worker-shape assertions; see §8's two-tests-one-name caveat) | — |

**Merge-gate note.** The two branch-protection required checks
(`Perl LSP Rust Small Result`, `ripr+ New Gap Gate`) must be green on the SHA
that lands each new test. The feature-gated integration tests (§7, §8) do NOT run
under a bare `--test` invocation — any CI lane proving these must pass
`--features expose_lsp_test_api` (and `workspace` for §7), or it green-lights 0
tests. Separately: any lane citing §0–§6 or the worker-shape half of §8 as
"covered" must be running against #3618 (or a branch/commit that has merged
it) — check that PR's state directly rather than trusting this document's
prose, which does not track live merge status.

---

## 10. Remaining gap

1. **§3b — Shutdown drain + self-join-from-callback-thread regression test.**
   The production fix (#3618's `Weak<LspServer>` downgrade in
   `install_default_parse_worker`) breaks the `LspServer <-> ParseWorker`
   reference cycle at its root, but `ParseWorker::drop` itself still has no
   self-join guard — see the §3b Correction for why this document's earlier
   "Drop guard skips joining its own handle" description was wrong. #3618
   separately added a regression test that drops the server's last `Arc`
   from a dedicated external thread (proving the cycle-break generally), but
   that test does not reproduce the callback-thread-holds-the-last-ref case
   §3b(ii) describes — it's protected only by "the suite doesn't hang," which
   a future refactor of `ParseWorker::drop` or the `Weak` callback could
   reintroduce undetected. Two small deterministic tests still needed
   (drain-on-shutdown; self-join-from-callback-thread with a bounded-timeout
   watchdog, run from *inside* a worker thread's own `on_published`
   invocation). Tracked for closure via #3618.

§7a and §7b (signature-help and diagnostics honesty-through-gap canaries) are
closed **by this PR (#3649)** — see §7 for the test names and receipts. §6 is
**partial**: its `document.rs` unit-level gate is production code today,
independent of #3618; its worker-level tests are not — do not read §6 as
either "fully done" or "fully gap."

Conditions 1, 2, 3a, 4, 5 (worker layer), 6 (worker layer), and the
worker-shape half of 8 are **fully and deterministically covered on the
branch that lands via #3618** — check that PR directly on GitHub for its
current merge status rather than trusting a snapshot in this document.
