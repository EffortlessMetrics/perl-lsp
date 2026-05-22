//! File watcher debouncer for batch processing bulk file changes
//!
//! Coalesces rapid `workspace/didChangeWatchedFiles` notifications into batched
//! callbacks after a configurable quiet period (default 500ms).
//!
//! When bulk file operations occur (e.g., `git checkout`, external tool rewrites)
//! the LSP server receives many notifications in rapid succession. Without
//! debouncing each notification triggers blocking I/O and workspace re-indexing,
//! causing the server to appear frozen to the user.
//!
//! The debouncer accumulates URIs over the window and delivers them as a single
//! `Vec<String>` batch to the caller-supplied callback. Duplicate URIs within
//! a window are deduplicated (last-write wins on the deadline).

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

const DEFAULT_DEBOUNCE_MS: u64 = 500;

enum WatcherMsg {
    Schedule(String),
    Shutdown,
}

/// Debouncer for file watcher change notifications.
///
/// Accumulates URIs from rapid `workspace/didChangeWatchedFiles` notifications
/// and delivers them as a single batch to the callback after a quiet period.
pub struct FileWatcherDebouncer {
    tx: std::sync::mpsc::Sender<WatcherMsg>,
    pending_count: Arc<AtomicUsize>,
}

impl FileWatcherDebouncer {
    /// Create a new debouncer with the default window (500ms).
    pub fn new<F>(publish_fn: F) -> Self
    where
        F: Fn(Vec<String>) + Send + 'static,
    {
        Self::with_interval(Duration::from_millis(DEFAULT_DEBOUNCE_MS), publish_fn)
    }

    /// Create a new debouncer with a custom debounce window.
    pub fn with_interval<F>(interval: Duration, publish_fn: F) -> Self
    where
        F: Fn(Vec<String>) + Send + 'static,
    {
        let (tx, rx) = std::sync::mpsc::channel();
        let pending_count = Arc::new(AtomicUsize::new(0));
        let worker_pending_count = Arc::clone(&pending_count);
        if let Err(e) = thread::Builder::new()
            .name("file-watcher-debounce".into())
            .spawn(move || worker_loop(rx, interval, publish_fn, worker_pending_count))
        {
            tracing::error!(error = %e, "file watcher debounce thread spawn failed");
        }
        Self { tx, pending_count }
    }

    /// Schedule a URI for debounced batch delivery.
    ///
    /// Repeated schedules of the same URI within the window reset its deadline.
    pub fn schedule(&self, uri: &str) {
        if let Err(e) = self.tx.send(WatcherMsg::Schedule(uri.to_string())) {
            tracing::debug!(error = %e, "file watcher debounce: channel closed on schedule");
        }
    }

    /// Number of unique URIs currently waiting in the debounce window.
    pub fn pending_uris(&self) -> usize {
        self.pending_count.load(Ordering::SeqCst)
    }
}

impl Drop for FileWatcherDebouncer {
    fn drop(&mut self) {
        if let Err(e) = self.tx.send(WatcherMsg::Shutdown) {
            tracing::debug!(error = %e, "file watcher debounce: channel closed on shutdown");
        }
    }
}

fn worker_loop<F>(
    rx: std::sync::mpsc::Receiver<WatcherMsg>,
    interval: Duration,
    publish_fn: F,
    pending_count: Arc<AtomicUsize>,
) where
    F: Fn(Vec<String>) + Send + 'static,
{
    // pending maps uri -> deadline (last scheduled deadline wins)
    let mut pending: HashMap<String, Instant> = HashMap::new();
    loop {
        let timeout = earliest_timeout(&pending);
        let msg = match timeout {
            Some(dur) if dur.is_zero() => {
                fire_expired(&mut pending, &publish_fn, &pending_count);
                match rx.try_recv() {
                    Ok(m) => Some(m),
                    Err(std::sync::mpsc::TryRecvError::Empty) => continue,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
                }
            }
            Some(dur) => match rx.recv_timeout(dur) {
                Ok(m) => Some(m),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    fire_expired(&mut pending, &publish_fn, &pending_count);
                    continue;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            },
            None => match rx.recv() {
                Ok(m) => Some(m),
                Err(_) => break,
            },
        };
        match msg {
            Some(WatcherMsg::Schedule(uri)) => {
                // Reset deadline on repeated schedules (last one wins)
                pending.insert(uri, Instant::now() + interval);
                pending_count.store(pending.len(), Ordering::SeqCst);
            }
            Some(WatcherMsg::Shutdown) => {
                // Flush all pending URIs before exiting
                let uris: Vec<String> = pending.keys().cloned().collect();
                if !uris.is_empty() {
                    publish_fn(uris);
                }
                pending.clear();
                pending_count.store(0, Ordering::SeqCst);
                break;
            }
            None => {}
        }
    }
}

fn earliest_timeout(pending: &HashMap<String, Instant>) -> Option<Duration> {
    if pending.is_empty() {
        return None;
    }
    let now = Instant::now();
    let earliest = pending.values().min().copied().unwrap_or(now);
    Some(earliest.saturating_duration_since(now))
}

fn fire_expired<F>(
    pending: &mut HashMap<String, Instant>,
    publish_fn: &F,
    pending_count: &AtomicUsize,
) where
    F: Fn(Vec<String>),
{
    let now = Instant::now();
    let expired: Vec<String> = pending
        .iter()
        .filter(|(_, deadline)| **deadline <= now)
        .map(|(uri, _)| uri.clone())
        .collect();
    if !expired.is_empty() {
        for uri in &expired {
            pending.remove(uri);
        }
        pending_count.store(pending.len(), Ordering::SeqCst);
        publish_fn(expired);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn earliest_timeout_reports_none_for_empty_pending_set() {
        let pending = HashMap::new();

        assert!(earliest_timeout(&pending).is_none());
    }

    #[test]
    fn earliest_timeout_saturates_when_deadline_already_passed() {
        let mut pending = HashMap::new();
        pending.insert("file:///expired.pl".to_string(), Instant::now() - Duration::from_millis(5));

        assert_eq!(earliest_timeout(&pending), Some(Duration::ZERO));
    }

    #[test]
    fn fire_expired_batches_only_ready_uris_and_keeps_pending_count() {
        let pending_count = AtomicUsize::new(0);
        let batches = Mutex::new(Vec::<Vec<String>>::new());
        let mut pending = HashMap::new();
        pending.insert("file:///ready-a.pl".to_string(), Instant::now() - Duration::from_millis(2));
        pending.insert("file:///ready-b.pl".to_string(), Instant::now() - Duration::from_millis(1));
        pending.insert("file:///later.pl".to_string(), Instant::now() + Duration::from_secs(30));
        pending_count.store(pending.len(), Ordering::SeqCst);

        fire_expired(
            &mut pending,
            &|mut uris| {
                uris.sort();
                batches.lock().push(uris);
            },
            &pending_count,
        );

        assert_eq!(
            batches.lock().as_slice(),
            [vec!["file:///ready-a.pl".to_string(), "file:///ready-b.pl".to_string()]]
        );
        assert!(!pending.contains_key("file:///ready-a.pl"));
        assert!(!pending.contains_key("file:///ready-b.pl"));
        assert!(pending.contains_key("file:///later.pl"));
        assert_eq!(pending_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn file_watcher_debouncer_fires_after_interval() {
        let count = Arc::new(AtomicUsize::new(0));
        let received = Arc::new(Mutex::new(Vec::<String>::new()));
        let c = Arc::clone(&count);
        let r = Arc::clone(&received);
        let debouncer =
            FileWatcherDebouncer::with_interval(Duration::from_millis(50), move |uris| {
                c.fetch_add(1, Ordering::SeqCst);
                r.lock().extend(uris);
            });
        debouncer.schedule("file:///test.pl");
        thread::sleep(Duration::from_millis(10));
        // Should not have fired yet
        assert_eq!(count.load(Ordering::SeqCst), 0);
        thread::sleep(Duration::from_millis(80));
        assert_eq!(count.load(Ordering::SeqCst), 1);
        assert!(received.lock().contains(&"file:///test.pl".to_string()));
    }

    #[test]
    fn file_watcher_debouncer_deduplicates_rapid_schedules() {
        let call_count = Arc::new(AtomicUsize::new(0));
        let uri_count = Arc::new(AtomicUsize::new(0));
        let c = Arc::clone(&call_count);
        let u = Arc::clone(&uri_count);
        let debouncer =
            FileWatcherDebouncer::with_interval(Duration::from_millis(80), move |uris| {
                c.fetch_add(1, Ordering::SeqCst);
                u.fetch_add(uris.len(), Ordering::SeqCst);
            });
        // Schedule the same URI 10 times rapidly
        for _ in 0..10 {
            debouncer.schedule("file:///same.pl");
        }
        thread::sleep(Duration::from_millis(200));
        // Should coalesce into a single batch call with 1 URI
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
        assert_eq!(uri_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn file_watcher_debouncer_reports_pending_uri_pressure() {
        let debouncer =
            FileWatcherDebouncer::with_interval(Duration::from_millis(80), move |_uris| {});

        debouncer.schedule("file:///pending.pl");
        for _ in 0..20 {
            if debouncer.pending_uris() == 1 {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(debouncer.pending_uris(), 1);

        thread::sleep(Duration::from_millis(150));
        assert_eq!(debouncer.pending_uris(), 0);
    }

    #[test]
    fn file_watcher_debouncer_batches_multiple_uris() {
        let batches = Arc::new(Mutex::new(Vec::<Vec<String>>::new()));
        let b = Arc::clone(&batches);
        let debouncer =
            FileWatcherDebouncer::with_interval(Duration::from_millis(50), move |uris| {
                let mut sorted = uris;
                sorted.sort();
                b.lock().push(sorted);
            });
        debouncer.schedule("file:///a.pl");
        debouncer.schedule("file:///b.pl");
        debouncer.schedule("file:///c.pl");
        thread::sleep(Duration::from_millis(150));
        let b = batches.lock();
        // All three URIs should arrive in 1-2 batches (all arrive before window expires,
        // but CI scheduler jitter may split into a second batch under load)
        assert!(b.len() <= 2, "Expected <=2 batches for 3 rapid changes, got {}", b.len());
        // All three URIs must be delivered exactly once across all batches
        let all_uris: Vec<String> = b.iter().flat_map(|v| v.iter().cloned()).collect();
        assert!(all_uris.contains(&"file:///a.pl".to_string()), "Missing file:///a.pl");
        assert!(all_uris.contains(&"file:///b.pl".to_string()), "Missing file:///b.pl");
        assert!(all_uris.contains(&"file:///c.pl".to_string()), "Missing file:///c.pl");
        assert_eq!(all_uris.len(), 3, "Expected exactly 3 URIs total, got {}", all_uris.len());
    }

    #[test]
    fn file_watcher_debouncer_flushes_on_drop() {
        let count = Arc::new(AtomicUsize::new(0));
        let c = Arc::clone(&count);
        let debouncer = FileWatcherDebouncer::with_interval(Duration::from_secs(5), move |_uris| {
            c.fetch_add(1, Ordering::SeqCst);
        });
        debouncer.schedule("file:///test.pl");
        drop(debouncer);
        // Give the background thread time to process the Shutdown message
        thread::sleep(Duration::from_millis(50));
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn file_watcher_debouncer_coalesces_bulk_operations() {
        // Simulates a git checkout with 50 file changes arriving rapidly
        let call_count = Arc::new(AtomicUsize::new(0));
        let total_uris = Arc::new(AtomicUsize::new(0));
        let c = Arc::clone(&call_count);
        let t = Arc::clone(&total_uris);
        let debouncer =
            FileWatcherDebouncer::with_interval(Duration::from_millis(100), move |uris| {
                c.fetch_add(1, Ordering::SeqCst);
                t.fetch_add(uris.len(), Ordering::SeqCst);
            });

        // Rapid-fire 50 distinct file changes
        for i in 0..50usize {
            debouncer.schedule(&format!("file:///test{i}.pl"));
        }

        // Wait for debounce window to expire
        thread::sleep(Duration::from_millis(300));

        let calls = call_count.load(Ordering::SeqCst);
        let uris = total_uris.load(Ordering::SeqCst);
        // Should coalesce into <=6 batch calls — under CI scheduler load the 100ms
        // window may fire before all 50 events arrive, splitting into a few batches.
        // The meaningful check is that coalescence occurred (not 50 individual calls).
        assert!(calls <= 6, "Expected <=6 batch calls for 50 rapid changes, got {calls}");
        assert_eq!(uris, 50, "All 50 URIs should be delivered, got {uris}");
    }
}
