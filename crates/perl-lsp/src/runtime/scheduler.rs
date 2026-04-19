//! Request classification, prioritization, and scheduling for concurrent dispatch.
//!
//! Classifies incoming LSP methods into scheduling categories that determine
//! how they are executed:
//!
//! - **Control**: Processed inline immediately (cancellation, progress cancel)
//! - **Lifecycle**: Exclusive access (initialize, shutdown, exit)
//! - **Mutation**: Exclusive access (didOpen, didChange, didClose, etc.)
//! - **ReadOnly**: Concurrent access (hover, completion, definition, etc.)
//!
//! # Request Prioritization
//!
//! Read-only requests are processed by priority rather than strict FIFO.
//! The priority ordering (highest to lowest) is:
//!
//! 1. `Hover` — cursor queries are latency-sensitive
//! 2. `Completion` — inline suggestions block the user
//! 3. `References` — navigation is slightly less urgent
//! 4. `Other` — background/bulk operations (workspace symbols, diagnostics, etc.)
//!
//! # Stale Request Cancellation
//!
//! For position-sensitive requests (hover, completion, definition), if a newer
//! request arrives for the same `(method, uri, line, character)` position before
//! the older one starts executing, the older request is cancelled with a
//! `RequestCancelled` error response. This prevents thrashing on rapid edits.
//!
//! The [`Scheduler`] struct manages dedicated worker queues so the ingress loop
//! never performs heavy work — it only classifies and enqueues.

use crate::protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, REQUEST_CANCELLED};
use crate::transport::log_response;
use std::cmp::Ordering as CmpOrdering;
use std::collections::{BinaryHeap, HashMap};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use tokio::sync::{Notify, Semaphore};
use tokio::task::JoinSet;

use super::LspServer;

// =========================================================================
// Request priority
// =========================================================================

/// Priority tier for read-only LSP requests.
///
/// Requests are dispatched highest-priority-first within the read queue.
/// Numeric value: lower = higher priority (so `BinaryHeap` max-heap works
/// correctly after inversion in `Ord`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RequestPriority {
    /// textDocument/hover — cursor queries, most latency-sensitive.
    Hover = 0,
    /// textDocument/completion — inline suggestions.
    Completion = 1,
    /// textDocument/references, definition, declaration.
    References = 2,
    /// Everything else (workspace/symbol, diagnostics, bulk operations).
    Other = 3,
}

impl RequestPriority {
    /// Numeric priority value; lower value = higher priority.
    fn value(self) -> u8 {
        self as u8
    }
}

/// Map an LSP method string to its dispatch priority.
pub(crate) fn request_priority(method: &str) -> RequestPriority {
    match method {
        "textDocument/hover" => RequestPriority::Hover,
        "textDocument/completion" | "completionItem/resolve" | "textDocument/inlineCompletion" => {
            RequestPriority::Completion
        }
        "textDocument/references"
        | "textDocument/definition"
        | "textDocument/declaration"
        | "textDocument/typeDefinition"
        | "textDocument/implementation" => RequestPriority::References,
        _ => RequestPriority::Other,
    }
}

// =========================================================================
// Dedup key for stale-request cancellation
// =========================================================================

/// A dedup key identifies a position-sensitive request by `(method, uri, line, character)`.
///
/// When a newer request arrives with the same key, the earlier pending request
/// is superseded and its slot is cancelled before execution begins.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct RequestDedupKey {
    /// Normalised LSP method (e.g. `"textDocument/hover"`).
    pub method: String,
    /// Document URI (e.g. `"file:///path/to/file.pl"`).
    pub uri: String,
    /// Zero-based line number.
    pub line: u64,
    /// Zero-based character offset.
    pub character: u64,
}

/// Extract a dedup key from a position-sensitive request, if possible.
///
/// Returns `None` for requests without a `textDocument` + `position` payload,
/// or when priority is `Other` (those are not deduplicated).
pub(crate) fn extract_dedup_key(
    method: &str,
    params: Option<&serde_json::Value>,
    priority: RequestPriority,
) -> Option<RequestDedupKey> {
    // Only deduplicate position-sensitive requests.
    if priority == RequestPriority::Other {
        return None;
    }

    let params = params?;
    let uri = params
        .get("textDocument")?
        .get("uri")?
        .as_str()?
        .to_string();
    let position = params.get("position")?;
    let line = position.get("line")?.as_u64()?;
    let character = position.get("character")?.as_u64()?;

    Some(RequestDedupKey {
        method: method.to_string(),
        uri,
        line,
        character,
    })
}

// =========================================================================
// Scheduling class
// =========================================================================

/// Scheduling class for an incoming LSP method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RequestClass {
    /// `$/cancelRequest`, `window/workDoneProgress/cancel`
    /// — processed inline, immediately, no lock.
    Control,
    /// `initialize`, `initialized`, `shutdown`, `exit`
    /// — exclusive, ordered.
    Lifecycle,
    /// `didOpen`, `didChange`, `didClose`, `didSave`, etc.
    /// — exclusive (document mutations).
    Mutation,
    /// `completion`, `hover`, `definition`, `references`, etc.
    /// — concurrent (read-only queries).
    ReadOnly,
}

/// Classify an LSP method string into its scheduling category.
pub(crate) fn classify(method: &str) -> RequestClass {
    match method {
        // Control: processed inline
        "$/cancelRequest" | "window/workDoneProgress/cancel" => RequestClass::Control,

        // Lifecycle: exclusive, ordered
        "initialize" | "initialized" | "shutdown" | "exit" | "$/setTrace" => {
            RequestClass::Lifecycle
        }

        // Mutation: exclusive (modifies document state)
        "textDocument/didOpen"
        | "textDocument/didChange"
        | "textDocument/didClose"
        | "textDocument/didSave"
        | "textDocument/willSave"
        | "textDocument/willSaveWaitUntil"
        | "notebookDocument/didOpen"
        | "notebookDocument/didChange"
        | "notebookDocument/didSave"
        | "notebookDocument/didClose"
        | "workspace/didChangeWatchedFiles"
        | "workspace/didChangeWorkspaceFolders"
        | "workspace/didChangeConfiguration"
        | "workspace/didRenameFiles"
        | "workspace/didDeleteFiles"
        | "workspace/didCreateFiles" => RequestClass::Mutation,

        // Everything else is read-only
        _ => RequestClass::ReadOnly,
    }
}

// =========================================================================
// Scheduler
// =========================================================================

/// Worker-queue scheduler for concurrent LSP dispatch.
///
/// Routes classified requests to dedicated worker queues:
///
/// - **Mutation worker**: Single exclusive worker processes lifecycle and mutation
///   requests one at a time (sequential drain from a bounded `mpsc` channel).
/// - **Read dispatcher**: A single dispatcher drains the read queue and launches
///   read-only work onto the blocking pool, capped by a semaphore. This avoids
///   receiver-lock contention while still bounding concurrency.
///
/// Read-only requests are ordered by priority (hover > completion > references > other)
/// and stale position-sensitive requests are cancelled before execution.
///
/// The ingress loop (`serve_async`) only reads, classifies, and enqueues.
/// Heavy work never blocks the message reader.
///
/// ## Shutdown policy
///
/// When the ingress channel closes (EOF / drop), `shutdown()` drops the sender
/// halves. Workers drain remaining items and exit. `spawn_blocking` tasks cannot
/// be aborted — they run to completion. This is cooperative shutdown.
pub(crate) struct Scheduler {
    /// Channel for mutation/lifecycle work (single exclusive worker drains this).
    mutation_tx: tokio::sync::mpsc::Sender<QueuedMutation>,
    /// Channel for read-only work (dispatcher drains this).
    read_tx: tokio::sync::mpsc::Sender<QueuedRead>,
    /// Join handles for background workers (used for shutdown drain).
    workers: Vec<tokio::task::JoinHandle<()>>,
    /// Monotonic sequence assigned to mutations/lifecycle requests at ingress.
    mutation_seq_next: Arc<AtomicU64>,
    /// Highest mutation sequence that has completed processing.
    mutation_seq_done: Arc<AtomicU64>,
    /// Wakes read workers waiting for earlier mutations to finish.
    mutation_notify: Arc<Notify>,
}

/// Bounded channel capacity for both mutation and read queues.
const QUEUE_CAPACITY: usize = 64;

/// Number of concurrent read-pool workers.
const READ_WORKERS: usize = 4;

/// Mutation/lifecycle request tagged with its ingress-order sequence number.
struct QueuedMutation {
    request: JsonRpcRequest,
    seq: u64,
}

/// Read-only request with priority metadata for ordered dispatch.
///
/// Implements `Ord` so that a `BinaryHeap<QueuedRead>` is a max-heap ordered
/// first by **highest priority** (lowest `priority.value()`), then by **latest
/// arrival** (highest `arrival_seq`) to break ties in favour of newer requests.
struct QueuedRead {
    /// The original JSON-RPC request.
    request: JsonRpcRequest,
    /// Latest mutation sequence observed at ingress; used to gate execution.
    wait_for_seq: u64,
    /// Dispatch priority computed from the request method.
    priority: RequestPriority,
    /// Monotonic ingress counter (used for tie-breaking and stale detection).
    arrival_seq: u64,
    /// Dedup key for stale-request cancellation (None for non-position requests).
    dedup_key: Option<RequestDedupKey>,
}

impl PartialEq for QueuedRead {
    fn eq(&self, other: &Self) -> bool {
        self.priority.value() == other.priority.value() && self.arrival_seq == other.arrival_seq
    }
}

impl Eq for QueuedRead {}

impl PartialOrd for QueuedRead {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueuedRead {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher priority (lower value) first.  On equal priority, prefer
        // the *newer* request (higher arrival_seq).
        match other.priority.value().cmp(&self.priority.value()) {
            CmpOrdering::Equal => self.arrival_seq.cmp(&other.arrival_seq),
            ord => ord,
        }
    }
}

/// Global read arrival counter; incremented at ingress for each read request.
static READ_ARRIVAL_SEQ: AtomicU64 = AtomicU64::new(0);

impl Scheduler {
    /// Create a new scheduler and spawn worker tasks.
    ///
    /// Spawns one exclusive mutation worker and one read dispatcher.
    /// All workers use `spawn_blocking` for CPU-bound handler execution.
    pub fn new(server: Arc<LspServer>) -> Self {
        let (mutation_tx, mutation_rx) = tokio::sync::mpsc::channel(QUEUE_CAPACITY);
        let (read_tx, read_rx) = tokio::sync::mpsc::channel(QUEUE_CAPACITY);
        let mutation_seq_next = Arc::new(AtomicU64::new(0));
        let mutation_seq_done = Arc::new(AtomicU64::new(0));
        let mutation_notify = Arc::new(Notify::new());

        let workers = vec![
            // Single exclusive mutation worker — processes lifecycle and mutation
            // requests one at a time, preserving ordering guarantees.
            tokio::spawn(Self::mutation_worker(
                mutation_rx,
                Arc::clone(&server),
                Arc::clone(&mutation_seq_done),
                Arc::clone(&mutation_notify),
            )),
            // Single dispatcher drains the read queue and fans work out to the
            // blocking pool, capped by a semaphore instead of a receiver mutex.
            tokio::spawn(Self::read_dispatcher(
                read_rx,
                Arc::clone(&server),
                Arc::clone(&mutation_seq_done),
                Arc::clone(&mutation_notify),
            )),
        ];

        // Install diagnostic debouncer now that server is wrapped in Arc.
        let debounce_server = Arc::clone(&server);
        let debouncer = super::diagnostic_debounce::DiagnosticDebouncer::new(move |uri| {
            debounce_server.publish_diagnostics(uri);
        });
        server.install_diagnostic_debouncer(debouncer);

        // Install file watcher debouncer now that server is wrapped in Arc.
        let fw_server = Arc::clone(&server);
        let fw_debouncer = super::file_watcher_debounce::FileWatcherDebouncer::new(move |uris| {
            fw_server.handle_watched_file_batch(uris);
        });
        server.install_file_watcher_debouncer(fw_debouncer);

        Self {
            mutation_tx,
            read_tx,
            workers,
            mutation_seq_next,
            mutation_seq_done,
            mutation_notify,
        }
    }

    /// Send a mutation or lifecycle request to the exclusive worker.
    ///
    /// Returns `Err(())` if the mutation worker has exited (channel closed).
    pub async fn send_mutation(&self, request: JsonRpcRequest) -> Result<(), ()> {
        let seq = self.mutation_seq_next.fetch_add(1, Ordering::SeqCst) + 1;
        self.mutation_tx
            .send(QueuedMutation { request, seq })
            .await
            .map_err(|_| {
                self.mutation_seq_done.store(seq, Ordering::SeqCst);
                self.mutation_notify.notify_waiters();
            })
    }

    /// Send a read-only request to the priority read pool.
    ///
    /// Computes the request priority and dedup key at ingress so the dispatcher
    /// can reorder and deduplicate without parsing params again.
    ///
    /// Returns `Err(())` if all read workers have exited (channel closed).
    pub async fn send_read(&self, request: JsonRpcRequest) -> Result<(), ()> {
        let wait_for_seq = self.mutation_seq_next.load(Ordering::SeqCst);
        let priority = request_priority(&request.method);
        let dedup_key = extract_dedup_key(&request.method, request.params.as_ref(), priority);
        let arrival_seq = READ_ARRIVAL_SEQ.fetch_add(1, Ordering::Relaxed);
        self.read_tx
            .send(QueuedRead {
                request,
                wait_for_seq,
                priority,
                arrival_seq,
                dedup_key,
            })
            .await
            .map_err(|_| ())
    }

    /// Shut down all workers by dropping senders and awaiting completion.
    ///
    /// Dropping the sender halves closes the channels. Workers drain any
    /// remaining items and exit. `spawn_blocking` tasks run to completion
    /// and cannot be aborted — this is cooperative shutdown by design.
    pub async fn shutdown(self) {
        // Drop senders so worker recv loops see channel closed.
        drop(self.mutation_tx);
        drop(self.read_tx);

        // Wait for all workers to finish draining.
        for handle in self.workers {
            let _ = handle.await;
        }
    }

    /// Single exclusive mutation worker.
    ///
    /// Drains the mutation channel sequentially, running each handler on the
    /// blocking thread pool via `spawn_blocking`. This ensures lifecycle and
    /// document-mutation requests never overlap.
    async fn mutation_worker(
        mut rx: tokio::sync::mpsc::Receiver<QueuedMutation>,
        server: Arc<LspServer>,
        mutation_seq_done: Arc<AtomicU64>,
        mutation_notify: Arc<Notify>,
    ) {
        while let Some(queued) = rx.recv().await {
            // Run on blocking thread: handlers are CPU-bound and use
            // parking_lot locks which must not block the tokio runtime.
            let srv = Arc::clone(&server);
            let result =
                tokio::task::spawn_blocking(move || srv.handle_request(queued.request)).await;

            // Reads that were enqueued after this mutation can proceed once state is updated.
            mutation_seq_done.store(queued.seq, Ordering::SeqCst);
            mutation_notify.notify_waiters();

            if let Ok(Some(response)) = result {
                log_response(&response);
                if server.outbound.send_response(response).is_err() {
                    break;
                }
            }
        }
    }

    /// Priority read queue dispatcher.
    ///
    /// Collects incoming read requests into a `BinaryHeap` and dispatches them
    /// highest-priority-first. Before dispatching each request, checks for stale
    /// deduplication: if a newer request with the same `(method, uri, position)`
    /// key was enqueued, the older request is cancelled immediately without
    /// running on the blocking pool.
    ///
    /// The semaphore enforces the desired concurrency limit. Each spawned task
    /// waits for preceding mutations to complete before executing.
    async fn read_dispatcher(
        mut rx: tokio::sync::mpsc::Receiver<QueuedRead>,
        server: Arc<LspServer>,
        mutation_seq_done: Arc<AtomicU64>,
        mutation_notify: Arc<Notify>,
    ) {
        let permits = Arc::new(Semaphore::new(READ_WORKERS));
        let mut in_flight = JoinSet::new();

        // Priority queue: highest-priority (lowest value) requests pop first.
        let mut pending: BinaryHeap<QueuedRead> = BinaryHeap::new();
        // Maps dedup key -> latest arrival_seq seen for that key.
        let mut latest_seq: HashMap<RequestDedupKey, u64> = HashMap::new();

        loop {
            // Drain all currently available messages into the priority heap
            // before committing to one, giving priority re-ordering a chance
            // to work even under burst traffic.
            loop {
                match rx.try_recv() {
                    Ok(queued) => {
                        // Track latest arrival_seq for dedup keys.
                        if let Some(ref key) = queued.dedup_key {
                            latest_seq
                                .entry(key.clone())
                                .and_modify(|seq| {
                                    if queued.arrival_seq > *seq {
                                        *seq = queued.arrival_seq;
                                    }
                                })
                                .or_insert(queued.arrival_seq);
                        }
                        pending.push(queued);
                    }
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                        // Channel closed — drain whatever is left in pending.
                        while let Some(queued) = pending.pop() {
                            Self::dispatch_one(
                                queued,
                                &latest_seq,
                                &permits,
                                &mut in_flight,
                                &server,
                                &mutation_seq_done,
                                &mutation_notify,
                            )
                            .await;
                        }
                        while in_flight.join_next().await.is_some() {}
                        return;
                    }
                }
            }

            if let Some(queued) = pending.pop() {
                Self::dispatch_one(
                    queued,
                    &latest_seq,
                    &permits,
                    &mut in_flight,
                    &server,
                    &mutation_seq_done,
                    &mutation_notify,
                )
                .await;

                while in_flight.len() >= READ_WORKERS {
                    if in_flight.join_next().await.is_none() {
                        break;
                    }
                }
            } else {
                // Heap is empty — block until a new message arrives.
                match rx.recv().await {
                    Some(queued) => {
                        if let Some(ref key) = queued.dedup_key {
                            latest_seq
                                .entry(key.clone())
                                .and_modify(|seq| {
                                    if queued.arrival_seq > *seq {
                                        *seq = queued.arrival_seq;
                                    }
                                })
                                .or_insert(queued.arrival_seq);
                        }
                        pending.push(queued);
                    }
                    None => {
                        // Channel closed.
                        while in_flight.join_next().await.is_some() {}
                        return;
                    }
                }
            }
        }
    }

    /// Dispatch a single read request — either cancel it (stale) or execute it.
    async fn dispatch_one(
        queued: QueuedRead,
        latest_seq: &HashMap<RequestDedupKey, u64>,
        permits: &Arc<Semaphore>,
        in_flight: &mut JoinSet<()>,
        server: &Arc<LspServer>,
        mutation_seq_done: &Arc<AtomicU64>,
        mutation_notify: &Arc<Notify>,
    ) {
        // Stale check: if a newer request with the same dedup key exists,
        // cancel this one immediately without consuming a worker slot.
        if let Some(ref key) = queued.dedup_key {
            if let Some(&latest) = latest_seq.get(key) {
                if queued.arrival_seq < latest {
                    // This request is superseded; send a cancellation response.
                    let cancelled_response = JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: queued.request.id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: REQUEST_CANCELLED,
                            message: format!(
                                "Request superseded by newer {} request",
                                queued.request.method
                            ),
                            data: None,
                        }),
                    };
                    log_response(&cancelled_response);
                    let _ = server.outbound.send_response(cancelled_response);
                    return;
                }
            }
        }

        let permit = match Arc::clone(permits).acquire_owned().await {
            Ok(permit) => permit,
            Err(_) => return,
        };

        let srv = Arc::clone(server);
        let outbound = server.outbound.clone();
        let seq_done = Arc::clone(mutation_seq_done);
        let notify = Arc::clone(mutation_notify);
        let wait_for = queued.wait_for_seq;

        in_flight.spawn(async move {
            let _permit = permit;

            // Wait for all mutations that were enqueued before this read.
            while seq_done.load(Ordering::SeqCst) < wait_for {
                notify.notified().await;
            }

            let result =
                tokio::task::spawn_blocking(move || srv.handle_request(queued.request)).await;

            if let Ok(Some(response)) = result {
                log_response(&response);
                let _ = outbound.send_response(response);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =====================================================================
    // Existing classification tests
    // =====================================================================

    #[test]
    fn cancel_is_control() {
        assert_eq!(classify("$/cancelRequest"), RequestClass::Control);
        assert_eq!(
            classify("window/workDoneProgress/cancel"),
            RequestClass::Control
        );
    }

    #[test]
    fn lifecycle_methods() {
        assert_eq!(classify("initialize"), RequestClass::Lifecycle);
        assert_eq!(classify("initialized"), RequestClass::Lifecycle);
        assert_eq!(classify("shutdown"), RequestClass::Lifecycle);
        assert_eq!(classify("exit"), RequestClass::Lifecycle);
    }

    #[test]
    fn mutation_methods() {
        assert_eq!(classify("textDocument/didOpen"), RequestClass::Mutation);
        assert_eq!(classify("textDocument/didChange"), RequestClass::Mutation);
        assert_eq!(classify("textDocument/didClose"), RequestClass::Mutation);
        assert_eq!(classify("textDocument/didSave"), RequestClass::Mutation);
        assert_eq!(classify("textDocument/willSave"), RequestClass::Mutation);
        assert_eq!(
            classify("textDocument/willSaveWaitUntil"),
            RequestClass::Mutation
        );
        assert_eq!(
            classify("workspace/didChangeConfiguration"),
            RequestClass::Mutation
        );
        assert_eq!(classify("workspace/didCreateFiles"), RequestClass::Mutation);
        assert_eq!(classify("workspace/didDeleteFiles"), RequestClass::Mutation);
        assert_eq!(classify("workspace/didRenameFiles"), RequestClass::Mutation);
        assert_eq!(
            classify("workspace/didChangeWorkspaceFolders"),
            RequestClass::Mutation
        );
    }

    #[test]
    fn notebook_mutation_methods() {
        assert_eq!(classify("notebookDocument/didOpen"), RequestClass::Mutation);
        assert_eq!(
            classify("notebookDocument/didChange"),
            RequestClass::Mutation
        );
        assert_eq!(classify("notebookDocument/didSave"), RequestClass::Mutation);
        assert_eq!(
            classify("notebookDocument/didClose"),
            RequestClass::Mutation
        );
    }

    #[test]
    fn set_trace_is_lifecycle() {
        assert_eq!(classify("$/setTrace"), RequestClass::Lifecycle);
    }

    #[test]
    fn read_only_methods() {
        assert_eq!(classify("textDocument/hover"), RequestClass::ReadOnly);
        assert_eq!(classify("textDocument/completion"), RequestClass::ReadOnly);
        assert_eq!(classify("textDocument/definition"), RequestClass::ReadOnly);
        assert_eq!(classify("textDocument/references"), RequestClass::ReadOnly);
        assert_eq!(classify("workspace/symbol"), RequestClass::ReadOnly);
    }

    #[test]
    fn unknown_methods_are_read_only() {
        assert_eq!(classify("custom/unknown"), RequestClass::ReadOnly);
    }

    // =====================================================================
    // Priority tests (issue #2354)
    // =====================================================================

    #[test]
    fn hover_has_highest_priority() {
        assert_eq!(
            request_priority("textDocument/hover"),
            RequestPriority::Hover
        );
        assert!(
            RequestPriority::Hover.value() < RequestPriority::Completion.value(),
            "hover must outrank completion"
        );
    }

    #[test]
    fn completion_priority() {
        assert_eq!(
            request_priority("textDocument/completion"),
            RequestPriority::Completion
        );
        assert_eq!(
            request_priority("completionItem/resolve"),
            RequestPriority::Completion
        );
    }

    #[test]
    fn references_priority() {
        assert_eq!(
            request_priority("textDocument/references"),
            RequestPriority::References
        );
        assert_eq!(
            request_priority("textDocument/definition"),
            RequestPriority::References
        );
        assert_eq!(
            request_priority("textDocument/declaration"),
            RequestPriority::References
        );
        assert_eq!(
            request_priority("textDocument/typeDefinition"),
            RequestPriority::References
        );
        assert_eq!(
            request_priority("textDocument/implementation"),
            RequestPriority::References
        );
    }

    #[test]
    fn workspace_symbol_is_other() {
        assert_eq!(request_priority("workspace/symbol"), RequestPriority::Other);
        assert_eq!(
            request_priority("workspace/diagnostic"),
            RequestPriority::Other
        );
        assert_eq!(request_priority("custom/unknown"), RequestPriority::Other);
    }

    #[test]
    fn priority_ordering_hover_beats_completion() {
        // In a BinaryHeap (max-heap), the item that compares "greater" pops first.
        // We want hover > completion, so hover must be Ord-greater than completion.
        let hover = make_queued_read("textDocument/hover", 0);
        let completion = make_queued_read("textDocument/completion", 1);
        assert!(
            hover > completion,
            "hover must be ordered before completion"
        );
    }

    #[test]
    fn priority_ordering_completion_beats_references() {
        let completion = make_queued_read("textDocument/completion", 0);
        let references = make_queued_read("textDocument/references", 1);
        assert!(
            completion > references,
            "completion must be ordered before references"
        );
    }

    #[test]
    fn priority_ordering_references_beats_other() {
        let references = make_queued_read("textDocument/references", 0);
        let other = make_queued_read("workspace/symbol", 1);
        assert!(
            references > other,
            "references must be ordered before workspace/symbol"
        );
    }

    #[test]
    fn same_priority_newer_arrival_wins() {
        // Two hover requests: the one with higher arrival_seq should pop first.
        let older = make_queued_read_with_seq("textDocument/hover", 0, 1);
        let newer = make_queued_read_with_seq("textDocument/hover", 0, 5);
        assert!(
            newer > older,
            "newer arrival should pop before older within the same priority"
        );
    }

    #[test]
    fn priority_heap_drains_hover_first() {
        let mut heap: BinaryHeap<QueuedRead> = BinaryHeap::new();
        heap.push(make_queued_read("workspace/symbol", 0));
        heap.push(make_queued_read("textDocument/references", 1));
        heap.push(make_queued_read("textDocument/completion", 2));
        heap.push(make_queued_read("textDocument/hover", 3));

        // Pop order must be: hover, completion, references, workspace/symbol
        assert_eq!(heap.pop().unwrap().request.method, "textDocument/hover");
        assert_eq!(
            heap.pop().unwrap().request.method,
            "textDocument/completion"
        );
        assert_eq!(
            heap.pop().unwrap().request.method,
            "textDocument/references"
        );
        assert_eq!(heap.pop().unwrap().request.method, "workspace/symbol");
    }

    // =====================================================================
    // Dedup key tests (issue #2354)
    // =====================================================================

    #[test]
    fn dedup_key_extracted_for_hover() {
        let params = serde_json::json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 5, "character": 10 }
        });
        let key = extract_dedup_key("textDocument/hover", Some(&params), RequestPriority::Hover);
        assert!(key.is_some());
        let key = key.unwrap();
        assert_eq!(key.method, "textDocument/hover");
        assert_eq!(key.uri, "file:///test.pl");
        assert_eq!(key.line, 5);
        assert_eq!(key.character, 10);
    }

    #[test]
    fn dedup_key_extracted_for_completion() {
        let params = serde_json::json!({
            "textDocument": { "uri": "file:///foo.pl" },
            "position": { "line": 0, "character": 3 }
        });
        let key = extract_dedup_key(
            "textDocument/completion",
            Some(&params),
            RequestPriority::Completion,
        );
        assert!(key.is_some());
    }

    #[test]
    fn dedup_key_none_for_other_priority() {
        let params = serde_json::json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 0, "character": 0 }
        });
        // workspace/symbol is Other priority — no dedup key.
        let key = extract_dedup_key("workspace/symbol", Some(&params), RequestPriority::Other);
        assert!(key.is_none());
    }

    #[test]
    fn dedup_key_none_when_no_params() {
        let key = extract_dedup_key("textDocument/hover", None, RequestPriority::Hover);
        assert!(key.is_none());
    }

    #[test]
    fn dedup_key_none_when_no_position() {
        let params = serde_json::json!({
            "textDocument": { "uri": "file:///test.pl" }
            // no "position"
        });
        let key = extract_dedup_key("textDocument/hover", Some(&params), RequestPriority::Hover);
        assert!(key.is_none());
    }

    #[test]
    fn dedup_key_none_when_uri_missing() {
        let params = serde_json::json!({
            "textDocument": {},
            "position": { "line": 1, "character": 2 }
        });
        let key = extract_dedup_key("textDocument/hover", Some(&params), RequestPriority::Hover);
        assert!(key.is_none());
    }

    #[test]
    fn dedup_key_is_supported_for_references_priority() {
        let params = serde_json::json!({
            "textDocument": { "uri": "file:///refs.pl" },
            "position": { "line": 12, "character": 4 }
        });
        let key = extract_dedup_key(
            "textDocument/references",
            Some(&params),
            RequestPriority::References,
        );

        assert_eq!(
            key,
            Some(RequestDedupKey {
                method: "textDocument/references".to_string(),
                uri: "file:///refs.pl".to_string(),
                line: 12,
                character: 4,
            })
        );
    }

    #[test]
    fn dedup_key_none_for_non_numeric_position_fields() {
        let params = serde_json::json!({
            "textDocument": { "uri": "file:///bad-pos.pl" },
            "position": { "line": "12", "character": "4" }
        });
        let key = extract_dedup_key("textDocument/hover", Some(&params), RequestPriority::Hover);
        assert!(key.is_none());
    }

    #[test]
    fn dedup_key_none_when_position_is_signed() {
        let params = serde_json::json!({
            "textDocument": { "uri": "file:///bad-pos.pl" },
            "position": { "line": 1, "character": -1 }
        });
        let key = extract_dedup_key("textDocument/hover", Some(&params), RequestPriority::Hover);
        assert!(key.is_none());
    }

    #[test]
    fn different_positions_produce_different_keys() {
        let params1 = serde_json::json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 1, "character": 0 }
        });
        let params2 = serde_json::json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 2, "character": 0 }
        });
        let key1 = extract_dedup_key("textDocument/hover", Some(&params1), RequestPriority::Hover);
        let key2 = extract_dedup_key("textDocument/hover", Some(&params2), RequestPriority::Hover);
        assert_ne!(key1, key2);
    }

    #[test]
    fn different_uris_produce_different_keys() {
        let params1 = serde_json::json!({
            "textDocument": { "uri": "file:///a.pl" },
            "position": { "line": 0, "character": 0 }
        });
        let params2 = serde_json::json!({
            "textDocument": { "uri": "file:///b.pl" },
            "position": { "line": 0, "character": 0 }
        });
        let key1 = extract_dedup_key("textDocument/hover", Some(&params1), RequestPriority::Hover);
        let key2 = extract_dedup_key("textDocument/hover", Some(&params2), RequestPriority::Hover);
        assert_ne!(key1, key2);
    }

    // =====================================================================
    // Helper constructors for tests
    // =====================================================================

    #[test]
    fn inline_completion_gets_completion_priority() {
        assert_eq!(
            request_priority("textDocument/inlineCompletion"),
            RequestPriority::Completion
        );
    }

    #[test]
    fn inline_completion_gets_dedup_key() {
        let params = serde_json::json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 5, "character": 10 }
        });
        let key = extract_dedup_key(
            "textDocument/inlineCompletion",
            Some(&params),
            RequestPriority::Completion,
        );
        assert!(key.is_some());
        let key = key.unwrap();
        assert_eq!(key.method, "textDocument/inlineCompletion");
        assert_eq!(key.uri, "file:///test.pl");
        assert_eq!(key.line, 5);
        assert_eq!(key.character, 10);
    }

    fn make_queued_read(method: &str, arrival_seq: u64) -> QueuedRead {
        make_queued_read_with_seq(method, 0, arrival_seq)
    }

    fn make_queued_read_with_seq(method: &str, wait_for_seq: u64, arrival_seq: u64) -> QueuedRead {
        let priority = request_priority(method);
        QueuedRead {
            request: JsonRpcRequest {
                _jsonrpc: "2.0".to_string(),
                id: None,
                method: method.to_string(),
                params: None,
            },
            wait_for_seq,
            priority,
            arrival_seq,
            dedup_key: None,
        }
    }
}
