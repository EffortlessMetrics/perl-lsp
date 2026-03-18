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
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};

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
/// - **Read pool**: Fixed number of workers (`READ_WORKERS`) process read-only
///   requests concurrently via `spawn_blocking` (CPU-bound handlers).
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
    /// Shared server handle used by spawned read-only tasks.
    server: Arc<LspServer>,
    /// Concurrency limiter for read-only work.
    read_permits: Arc<Semaphore>,
    /// Join handles for long-lived/background workers.
    workers: Vec<tokio::task::JoinHandle<()>>,
    /// Join handles for spawned read-only request tasks.
    read_tasks: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
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

        // Single exclusive mutation worker — processes lifecycle and mutation
        // requests one at a time, preserving ordering guarantees.
        let workers = vec![tokio::spawn(Self::mutation_worker(mutation_rx, Arc::clone(&server)))];

        Self {
            mutation_tx,
            server,
            read_permits: Arc::new(Semaphore::new(READ_WORKERS)),
            workers,
            read_tasks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Send a mutation or lifecycle request to the exclusive worker.
    ///
    /// Returns `Err(())` if the mutation worker has exited (channel closed).
    pub async fn send_mutation(&self, request: JsonRpcRequest) -> Result<(), ()> {
        self.mutation_tx.send(request).await.map_err(|_| ())
    }

    /// Send a read-only request to the read pool.
    ///
    /// Returns `Err(())` if the scheduler is shutting down.
    pub async fn send_read(&self, request: JsonRpcRequest) -> Result<(), ()> {
        let permit = Arc::clone(&self.read_permits).acquire_owned().await.map_err(|_| ())?;

        Self::spawn_read_task(
            Arc::clone(&self.read_tasks),
            Arc::clone(&self.server),
            request,
            permit,
        )
        .await;
        Ok(())
    }

    /// Shut down all workers by dropping senders and awaiting completion.
    ///
    /// Dropping the sender halves closes the channels. Workers drain any
    /// remaining items and exit. `spawn_blocking` tasks run to completion
    /// and cannot be aborted — this is cooperative shutdown by design.
    pub async fn shutdown(self) {
        // Drop senders so worker recv loops see channel closed.
        drop(self.mutation_tx);
        self.read_permits.close();

        // Wait for all workers to finish draining.
        for handle in self.workers {
            let _ = handle.await;
        }

        let mut read_tasks = self.read_tasks.lock().await;
        let pending = std::mem::take(&mut *read_tasks);
        drop(read_tasks);

        for handle in pending {
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

    /// Spawn a single read-only request under semaphore control.
    ///
    /// This avoids a shared receiver mutex across workers and lets the ingress
    /// task dispatch directly into Tokio's scheduler while still bounding
    /// concurrent CPU-bound work.
    async fn spawn_read_task(
        read_tasks: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
        server: Arc<LspServer>,
        request: JsonRpcRequest,
        permit: OwnedSemaphorePermit,
    ) {
        let handle = tokio::spawn(async move {
            let _permit = permit;
            let srv = Arc::clone(&server);
            let outbound = server.outbound.clone();

            let result = tokio::task::spawn_blocking(move || srv.handle_request(request)).await;

            if let Ok(Some(response)) = result {
                log_response(&response);
                let _ = outbound.send_response(response);
            }
        });

        read_tasks.lock().await.push(handle);
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
