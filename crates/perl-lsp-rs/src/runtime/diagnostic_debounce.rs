//! Diagnostic publication debouncer
//!
//! Coalesces rapid `didChange` diagnostic updates into a single publication
//! after a configurable quiet period (default 250ms).

use std::collections::HashMap;
use std::thread;
use std::time::{Duration, Instant};

const DEFAULT_DEBOUNCE_MS: u64 = 250;

enum DebounceMsg {
    Schedule(String),
    Shutdown,
}

pub(crate) struct DiagnosticDebouncer {
    tx: std::sync::mpsc::Sender<DebounceMsg>,
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
        if let Err(e) = thread::Builder::new()
            .name("diag-debounce".into())
            .spawn(move || worker_loop(rx, interval, publish_fn))
        {
            tracing::error!(error = %e, "diagnostic debounce thread spawn failed");
        }
        Self { tx }
    }

    pub(crate) fn schedule(&self, uri: &str) {
        if let Err(e) = self.tx.send(DebounceMsg::Schedule(uri.to_string())) {
            tracing::debug!(error = %e, "diagnostic debounce: channel closed on schedule");
        }
    }
}

impl Drop for DiagnosticDebouncer {
    fn drop(&mut self) {
        if let Err(e) = self.tx.send(DebounceMsg::Shutdown) {
            tracing::debug!(error = %e, "diagnostic debounce: channel closed on shutdown");
        }
    }
}

fn worker_loop<F>(rx: std::sync::mpsc::Receiver<DebounceMsg>, interval: Duration, publish_fn: F)
where
    F: Fn(&str) + Send + 'static,
{
    let mut pending: HashMap<String, Instant> = HashMap::new();
    loop {
        let timeout = earliest_timeout(&pending);
        let msg = match timeout {
            Some(dur) if dur.is_zero() => {
                fire_expired(&mut pending, &publish_fn);
                match rx.try_recv() {
                    Ok(m) => Some(m),
                    Err(std::sync::mpsc::TryRecvError::Empty) => continue,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
                }
            }
            Some(dur) => match rx.recv_timeout(dur) {
                Ok(m) => Some(m),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    fire_expired(&mut pending, &publish_fn);
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
            }
            Some(DebounceMsg::Shutdown) => {
                for (uri, _) in pending.drain() {
                    publish_fn(&uri);
                }
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

fn fire_expired<F>(pending: &mut HashMap<String, Instant>, publish_fn: &F)
where
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
        let debouncer =
            DiagnosticDebouncer::with_interval(Duration::from_millis(5000), move |_| {
                c.fetch_add(1, Ordering::SeqCst);
            });
        debouncer.schedule("file:///test.pl");
        drop(debouncer);
        thread::sleep(Duration::from_millis(50));
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }
}
