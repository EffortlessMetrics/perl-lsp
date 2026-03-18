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
use std::sync::Arc;
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
/// - **Read lane**: A lightweight dispatcher receives read-only requests and
///   fans them out into bounded concurrent `spawn_blocking` tasks. This avoids
///   a contended async mutex around the receiver while preserving the same
///   `READ_WORKERS` concurrency cap.
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
    mutation_tx: tokio::sync::mpsc::Sender<JsonRpcRequest>,
    /// Channel for read-only work (bounded concurrent dispatcher drains this).
    read_tx: tokio::sync::mpsc::Sender<JsonRpcRequest>,
    /// Join handles for background workers (used for shutdown drain).
    workers: Vec<tokio::task::JoinHandle<()>>,
}

/// Bounded channel capacity for both mutation and read queues.
const QUEUE_CAPACITY: usize = 64;

/// Number of concurrent read-pool workers.
const READ_WORKERS: usize = 4;

impl Scheduler {
    /// Create a new scheduler and spawn worker tasks.
    ///
    /// Spawns one exclusive mutation worker and `READ_WORKERS` read-pool workers.
    /// All workers use `spawn_blocking` for CPU-bound handler execution.
    pub fn new(server: Arc<LspServer>) -> Self {
        let (mutation_tx, mutation_rx) = tokio::sync::mpsc::channel(QUEUE_CAPACITY);
        let (read_tx, read_rx) = tokio::sync::mpsc::channel(QUEUE_CAPACITY);

        let mut workers = Vec::with_capacity(1 + READ_WORKERS);

        // Single exclusive mutation worker — processes lifecycle and mutation
        // requests one at a time, preserving ordering guarantees.
        workers.push(tokio::spawn(Self::mutation_worker(mutation_rx, Arc::clone(&server))));

        // Read lane: one async dispatcher keeps receiving work and fans it out
        // into bounded concurrent tasks. This removes the receiver mutex from
        // the hot path while still enforcing the READ_WORKERS cap.
        workers.push(tokio::spawn(Self::read_dispatcher(read_rx, Arc::clone(&server))));

        Self { mutation_tx, read_tx, workers }
    }

    /// Send a mutation or lifecycle request to the exclusive worker.
    ///
    /// Returns `Err(())` if the mutation worker has exited (channel closed).
    pub async fn send_mutation(&self, request: JsonRpcRequest) -> Result<(), ()> {
        self.mutation_tx.send(request).await.map_err(|_| ())
    }

    /// Send a read-only request to the read pool.
    ///
    /// Returns `Err(())` if all read workers have exited (channel closed).
    pub async fn send_read(&self, request: JsonRpcRequest) -> Result<(), ()> {
        self.read_tx.send(request).await.map_err(|_| ())
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
        mut rx: tokio::sync::mpsc::Receiver<JsonRpcRequest>,
        server: Arc<LspServer>,
    ) {
        while let Some(request) = rx.recv().await {
            // Run on blocking thread: handlers are CPU-bound and use
            // parking_lot locks which must not block the tokio runtime.
            let srv = Arc::clone(&server);
            let result = tokio::task::spawn_blocking(move || srv.handle_request(request)).await;

            if let Ok(Some(response)) = result {
                log_response(&response);
                if server.outbound.send_response(response).is_err() {
                    break;
                }
            }
        }
    }

    /// Read-only dispatcher with bounded concurrent task fan-out.
    ///
    /// A single async receiver avoids the previous receiver mutex hot spot. Each
    /// incoming request acquires a semaphore permit before being launched onto
    /// the blocking pool, preserving the same `READ_WORKERS` upper bound. When
    /// the channel closes, the dispatcher waits for all in-flight tasks to drain.
    async fn read_dispatcher(
        mut rx: tokio::sync::mpsc::Receiver<JsonRpcRequest>,
        server: Arc<LspServer>,
    ) {
        let semaphore = Arc::new(tokio::sync::Semaphore::new(READ_WORKERS));
        let mut in_flight = JoinSet::new();
        let mut channel_open = true;

        loop {
            tokio::select! {
                biased;

                Some(joined) = in_flight.join_next(), if !in_flight.is_empty() => {
                    if let Err(error) = joined {
                        eprintln!("Read dispatcher task failed: {error}");
                    }
                }

                maybe_request = rx.recv(), if channel_open => {
                    match maybe_request {
                        Some(request) => {
                            let permit = match Arc::clone(&semaphore).acquire_owned().await {
                                Ok(permit) => permit,
                                Err(_) => break,
                            };
                            let srv = Arc::clone(&server);
                            let outbound = server.outbound.clone();

                            in_flight.spawn(async move {
                                let _permit = permit;
                                let result = tokio::task::spawn_blocking(move || srv.handle_request(request)).await;

                                if let Ok(Some(response)) = result {
                                    log_response(&response);
                                    let _ = outbound.send_response(response);
                                }
                            });
                        }
                        None => channel_open = false,
                    }
                }

                else => break,
            }
        }
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
