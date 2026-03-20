//! Request classification and scheduling for concurrent dispatch.
//!
//! Classifies incoming LSP methods into scheduling categories that determine
//! how they are executed:
//!
//! - **Control**: Processed inline immediately (cancellation, progress cancel)
//! - **Lifecycle**: Exclusive access (initialize, shutdown, exit)
//! - **Mutation**: Exclusive access (didOpen, didChange, didClose, etc.)
//! - **ReadOnly**: Concurrent access (hover, completion, definition, etc.)
//!
//! The [`Scheduler`] struct manages dedicated worker queues so the ingress loop
//! never performs heavy work — it only classifies and enqueues.

use crate::protocol::JsonRpcRequest;
use crate::transport::log_response;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use tokio::sync::{Notify, Semaphore};
use tokio::task::JoinSet;

use super::LspServer;

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

/// Read-only request tagged with the latest mutation sequence observed at ingress.
struct QueuedRead {
    request: JsonRpcRequest,
    wait_for_seq: u64,
}

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
        self.mutation_tx.send(QueuedMutation { request, seq }).await.map_err(|_| {
            self.mutation_seq_done.store(seq, Ordering::SeqCst);
            self.mutation_notify.notify_waiters();
        })
    }

    /// Send a read-only request to the read pool.
    ///
    /// Returns `Err(())` if all read workers have exited (channel closed).
    pub async fn send_read(&self, request: JsonRpcRequest) -> Result<(), ()> {
        let wait_for_seq = self.mutation_seq_next.load(Ordering::SeqCst);
        self.read_tx.send(QueuedRead { request, wait_for_seq }).await.map_err(|_| ())
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

    /// Read queue dispatcher.
    ///
    /// Drains the read-only queue, launching each request onto the blocking
    /// pool while a semaphore enforces the desired concurrency limit. This
    /// avoids serializing workers on a shared receiver mutex. Each spawned
    /// task waits for preceding mutations to complete before executing.
    async fn read_dispatcher(
        mut rx: tokio::sync::mpsc::Receiver<QueuedRead>,
        server: Arc<LspServer>,
        mutation_seq_done: Arc<AtomicU64>,
        mutation_notify: Arc<Notify>,
    ) {
        let permits = Arc::new(Semaphore::new(READ_WORKERS));
        let mut in_flight = JoinSet::new();

        while let Some(queued) = rx.recv().await {
            let permit = match Arc::clone(&permits).acquire_owned().await {
                Ok(permit) => permit,
                Err(_) => break,
            };

            let srv = Arc::clone(&server);
            let outbound = server.outbound.clone();
            let seq_done = Arc::clone(&mutation_seq_done);
            let notify = Arc::clone(&mutation_notify);
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

            while in_flight.len() >= READ_WORKERS {
                if in_flight.join_next().await.is_none() {
                    break;
                }
            }
        }

        while in_flight.join_next().await.is_some() {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_is_control() {
        assert_eq!(classify("$/cancelRequest"), RequestClass::Control);
        assert_eq!(classify("window/workDoneProgress/cancel"), RequestClass::Control);
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
}
