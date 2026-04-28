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
    ///
    /// The `events_provider` closure is called on each poll cycle to get the
    /// current event snapshot. Use `peek_events` (non-draining) to allow
    /// repeated calls to see events that arrived since the last drain.
    ///
    /// # Timeout behaviour
    ///
    /// At least one predicate check always runs before the deadline is tested.
    /// If the predicate still has not matched when the deadline expires, `None`
    /// is returned on the very next iteration — so the effective ceiling is
    /// `timeout + poll_interval` (50 ms by default).
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
            if let Some(diagnostics) = Self::latest_for_uri(&events, uri) {
                if predicate(&diagnostics) {
                    return Some(diagnostics);
                }
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
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

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

    #[test]
    fn latest_for_uri_returns_none_when_no_match() {
        let events = vec![LspEvent::Diagnostics {
            uri: "file:///b.pl".to_string(),
            diagnostics: vec![json!({"message": "other"})],
        }];
        let latest = DiagnosticsTracker::latest_for_uri(&events, "file:///a.pl");
        assert!(latest.is_none());
    }

    /// `wait_for_uri_matching` returns immediately when the first event snapshot
    /// already satisfies the predicate — no polling delay.
    #[test]
    fn wait_for_uri_matching_returns_on_immediate_match() {
        let events = vec![LspEvent::Diagnostics {
            uri: "file:///a.pl".to_string(),
            diagnostics: vec![],
        }];
        let result = DiagnosticsTracker::wait_for_uri_matching(
            || events.clone(),
            "file:///a.pl",
            Duration::from_millis(500),
            |diags| diags.is_empty(),
        );
        assert_eq!(result, Some(vec![]), "expected immediate match on empty diagnostics");
    }

    /// `wait_for_uri_matching` returns `None` when the predicate never matches
    /// within the timeout, without blocking for longer than necessary.
    #[test]
    fn wait_for_uri_matching_returns_none_on_timeout() {
        // Provider always returns a non-empty diagnostic — predicate for empty never fires.
        let result = DiagnosticsTracker::wait_for_uri_matching(
            || {
                vec![LspEvent::Diagnostics {
                    uri: "file:///a.pl".to_string(),
                    diagnostics: vec![json!({"message": "err"})],
                }]
            },
            "file:///a.pl",
            Duration::from_millis(120), // short timeout so the test runs quickly
            |diags| diags.is_empty(),   // never satisfied
        );
        assert!(result.is_none(), "expected None when predicate never matches within timeout");
    }

    /// `wait_for_uri_matching` returns the payload once the predicate is satisfied
    /// on a later poll cycle (simulated by a counter-based provider).
    #[test]
    fn wait_for_uri_matching_returns_when_predicate_satisfied_later() {
        // First two calls return errors; third returns empty (cleared).
        let call_count = Arc::new(Mutex::new(0usize));
        let call_count_clone = call_count.clone();

        let result = DiagnosticsTracker::wait_for_uri_matching(
            move || {
                let mut count = call_count_clone.lock().unwrap();
                *count += 1;
                let diags = if *count < 3 {
                    vec![json!({"message": "err"})]
                } else {
                    vec![] // cleared on third call
                };
                vec![LspEvent::Diagnostics {
                    uri: "file:///a.pl".to_string(),
                    diagnostics: diags,
                }]
            },
            "file:///a.pl",
            Duration::from_secs(5),
            |diags| diags.is_empty(),
        );

        assert_eq!(result, Some(vec![]), "expected empty payload when diagnostics clear");
        let calls = *call_count.lock().unwrap();
        assert!(calls >= 3, "expected at least 3 provider calls, got {}", calls);
    }

    /// `wait_for_uri_matching` ignores events for other URIs and does not
    /// falsely trigger the predicate on them.
    #[test]
    fn wait_for_uri_matching_ignores_other_uris() {
        // Provider always returns empty diagnostics but for the WRONG URI.
        let result = DiagnosticsTracker::wait_for_uri_matching(
            || {
                vec![LspEvent::Diagnostics {
                    uri: "file:///b.pl".to_string(), // different URI
                    diagnostics: vec![],
                }]
            },
            "file:///a.pl", // we're waiting on a.pl
            Duration::from_millis(120),
            |diags| diags.is_empty(),
        );
        assert!(result.is_none(), "should not match events for a different URI");
    }
}
