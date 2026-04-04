//! Stream session manager for progressive inline completion.
//!
//! Manages active streaming sessions with cancel-previous semantics,
//! enabling progressive ghost text delivery via `$/progress` notifications.
//! Each session tracks cumulative text, a sequence counter, and a
//! cancellation flag so that stale streams are promptly terminated
//! when the user types or moves the cursor.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Key identifying a unique stream session.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionKey {
    pub uri: String,
    pub document_version: i64,
    pub line: u64,
    pub character: u64,
}

/// A live stream session.
#[allow(dead_code)] // Fields used by streaming handler when AI backend is wired
pub struct StreamSession {
    /// Unique session ID.
    pub session_id: String,
    /// Cancellation flag -- set to true to stop the stream.
    pub cancelled: AtomicBool,
    /// Current cumulative text.
    pub current_text: std::sync::Mutex<String>,
    /// Monotonically increasing sequence number.
    pub sequence: AtomicU64,
    /// The replacement range start line (set once).
    pub start_line: u64,
    /// The replacement range start character (set once).
    pub start_character: u64,
}

impl StreamSession {
    pub fn new(session_id: String, line: u64, character: u64) -> Self {
        Self {
            session_id,
            cancelled: AtomicBool::new(false),
            current_text: std::sync::Mutex::new(String::new()),
            sequence: AtomicU64::new(0),
            start_line: line,
            start_character: character,
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    pub fn next_sequence(&self) -> u64 {
        self.sequence.fetch_add(1, Ordering::Relaxed)
    }
}

/// Manages active stream sessions with cancel-previous semantics.
pub struct StreamSessionManager {
    sessions: std::sync::Mutex<HashMap<SessionKey, Arc<StreamSession>>>,
    generation: AtomicU64,
}

impl StreamSessionManager {
    pub fn new() -> Self {
        Self { sessions: std::sync::Mutex::new(HashMap::new()), generation: AtomicU64::new(0) }
    }

    /// Start a new session, cancelling any existing session for the same key.
    pub fn start_session(&self, key: SessionKey) -> Arc<StreamSession> {
        let generation = self.generation.fetch_add(1, Ordering::Relaxed);
        let session_id = format!("sess-{generation:x}");
        let session = Arc::new(StreamSession::new(session_id, key.line, key.character));

        let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());

        // Cancel previous session for this key
        if let Some(old) = sessions.insert(key, Arc::clone(&session)) {
            old.cancel();
        }

        session
    }

    /// Cancel all sessions for a given URI (on didChange/didClose).
    pub fn cancel_for_uri(&self, uri: &str) {
        let sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        for (key, session) in sessions.iter() {
            if key.uri == uri {
                session.cancel();
            }
        }
    }

    /// Cancel all sessions for a given URI where the document version is older
    /// than the supplied version (on document version change).
    pub fn cancel_for_uri_version(&self, uri: &str, version: i64) {
        let sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        for (key, session) in sessions.iter() {
            if key.uri == uri && key.document_version < version {
                session.cancel();
            }
        }
    }

    /// Remove completed/cancelled sessions (housekeeping).
    pub fn cleanup(&self) {
        let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        sessions.retain(|_, session| !session.is_cancelled());
    }
}

impl Default for StreamSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_session_creates_unique_ids() {
        let mgr = StreamSessionManager::new();
        let key =
            SessionKey { uri: "file:///a.pl".into(), document_version: 1, line: 5, character: 10 };
        let s1 = mgr.start_session(key.clone());
        let s2 = mgr.start_session(key);
        assert_ne!(s1.session_id, s2.session_id);
    }

    #[test]
    fn start_session_cancels_previous() {
        let mgr = StreamSessionManager::new();
        let key =
            SessionKey { uri: "file:///a.pl".into(), document_version: 1, line: 5, character: 10 };
        let s1 = mgr.start_session(key.clone());
        assert!(!s1.is_cancelled());
        let _s2 = mgr.start_session(key);
        assert!(s1.is_cancelled());
    }

    #[test]
    fn cancel_for_uri_cancels_matching() {
        let mgr = StreamSessionManager::new();
        let s1 = mgr.start_session(SessionKey {
            uri: "file:///a.pl".into(),
            document_version: 1,
            line: 0,
            character: 0,
        });
        let s2 = mgr.start_session(SessionKey {
            uri: "file:///b.pl".into(),
            document_version: 1,
            line: 0,
            character: 0,
        });
        mgr.cancel_for_uri("file:///a.pl");
        assert!(s1.is_cancelled());
        assert!(!s2.is_cancelled());
    }

    #[test]
    fn cancel_for_uri_version_cancels_older() {
        let mgr = StreamSessionManager::new();
        let s1 = mgr.start_session(SessionKey {
            uri: "file:///a.pl".into(),
            document_version: 1,
            line: 0,
            character: 0,
        });
        let s2 = mgr.start_session(SessionKey {
            uri: "file:///a.pl".into(),
            document_version: 3,
            line: 5,
            character: 0,
        });
        mgr.cancel_for_uri_version("file:///a.pl", 2);
        assert!(s1.is_cancelled());
        assert!(!s2.is_cancelled());
    }

    #[test]
    fn cleanup_removes_cancelled() {
        let mgr = StreamSessionManager::new();
        let key =
            SessionKey { uri: "file:///a.pl".into(), document_version: 1, line: 0, character: 0 };
        let session = mgr.start_session(key);
        session.cancel();
        mgr.cleanup();
        // After cleanup, cancelled sessions are removed
        let sessions = mgr.sessions.lock().unwrap_or_else(|e| e.into_inner());
        assert!(sessions.is_empty());
    }

    #[test]
    fn sequence_increments() {
        let session = StreamSession::new("test".into(), 0, 0);
        assert_eq!(session.next_sequence(), 0);
        assert_eq!(session.next_sequence(), 1);
        assert_eq!(session.next_sequence(), 2);
    }
}
