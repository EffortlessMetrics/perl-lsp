//! Diagnostics-focused helpers for UX harness orchestration.

use crate::LspEvent;
use serde_json::Value;
use std::time::{Duration, Instant};

/// Polling helper for diagnostics events in the UX harness queue.
pub struct DiagnosticsTracker;

impl DiagnosticsTracker {
    /// Return the most recent diagnostics payload seen for `uri`.
    pub fn latest_for_uri(events: &[LspEvent], uri: &str) -> Option<Vec<Value>> {
        events.iter().rev().find_map(|event| match event {
            LspEvent::Diagnostics { uri: event_uri, diagnostics } if event_uri == uri => {
                Some(diagnostics.clone())
            }
            _ => None,
        })
    }

    /// Wait until diagnostics for `uri` satisfy `predicate`, returning the
    /// matching payload. Returns `None` on timeout.
    pub fn wait_for_uri_matching<F>(
        mut events_provider: impl FnMut() -> Vec<LspEvent>,
        uri: &str,
        timeout: Duration,
        mut predicate: F,
    ) -> Option<Vec<Value>>
    where
        F: FnMut(&[Value]) -> bool,
    {
        let deadline = Instant::now() + timeout;
        loop {
            let events = events_provider();
            if let Some(diagnostics) = Self::latest_for_uri(&events, uri)
                && predicate(&diagnostics)
            {
                return Some(diagnostics);
            }

            if Instant::now() >= deadline {
                return None;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DiagnosticsTracker;
    use crate::LspEvent;
    use serde_json::json;

    #[test]
    fn latest_for_uri_prefers_most_recent_payload() {
        let events = vec![
            LspEvent::Diagnostics {
                uri: "file:///a.pl".to_string(),
                diagnostics: vec![json!({"message": "old"})],
            },
            LspEvent::Diagnostics {
                uri: "file:///b.pl".to_string(),
                diagnostics: vec![json!({"message": "other"})],
            },
            LspEvent::Diagnostics {
                uri: "file:///a.pl".to_string(),
                diagnostics: vec![json!({"message": "new"})],
            },
        ];

        let latest = DiagnosticsTracker::latest_for_uri(&events, "file:///a.pl");
        assert_eq!(latest, Some(vec![json!({"message": "new"})]));
    }
}
