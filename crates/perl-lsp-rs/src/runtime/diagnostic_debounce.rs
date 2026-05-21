//! Diagnostic publication debouncer
//!
//! Coalesces rapid `didChange` diagnostic updates into a single publication
//! after a configurable quiet period (default 250ms).

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

const DEFAULT_DEBOUNCE_MS: u64 = 250;

enum DebounceMsg {
    Schedule(String),
    Shutdown,
}

pub(crate) struct DiagnosticDebouncer {
    tx: std::sync::mpsc::Sender<DebounceMsg>,
    #[allow(dead_code)] // Read by test/debug runtime pressure snapshots.
    pending_count: Arc<AtomicUsize>,
}

impl DiagnosticDebouncer {
    pub(crate) fn new<F>(publish_fn: F) -> Self
    where
        F: Fn(&str) + Send + 'static,
    {
        Self::with_interval(Duration::from_millis(DEFAULT_DEBOUNCE_MS), publish_fn)
    }

    pub(crate) fn with_interval<F>(interval: Duration, publish_fn: F) -> Self
    where
        F: Fn(&str) + Send + 'static,
    {
        let (tx, rx) = std::sync::mpsc::channel();
        let pending_count = Arc::new(AtomicUsize::new(0));
        let worker_pending_count = Arc::clone(&pending_count);
        if let Err(e) = thread::Builder::new()
            .name("diag-debounce".into())
            .spawn(move || worker_loop(rx, interval, publish_fn, worker_pending_count))
        {
            tracing::error!(error = %e, "diagnostic debounce thread spawn failed");
        }
        Self { tx, pending_count }
    }

    pub(crate) fn schedule(&self, uri: &str) {
        if let Err(e) = self.tx.send(DebounceMsg::Schedule(uri.to_string())) {
            tracing::debug!(error = %e, "diagnostic debounce: channel closed on schedule");
        }
    }

    #[allow(dead_code)] // Read by test/debug runtime pressure snapshots.
    pub(crate) fn pending_uris(&self) -> usize {
        self.pending_count.load(Ordering::SeqCst)
    }
}

impl Drop for DiagnosticDebouncer {
    fn drop(&mut self) {
        if let Err(e) = self.tx.send(DebounceMsg::Shutdown) {
            tracing::debug!(error = %e, "diagnostic debounce: channel closed on shutdown");
        }
    }
}

fn worker_loop<F>(
    rx: std::sync::mpsc::Receiver<DebounceMsg>,
    interval: Duration,
    publish_fn: F,
    pending_count: Arc<AtomicUsize>,
) where
    F: Fn(&str) + Send + 'static,
{
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
            Some(DebounceMsg::Schedule(uri)) => {
                pending.insert(uri, Instant::now() + interval);
                pending_count.store(pending.len(), Ordering::SeqCst);
            }
            Some(DebounceMsg::Shutdown) => {
                for (uri, _) in pending.drain() {
                    publish_fn(&uri);
                }
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
    F: Fn(&str),
{
    let now = Instant::now();
    let expired: Vec<String> = pending
        .iter()
        .filter(|(_, deadline)| **deadline <= now)
        .map(|(uri, _)| uri.clone())
        .collect();
    for uri in expired {
        pending.remove(&uri);
        pending_count.store(pending.len(), Ordering::SeqCst);
        publish_fn(&uri);
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
    fn fire_expired_publishes_only_ready_uris_and_keeps_pending_count() {
        let pending_count = AtomicUsize::new(0);
        let published = Mutex::new(Vec::<String>::new());
        let mut pending = HashMap::new();
        pending.insert("file:///ready.pl".to_string(), Instant::now() - Duration::from_millis(1));
        pending.insert("file:///later.pl".to_string(), Instant::now() + Duration::from_secs(30));
        pending_count.store(pending.len(), Ordering::SeqCst);

        fire_expired(&mut pending, &|uri| published.lock().push(uri.to_string()), &pending_count);

        assert_eq!(published.lock().as_slice(), ["file:///ready.pl"]);
        assert!(!pending.contains_key("file:///ready.pl"));
        assert!(pending.contains_key("file:///later.pl"));
        assert_eq!(pending_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn debouncer_fires_after_interval() {
        let count = Arc::new(AtomicUsize::new(0));
        let last_uri = Arc::new(Mutex::new(String::new()));
        let c = Arc::clone(&count);
        let u = Arc::clone(&last_uri);
        let debouncer = DiagnosticDebouncer::with_interval(Duration::from_millis(50), move |uri| {
            c.fetch_add(1, Ordering::SeqCst);
            *u.lock() = uri.to_string();
        });
        debouncer.schedule("file:///test.pl");
        thread::sleep(Duration::from_millis(10));
        assert_eq!(count.load(Ordering::SeqCst), 0);
        thread::sleep(Duration::from_millis(80));
        assert_eq!(count.load(Ordering::SeqCst), 1);
        assert_eq!(*last_uri.lock(), "file:///test.pl");
    }

    #[test]
    fn debouncer_resets_on_repeated_schedule() {
        let count = Arc::new(AtomicUsize::new(0));
        let c = Arc::clone(&count);
        let debouncer = DiagnosticDebouncer::with_interval(Duration::from_millis(80), move |_| {
            c.fetch_add(1, Ordering::SeqCst);
        });
        debouncer.schedule("file:///test.pl");
        thread::sleep(Duration::from_millis(40));
        debouncer.schedule("file:///test.pl");
        thread::sleep(Duration::from_millis(40));
        assert_eq!(count.load(Ordering::SeqCst), 0);
        thread::sleep(Duration::from_millis(80));
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn debouncer_reports_pending_uri_pressure() {
        let debouncer = DiagnosticDebouncer::with_interval(Duration::from_millis(80), move |_| {});

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
    fn debouncer_handles_multiple_uris() {
        let fired = Arc::new(Mutex::new(Vec::<String>::new()));
        let f = Arc::clone(&fired);
        let debouncer = DiagnosticDebouncer::with_interval(Duration::from_millis(50), move |uri| {
            f.lock().push(uri.to_string());
        });
        debouncer.schedule("file:///a.pl");
        debouncer.schedule("file:///b.pl");
        thread::sleep(Duration::from_millis(120));
        let mut uris = fired.lock().clone();
        uris.sort();
        assert_eq!(uris, vec!["file:///a.pl", "file:///b.pl"]);
    }

    #[test]
    fn debouncer_fires_pending_on_drop() {
        let count = Arc::new(AtomicUsize::new(0));
        let c = Arc::clone(&count);
        let debouncer = DiagnosticDebouncer::with_interval(Duration::from_secs(5), move |_| {
            c.fetch_add(1, Ordering::SeqCst);
        });
        debouncer.schedule("file:///test.pl");
        drop(debouncer);
        thread::sleep(Duration::from_millis(50));
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }
}
