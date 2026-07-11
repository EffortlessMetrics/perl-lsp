//! Off-lock async parse worker (#3396 Phase 3 -- Fresh Facts Fast lane).
//!
//! Moves full-parse + parent-map construction OUT of the `didChange`
//! mutation path. `didChange` (see `runtime/text_sync.rs`) applies the text
//! edit, bumps the document generation, enqueues a coalescing parse job on
//! this worker, and returns -- no parse, no parent-map build, before the
//! notification handler returns.
//!
//! ## Shape: bounded pool, per-URI latest-only
//!
//! This is deliberately **not** a single global serial worker (a slow parse
//! on one large file must not block every other open document) and
//! deliberately **not** a thread-per-open-file or task-per-keystroke design
//! (unbounded resource growth). Instead:
//!
//! - A fixed pool of [`PARSE_WORKERS`] dedicated `std::thread`s share one
//!   [`Coordinator`].
//! - At most one job is retained *pending* per URI -- a newer edit's job
//!   replaces (coalesces) an older, not-yet-started job for the same URI.
//! - Different URIs are dispatched to different pool threads and make
//!   progress independently and concurrently; only edits to the *same* URI
//!   are serialized against each other.
//!
//! `Coordinator::{enqueue, take_next, finish}` all lock the *same* single
//! `QueueState` mutex, so the "is this URI already owned by a worker" check
//! and the "did a newer job land while I was parsing" check are atomic with
//! respect to each other -- an earlier draft that split this bookkeeping
//! across two separately-locked maps had a TOCTOU window where a job could
//! be orphaned in the pending map with no worker watching it.
//!
//! Modeled on `diagnostic_debounce::DiagnosticDebouncer` for the
//! thread+callback installation pattern: a dedicated `std::thread` per
//! worker, not a tokio task, so the worker pool works identically whether
//! or not a tokio runtime exists on the calling thread -- many
//! unit/integration tests construct `LspServer` directly with no runtime at
//! all, and this worker must not require one.
//!
//! ## Coalescing + freshness-gated publish, not cancellation
//!
//! The worker does not cooperatively interrupt an in-flight parse when a
//! newer edit arrives (unlike the synchronous fallback path's
//! `Parser::new_with_cancellation`). Correctness instead comes from two
//! independent, always-on gates:
//!
//! 1. **Coalescing before start**: at most one pending job per URI. A newer
//!    edit's job silently replaces an older, not-yet-started job for the
//!    same URI ([`ParseWorkerMetrics::jobs_coalesced`]).
//! 2. **Freshness-gated publish**: once a job's parse completes, publishing
//!    goes through
//!    [`crate::state::DocumentState::publish_parsed_if_current`] (#3579),
//!    which rejects a publish whose generation no longer matches the
//!    document's current generation
//!    ([`ParseWorkerMetrics::jobs_rejected_stale`]).
//!
//! Both gates only need to be "eventually correct" -- a wasted parse that
//! is later rejected at publish is a bounded CPU cost, never a correctness
//! hazard, because publication is always freshness-gated. `jobs_cancelled`
//! stays 0 today; wiring true mid-parse cancellation into this queue is a
//! deliberate follow-up, not required for correctness here.
//!
//! ## Publication transaction
//!
//! `process_job` parses into **private** locals, builds a **private**
//! `ParsedSnapshot`, then takes the `documents` lock exactly once to check
//! document-instance identity (see below) and attempt
//! `publish_parsed_if_current`. If rejected: return immediately, zero side
//! effects -- no diagnostics, no index update, no symbol reindex. If
//! accepted: the post-publish callback receives the data captured at parse
//! time directly (never a fresh lookup of "the document" after the fact,
//! which would reopen exactly the staleness window the publish gate just
//! closed).
//!
//! A parse failure (`ast: None`) still builds and attempts to publish a
//! `ParsedSnapshot` with `degradation_tier: Minimal` -- a current-generation
//! parse failure must supersede an older successful snapshot, not leave it
//! current forever.
//!
//! ## Document-instance identity (the close/reopen ABA hazard)
//!
//! A plain `u32` generation compare is not enough: `textDocument/didClose`
//! removes the `DocumentState` entirely (see
//! `LspServer::evict_open_document_session_state`), and a subsequent
//! `didOpen` for the *same URI* installs a brand-new `DocumentState` with a
//! fresh `Arc<AtomicU32>` generation counter starting back at 0. A parse job
//! queued against the old document could, in principle, be dequeued after
//! the close+reopen cycle and find a *numerically* matching generation on
//! the new document purely by coincidence. Each [`ParseJob`] therefore
//! carries the exact `Arc<AtomicU32>` handle it was enqueued against, and
//! the worker requires `Arc::ptr_eq` identity -- not just numeric equality
//! -- before it will even attempt `publish_parsed_if_current`. A job whose
//! document instance was closed (and possibly reopened) under it is
//! rejected regardless of what generation number the new instance happens
//! to be at.

use crate::state::{DocumentState, ParsedSnapshot};
use parking_lot::{Condvar, Mutex};
use perl_lsp_rs_core::tooling::performance::AstCache;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::thread;
use std::time::Instant;

/// `Arc`-wrapped documents map, made `Send + Sync` for background threads
/// and tasks that need to touch it outside `LspServer`'s own methods.
///
/// `DocumentState`'s `ParentMap` (inside a published `ParsedSnapshot`)
/// contains `*const Node` raw pointers, so `HashMap<String, DocumentState>`
/// is not auto-`Send`/`Sync`. `LspServer` itself already carries the exact
/// same `unsafe impl Send/Sync` justification (see `runtime/mod.rs`): those
/// pointers are only ever accessed through this `Mutex`, which provides the
/// synchronization the raw pointers themselves cannot. This newtype exists
/// so background workers/tasks -- which only ever touch the documents map
/// through the same `Mutex` -- can carry the same guarantee without a
/// second unsafe impl scattered across every such call site. Used by both
/// the parse worker pool below and the background workspace-index task's
/// own freshness re-check in `runtime/text_sync.rs`.
#[derive(Clone)]
pub(crate) struct DocumentsHandle(pub(crate) Arc<Mutex<HashMap<String, DocumentState>>>);

// SAFETY: see the doc comment on `DocumentsHandle` above, and the identical
// justification on `unsafe impl Send/Sync for LspServer` in `runtime/mod.rs`
// -- the raw pointers inside `ParentMap` are only ever read/written while
// holding this `Mutex`, which is the actual synchronization boundary.
#[allow(unsafe_code)]
unsafe impl Send for DocumentsHandle {}
#[allow(unsafe_code)]
unsafe impl Sync for DocumentsHandle {}

impl std::ops::Deref for DocumentsHandle {
    type Target = Mutex<HashMap<String, DocumentState>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Number of dedicated worker threads sharing the coordinator. Bounded
/// (not one thread per open document, not one global thread) so a slow
/// parse on one large file occupies at most one slot while other documents
/// continue to make progress on the remaining threads. Matches
/// `scheduler::READ_WORKERS`'s order of magnitude for consistency.
const PARSE_WORKERS: usize = 4;

/// Immutable inputs for one off-lock parse attempt.
struct ParseJob {
    /// Original (non-normalized) client URI -- required by side-effect
    /// callbacks (diagnostics, symbol reindex) which expect the client's
    /// URI shape, not the normalized map key.
    uri: String,
    normalized_uri: String,
    generation: u32,
    /// Document-instance identity handle -- see the module-level "ABA
    /// hazard" docs.
    generation_handle: Arc<AtomicU32>,
    /// Captured immutable source text. The worker NEVER re-reads
    /// `DocumentState::text` after enqueue -- re-reading current mutable
    /// text here would silently parse a *different* edit than the one this
    /// job's generation number claims to represent.
    text: Arc<str>,
    /// Enqueue time, for the (opt-in) `parse_worker.edit_to_publish` timing
    /// span.
    enqueued_at: Instant,
}

/// Proof that a `ParsedSnapshot` was accepted by `publish_parsed_if_current`
/// for a specific document instance + generation. Constructed only after a
/// successful, freshness-gated publish -- carries the exact data that was
/// just accepted so the callback never needs to re-look-up the document
/// (which would reopen a staleness window).
///
/// This is the ONLY sanctioned input to
/// `LspServer::commit_parse_effect_if_current` -- every deferred post-parse
/// side effect (diagnostics, document-symbol reindex, workspace-index
/// replacement, symbol-cache updates, semantic-fact publication, any
/// freshness-claiming trace) commits through that function with this ticket,
/// which re-validates `(document_instance, generation)` at the moment of
/// commit rather than trusting that this ticket was valid when constructed.
pub(crate) struct PublishedParseTicket {
    pub uri: String,
    /// Document-instance identity -- the exact `Arc<AtomicU32>` this parse
    /// was performed against. See the module docs' "close/reopen ABA
    /// hazard" section for why this, not just the generation number, is
    /// required.
    pub document_instance: Arc<AtomicU32>,
    pub generation: u32,
    pub snapshot: Arc<ParsedSnapshot>,
    pub text: Arc<str>,
    /// Whether the pending-parse lifecycle this ticket belongs to already
    /// has its `IndexCoordinator::notify_parse_complete` owned by the async
    /// worker's settle hook (`ParseWorker::spawn_with_pending_count_hooks`'s
    /// `on_settled`, fired exactly once per lifecycle from
    /// `Coordinator::finish`'s terminal branch -- see #3660). `true` for
    /// every ticket `process_job` constructs (the off-lock async path,
    /// where `install_default_parse_worker` wires `on_settled` to do this);
    /// `false` for the synchronous fallback path, which has no worker
    /// queue / `finish()` / settle hook at all and must keep firing
    /// `notify_parse_complete` itself the way it always has.
    /// `run_post_parse_side_effects` (shared by both paths) checks this to
    /// avoid double-crediting the async path's decrement while still
    /// crediting the sync path's.
    pub settle_notified_by_worker: bool,
}

/// Worker-visible counters, read by tests and (future) diagnostics.
#[derive(Debug, Default)]
pub(crate) struct ParseWorkerMetrics {
    /// Total `enqueue` calls, regardless of coalescing outcome.
    pub jobs_enqueued: AtomicU64,
    /// Jobs actually dequeued and parsed (excludes jobs coalesced away
    /// before ever being dequeued).
    pub jobs_started: AtomicU64,
    /// Jobs replaced in the pending slot before a worker ever started them.
    pub jobs_coalesced: AtomicU64,
    /// Reserved: jobs cooperatively cancelled mid-parse. Always 0 today --
    /// see the module docs' "coalescing + freshness-gated publish, not
    /// cancellation" section. Read via `ParseWorkerMetricsSnapshot` (test
    /// API only); not incremented or read anywhere in the default build.
    #[allow(dead_code)]
    pub jobs_cancelled: AtomicU64,
    /// Jobs that were dequeued, parsed, but rejected at publish time
    /// (superseded generation or a document-instance mismatch).
    pub jobs_rejected_stale: AtomicU64,
    /// Jobs whose publish succeeded.
    pub jobs_published: AtomicU64,
    /// Subset of `jobs_published` where the published snapshot carried
    /// `ast: None` (a current-generation parse failure that still had to
    /// supersede an older successful snapshot).
    pub failures_published: AtomicU64,
    /// High-water mark of `QueueState::pending.len()`, observed at enqueue
    /// time. A gauge, not a counter.
    pub queue_depth_max: AtomicU64,
    /// Jobs whose `process_job` call panicked (e.g. a pathological input
    /// panicking inside the parser). The worker recovers via
    /// `std::panic::catch_unwind`, still releases the URI via
    /// `Coordinator::finish` (so the document is never permanently
    /// orphaned), and keeps the worker thread alive to process further
    /// jobs -- see the `worker_loop` doc comment.
    pub jobs_panicked: AtomicU64,
}

/// Point-in-time snapshot of [`ParseWorkerMetrics`].
///
/// Only constructed by [`ParseWorkerMetrics::snapshot`], which is itself
/// only reachable via `ParseWorker::metrics()` -- both test-API-only
/// consumers (`runtime/test_api.rs`'s `test_parse_worker_metrics`). Dead in
/// the default (no `expose_lsp_test_api`, non-test) build.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct ParseWorkerMetricsSnapshot {
    pub jobs_enqueued: u64,
    pub jobs_started: u64,
    pub jobs_coalesced: u64,
    pub jobs_cancelled: u64,
    pub jobs_rejected_stale: u64,
    pub jobs_published: u64,
    pub failures_published: u64,
    pub queue_depth_max: u64,
    pub jobs_panicked: u64,
}

impl ParseWorkerMetrics {
    fn bump_queue_depth(&self, depth: usize) {
        let depth = depth as u64;
        self.queue_depth_max.fetch_max(depth, Ordering::SeqCst);
    }

    /// Test-API-only consumer (`ParseWorker::metrics()` ->
    /// `test_parse_worker_metrics`); dead in the default build.
    #[allow(dead_code)]
    fn snapshot(&self) -> ParseWorkerMetricsSnapshot {
        ParseWorkerMetricsSnapshot {
            jobs_enqueued: self.jobs_enqueued.load(Ordering::SeqCst),
            jobs_started: self.jobs_started.load(Ordering::SeqCst),
            jobs_coalesced: self.jobs_coalesced.load(Ordering::SeqCst),
            jobs_cancelled: self.jobs_cancelled.load(Ordering::SeqCst),
            jobs_rejected_stale: self.jobs_rejected_stale.load(Ordering::SeqCst),
            jobs_published: self.jobs_published.load(Ordering::SeqCst),
            failures_published: self.failures_published.load(Ordering::SeqCst),
            queue_depth_max: self.queue_depth_max.load(Ordering::SeqCst),
            jobs_panicked: self.jobs_panicked.load(Ordering::SeqCst),
        }
    }
}

// =========================================================================
// Coordinator: single-lock bookkeeping shared by all worker threads
// =========================================================================

struct QueueState {
    /// Latest coalesced job per URI, not yet picked up by a worker.
    pending: HashMap<String, ParseJob>,
    /// URIs with a pending job waiting for a free worker slot (FIFO).
    ready: VecDeque<String>,
    /// URIs currently "owned" by a worker -- either sitting in `ready` or
    /// actively being parsed. Prevents the same URI being dispatched to two
    /// worker threads at once.
    active: HashSet<String>,
}

struct Coordinator {
    state: Mutex<QueueState>,
    cvar: Condvar,
    shutdown: AtomicBool,
    metrics: Arc<ParseWorkerMetrics>,
    /// Fired synchronously, on the enqueuing thread, exactly when a call to
    /// `enqueue` newly claims `active` ownership of a URI (`newly_active`) --
    /// called WHILE STILL HOLDING `self.state`'s lock, before it is
    /// released. Pairs with `ParseWorker::spawn_with_pending_count_hooks`'s
    /// `on_settled` (fired from `finish()`'s terminal branch) to couple
    /// BOTH the increment and the decrement of the pending-parse lifecycle
    /// to active-claim ownership under this same `state` lock -- see
    /// #3618's settle-before-increment race (cubic, two rounds):
    ///
    /// Round 1: calling the increment from the CALLER of
    /// `ParseWorker::enqueue`, after it returns, left a window where an
    /// unusually fast worker could dequeue, process, and settle (calling
    /// `on_settled`'s decrement, which floors at 0 via `checked_sub`)
    /// before the caller's own increment call ever ran, permanently
    /// stranding the counter at 1.
    ///
    /// Round 2 (this field): moving the call inside `enqueue` but AFTER
    /// `drop(state)` (still before `notify_one()`) narrowed but did not
    /// close the same class of race: `take_next()` acquires `self.state`
    /// unconditionally at the top of its own loop, not only via
    /// `notify_one()`'s wakeup -- a worker thread already contending for
    /// the lock (e.g. one that just returned from `finish()` on a
    /// different job and looped straight back into `take_next()`) could
    /// win an unfair `parking_lot::Mutex` acquisition the instant
    /// `drop(state)` ran, see this URI already pushed to `ready`, and
    /// dequeue + process + settle it before a POST-`drop(state)` call to
    /// this hook ever executed on the enqueuing thread.
    ///
    /// Calling this hook BEFORE `drop(state)` -- while the lock is still
    /// held -- closes this completely: no other thread can acquire `state`
    /// (and therefore cannot reach `take_next`'s dequeue at all) until this
    /// call returns. The production implementation
    /// (`IndexCoordinator::notify_change` via a `Weak<LspServer>` upgrade)
    /// is fast, non-blocking, and touches only `IndexCoordinator`'s own
    /// separate internal lock -- never this `Coordinator`'s `state` --
    /// so holding `state` across the call carries no deadlock or
    /// reentrancy risk.
    on_activated: Arc<dyn Fn(&str) + Send + Sync>,
}

impl Coordinator {
    fn new(
        metrics: Arc<ParseWorkerMetrics>,
        on_activated: Arc<dyn Fn(&str) + Send + Sync>,
    ) -> Self {
        Self {
            state: Mutex::new(QueueState {
                pending: HashMap::new(),
                ready: VecDeque::new(),
                active: HashSet::new(),
            }),
            cvar: Condvar::new(),
            shutdown: AtomicBool::new(false),
            metrics,
            on_activated,
        }
    }

    /// Enqueue (or coalesce-replace) a parse job for its URI. Returns `true`
    /// iff this call newly claimed `active` ownership of the URI (nothing
    /// was queued or in-flight for it) -- see the doc comment on
    /// `ParseWorker::enqueue`, the public wrapper this backs.
    fn enqueue(&self, job: ParseJob) -> bool {
        self.metrics.jobs_enqueued.fetch_add(1, Ordering::SeqCst);
        let uri = job.normalized_uri.clone();
        let mut state = self.state.lock();
        let replaced = state.pending.insert(uri.clone(), job).is_some();
        self.metrics.bump_queue_depth(state.pending.len());
        let newly_active = state.active.insert(uri.clone());
        if newly_active {
            // Wasn't already owned by a worker -- needs dispatching.
            state.ready.push_back(uri.clone());
            // While STILL HOLDING `state` -- not after `drop(state)` -- see
            // `on_activated`'s doc comment. `take_next()` unconditionally
            // acquires this SAME lock at the top of its loop (not only via
            // `notify_one()`'s wakeup path): a worker thread already
            // contending for it (e.g. one that just returned from
            // `finish()` on a different job and looped straight back into
            // `take_next()`) could win an unfair `parking_lot::Mutex`
            // acquisition the instant the lock is released, see this URI
            // already in `ready`, and dequeue it -- all before a
            // post-`drop(state)` call to `on_activated` on THIS thread ever
            // ran. Calling it before `drop(state)` closes that window
            // completely: no other thread can acquire `state` (and
            // therefore cannot reach `take_next`'s dequeue) until this call
            // returns, so there is no ordering race with the eventual
            // `on_settled` decrement for this same lifecycle, full stop --
            // not merely "before `notify_one()`, which is usually enough."
            (self.on_activated)(&uri);
            drop(state);
            self.cvar.notify_one();
        } else if replaced {
            // Already owned by a worker (queued or in-flight); this enqueue
            // replaced a not-yet-started job that was waiting behind it.
            self.metrics.jobs_coalesced.fetch_add(1, Ordering::SeqCst);
        }
        newly_active
    }

    /// Block until a URI is ready, then atomically pop it from `ready` and
    /// remove+return its current pending job. Returns `None` once shutdown
    /// has been requested and no more work remains.
    fn take_next(&self) -> Option<(String, ParseJob)> {
        let mut state = self.state.lock();
        loop {
            if let Some(uri) = state.ready.pop_front() {
                if let Some(job) = state.pending.remove(&uri) {
                    return Some((uri, job));
                }
                // Defensive: a URI in `ready` always has a matching pending
                // entry by construction (`enqueue` always inserts before
                // pushing to `ready`, and `finish` only re-pushes when
                // `pending` still holds an entry). Loop rather than panic.
                continue;
            }
            if self.shutdown.load(Ordering::SeqCst) {
                return None;
            }
            self.cvar.wait(&mut state);
        }
    }

    /// Called by a worker after finishing a job for `uri`: if a newer job
    /// landed in `pending` while this one was being processed, re-queue it
    /// (still latest-only -- no thread-per-keystroke); otherwise release
    /// ownership of the URI.
    ///
    /// Returns `true` iff this call released ownership -- the terminal
    /// settle for this URI's pending-parse lifecycle (see
    /// `ParseWorker::spawn`'s `on_settled` hook) -- or `false` if a
    /// coalesced successor was re-queued and the lifecycle continues.
    fn finish(&self, uri: &str) -> bool {
        let mut state = self.state.lock();
        let settled = if state.pending.contains_key(uri) {
            state.ready.push_back(uri.to_string());
            false
        } else {
            state.active.remove(uri);
            true
        };
        // Notify unconditionally (not just on re-queue) so
        // `wait_until_settled` waiters wake up on every completion, not
        // only on jobs that get re-queued.
        drop(state);
        self.cvar.notify_all();
        settled
    }

    fn request_shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
        self.cvar.notify_all();
    }

    /// Block (condvar-based, never a sleep loop) until `uri` has no pending
    /// or in-flight job, or `timeout` elapses. Convenience for
    /// non-correctness-critical callers (e.g. a receipt test waiting for a
    /// burst of edits to settle) that don't need the zero-flake barrier
    /// control the deterministic invariant tests use. Test-API-only
    /// consumer (`test_wait_for_parse_worker_settled`); dead in the default
    /// build.
    #[allow(dead_code)]
    fn wait_until_settled(&self, uri: &str, timeout: std::time::Duration) -> bool {
        let deadline = std::time::Instant::now() + timeout;
        let mut state = self.state.lock();
        loop {
            if !state.active.contains(uri) {
                return true;
            }
            let now = std::time::Instant::now();
            if now >= deadline {
                return false;
            }
            let remaining = deadline - now;
            self.cvar.wait_for(&mut state, remaining);
        }
    }
}

/// RAII guard: calls `coord.finish(uri)` when dropped, recording whether
/// that call was the terminal settle into `settled` (see `Coordinator::finish`'s
/// return value). Constructed BEFORE calling `process_job` in the worker
/// loop (not inside the `catch_unwind` closure), so a job's URI is released
/// on every exit from that loop iteration -- normal completion, a panic
/// recovered by `catch_unwind`, or (defensively) a panic somewhere outside
/// that wrapped closure. See the worker loop's own doc comment in
/// `ParseWorker::spawn` for the full panic-recovery model and why this
/// guard's lock acquisition is never nested inside another lock during
/// unwind.
///
/// `settled` is a `&Cell<bool>` (not a return value) because `Drop::drop`
/// cannot itself return anything to the caller -- the guard's owning scope
/// reads `settled` after the guard has dropped to decide whether to fire
/// `on_settled` (see #3660: a job that never reaches `on_published` --
/// panic or terminal stale-reject -- must still credit exactly one settle
/// per lifecycle, or the pending-parse counter leaks).
struct FinishGuard<'a> {
    coord: &'a Coordinator,
    uri: &'a str,
    settled: &'a std::cell::Cell<bool>,
}

impl Drop for FinishGuard<'_> {
    fn drop(&mut self) {
        self.settled.set(self.coord.finish(self.uri));
    }
}

/// Log a worker job panic without silently swallowing it. Extracts a
/// human-readable message from the panic payload when possible (the common
/// `&str` / `String` payload shapes `panic!`/`assert!` produce); falls back
/// to a generic marker for non-string payloads rather than failing to log
/// at all.
fn record_worker_panic(uri: &str, generation: u32, payload: &(dyn std::any::Any + Send)) {
    let message = payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "<non-string panic payload>".to_string());
    tracing::error!(
        uri = %uri,
        generation,
        panic_message = %message,
        "parse worker: job panicked, recovering (job discarded, worker continues, URI released)"
    );
}

// =========================================================================
// Test-only pause/release barrier
// =========================================================================

/// Test-only pause/release gate so deterministic tests can freeze a worker
/// immediately before it attempts to publish a specific `(uri, generation)`
/// pair, without sleeps. Keyed by URI *and* generation -- two different
/// documents can independently reach generation 1, and a test pausing one
/// document must never accidentally freeze an unrelated document that
/// happens to share the same generation number.
#[cfg(any(test, feature = "expose_lsp_test_api"))]
#[derive(Default)]
struct BarrierState {
    armed: Option<(String, u32)>,
    paused: bool,
    release: bool,
}

#[cfg(any(test, feature = "expose_lsp_test_api"))]
#[derive(Default)]
pub(crate) struct ParseWorkerTestBarrier {
    state: Mutex<BarrierState>,
    cvar: Condvar,
}

#[cfg(any(test, feature = "expose_lsp_test_api"))]
impl ParseWorkerTestBarrier {
    /// Arm the barrier: a worker processing `(normalized_uri, generation)`
    /// will pause immediately before attempting to publish.
    pub(crate) fn arm(&self, normalized_uri: &str, generation: u32) {
        let mut state = self.state.lock();
        *state = BarrierState {
            armed: Some((normalized_uri.to_string(), generation)),
            paused: false,
            release: false,
        };
        self.cvar.notify_all();
    }

    /// Block until the worker reports it has paused for the armed job.
    pub(crate) fn wait_until_paused(&self) {
        let mut state = self.state.lock();
        while !state.paused {
            self.cvar.wait(&mut state);
        }
    }

    /// Release the paused worker.
    pub(crate) fn release(&self) {
        let mut state = self.state.lock();
        state.release = true;
        self.cvar.notify_all();
    }

    /// Called by a worker immediately before publishing `(uri,
    /// generation)`. A no-op unless the barrier is currently armed for
    /// exactly this pair.
    fn maybe_pause(&self, normalized_uri: &str, generation: u32) {
        let mut state = self.state.lock();
        let armed_matches =
            state.armed.as_ref().map(|(uri, armed_generation)| (uri.as_str(), *armed_generation))
                == Some((normalized_uri, generation));
        if !armed_matches {
            return;
        }
        state.paused = true;
        self.cvar.notify_all();
        while !state.release {
            self.cvar.wait(&mut state);
        }
        *state = BarrierState::default();
    }
}

/// Test-only one-shot panic injector: lets a deterministic test force
/// `process_job` to panic for a specific `(uri, generation)`, without
/// relying on the parser itself panicking on some pathological input.
/// Proves the worker's panic-recovery path (see `worker_loop`) actually
/// releases the URI and keeps the worker thread alive, rather than trusting
/// that property from code inspection alone.
#[cfg(any(test, feature = "expose_lsp_test_api"))]
#[derive(Default)]
pub(crate) struct ParseWorkerPanicInjector {
    armed: Mutex<Option<(String, u32)>>,
}

#[cfg(any(test, feature = "expose_lsp_test_api"))]
impl ParseWorkerPanicInjector {
    /// Arm: the next job processed for `(normalized_uri, generation)` will
    /// panic instead of parsing.
    pub(crate) fn arm(&self, normalized_uri: &str, generation: u32) {
        *self.armed.lock() = Some((normalized_uri.to_string(), generation));
    }

    /// Consume the armed trigger if it matches `(normalized_uri,
    /// generation)`; fires (and disarms) at most once.
    fn should_panic(&self, normalized_uri: &str, generation: u32) -> bool {
        let mut armed = self.armed.lock();
        let matches =
            armed.as_ref().map(|(uri, armed_generation)| (uri.as_str(), *armed_generation))
                == Some((normalized_uri, generation));
        if matches {
            *armed = None;
        }
        matches
    }
}

// =========================================================================
// ParseWorker: public handle
// =========================================================================

/// Off-lock async parse worker handle.
///
/// Owns the [`Coordinator`] and the pool's join handles. Dropping the last
/// handle requests shutdown and joins every worker thread; each worker
/// finishes draining whatever is left in `ready` before exiting (mirrors
/// `DiagnosticDebouncer`'s drain-on-shutdown spirit without needing special
/// casing -- `take_next` only returns `None` once `ready` is empty *and*
/// shutdown was requested).
pub(crate) struct ParseWorker {
    coordinator: Arc<Coordinator>,
    handles: Mutex<Vec<thread::JoinHandle<()>>>,
    /// Pauses a worker immediately before it attempts to publish.
    #[cfg(any(test, feature = "expose_lsp_test_api"))]
    test_barrier: Arc<ParseWorkerTestBarrier>,
    /// Pauses a worker immediately after a successful publish but before
    /// invoking `on_published` -- a deliberately SEPARATE barrier instance
    /// from `test_barrier`, not a second use of the same one. Publication
    /// validity (`publish_parsed_if_current` succeeding) does not imply
    /// side-effect validity (the deferred diagnostics/index/symbol work
    /// still being current by the time it commits) -- this barrier lets a
    /// test force exactly that race deterministically: pause after N
    /// publishes, commit N+1 for real, then release N's side effects and
    /// assert they never fired. See
    /// `LspServer::run_post_parse_side_effects`'s own freshness re-check,
    /// which is the production-code fix this barrier proves.
    #[cfg(any(test, feature = "expose_lsp_test_api"))]
    side_effect_barrier: Arc<ParseWorkerTestBarrier>,
    #[cfg(any(test, feature = "expose_lsp_test_api"))]
    panic_injector: Arc<ParseWorkerPanicInjector>,
}

impl ParseWorker {
    /// Spawn the worker pool.
    ///
    /// `on_published` is invoked (off the `documents` lock) after a
    /// successful, freshness-gated publish -- the caller (see
    /// `LspServer::install_default_parse_worker`) wires it to
    /// `LspServer::run_post_parse_side_effects` via a captured
    /// `Arc<LspServer>`, exactly like `Scheduler::new` wires the diagnostic
    /// debouncer's `publish_fn`.
    ///
    /// Thin wrapper over [`Self::spawn_with_pending_count_hooks`] with no-op
    /// hooks -- every existing test call site constructs a bare
    /// `ParseWorker` with no `IndexCoordinator` in the picture at all, so
    /// there is nothing for either hook to notify. `#[cfg(test)]`: the only
    /// callers are this module's own unit tests; production code
    /// (`LspServer::install_default_parse_worker`) calls
    /// `spawn_with_pending_count_hooks` directly to wire the real hooks.
    #[cfg(test)]
    pub(crate) fn spawn(
        documents: Arc<Mutex<HashMap<String, DocumentState>>>,
        ast_cache: Arc<AstCache>,
        on_published: Arc<dyn Fn(PublishedParseTicket) + Send + Sync>,
    ) -> Self {
        Self::spawn_with_pending_count_hooks(
            documents,
            ast_cache,
            on_published,
            Arc::new(|_uri: &str| {}),
            Arc::new(|_uri: &str| {}),
        )
    }

    /// Spawn the worker pool with explicit pending-parse-count hooks.
    ///
    /// `on_activated` is invoked exactly once per pending-parse lifecycle --
    /// synchronously, on the enqueuing thread, WHILE STILL HOLDING
    /// `Coordinator::state`'s lock, when `enqueue` newly claims `active`
    /// ownership of a URI. No other thread can acquire that lock (and
    /// therefore no worker thread can reach `take_next`'s dequeue) until
    /// this call returns -- see `Coordinator::on_activated`'s doc comment
    /// for the two-round settle-before-increment race this closes: calling
    /// the increment from the CALLER of `enqueue` after it returned (round
    /// 1), or from inside `enqueue` but after releasing the lock (round 2),
    /// each left a real window where an unusually fast/contending worker
    /// could settle first.
    ///
    /// `on_settled` is invoked exactly once per pending-parse lifecycle --
    /// when a URI's LAST outstanding job for that lifecycle finishes
    /// processing, on WHATEVER path it ended (successful publish, terminal
    /// stale-reject, or a panic caught by `catch_unwind`), never more than
    /// once and never zero times for a lifecycle that actually started (see
    /// `Coordinator::finish`'s return value and `FinishGuard`).
    ///
    /// Both hooks are coupled to `active`-claim ownership under the SAME
    /// `Coordinator::state` lock (`on_activated` fires from inside
    /// `enqueue`'s critical section; `on_settled` fires from `finish`'s,
    /// via `FinishGuard`) -- the increment and decrement of a lifecycle's
    /// pending-parse count can never race each other, because neither can
    /// ever run concurrently with the other's determining lock acquisition.
    /// The production caller (`LspServer::install_default_parse_worker`)
    /// wires `on_activated` to `IndexCoordinator::notify_change` and
    /// `on_settled` to `IndexCoordinator::notify_parse_complete` -- every
    /// lifecycle that increments the pending-parse counter gets exactly one
    /// matching decrement, regardless of how many coalesced edits landed or
    /// how the lifecycle's last job ended (#3660 and its two follow-up leaks:
    /// panic / terminal-stale-reject not crediting the decrement, and the
    /// increment/decrement ordering race this revision closes).
    pub(crate) fn spawn_with_pending_count_hooks(
        documents: Arc<Mutex<HashMap<String, DocumentState>>>,
        ast_cache: Arc<AstCache>,
        on_published: Arc<dyn Fn(PublishedParseTicket) + Send + Sync>,
        on_activated: Arc<dyn Fn(&str) + Send + Sync>,
        on_settled: Arc<dyn Fn(&str) + Send + Sync>,
    ) -> Self {
        let documents = DocumentsHandle(documents);
        let metrics = Arc::new(ParseWorkerMetrics::default());
        let coordinator = Arc::new(Coordinator::new(Arc::clone(&metrics), on_activated));
        #[cfg(any(test, feature = "expose_lsp_test_api"))]
        let test_barrier = Arc::new(ParseWorkerTestBarrier::default());
        #[cfg(any(test, feature = "expose_lsp_test_api"))]
        let side_effect_barrier = Arc::new(ParseWorkerTestBarrier::default());
        #[cfg(any(test, feature = "expose_lsp_test_api"))]
        let panic_injector = Arc::new(ParseWorkerPanicInjector::default());

        let mut handles = Vec::with_capacity(PARSE_WORKERS);
        for idx in 0..PARSE_WORKERS {
            let coord = Arc::clone(&coordinator);
            let documents = documents.clone();
            let ast_cache = Arc::clone(&ast_cache);
            let on_published = Arc::clone(&on_published);
            let on_settled = Arc::clone(&on_settled);
            let metrics = Arc::clone(&metrics);
            #[cfg(any(test, feature = "expose_lsp_test_api"))]
            let test_barrier = Arc::clone(&test_barrier);
            #[cfg(any(test, feature = "expose_lsp_test_api"))]
            let side_effect_barrier = Arc::clone(&side_effect_barrier);
            #[cfg(any(test, feature = "expose_lsp_test_api"))]
            let panic_injector = Arc::clone(&panic_injector);

            let spawned =
                thread::Builder::new().name(format!("parse-worker-{idx}")).spawn(move || {
                    // `finish(&uri)` must run exactly once per dequeued job
                    // regardless of whether `process_job` panics -- a
                    // pathological input panicking inside the parser must
                    // not permanently orphan the URI (never re-queued,
                    // `active` never cleared) or permanently shrink the
                    // pool. Two independent, complementary mechanisms:
                    //
                    // 1. `catch_unwind` keeps the WORKER THREAD alive across
                    //    a panic inside `process_job` (no thread-respawn
                    //    model -- see module docs) and is the primary
                    //    recovery path: it converts the panic to `Err`
                    //    before it can unwind any further, so in the common
                    //    case `_finish_guard` below simply drops normally at
                    //    the end of the inner scope, panic or not.
                    // 2. `FinishGuard` is an RAII guard constructed BEFORE
                    //    calling `process_job` (in this outer scope, not
                    //    inside the `catch_unwind` closure) whose `Drop`
                    //    calls `coord.finish(&uri)`. Its value is the
                    //    residual case `catch_unwind` alone does not cover:
                    //    if something between dequeue and the normal end of
                    //    this loop body panics OUTSIDE the wrapped closure
                    //    (e.g. the panic-recovery bookkeeping itself), the
                    //    guard still releases the URI as the panic unwinds
                    //    past this scope, rather than leaving it orphaned.
                    //    Any lock `process_job` itself held at panic time is
                    //    released by its OWN (inner-scope) guard before
                    //    unwinding reaches `FinishGuard` -- Rust drops
                    //    inner-scope values before outer-scope ones -- so
                    //    `finish()`'s lock acquisition is never nested
                    //    inside another lock during unwind.
                    while let Some((uri, job)) = coord.take_next() {
                        // `settled` is written by `FinishGuard::drop` (see
                        // its own doc comment) and read AFTER the guard's
                        // scope ends -- `Drop::drop` can't return a value
                        // directly, so this `Cell` is the handoff.
                        let settled = std::cell::Cell::new(false);
                        {
                            let _finish_guard =
                                FinishGuard { coord: &coord, uri: &uri, settled: &settled };
                            let result =
                                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                    process_job(
                                        &job,
                                        &documents,
                                        &ast_cache,
                                        &on_published,
                                        &metrics,
                                        #[cfg(any(test, feature = "expose_lsp_test_api"))]
                                        &test_barrier,
                                        #[cfg(any(test, feature = "expose_lsp_test_api"))]
                                        &side_effect_barrier,
                                        #[cfg(any(test, feature = "expose_lsp_test_api"))]
                                        &panic_injector,
                                    );
                                }));
                            if let Err(payload) = result {
                                metrics.jobs_panicked.fetch_add(1, Ordering::SeqCst);
                                record_worker_panic(&uri, job.generation, &payload);
                            }
                            // `_finish_guard` drops here at the normal end
                            // of this inner scope, calling
                            // `coord.finish(&uri)` exactly once for this
                            // dequeued job and recording into `settled`
                            // whether it was the terminal settle.
                        }
                        // #3660 follow-up: fire the settle hook exactly
                        // once per lifecycle, on WHATEVER path it ended.
                        // `on_published` alone is not a reliable settle
                        // signal -- a panic (caught above) or a terminal
                        // stale-reject (`process_job` returning without
                        // calling `on_published`) never invoke it, which
                        // would otherwise leave this lifecycle's
                        // `notify_change` permanently uncredited.
                        //
                        // `catch_unwind`-wrapped for the same reason
                        // `process_job` above is: `on_settled` runs
                        // arbitrary caller code (in production, it calls
                        // into `IndexCoordinator::notify_parse_complete`,
                        // which has no reason to panic today, but nothing
                        // in this generic worker module should assume that
                        // forever) OUTSIDE the `FinishGuard`'s scope --
                        // unlike `process_job`, an unhandled panic here
                        // would propagate past this `while` loop's body and
                        // terminate the whole worker thread, permanently
                        // shrinking the pool (no thread-respawn model --
                        // see module docs), instead of being recovered the
                        // same way a panicking parse already is.
                        if settled.get() {
                            let result =
                                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                    on_settled(&uri)
                                }));
                            if let Err(payload) = result {
                                record_worker_panic(&uri, job.generation, &payload);
                            }
                        }
                    }
                });
            match spawned {
                Ok(handle) => handles.push(handle),
                Err(e) => tracing::error!(error = %e, idx, "parse worker thread spawn failed"),
            }
        }

        Self {
            coordinator,
            handles: Mutex::new(handles),
            #[cfg(any(test, feature = "expose_lsp_test_api"))]
            test_barrier,
            #[cfg(any(test, feature = "expose_lsp_test_api"))]
            side_effect_barrier,
            #[cfg(any(test, feature = "expose_lsp_test_api"))]
            panic_injector,
        }
    }

    /// Whether at least one worker thread is actually alive to dequeue and
    /// process jobs. `spawn` never fails outright -- a `thread::Builder`
    /// spawn error is logged and skipped per-thread (see the loop above) so
    /// one failure doesn't abort constructing the rest of the pool -- but if
    /// EVERY spawn attempt failed (thread/resource exhaustion), the returned
    /// `ParseWorker` would silently accept `enqueue`d jobs that no thread
    /// will ever dequeue: `didChange` on the async path returns immediately
    /// having only committed the text mutation, and the document's parse,
    /// diagnostics, and index update never happen -- a permanent, silent
    /// stall rather than a crash. Callers (see
    /// `LspServer::install_default_parse_worker`) must check this before
    /// installing the worker and fall back to the synchronous path if it is
    /// `false`.
    pub(crate) fn is_operational(&self) -> bool {
        !self.handles.lock().is_empty()
    }

    /// Enqueue (or coalesce-replace) a parse job for `normalized_uri`.
    ///
    /// Returns `true` if this call established a NEW pending-parse lifecycle
    /// for this URI (nothing was queued or in-flight for it a moment ago),
    /// `false` if it coalesced into an already-outstanding one. Callers use
    /// this to decide whether to notify a pending-parse counter (see
    /// `IndexCoordinator::notify_change` in perl-lsp-rs) exactly once per
    /// lifecycle rather than once per edit -- otherwise a rapid same-URI
    /// burst increments the counter once per coalesced-away edit but only
    /// ever decrements it once (when the *one* surviving job eventually
    /// publishes), permanently over-counting (#3660).
    pub(crate) fn enqueue(
        &self,
        uri: String,
        normalized_uri: String,
        generation: u32,
        generation_handle: Arc<AtomicU32>,
        text: Arc<str>,
    ) -> bool {
        self.coordinator.enqueue(ParseJob {
            uri,
            normalized_uri,
            generation,
            generation_handle,
            text,
            enqueued_at: Instant::now(),
        })
    }

    /// Test-API-only consumer (`test_parse_worker_metrics`); dead in the
    /// default build.
    #[allow(dead_code)]
    pub(crate) fn metrics(&self) -> ParseWorkerMetricsSnapshot {
        self.coordinator.metrics.snapshot()
    }

    /// Block (condvar-based) until `normalized_uri` has no pending or
    /// in-flight job, or `timeout` elapses. See
    /// `Coordinator::wait_until_settled`. Test-API-only consumer
    /// (`test_wait_for_parse_worker_settled`); dead in the default build.
    #[allow(dead_code)]
    pub(crate) fn wait_until_settled(
        &self,
        normalized_uri: &str,
        timeout: std::time::Duration,
    ) -> bool {
        self.coordinator.wait_until_settled(normalized_uri, timeout)
    }

    #[cfg(any(test, feature = "expose_lsp_test_api"))]
    pub(crate) fn test_barrier(&self) -> Arc<ParseWorkerTestBarrier> {
        Arc::clone(&self.test_barrier)
    }

    /// Barrier that pauses a worker immediately after a successful publish
    /// but before invoking `on_published` -- see the field doc comment on
    /// `ParseWorker::side_effect_barrier`.
    #[cfg(any(test, feature = "expose_lsp_test_api"))]
    pub(crate) fn side_effect_barrier(&self) -> Arc<ParseWorkerTestBarrier> {
        Arc::clone(&self.side_effect_barrier)
    }

    /// Test-only panic injector -- see [`ParseWorkerPanicInjector`].
    #[cfg(any(test, feature = "expose_lsp_test_api"))]
    pub(crate) fn panic_injector(&self) -> Arc<ParseWorkerPanicInjector> {
        Arc::clone(&self.panic_injector)
    }
}

impl Drop for ParseWorker {
    fn drop(&mut self) {
        self.coordinator.request_shutdown();
        let mut handles = self.handles.lock();
        for handle in handles.drain(..) {
            let _ = handle.join();
        }
    }
}

// =========================================================================
// Per-job processing (runs on a worker thread, off the documents lock)
// =========================================================================

fn process_job(
    job: &ParseJob,
    documents: &DocumentsHandle,
    ast_cache: &Arc<AstCache>,
    on_published: &Arc<dyn Fn(PublishedParseTicket) + Send + Sync>,
    metrics: &Arc<ParseWorkerMetrics>,
    #[cfg(any(test, feature = "expose_lsp_test_api"))] test_barrier: &Arc<ParseWorkerTestBarrier>,
    #[cfg(any(test, feature = "expose_lsp_test_api"))] side_effect_barrier: &Arc<
        ParseWorkerTestBarrier,
    >,
    #[cfg(any(test, feature = "expose_lsp_test_api"))] panic_injector: &Arc<
        ParseWorkerPanicInjector,
    >,
) {
    metrics.jobs_started.fetch_add(1, Ordering::SeqCst);

    // Deliberate test-only panic injection to prove the worker's own
    // panic-recovery path (`catch_unwind` + `FinishGuard` in
    // `ParseWorker::spawn`) actually releases the URI and keeps the pool
    // alive -- see `panicking_job_still_releases_its_uri_and_the_worker_keeps_processing`.
    // Never reachable outside `#[cfg(test)]` / `expose_lsp_test_api` builds,
    // and even then only when a test explicitly arms it.
    #[cfg(any(test, feature = "expose_lsp_test_api"))]
    #[allow(clippy::panic)]
    if panic_injector.should_panic(&job.normalized_uri, job.generation) {
        panic!("parse_worker test panic injection for {}:{}", job.normalized_uri, job.generation);
    }

    // Parse into PRIVATE locals, off the documents lock. Cache lookup /
    // populate mirrors the synchronous fallback path in
    // `runtime/text_sync.rs`.
    let (ast, errors) = if let Some(cached_ast) = ast_cache.get(&job.uri, &job.text) {
        (Some(cached_ast), Vec::new())
    } else {
        let code_text = crate::util::code_slice(&job.text);
        let mut parser = perl_parser::Parser::new(code_text);
        match parser.parse() {
            Ok(ast) => {
                let errors = parser.errors().to_vec();
                let arc_ast = Arc::new(ast);
                ast_cache.put(job.uri.clone(), &job.text, Arc::clone(&arc_ast));
                (Some(arc_ast), errors)
            }
            // A parse failure still produces a snapshot -- `ast: None` maps
            // to `DegradationTier::Minimal` inside `from_parse_result`, and
            // that failure snapshot still needs to reach the publish gate
            // below so it can correctly supersede an older successful one.
            Err(e) => (None, vec![e]),
        }
    };
    let is_failure = ast.is_none();

    let snapshot =
        Arc::new(ParsedSnapshot::from_parse_result(job.generation, &job.text, ast.clone(), errors));

    if crate::runtime::timing::is_enabled() {
        crate::runtime::timing::emit(crate::runtime::timing::TimingSpan::labeled(
            "parse_worker.parse",
            crate::runtime::timing::elapsed_ms(job.enqueued_at),
            job.uri.clone(),
        ));
    }

    #[cfg(any(test, feature = "expose_lsp_test_api"))]
    test_barrier.maybe_pause(&job.normalized_uri, job.generation);

    // Single lock acquisition: identity check + freshness-gated publish.
    // Parsing already happened above, off this lock.
    let t_publish_lock = Instant::now();
    let published = {
        let mut docs = documents.lock();
        match docs.get_mut(&job.normalized_uri) {
            // `Arc::ptr_eq` closes the close/reopen ABA hole described in
            // the module docs -- a numeric generation match alone is not
            // sufficient once the underlying `DocumentState` may have been
            // replaced wholesale by a didClose+didOpen cycle.
            Some(doc) if Arc::ptr_eq(&doc.generation, &job.generation_handle) => {
                doc.publish_parsed_if_current(job.generation, Arc::clone(&snapshot))
            }
            _ => false,
        }
    };
    if crate::runtime::timing::is_enabled() {
        crate::runtime::timing::emit(crate::runtime::timing::TimingSpan::labeled(
            "parse_worker.publish_lock_hold",
            crate::runtime::timing::elapsed_ms(t_publish_lock),
            job.uri.clone(),
        ));
    }

    if !published {
        metrics.jobs_rejected_stale.fetch_add(1, Ordering::SeqCst);
        return;
    }
    metrics.jobs_published.fetch_add(1, Ordering::SeqCst);
    if is_failure {
        metrics.failures_published.fetch_add(1, Ordering::SeqCst);
    }
    if crate::runtime::timing::is_enabled() {
        crate::runtime::timing::emit(crate::runtime::timing::TimingSpan::labeled(
            "parse_worker.edit_to_publish",
            crate::runtime::timing::elapsed_ms(job.enqueued_at),
            job.uri.clone(),
        ));
    }

    // Deliberately pausable HERE (a separate barrier instance from the
    // pre-publish one above) so a test can force the "publication valid,
    // side effect about to become stale" race: pause after N's publish
    // succeeds, let a real N+1 edit commit for real, then release and
    // assert N's side effects never fired. Production code closes this
    // race in `LspServer::run_post_parse_side_effects`'s own freshness
    // re-check (and the background workspace-index task's own re-check) --
    // this pause point exists to let a test PROVE that fix, not to gate
    // production behavior itself.
    #[cfg(any(test, feature = "expose_lsp_test_api"))]
    side_effect_barrier.maybe_pause(&job.normalized_uri, job.generation);

    // Accepted -- hand the callback the data captured at parse time
    // directly. No re-lookup of "the document" here: a fresh lookup would
    // reopen exactly the staleness window `publish_parsed_if_current` just
    // closed (a newer edit could have landed in the instant between the
    // publish above and a hypothetical re-fetch here). `on_published`'s
    // own implementation (`LspServer::run_post_parse_side_effects`) still
    // re-validates freshness itself immediately before mutating anything,
    // since real time may have passed here even without a test pausing it.
    on_published(PublishedParseTicket {
        uri: job.uri.clone(),
        document_instance: Arc::clone(&job.generation_handle),
        generation: job.generation,
        snapshot,
        text: Arc::clone(&job.text),
        // The async worker's settle hook (fired from `finish()`'s terminal
        // branch, back in the caller's loop) owns this lifecycle's
        // `notify_parse_complete` -- see `PublishedParseTicket`'s doc
        // comment and #3660.
        settle_notified_by_worker: true,
    });
}

// =========================================================================
// Deterministic invariant tests (barriers/channels only -- never sleeps)
// =========================================================================
//
// These construct `ParseWorker` directly (not through `LspServer`) so each
// test controls exactly the document-map shape and generation sequence it
// needs, and can install a counting `on_published` stub to make "zero side
// effects for a rejected job" a precise assertion rather than an implicit
// property of the code structure.

#[cfg(test)]
mod tests {
    use super::*;
    use perl_tdd_support::must_some;
    use std::sync::atomic::AtomicUsize;
    use std::time::Duration;

    const TEST_TIMEOUT: Duration = Duration::from_secs(5);

    fn ast_cache() -> Arc<AstCache> {
        Arc::new(AstCache::new(10, 60))
    }

    /// Build a one-document `documents` map plus that document's
    /// `Arc<AtomicU32>` generation handle and normalized URI.
    fn one_doc(
        uri: &str,
        source: &str,
    ) -> (Arc<Mutex<HashMap<String, DocumentState>>>, Arc<AtomicU32>) {
        let doc = DocumentState::new(source, 1);
        let generation_handle = doc.generation.clone();
        let mut map = HashMap::new();
        map.insert(uri.to_string(), doc);
        (Arc::new(Mutex::new(map)), generation_handle)
    }

    /// A counting `on_published` stub -- records every call so a test can
    /// assert exactly how many times side effects fired, and for which
    /// (uri, generation) pairs.
    fn counting_callback()
    -> (Arc<dyn Fn(PublishedParseTicket) + Send + Sync>, Arc<Mutex<Vec<(String, u32)>>>) {
        let calls: Arc<Mutex<Vec<(String, u32)>>> = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&calls);
        let cb: Arc<dyn Fn(PublishedParseTicket) + Send + Sync> =
            Arc::new(move |p: PublishedParseTicket| {
                recorded.lock().push((p.uri, p.generation));
            });
        (cb, calls)
    }

    fn wait_for<F: Fn() -> bool>(predicate: F, timeout: Duration) -> bool {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            if predicate() {
                return true;
            }
            if std::time::Instant::now() >= deadline {
                return false;
            }
            std::thread::yield_now();
        }
    }

    // ---- Invariant 1: didChange returns before parse completes ----------

    #[test]
    fn worker_publish_is_gated_behind_the_test_barrier() {
        let uri = "file:///barrier.pl";
        let (documents, generation_handle) = one_doc(uri, "my $a = 1;\n");
        let (cb, calls) = counting_callback();
        let worker = ParseWorker::spawn(Arc::clone(&documents), ast_cache(), cb);
        let barrier = worker.test_barrier();

        // Simulate a real edit: bump the generation (as `didChange` does)
        // BEFORE enqueueing -- generation 1.
        generation_handle.fetch_add(1, Ordering::SeqCst);
        barrier.arm(uri, 1);
        worker.enqueue(
            uri.to_string(),
            uri.to_string(),
            1,
            Arc::clone(&generation_handle),
            Arc::from("my $aa = 1;\n"),
        );

        // Block until the worker has actually reached the pause point --
        // proves the worker got as far as finishing the parse but has not
        // yet published.
        barrier.wait_until_paused();

        // While paused: current_parsed() must be None (nothing published
        // for gen 1 yet); no side effects have fired.
        {
            let docs = documents.lock();
            let doc = must_some(docs.get(uri));
            assert!(
                doc.current_parsed().is_none(),
                "current_parsed() must be None while the worker is paused before publish"
            );
        }
        assert_eq!(calls.lock().len(), 0, "no side effects before publish");
        assert_eq!(worker.metrics().jobs_published, 0);

        barrier.release();

        // Wait on the side effect actually firing, not on `jobs_published`
        // -- the metric is incremented BEFORE `on_published` is invoked
        // (see `process_job`), so polling it alone races with the callback
        // itself and can observe `jobs_published == 1` before `calls` has
        // been populated.
        assert!(
            wait_for(|| !calls.lock().is_empty(), TEST_TIMEOUT),
            "worker must invoke the side-effect callback once released"
        );
        {
            let docs = documents.lock();
            let doc = must_some(docs.get(uri));
            let current = must_some(doc.current_parsed());
            assert_eq!(current.generation(), 1);
        }
        assert_eq!(worker.metrics().jobs_published, 1);
        assert_eq!(calls.lock().as_slice(), &[(uri.to_string(), 1)]);
    }

    // ---- Invariant 2: latest generation wins -----------------------------

    #[test]
    fn stale_generation_is_rejected_latest_generation_publishes() {
        let uri = "file:///latest_wins.pl";
        let (documents, generation_handle) = one_doc(uri, "my $a = 1;\n");
        let (cb, calls) = counting_callback();
        let worker = ParseWorker::spawn(Arc::clone(&documents), ast_cache(), cb);
        let barrier = worker.test_barrier();

        // Job N: generation 1, paused right before publish.
        generation_handle.fetch_add(1, Ordering::SeqCst);
        barrier.arm(uri, 1);
        worker.enqueue(
            uri.to_string(),
            uri.to_string(),
            1,
            Arc::clone(&generation_handle),
            Arc::from("my $aa = 1;\n"),
        );
        barrier.wait_until_paused();

        // While N is paused, a newer edit lands: bump to generation 2 and
        // enqueue N+1. N is already dequeued (not coalesced) -- this
        // exercises the "already started, rejected at publish" path, not
        // the coalescing path. Deliberately NOT re-arming the barrier for
        // N+1 here: `maybe_pause`'s cleanup (`*state =
        // BarrierState::default()`) runs after N is released below, and it
        // would silently clobber an `arm()` call made for N+1 in the window
        // before N's release/cleanup completes -- one barrier instance can
        // only safely gate one in-flight pause/release cycle at a time. N+1
        // is left to publish freely; invariant 2 only requires proving N
        // gets rejected and N+1 (the latest) wins, not that N+1 itself
        // pauses.
        generation_handle.fetch_add(1, Ordering::SeqCst);
        worker.enqueue(
            uri.to_string(),
            uri.to_string(),
            2,
            Arc::clone(&generation_handle),
            Arc::from("my $aaa = 1;\n"),
        );

        // Release N: its publish must be rejected (generation moved to 2).
        barrier.release();
        assert!(
            wait_for(|| worker.metrics().jobs_rejected_stale >= 1, TEST_TIMEOUT),
            "job N must be rejected once generation has moved on"
        );

        // Wait on the side-effect callback itself, not `jobs_published`
        // (incremented before the callback runs -- see `process_job`).
        assert!(
            wait_for(|| !calls.lock().is_empty(), TEST_TIMEOUT),
            "exactly one publish (generation 2) must land and invoke its side effect"
        );
        {
            let docs = documents.lock();
            let doc = must_some(docs.get(uri));
            let current = must_some(doc.current_parsed());
            assert_eq!(current.generation(), 2, "the final published generation must be 2");
        }
        assert_eq!(worker.metrics().jobs_published, 1);
        assert_eq!(
            calls.lock().as_slice(),
            &[(uri.to_string(), 2)],
            "side effects must only have fired for the winning generation"
        );
    }

    // ---- Invariant 3: burst coalescing -----------------------------------

    #[test]
    fn rapid_burst_coalesces_to_far_fewer_jobs_than_edits() {
        let uri = "file:///burst.pl";
        let (documents, generation_handle) = one_doc(uri, "my $a = 1;\n");
        let (cb, _calls) = counting_callback();
        let worker = ParseWorker::spawn(Arc::clone(&documents), ast_cache(), cb);

        const EDITS: u32 = 20;
        for i in 1..=EDITS {
            generation_handle.fetch_add(1, Ordering::SeqCst);
            worker.enqueue(
                uri.to_string(),
                uri.to_string(),
                i,
                Arc::clone(&generation_handle),
                Arc::from(format!("my $a = {i};\n").as_str()),
            );
        }

        assert!(
            worker.wait_until_settled(uri, TEST_TIMEOUT),
            "burst must settle within the timeout"
        );

        let metrics = worker.metrics();
        assert_eq!(
            metrics.jobs_published, 1,
            "exactly one generation (the final one) must publish from the burst"
        );
        assert!(
            metrics.jobs_started < u64::from(EDITS),
            "coalescing must start far fewer jobs than edits enqueued; started={}",
            metrics.jobs_started
        );
        assert!(metrics.jobs_coalesced > 0, "at least one job must have been coalesced away");

        let docs = documents.lock();
        let doc = must_some(docs.get(uri));
        let current = must_some(doc.current_parsed());
        assert_eq!(current.generation(), EDITS, "the final generation must be the one published");
    }

    // ---- Invariant 4: independent documents do not block each other -----

    #[test]
    fn one_document_paused_does_not_block_another_documents_publish() {
        let uri_a = "file:///doc_a.pl";
        let uri_b = "file:///doc_b.pl";
        let doc_a = DocumentState::new("my $a = 1;\n", 1);
        let gen_a = doc_a.generation.clone();
        let doc_b = DocumentState::new("my $b = 1;\n", 1);
        let gen_b = doc_b.generation.clone();
        let mut map = HashMap::new();
        map.insert(uri_a.to_string(), doc_a);
        map.insert(uri_b.to_string(), doc_b);
        let documents = Arc::new(Mutex::new(map));

        let (cb, calls) = counting_callback();
        let worker = ParseWorker::spawn(Arc::clone(&documents), ast_cache(), cb);
        let barrier = worker.test_barrier();

        // Pause document A at generation 1 -- it will not be released
        // during this test.
        gen_a.fetch_add(1, Ordering::SeqCst);
        barrier.arm(uri_a, 1);
        worker.enqueue(
            uri_a.to_string(),
            uri_a.to_string(),
            1,
            Arc::clone(&gen_a),
            Arc::from("my $aa = 1;\n"),
        );
        barrier.wait_until_paused();

        // Document B must still be able to publish while A sits paused --
        // this is the test that would have caught a single-global-worker
        // regression (A occupying the only thread would starve B forever).
        gen_b.fetch_add(1, Ordering::SeqCst);
        worker.enqueue(
            uri_b.to_string(),
            uri_b.to_string(),
            1,
            Arc::clone(&gen_b),
            Arc::from("my $bb = 1;\n"),
        );

        // Wait on document B's side-effect callback itself, not
        // `jobs_published` (incremented before the callback runs).
        assert!(
            wait_for(
                || calls.lock().iter().any(|(uri, generation)| uri == uri_b && *generation == 1),
                TEST_TIMEOUT
            ),
            "document B must publish and invoke side effects without waiting for document A's barrier to release"
        );
        {
            let docs = documents.lock();
            let doc_b_state = must_some(docs.get(uri_b));
            let current = must_some(doc_b_state.current_parsed());
            assert_eq!(current.generation(), 1);
        }
        assert_eq!(worker.metrics().jobs_published, 1);

        // A is still paused, unpublished -- clean up by releasing it so the
        // worker threads can be joined when `worker` drops.
        {
            let docs = documents.lock();
            let doc_a_state = must_some(docs.get(uri_a));
            assert!(
                doc_a_state.current_parsed().is_none(),
                "document A must still be unpublished while paused"
            );
        }
        barrier.release();
        assert!(wait_for(|| worker.metrics().jobs_published >= 2, TEST_TIMEOUT));
    }

    // ---- Invariant 5: zero stale side effects ----------------------------
    // (Exercised precisely by `stale_generation_is_rejected_latest_generation_publishes`
    // above via the `calls` call-counter, which asserts the rejected
    // generation-1 job never appears in the recorded callback invocations.
    // This test adds an explicit, standalone assertion focused only on that
    // one property, using a fresh scenario.)

    #[test]
    fn rejected_publish_never_invokes_the_side_effect_callback() {
        let uri = "file:///zero_stale_side_effects.pl";
        let (documents, generation_handle) = one_doc(uri, "my $a = 1;\n");
        let call_count = Arc::new(AtomicUsize::new(0));
        let recorded_count = Arc::clone(&call_count);
        let cb: Arc<dyn Fn(PublishedParseTicket) + Send + Sync> =
            Arc::new(move |_p: PublishedParseTicket| {
                recorded_count.fetch_add(1, Ordering::SeqCst);
            });
        let worker = ParseWorker::spawn(Arc::clone(&documents), ast_cache(), cb);
        let barrier = worker.test_barrier();

        generation_handle.fetch_add(1, Ordering::SeqCst);
        barrier.arm(uri, 1);
        worker.enqueue(
            uri.to_string(),
            uri.to_string(),
            1,
            Arc::clone(&generation_handle),
            Arc::from("my $aa = 1;\n"),
        );
        barrier.wait_until_paused();

        // Supersede generation 1 while it's paused.
        generation_handle.fetch_add(1, Ordering::SeqCst);
        barrier.release();

        assert!(
            wait_for(|| worker.metrics().jobs_rejected_stale >= 1, TEST_TIMEOUT),
            "generation 1 must be rejected"
        );
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            0,
            "a rejected publish must never invoke the side-effect callback"
        );
    }

    // ---- Invariant 6: close/reopen instance identity ---------------------

    #[test]
    fn stale_job_cannot_publish_into_a_reopened_document_instance() {
        let uri = "file:///close_reopen.pl";
        let instance_a = DocumentState::new("my $a = 1;\n", 1);
        let gen_a = instance_a.generation.clone();
        let mut map = HashMap::new();
        map.insert(uri.to_string(), instance_a);
        let documents = Arc::new(Mutex::new(map));

        let (cb, calls) = counting_callback();
        let worker = ParseWorker::spawn(Arc::clone(&documents), ast_cache(), cb);
        let barrier = worker.test_barrier();

        // Start a parse for instance A at generation 1, pause before publish.
        gen_a.fetch_add(1, Ordering::SeqCst);
        barrier.arm(uri, 1);
        worker.enqueue(
            uri.to_string(),
            uri.to_string(),
            1,
            Arc::clone(&gen_a),
            Arc::from("my $aa = 1;\n"),
        );
        barrier.wait_until_paused();

        // Simulate didClose + didOpen on the same URI: replace the map
        // entry with a brand-new `DocumentState` (fresh `Arc<AtomicU32>`
        // generation counter starting back at 0, then bumped to 1 by one
        // edit) -- deliberately landing on the SAME numeric generation (1)
        // that instance A's paused job is about to try to publish, so a
        // plain `u32` compare alone would have accepted this incorrectly.
        let instance_b = DocumentState::new("my $b = 1;\n", 1);
        let gen_b = instance_b.generation.clone();
        {
            let mut docs = documents.lock();
            docs.insert(uri.to_string(), instance_b);
        }
        gen_b.fetch_add(1, Ordering::SeqCst);

        // Release A's paused job -- its publish must be rejected: same
        // numeric generation (1), different document instance.
        barrier.release();
        assert!(
            wait_for(|| worker.metrics().jobs_rejected_stale >= 1, TEST_TIMEOUT),
            "instance A's stale job must be rejected"
        );
        assert!(calls.lock().is_empty(), "instance A's stale job must trigger zero side effects");

        {
            let docs = documents.lock();
            let doc = must_some(docs.get(uri));
            assert!(
                Arc::ptr_eq(&doc.generation, &gen_b),
                "the document map must still hold instance B, untouched by A's stale publish"
            );
            assert!(
                doc.current_parsed().is_none(),
                "instance B must not have been given instance A's stale parse result"
            );
        }
    }

    // ---- Panic recovery: a panicking job never orphans its URI or ------
    // ---- shrinks the worker pool -----------------------------------------

    #[test]
    fn panicking_job_still_releases_its_uri_and_the_worker_keeps_processing() {
        let uri = "file:///panic_recovery.pl";
        let (documents, generation_handle) = one_doc(uri, "my $a = 1;\n");
        let (cb, _calls) = counting_callback();
        let worker = ParseWorker::spawn(Arc::clone(&documents), ast_cache(), cb);
        let injector = worker.panic_injector();

        // Generation 1: armed to panic instead of parsing.
        generation_handle.fetch_add(1, Ordering::SeqCst);
        injector.arm(uri, 1);
        worker.enqueue(
            uri.to_string(),
            uri.to_string(),
            1,
            Arc::clone(&generation_handle),
            Arc::from("my $aa = 1;\n"),
        );

        assert!(
            wait_for(|| worker.metrics().jobs_panicked >= 1, TEST_TIMEOUT),
            "the panicking job must be recorded as recovered"
        );
        // The panicking generation must never have published (it never got
        // that far).
        assert_eq!(worker.metrics().jobs_published, 0);

        // The URI must not be permanently orphaned: a subsequent real edit
        // to the SAME uri must still parse and publish normally. If
        // `finish(&uri)` were skipped on panic, `active` would retain this
        // URI forever and this enqueue would never be picked up.
        generation_handle.fetch_add(1, Ordering::SeqCst);
        worker.enqueue(
            uri.to_string(),
            uri.to_string(),
            2,
            Arc::clone(&generation_handle),
            Arc::from("my $aaa = 1;\n"),
        );

        assert!(
            wait_for(|| worker.metrics().jobs_published == 1, TEST_TIMEOUT),
            "a subsequent edit to the same URI must still parse and publish after a prior panic"
        );
        let docs = documents.lock();
        let doc = must_some(docs.get(uri));
        let current = must_some(doc.current_parsed());
        assert_eq!(current.generation(), 2, "the post-panic edit must be the one that publishes");
    }

    // ---- Publication validity != side-effect validity --------------------
    // (production fix lives in `LspServer::run_post_parse_side_effects`'s
    // own freshness re-check -- see
    // `text_sync::tests::stale_generation_side_effects_never_reindex_symbols`
    // for the end-to-end proof against the real symbol index. This test
    // proves the WORKER's side-effect barrier itself pauses at the right
    // point and that a real newer edit can commit while paused there,
    // which that other test's direct-call style cannot exercise on its
    // own.)

    #[test]
    fn side_effect_barrier_pauses_after_publish_and_a_newer_edit_can_commit_while_paused() {
        let uri = "file:///side_effect_barrier.pl";
        let (documents, generation_handle) = one_doc(uri, "my $a = 1;\n");
        let (cb, calls) = counting_callback();
        let worker = ParseWorker::spawn(Arc::clone(&documents), ast_cache(), cb);
        let side_effect_barrier = worker.side_effect_barrier();

        generation_handle.fetch_add(1, Ordering::SeqCst);
        side_effect_barrier.arm(uri, 1);
        worker.enqueue(
            uri.to_string(),
            uri.to_string(),
            1,
            Arc::clone(&generation_handle),
            Arc::from("my $aa = 1;\n"),
        );
        side_effect_barrier.wait_until_paused();

        // The publish itself must have ALREADY succeeded (this barrier
        // pauses AFTER publish, before the side-effect callback) --
        // current_parsed() must already report generation 1.
        {
            let docs = documents.lock();
            let doc = must_some(docs.get(uri));
            let current = must_some(doc.current_parsed());
            assert_eq!(
                current.generation(),
                1,
                "publish must land before the side-effect barrier pauses"
            );
        }
        // But the side-effect callback must not have fired yet.
        assert_eq!(calls.lock().len(), 0, "side effects must not fire before the barrier releases");

        // A real newer edit commits for real while paused.
        generation_handle.fetch_add(1, Ordering::SeqCst);

        side_effect_barrier.release();

        // The paused (now-stale) generation's callback still fires in this
        // low-level worker test (the counting stub itself doesn't
        // re-validate -- only `LspServer::run_post_parse_side_effects`
        // does, proven separately). What this test proves is the RACE
        // WINDOW itself is real and reachable: the callback fires for
        // generation 1 even though generation 2 is already current by the
        // time it does.
        assert!(
            wait_for(|| !calls.lock().is_empty(), TEST_TIMEOUT),
            "the paused callback must eventually fire once released"
        );
        assert_eq!(calls.lock().as_slice(), &[(uri.to_string(), 1)]);
        {
            let docs = documents.lock();
            let doc = must_some(docs.get(uri));
            assert_eq!(
                doc.current_generation(),
                2,
                "the document must already be at generation 2 when generation 1's side effect fires -- \
                 this is the exact race `LspServer::run_post_parse_side_effects` must guard against"
            );
        }
    }

    // ---- Pending-count hooks: increment/decrement coupled to active-claim
    // ownership, no ordering race possible ---------------------------------

    /// #3618 settle-before-increment race (cubic): proves `on_activated`
    /// (the pending-parse increment hook) is fully complete and observable
    /// on the ENQUEUING thread by the time `ParseWorker::enqueue` returns --
    /// strictly BEFORE any worker thread can even be woken to look at the
    /// job, let alone dequeue, process, and settle it. This makes the
    /// bug's ordering (an unusually fast worker settling -- and firing
    /// `on_settled`'s decrement -- before the increment for that same
    /// lifecycle had run) structurally impossible, not merely unlikely: the
    /// old code called the increment from `enqueue`'s CALLER, after
    /// `enqueue` had already returned, a genuinely separate later call the
    /// scheduler could interleave a woken worker's full settle in front of.
    ///
    /// Uses the pre-publish test barrier to make the ordering directly
    /// observable rather than inferred: arm it BEFORE enqueueing, so the
    /// worker (once woken) blocks deep inside `process_job`, well past
    /// dequeue -- if `on_activated` had not already fired by the time
    /// `enqueue` returns to this test, the sequence below could not
    /// reliably distinguish "fired before enqueue returned" from "fired
    /// racily soon after"; observing it in the log immediately after
    /// `enqueue` returns, with the worker still provably blocked at the
    /// barrier, proves the former.
    #[test]
    fn on_activated_completes_before_enqueue_returns_no_race_with_worker_settle() {
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        enum Event {
            Activated,
            Settled,
        }

        let uri = "file:///activation_ordering.pl";
        let (documents, generation_handle) = one_doc(uri, "my $a = 1;\n");
        let (cb, _calls) = counting_callback();
        let log: Arc<Mutex<Vec<Event>>> = Arc::new(Mutex::new(Vec::new()));
        let log_for_activated = Arc::clone(&log);
        let log_for_settled = Arc::clone(&log);
        let worker = ParseWorker::spawn_with_pending_count_hooks(
            Arc::clone(&documents),
            ast_cache(),
            cb,
            Arc::new(move |_uri: &str| log_for_activated.lock().push(Event::Activated)),
            Arc::new(move |_uri: &str| log_for_settled.lock().push(Event::Settled)),
        );
        let barrier = worker.test_barrier();

        generation_handle.fetch_add(1, Ordering::SeqCst);
        barrier.arm(uri, 1);
        worker.enqueue(
            uri.to_string(),
            uri.to_string(),
            1,
            Arc::clone(&generation_handle),
            Arc::from("my $aa = 1;\n"),
        );

        // Immediately after `enqueue` returns -- before waiting for
        // anything -- `on_activated` must already have run. This is the
        // load-bearing assertion: it can only pass if `on_activated` is
        // called from INSIDE `enqueue`'s own call stack (as the fix does),
        // never from a separate call the caller makes afterward.
        assert_eq!(
            log.lock().as_slice(),
            &[Event::Activated],
            "on_activated must be observable immediately after enqueue() returns, before this \
             test does anything else -- proving it cannot race a worker's settle"
        );

        // The worker is still blocked at the pre-publish barrier -- proves
        // the ordering above wasn't a lucky race, but structural: nothing
        // downstream of dequeue has even started influencing `log` yet.
        barrier.wait_until_paused();
        assert_eq!(
            log.lock().as_slice(),
            &[Event::Activated],
            "on_settled must not fire while the worker is still paused mid-processing"
        );

        barrier.release();
        assert!(
            wait_for(|| log.lock().len() == 2, TEST_TIMEOUT),
            "on_settled must eventually fire once the worker is released and finishes"
        );
        assert_eq!(
            log.lock().as_slice(),
            &[Event::Activated, Event::Settled],
            "activation must always precede settle for the same lifecycle"
        );
    }

    /// #3618 round 2 (cubic, via review-3660): the previous test proves
    /// `on_activated` completes before `enqueue()` RETURNS, but that alone
    /// does not prove no OTHER thread could have raced in during the call --
    /// it doesn't distinguish "called while still holding `state`'s lock"
    /// from "called just after releasing it, but still fast enough that a
    /// woken worker didn't win the barrier-gated race in THIS particular
    /// run." Swapping `on_activated`'s call and `drop(state)`'s order still
    /// passes the previous test, because the barrier keeps the worker
    /// parked regardless.
    ///
    /// This test instead proves the STRONGER, load-bearing invariant
    /// directly and deterministically: `on_activated` observes
    /// `Coordinator::state`'s lock as ALREADY HELD (by itself, reentrantly,
    /// via `try_lock` failing to acquire it) at the moment it runs. Since
    /// `enqueue` is the only place that ever holds this lock while calling
    /// `on_activated`, this can only be true if the call happens from
    /// INSIDE `enqueue`'s critical section, before `drop(state)` --
    /// exactly what makes it impossible for `take_next` (which
    /// unconditionally acquires the same lock at the top of its loop) to
    /// ever run concurrently with it, regardless of `notify_one()` or any
    /// other thread's scheduling.
    #[test]
    fn on_activated_observes_the_state_lock_as_still_held() {
        let uri = "file:///activation_holds_lock.pl";
        let (documents, generation_handle) = one_doc(uri, "my $a = 1;\n");
        let (cb, _calls) = counting_callback();

        // Filled in with a `Weak<Coordinator>` AFTER `worker` is
        // constructed (chicken-and-egg: the closure needs the coordinator
        // to probe its lock, but the coordinator doesn't exist until this
        // closure has already been built and passed in) -- safe because
        // `on_activated` is only ever CALLED later, by `enqueue`, well
        // after this cell has been populated below.
        let coord_cell: Arc<Mutex<Option<std::sync::Weak<Coordinator>>>> =
            Arc::new(Mutex::new(None));
        let coord_cell_for_closure = Arc::clone(&coord_cell);
        let observed_locked: Arc<Mutex<Option<bool>>> = Arc::new(Mutex::new(None));
        let observed_locked_for_closure = Arc::clone(&observed_locked);

        let worker = ParseWorker::spawn_with_pending_count_hooks(
            Arc::clone(&documents),
            ast_cache(),
            cb,
            Arc::new(move |_uri: &str| {
                if let Some(coord) =
                    coord_cell_for_closure.lock().as_ref().and_then(std::sync::Weak::upgrade)
                {
                    // `try_lock` returning `None` means the lock is
                    // currently held (by this very thread, reentrantly,
                    // from inside `enqueue`) -- `parking_lot::Mutex` is
                    // not reentrant, so a second acquisition attempt from
                    // the SAME thread fails exactly like a genuine
                    // cross-thread contender would.
                    *observed_locked_for_closure.lock() = Some(coord.state.try_lock().is_none());
                }
            }),
            Arc::new(|_uri: &str| {}),
        );
        *coord_cell.lock() = Some(Arc::downgrade(&worker.coordinator));

        generation_handle.fetch_add(1, Ordering::SeqCst);
        worker.enqueue(
            uri.to_string(),
            uri.to_string(),
            1,
            Arc::clone(&generation_handle),
            Arc::from("my $aa = 1;\n"),
        );

        assert!(
            wait_for(|| worker.metrics().jobs_published >= 1, TEST_TIMEOUT),
            "the job must publish for this test to have exercised on_activated at all"
        );
        assert_eq!(
            *observed_locked.lock(),
            Some(true),
            "on_activated must observe Coordinator::state as already held (by itself) -- \
             proving the call happens from inside enqueue's critical section, before \
             drop(state), not merely before notify_one()"
        );
    }

    /// Deterministic (no scheduler luck required) demonstration of WHY the
    /// settle-before-increment ordering the previous test rules out would
    /// actually matter: constructs the OLD buggy caller pattern by hand
    /// (increment a real `IndexMetrics`-style saturating counter manually,
    /// AFTER explicitly waiting for the job to fully settle first, exactly
    /// mirroring what a caller who incremented after `enqueue()` returned
    /// -- instead of `enqueue()` incrementing internally before any worker
    /// could act -- could race into) against the REAL semantics
    /// (`IndexMetrics::decrement_pending_parses` floors at 0 via
    /// `checked_sub`, so a decrement that arrives before its matching
    /// increment is a silent no-op, not a error). Proves the counter ends
    /// up permanently stuck at 1 in that ordering, never able to reach 0
    /// again for this lifecycle -- the exact `Degraded{ParseStorm}`-forever
    /// failure mode.
    #[test]
    fn settle_before_deferred_increment_would_permanently_strand_the_pending_count() {
        use perl_workspace::monitoring::IndexMetrics;

        let uri = "file:///deferred_increment.pl";
        let (documents, generation_handle) = one_doc(uri, "my $a = 1;\n");
        let (cb, _calls) = counting_callback();
        // No-op `on_activated` -- deliberately NOT wired to `metrics` below,
        // simulating the OLD design where the increment lived in the
        // CALLER of `ParseWorker::enqueue`, not inside `enqueue` itself.
        let worker = ParseWorker::spawn_with_pending_count_hooks(
            Arc::clone(&documents),
            ast_cache(),
            cb,
            Arc::new(|_uri: &str| {}),
            Arc::new(|_uri: &str| {}),
        );

        let metrics = IndexMetrics::new();
        assert_eq!(metrics.pending_count(), 0, "baseline must be zero");

        generation_handle.fetch_add(1, Ordering::SeqCst);
        // No barrier armed -- let the (trivial, near-instant) job run to
        // completion as fast as possible.
        worker.enqueue(
            uri.to_string(),
            uri.to_string(),
            1,
            Arc::clone(&generation_handle),
            Arc::from("my $aa = 1;\n"),
        );

        // Explicitly wait for the job to fully settle BEFORE this test's
        // own (simulated-caller) increment ever runs -- deterministically
        // constructing the exact bad ordering `on_activated`'s new
        // placement inside `enqueue` rules out, rather than hoping a real
        // scheduler race reproduces it.
        assert!(
            worker.wait_until_settled(uri, TEST_TIMEOUT),
            "the job must fully settle before this test simulates the deferred increment"
        );

        // The (simulated) old-style caller-side decrement never ran either
        // in this harness -- `on_settled` is a no-op above -- so drive the
        // metrics by hand in the SAME order the old code's threads could
        // interleave in: decrement first (as if a fast worker's settle-
        // triggered decrement had already run), increment second (as if
        // the caller's separate post-`enqueue` notify call finally got
        // scheduled).
        let after_premature_decrement = metrics.decrement_pending_parses();
        assert_eq!(
            after_premature_decrement, 0,
            "a decrement arriving before its increment floors at 0 (checked_sub), not an error -- \
             this silent floor is exactly why the ordering bug was invisible until it accumulated"
        );
        metrics.increment_pending_parses();

        assert_eq!(
            metrics.pending_count(),
            1,
            "in the OLD ordering (decrement before its matching increment), the counter is \
             permanently stranded at 1 for this lifecycle -- nothing will ever decrement it again, \
             since `on_settled` for this lifecycle already fired. This is exactly why `on_activated` \
             must run before any worker can possibly settle, not merely 'usually' before it."
        );
    }

    /// cubic P2: `on_settled` runs OUTSIDE `FinishGuard`'s scope and, prior
    /// to this fix, outside `catch_unwind` too -- unlike `process_job`,
    /// a panic there would propagate past the worker loop's body and kill
    /// the thread outright, permanently shrinking the pool (no thread-
    /// respawn model).
    ///
    /// A single panic-then-reuse round trip on ONE uri cannot distinguish
    /// "the fix works" from "the pool merely has spare capacity" -- with
    /// `PARSE_WORKERS` > 1, a DIFFERENT thread can pick up the next job
    /// even if the one that panicked genuinely died, silently masking a
    /// real regression. This test instead fires one `on_settled`-panicking
    /// job per DISTINCT uri, `PARSE_WORKERS` times over -- enough, if each
    /// panic actually killed its thread, to exhaust the entire pool -- then
    /// proves the pool still has live capacity by enqueuing one more job on
    /// a fresh uri and confirming it still gets dequeued and published
    /// within the timeout (a fully exhausted pool would never pick it up;
    /// `take_next` would simply have no thread left to call it).
    #[test]
    fn panicking_on_settled_does_not_exhaust_the_worker_pool() {
        let mut map = HashMap::new();
        let mut docs_and_gens = Vec::with_capacity(PARSE_WORKERS);
        for idx in 0..PARSE_WORKERS {
            let uri = format!("file:///panicking_settle_hook_{idx}.pl");
            let doc = DocumentState::new("my $a = 1;\n", 1);
            let gen_handle = doc.generation.clone();
            map.insert(uri.clone(), doc);
            docs_and_gens.push((uri, gen_handle));
        }
        let documents = Arc::new(Mutex::new(map));

        let (cb, _calls) = counting_callback();
        #[allow(clippy::panic)]
        let worker = ParseWorker::spawn_with_pending_count_hooks(
            Arc::clone(&documents),
            ast_cache(),
            cb,
            Arc::new(|_uri: &str| {}),
            Arc::new(|_uri: &str| panic!("injected on_settled panic for pool-exhaustion proof")),
        );

        for (uri, gen_handle) in &docs_and_gens {
            gen_handle.fetch_add(1, Ordering::SeqCst);
            worker.enqueue(
                uri.clone(),
                uri.clone(),
                1,
                Arc::clone(gen_handle),
                Arc::from("my $aa = 1;\n"),
            );
        }
        assert!(
            wait_for(|| worker.metrics().jobs_published >= PARSE_WORKERS as u64, TEST_TIMEOUT),
            "all {PARSE_WORKERS} jobs must publish -- `on_settled` panicking must not have \
             prevented `on_published` (which runs first) from completing; metrics={:?}",
            worker.metrics()
        );

        // Every worker thread has now had its `on_settled` panic at least
        // once. If the fix's `catch_unwind` were missing, this next job --
        // on a FRESH uri, so it cannot be satisfied by any already-active
        // worker reusing its claim -- would never be picked up.
        let final_uri = "file:///panicking_settle_hook_final.pl";
        let final_doc = DocumentState::new("my $z = 1;\n", 1);
        let final_gen = final_doc.generation.clone();
        documents.lock().insert(final_uri.to_string(), final_doc);
        final_gen.fetch_add(1, Ordering::SeqCst);
        worker.enqueue(
            final_uri.to_string(),
            final_uri.to_string(),
            1,
            Arc::clone(&final_gen),
            Arc::from("my $zz = 1;\n"),
        );
        assert!(
            wait_for(|| worker.metrics().jobs_published >= PARSE_WORKERS as u64 + 1, TEST_TIMEOUT),
            "a job on a brand-new URI must still be picked up and published after every worker \
             thread's `on_settled` panicked once -- the pool must not have been exhausted; \
             metrics={:?}",
            worker.metrics()
        );
        let docs = documents.lock();
        let doc = must_some(docs.get(final_uri));
        let current = must_some(doc.current_parsed());
        assert_eq!(current.generation(), 1, "the final URI's job must be the one that publishes");
    }

    // ---- Operability: zero live threads must be detectable -------------

    /// `ParseWorker::spawn` never fails outright -- a per-thread spawn error
    /// is logged and skipped (see the `match spawned` loop in `spawn`), so
    /// the pool degrades thread-by-thread rather than aborting construction.
    /// `is_operational()` is the guard `install_default_parse_worker` relies
    /// on to detect the all-threads-failed case (every `thread::Builder`
    /// spawn errored) and fall back to the synchronous parse path instead of
    /// installing a worker that accepts jobs no thread will ever process.
    /// A real OS-level thread-exhaustion failure isn't reproducible
    /// deterministically in a unit test, so this proves the method's
    /// contract directly: true immediately after a normal spawn (threads
    /// are live), false once every handle is gone (the all-spawns-failed
    /// state `spawn` would have produced).
    #[test]
    fn is_operational_reflects_whether_any_worker_thread_is_alive() {
        let uri = "file:///operability.pl";
        let (documents, _generation_handle) = one_doc(uri, "my $a = 1;\n");
        let (cb, _calls) = counting_callback();
        let worker = ParseWorker::spawn(documents, ast_cache(), cb);
        assert!(
            worker.is_operational(),
            "a freshly spawned pool must report at least one live worker thread"
        );

        // Simulate the all-spawns-failed case `spawn` would produce if every
        // `thread::Builder::spawn` call in its loop returned `Err`. Signal
        // shutdown first so the real live threads see it and exit on their
        // own (there is nothing enqueued, so this is immediate) before
        // dropping their `JoinHandle`s -- clearing `handles` without this
        // would leak live OS threads that nothing ever joins.
        worker.coordinator.request_shutdown();
        worker.handles.lock().clear();
        assert!(
            !worker.is_operational(),
            "zero live handles must report not-operational -- this is exactly the state \
             `install_default_parse_worker` must detect and refuse to install"
        );
    }

    // ---- Lifecycle: LspServer <-> ParseWorker must not form an Arc cycle -

    /// End-to-end proof for #3618: `LspServer::install_default_parse_worker`
    /// captures `Weak<LspServer>` (not `Arc<LspServer>`) in the worker's
    /// `on_published` closure. Before that fix, the reference chain
    /// `LspServer -> parse_worker_handle -> ParseWorker -> [4 worker
    /// threads] -> on_published closure -> Arc<LspServer>` was a genuine
    /// cycle: the server's strong count could never reach zero while the
    /// worker threads were alive, and the worker threads never exit (they
    /// only stop once `ParseWorker::drop` requests shutdown -- see
    /// `impl Drop for ParseWorker` above -- which never runs because it is
    /// only reachable through `LspServer`'s own drop). Both sides wait on
    /// each other forever: a leak, not a deadlock in the panicking sense,
    /// but nothing ever joins.
    ///
    /// This test drops the only strong `Arc<LspServer>` on a dedicated
    /// thread and waits on a channel with a bounded timeout rather than
    /// asserting synchronously in the test thread: if the cycle were still
    /// present, `drop(server)` would never return (the field drop of
    /// `parse_worker_handle` blocks inside `ParseWorker::drop`'s
    /// `handle.join()` forever), and a bare `drop(server)` on the test
    /// thread would hang the whole test binary instead of failing cleanly.
    #[test]
    fn dropping_the_server_joins_the_installed_parse_worker_threads()
    -> Result<(), Box<dyn std::error::Error>> {
        use std::sync::mpsc;

        let server = Arc::new(crate::runtime::LspServer::new());
        server.install_default_parse_worker();
        assert!(
            server.parse_worker().is_some(),
            "the real production worker must be installed before this test can \
             prove anything about its shutdown behaviour"
        );

        // `didOpen` is ALWAYS synchronous (`handle_did_open` never touches
        // `self.parse_worker()`) -- this alone does not enqueue a job or
        // invoke `on_published`. It's still meaningful setup: it proves the
        // worker threads are live (holding their captured `cb_server`) and
        // gives the document a real `DocumentState` to normalize/settle
        // against, but it is NOT the thing that exercises the real
        // enqueue -> parse -> publish -> `on_published` cycle -- see the
        // real `didChange` right below for that (#3618 review, cubic).
        let uri = "file:///lifecycle_drop.pl";
        server.test_apply_did_open(uri, "my $a = 1;\n", 1)?;
        let normalized_uri = server.normalize_uri_key(uri);
        let settled =
            must_some(server.parse_worker()).wait_until_settled(&normalized_uri, TEST_TIMEOUT);
        assert!(settled, "the initial open's parse must settle before this test drops the server");

        // Now exercise the worker for real through one full
        // enqueue -> parse -> publish -> `on_published` cycle over the
        // exact `Weak<LspServer>` wiring `install_default_parse_worker`
        // uses in production, via a genuine async `didChange` (not a
        // synthetic callback like the other tests in this module). This is
        // NOT required to prove the cycle-vs-no-cycle fix itself -- Rust
        // closures capture their environment at construction time, so
        // whether `cb_server` above is `Arc` (cycle) or `Weak` (no cycle) is
        // already decided the moment `install_default_parse_worker` builds
        // the `on_published` closure, before any job ever runs; the
        // drop-and-join proof below would catch a reverted fix even with
        // ONLY the didOpen above (confirmed independently by reverting
        // `cb_server` to `Arc::clone` during PR review: the test still
        // failed, on the deallocation assertion, with no real async publish
        // in between). Exercising the real callback path here is a
        // meaningful hardening on top of that, not a prerequisite: it also
        // proves a live `on_published` invocation's temporary
        // `cb_server.upgrade()` doesn't itself leave behind an extra strong
        // reference once the callback returns.
        server.test_apply_did_change(uri, "my $aa = 1;\n", 2)?;
        let settled_after_change =
            must_some(server.parse_worker()).wait_until_settled(&normalized_uri, TEST_TIMEOUT);
        assert!(
            settled_after_change,
            "the real async didChange's enqueue -> parse -> publish -> on_published cycle \
             must settle before this test drops the server"
        );

        // `Weak`, not a second `Arc` -- observing this must NOT keep the
        // server alive; it only lets the test check, after the drop, that
        // the server really was deallocated rather than merely having its
        // strong count decremented by one of several outstanding refs.
        let server_weak = Arc::downgrade(&server);

        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            drop(server);
            let _ = tx.send(());
        });

        assert!(
            rx.recv_timeout(TEST_TIMEOUT).is_ok(),
            "dropping the server's last Arc must complete within the timeout -- a hang \
             here means `on_published` is still holding a strong Arc<LspServer>, which \
             recreates the pre-#3618 LspServer<->ParseWorker reference cycle and leaks \
             every worker thread"
        );
        assert!(
            server_weak.upgrade().is_none(),
            "the server must actually deallocate once its last strong Arc drops -- a \
             lingering strong reference held by the parse worker's callback would keep \
             it alive forever even if `drop()` itself happened to return"
        );
        Ok(())
    }
}
