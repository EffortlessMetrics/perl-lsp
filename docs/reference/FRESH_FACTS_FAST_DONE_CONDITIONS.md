# Fresh Facts Fast — Done-Condition Proof Suite

**Status:** living spec · **Program:** Fresh Facts Fast (off-lock async parse worker, #3396) · **Program state (2026-07-11):** **COMPLETE (2026-07-11)** — #3816 self-join fix merged (#3825); §3b merge-proven — see Reconciliation below

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

## Reconciliation to current truth (2026-07-11)

The substrate PRs this document was written *ahead of* have since merged. Verified
against `origin/main` @ `d1b5222e6`; check GitHub for anything dated after this.

**Merged (all of §0–§8's substrate now lands on `main`):**

| PR | State | What it landed |
|----|-------|----------------|
| #3618 | MERGED 2026-07-11 | Off-lock async parse worker; `didChange` returns before parsing; `Weak<LspServer>` downgrade breaks the Arc cycle (#3396) |
| #3589 | MERGED 2026-07-10 | Explicit provider behavior on the pending-parse gap + `sub_foo` freshness canary |
| #3649 | MERGED 2026-07-10 | Signature-help + diagnostics honesty canaries (§7a/§7b) + this done-condition spec |
| #3765 | MERGED 2026-07-11 | Generation-owned lazy analyzer + type environment on `ParsedSnapshot` |
| #3811 | MERGED 2026-07-11 | Hover migrated to generation-owned analyzer/type_environment; retired uri+hash caches |
| #3817 | MERGED 2026-07-11 | §3b deterministic shutdown-drain test + self-join repro (repro `#[ignore]`d at the time, pending #3816; un-`#[ignore]`d by #3825 below — see §3b) |
| #3825 | MERGED 2026-07-11 (`fc1a1dde2629`) | Self-join guard in `Drop for ParseWorker` (self-thread-id skip); un-`#[ignore]`s the §3b self-join repro, which now passes (closes #3816) |

**Provider freshness migration (generation-owned facts):**

- **Completion** — migrated to generation-owned cells (#3765). Done.
- **Hover** — migrated to generation-owned analyzer/type_environment; old
  uri+hash caches deleted (#3811). Done.
- **References / rename** — **verified already-fresh; no migration needed.**
  Both are generation-gated through `current_parsed()` (they fail closed during
  the pending-parse gap — see §7's `references_fail_closed_during_pending_parse_gap`
  and `rename_fails_closed_during_pending_parse_gap`). `find_all_references`
  answers from symbol-table byte-ranges, not from source-text/type features, so
  there is no stale-fact surface to migrate off (per the #3396 investigation).
  Recorded here so a fresh session does **not** re-attempt a non-existent
  migration.

**Deterministic concurrency suite — 6/6 conditions COVERED.** Conditions 1, 2,
3a, 4, 5, 6 each have a deterministic (barrier/condvar/shutdown-signal, never
sleep-based) test that runs. §3b (condition 3) is now **COVERED**: its
drain-on-shutdown half was already proven, and its self-join repro
(`self_join_from_a_worker_callback_thread_does_not_deadlock_shutdown`) is
un-`#[ignore]`d and passing on `main` now that #3825 landed the self-thread-id
guard in `Drop for ParseWorker` (see §3b for detail).

**`on_activated_completes_before_enqueue…` is not a defect.** The flaky signal
observed on that test is a **contention symptom** — an unbounded condvar wait
that starves under CPU pressure, not a correctness bug in the worker. A
bounded-wait guard was added as a robustness nicety; the underlying invariant
was never wrong.

**Last outstanding code item — RESOLVED: #3816** — the self-join deadlock in
`Drop for ParseWorker` (the last `Arc` dropped on a worker thread) is fixed by
a self-thread-id skip-guard, merged via **#3825** (`fc1a1dde2629`, 2026-07-11);
#3816 is closed. The §3b self-join repro is un-`#[ignore]`d and passing on
`main`. With that, the program is **COMPLETE** — no outstanding code items
remain.

> The per-section "check #3618 directly on GitHub for its current merge state"
> hedges below were written before these PRs merged. They are now resolved by
> the table above; the **name-not-line-number** reference discipline still
> applies. Sections still describe seams by stable symbol name — re-grep, don't
> trust line numbers.

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
| `Weak<LspServer>` downgrade in `on_published` (breaks the Arc cycle) | `crates/perl-lsp-rs/src/runtime/mod.rs`, `install_default_parse_worker` (tracks #3618) | The callback only ever holds a transient strong ref via `cb_server.upgrade()`, so the server's strong count can reach zero without a worker thread being forced to join itself — narrowing the self-join-from-callback-thread window at the cycle root. `ParseWorker::drop` itself (`parse_worker.rs`) additionally gained a self-thread-id skip-guard via #3825, so it no longer self-joins even if a worker thread does end up dropping the last strong `Arc<ParseWorker>` — see §3b. |
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

### 3b. Shutdown drain + self-join safety — **COVERED (merge-proven)**

> **Update (2026-07-11, #3825 merged — §3b now merge-proven).** Both
> deterministic regression tests this section called for exist in the
> `parse_worker.rs` test module and pass on `main`:
> - **Drain-on-shutdown — DONE, passing.**
>   `shutdown_drains_a_coalesced_job_never_itself_dequeued_before_the_request`
>   proves a queued-but-unstarted job is drained on drop.
> - **Self-join-from-callback-thread — DONE, passing (un-`#[ignore]`d).**
>   `self_join_from_a_worker_callback_thread_does_not_deadlock_shutdown`
>   constructs the exact ordering directly against `ParseWorker` (a worker
>   thread resurrects a strong `Arc<ParseWorker>` from a `Weak`, becomes the
>   last owner, and drops it — driving `ParseWorker::drop`'s `handle.join()`
>   into a would-be self-join) with a **bounded** (`wait_for`/`TEST_TIMEOUT`)
>   watchdog. It was `#[ignore]`d while it reproduced a **real, confirmed**
>   deadlock, because `Drop for ParseWorker` had no self-thread-id guard. #3825
>   (`fc1a1dde2629`, 2026-07-11) added the self-thread-id skip-guard to
>   `ParseWorker::drop` and un-`#[ignore]`d this test — it now runs and passes,
>   closing #3816. §3b is merge-proven.
>
> The `#3618`-added `dropping_the_server_joins_the_installed_parse_worker_threads`
> (external-thread drop) remains as described below — it proves the cycle-break
> generally but does not reproduce the callback-thread case, which is exactly
> what the now-written `self_join_…` test does.

The remainder of this section preserves the original root-cause analysis for its
design detail, **reconciled inline to the current post-#3825 state** — the
Drop-level guard and both regression tests now exist and pass on `main` (see the
update block above for the summary):

**Invariant.** (i) Dropping the `ParseWorker` requests shutdown and joins every
worker thread, draining any jobs still in `ready` before exit (`take_next`
returns `None` only when `ready` is empty *and* shutdown was requested). (ii)
Dropping the **last** `Arc<LspServer>` from *inside* a worker thread's
`on_published` callback (the `Weak::upgrade()` temp going out of scope) must
NOT self-join-deadlock.

**Two-layer defense (deep review #3649 → direct fix #3825).** Scenario (ii) is
now closed by **two** independent defenses, and both live on `main`:

- **First layer — cycle break (#3618/#3649).** `LspServer::install_default_parse_worker`
  (`crates/perl-lsp-rs/src/runtime/mod.rs`) captures `cb_server: Weak<LspServer>`
  in `on_published`'s closure via `Arc::downgrade(self)` rather than a strong
  `Arc`, so the callback only ever holds a transient strong ref
  (`cb_server.upgrade()`) for the duration of `run_post_parse_side_effects`,
  breaking the `LspServer -> ParseWorker -> worker threads -> on_published ->
  Arc<LspServer>` reference cycle at its root. At the time of the #3649 deep
  review this was the *only* defense — `impl Drop for ParseWorker` was then an
  unconditional `for handle in handles.drain(..) { let _ = handle.join(); }`
  loop with no thread-identity check, so it still had no self-join protection of
  its own.
- **Second layer — Drop-level self-join skip-guard (#3825, `fc1a1dde2629`).**
  #3825 added the missing guard directly to `ParseWorker::drop`: it reads
  `thread::current().id()` and, for any handle whose `handle.thread().id()`
  equals it, `continue`s past the `join()` (detaching that thread's own
  `JoinHandle` — safe, because the worker loop observes the shutdown flag set at
  the top of `drop` and exits on its own once `ready` drains), while still
  joining every other handle. So `ParseWorker::drop` is now structurally safe
  even if a worker thread *does* end up dropping the last strong
  `Arc<ParseWorker>`.

**`ParseWorker::drop` is no longer unguarded.** The earlier statement that the
only fix lived one layer up was accurate as of #3649 but is superseded by #3825.

**Existing coverage — dedicated and deterministic (both invariants).** Both
regression tests this section formerly said were missing now exist in the
`parse_worker.rs` test module and pass on `main`:
- **(a) Drain-on-shutdown** — `shutdown_drains_a_coalesced_job_never_itself_dequeued_before_the_request`
  (#3817) asserts a queued-but-unstarted job is drained on drop.
- **(b) Self-join-from-callback-thread** — `self_join_from_a_worker_callback_thread_does_not_deadlock_shutdown`
  (written #3817, `#[ignore]` removed by #3825) reproduces scenario (ii)
  directly: a worker thread resurrects a strong `Arc<ParseWorker>` from a
  `Weak`, becomes the last owner, and drops it — driving `ParseWorker::drop`
  into a would-be self-join — under a bounded (`wait_for`/`TEST_TIMEOUT`)
  watchdog, and now passes against the #3825 skip-guard.

The earlier "no dedicated regression test / the instrument is the only witness"
framing is **historical**: the guard is no longer protected only by code
inspection and the-suite-doesn't-hang. #3618 separately added
`dropping_the_server_joins_the_installed_parse_worker_threads` (`parse_worker.rs`
test module) — a bounded-timeout test that drops the server's last strong
`Arc<LspServer>` **on a dedicated thread it spawns for the purpose**, proving the
cycle-break generally; because that drop happens from an *external* thread it
does not by itself reproduce scenario (ii), which is exactly why the
`self_join_…` test above exists and now closes (ii).

**Both deterministic regression tests now exist and pass** (drain-on-shutdown;
self-join-from-callback-thread) — landed via #3817 and un-`#[ignore]`d by #3825,
closing this gap. A prior revision of this section proposed a specific self-join
test design and then removed it after review found the sketch could not
reproduce the callback-thread-holds-the-last-ref scenario as written (the
side-effect barrier paused *before* `on_published` ran, i.e. before
`Weak::upgrade()`). The test as actually landed places the handshake *inside*
the callback, after a successful `upgrade()`, so the external strong reference
is dropped while that callback is still holding its own — the witness the
sketch lacked. Recorded here as resolved history, not outstanding work.

**Receipt.** `jobs_panicked` (3a, existing); for 3b, the two tests'
bounded-timeout "drop returned within `TEST_TIMEOUT`" assertions — the absence
of a hang IS the receipt, and both now run green on `main`.

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
| 3b | Shutdown drain + self-join | ✅ Weak-downgrade breaks the cycle (#3618); `ParseWorker::drop` now has a self-thread-id skip-guard (#3825, `fc1a1dde2629`) | ✅ drain test `shutdown_drains_a_coalesced_job…` (#3817, passing); self-join repro `self_join_from_a_worker_callback_thread…` un-`#[ignore]`d and passing (#3825) | — |
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

## 10. No remaining gaps — program COMPLETE (2026-07-11)

1. **§3b — self-join `Drop for ParseWorker` fix — RESOLVED (#3816 closed via
   #3825).** The former single outstanding code item is done. The production
   cycle-break (#3618's `Weak<LspServer>` downgrade in
   `install_default_parse_worker`) merged, #3817 landed **both** deterministic
   regression tests —
   `shutdown_drains_a_coalesced_job_never_itself_dequeued_before_the_request`
   (drain-on-shutdown, passing) and
   `self_join_from_a_worker_callback_thread_does_not_deadlock_shutdown`
   (self-join repro, bounded-timeout) — and **#3825** (`fc1a1dde2629`,
   2026-07-11) added the self-thread-id skip-guard to `ParseWorker::drop`,
   un-`#[ignore]`d the self-join repro (which now passes), and closed #3816.
   §3b is merge-proven; the program has moved from code-complete to
   **COMPLETE**.

§7a and §7b (signature-help and diagnostics honesty-through-gap canaries) are
closed **by #3649** — see §7 for the test names and receipts. §6 is
**partial**: its `document.rs` unit-level gate is production code today; its
worker-level tests landed via #3618 (merged) — do not read §6 as either
"fully done" or "fully gap."

Conditions 1, 2, 3a, **3b**, 4, 5 (worker layer), 6 (worker layer), and the
worker-shape half of 8 are **fully and deterministically covered on `main`**
(the substrate merged via #3618, with §3b's self-join guard landing via #3825 —
see the Reconciliation section for the merge table). Every done-condition now
has a deterministic proof that runs on `main`; no condition is
written-but-not-yet-runnable.
