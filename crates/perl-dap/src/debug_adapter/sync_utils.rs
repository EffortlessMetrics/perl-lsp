use super::DapMessage;
use serde_json::Value;
use std::sync::mpsc::Sender;
use std::sync::{Mutex, MutexGuard};

/// Poison-safe mutex lock that recovers from poisoned state.
pub(crate) fn lock_or_recover<'a, T>(mutex: &'a Mutex<T>, ctx: &'static str) -> MutexGuard<'a, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            tracing::warn!(ctx, "Poisoned mutex recovered");
            poisoned.into_inner()
        }
    }
}

/// Send a DAP event through the event channel with poison-safe sequence numbering.
///
/// Returns `true` if the event was successfully sent, `false` otherwise.
pub(crate) fn emit_event_safe(
    sender: &Sender<DapMessage>,
    seq: &Mutex<i64>,
    event: &str,
    body: Option<Value>,
) -> bool {
    let mut seq_lock = lock_or_recover(seq, "emit_event_safe.seq");
    *seq_lock += 1;
    sender.send(DapMessage::Event { seq: *seq_lock, event: event.to_string(), body }).is_ok()
}
