//! Server→Client Refresh Controller
//!
//! Manages debounced refresh requests to clients for LSP 3.16+ dynamic refresh capabilities.
//!
//! ## Features
//! - **Debouncing**: Prevents flooding the client with rapid refresh requests (250-500ms window)
//! - **Capability-aware**: Only sends refresh requests if client advertised support
//! - **Per-type timers**: Independent debounce timers for each refresh type
//!
//! ## Refresh Types Supported
//! - Code Lens (`workspace/codeLens/refresh`)
//! - Semantic Tokens (`workspace/semanticTokens/refresh`)
//! - Inlay Hints (`workspace/inlayHint/refresh`)
//! - Inline Values (`workspace/inlineValue/refresh`)
//! - Diagnostics (`workspace/diagnostic/refresh`)
//! - Folding Ranges (`workspace/foldingRange/refresh`) — proposed

use parking_lot::Mutex;
use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Debounce window for refresh requests (milliseconds)
const DEBOUNCE_MS: u64 = 350;

/// Tracks last refresh time for a specific refresh type
#[derive(Debug, Clone)]
struct RefreshTimer {
    /// Last time a refresh request was sent
    last_refresh: Option<Instant>,
}

impl RefreshTimer {
    /// Create a new refresh timer (never sent)
    fn new() -> Self {
        Self { last_refresh: None }
    }

    /// Check if enough time has passed since last refresh
    fn should_refresh(&self, debounce_duration: Duration) -> bool {
        match self.last_refresh {
            None => true,
            Some(last) => last.elapsed() >= debounce_duration,
        }
    }

    /// Mark refresh as sent
    fn mark_refreshed(&mut self) {
        self.last_refresh = Some(Instant::now());
    }
}

/// Controller for debounced server→client refresh requests
///
/// ## Usage Pattern
/// ```rust,ignore
/// // Create controller
/// let controller = RefreshController::new();
///
/// // Register with server
/// server.set_refresh_controller(controller.clone());
///
/// // Trigger refreshes after operations
/// controller.refresh_code_lens(server)?;
/// controller.refresh_diagnostics(server)?;
/// ```
#[derive(Debug, Clone)]
pub(crate) struct RefreshController {
    /// Timers for each refresh type (protected by mutex for interior mutability)
    code_lens_timer: Arc<Mutex<RefreshTimer>>,
    semantic_tokens_timer: Arc<Mutex<RefreshTimer>>,
    inlay_hint_timer: Arc<Mutex<RefreshTimer>>,
    inline_value_timer: Arc<Mutex<RefreshTimer>>,
    diagnostic_timer: Arc<Mutex<RefreshTimer>>,
    folding_range_timer: Arc<Mutex<RefreshTimer>>,

    /// Debounce duration (configurable per instance)
    debounce_duration: Duration,
}

impl RefreshController {
    /// Create a new refresh controller with default debounce window
    pub(crate) fn new() -> Self {
        Self::with_debounce(Duration::from_millis(DEBOUNCE_MS))
    }

    /// Create a refresh controller with custom debounce window
    #[allow(dead_code)] // Available for custom debounce configuration
    pub(crate) fn with_debounce(debounce_duration: Duration) -> Self {
        Self {
            code_lens_timer: Arc::new(Mutex::new(RefreshTimer::new())),
            semantic_tokens_timer: Arc::new(Mutex::new(RefreshTimer::new())),
            inlay_hint_timer: Arc::new(Mutex::new(RefreshTimer::new())),
            inline_value_timer: Arc::new(Mutex::new(RefreshTimer::new())),
            diagnostic_timer: Arc::new(Mutex::new(RefreshTimer::new())),
            folding_range_timer: Arc::new(Mutex::new(RefreshTimer::new())),
            debounce_duration,
        }
    }

    /// Request code lens refresh with debounce
    ///
    /// Sends `workspace/codeLens/refresh` request to client if:
    /// - Client advertised `workspace.codeLens.refreshSupport` capability
    /// - Debounce window has elapsed since last refresh
    ///
    /// # Errors
    /// Returns IO error if sending request fails
    pub(crate) fn refresh_code_lens(&self, server: &super::LspServer) -> io::Result<()> {
        if !server.client_capabilities.code_lens_refresh_support {
            return Ok(());
        }

        let mut timer = self.code_lens_timer.lock();
        if timer.should_refresh(self.debounce_duration) {
            server.request_code_lens_refresh()?;
            timer.mark_refreshed();
        }
        Ok(())
    }

    /// Request semantic tokens refresh with debounce
    ///
    /// Sends `workspace/semanticTokens/refresh` request to client if:
    /// - Client advertised `workspace.semanticTokens.refreshSupport` capability
    /// - Debounce window has elapsed since last refresh
    ///
    /// # Errors
    /// Returns IO error if sending request fails
    pub(crate) fn refresh_semantic_tokens(&self, server: &super::LspServer) -> io::Result<()> {
        if !server.client_capabilities.semantic_tokens_refresh_support {
            return Ok(());
        }

        let mut timer = self.semantic_tokens_timer.lock();
        if timer.should_refresh(self.debounce_duration) {
            server.request_semantic_tokens_refresh()?;
            timer.mark_refreshed();
        }
        Ok(())
    }

    /// Request inlay hints refresh with debounce
    ///
    /// Sends `workspace/inlayHint/refresh` request to client if:
    /// - Client advertised `workspace.inlayHint.refreshSupport` capability
    /// - Debounce window has elapsed since last refresh
    ///
    /// # Errors
    /// Returns IO error if sending request fails
    pub(crate) fn refresh_inlay_hints(&self, server: &super::LspServer) -> io::Result<()> {
        if !server.client_capabilities.inlay_hint_refresh_support {
            return Ok(());
        }

        let mut timer = self.inlay_hint_timer.lock();
        if timer.should_refresh(self.debounce_duration) {
            server.request_inlay_hint_refresh()?;
            timer.mark_refreshed();
        }
        Ok(())
    }

    /// Request inline values refresh with debounce
    ///
    /// Sends `workspace/inlineValue/refresh` request to client if:
    /// - Client advertised `workspace.inlineValue.refreshSupport` capability
    /// - Debounce window has elapsed since last refresh
    ///
    /// # Errors
    /// Returns IO error if sending request fails
    pub(crate) fn refresh_inline_values(&self, server: &super::LspServer) -> io::Result<()> {
        if !server.client_capabilities.inline_value_refresh_support {
            return Ok(());
        }

        let mut timer = self.inline_value_timer.lock();
        if timer.should_refresh(self.debounce_duration) {
            server.request_inline_value_refresh()?;
            timer.mark_refreshed();
        }
        Ok(())
    }

    /// Request diagnostics refresh with debounce
    ///
    /// Sends `workspace/diagnostic/refresh` request to client if:
    /// - Client advertised `workspace.diagnosticProvider.refreshSupport` capability
    /// - Debounce window has elapsed since last refresh
    ///
    /// # Errors
    /// Returns IO error if sending request fails
    pub(crate) fn refresh_diagnostics(&self, server: &super::LspServer) -> io::Result<()> {
        if !server.client_capabilities.diagnostic_refresh_support {
            return Ok(());
        }

        let mut timer = self.diagnostic_timer.lock();
        if timer.should_refresh(self.debounce_duration) {
            server.request_diagnostic_refresh()?;
            timer.mark_refreshed();
        }
        Ok(())
    }

    /// Request folding ranges refresh with debounce (proposed capability)
    ///
    /// Sends `workspace/foldingRange/refresh` request to client if:
    /// - Client advertised `workspace.foldingRange.refreshSupport` capability
    /// - Debounce window has elapsed since last refresh
    ///
    /// **Note**: This is a proposed LSP capability and may not be supported by all clients.
    ///
    /// # Errors
    /// Returns IO error if sending request fails
    pub(crate) fn refresh_folding_ranges(&self, server: &super::LspServer) -> io::Result<()> {
        if !server.client_capabilities.folding_range_refresh_support {
            return Ok(());
        }

        let mut timer = self.folding_range_timer.lock();
        if timer.should_refresh(self.debounce_duration) {
            server.request_folding_range_refresh()?;
            timer.mark_refreshed();
        }
        Ok(())
    }

    /// Trigger all applicable refreshes (bulk operation)
    ///
    /// Useful after workspace-wide operations like:
    /// - Configuration changes
    /// - Workspace folder updates
    /// - Index rebuilds
    ///
    /// Only sends refresh requests for capabilities the client advertised support for.
    ///
    /// # Errors
    /// Returns first IO error encountered. Subsequent refresh attempts are skipped.
    pub(crate) fn refresh_all(&self, server: &super::LspServer) -> io::Result<()> {
        // Refresh in dependency order (least to most expensive)
        self.refresh_folding_ranges(server)?;
        self.refresh_inlay_hints(server)?;
        self.refresh_inline_values(server)?;
        self.refresh_code_lens(server)?;
        self.refresh_semantic_tokens(server)?;
        self.refresh_diagnostics(server)?;
        Ok(())
    }

    /// Force refresh without debounce check (testing/emergency use)
    ///
    /// Bypasses debounce timers and immediately sends all refresh requests
    /// for capabilities the client supports. Use sparingly in production.
    ///
    /// # Errors
    /// Returns first IO error encountered. Subsequent refresh attempts are skipped.
    #[allow(dead_code)]
    pub(crate) fn force_refresh_all(&self, server: &super::LspServer) -> io::Result<()> {
        // Reset all timers to force immediate refresh
        *self.code_lens_timer.lock() = RefreshTimer::new();
        *self.semantic_tokens_timer.lock() = RefreshTimer::new();
        *self.inlay_hint_timer.lock() = RefreshTimer::new();
        *self.inline_value_timer.lock() = RefreshTimer::new();
        *self.diagnostic_timer.lock() = RefreshTimer::new();
        *self.folding_range_timer.lock() = RefreshTimer::new();

        // Trigger all refreshes
        self.refresh_all(server)
    }
}

impl Default for RefreshController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timer_should_refresh_initially() {
        let timer = RefreshTimer::new();
        assert!(timer.should_refresh(Duration::from_millis(100)));
    }

    #[test]
    fn timer_respects_debounce_window() {
        let mut timer = RefreshTimer::new();
        timer.mark_refreshed();

        // Should not refresh immediately
        assert!(!timer.should_refresh(Duration::from_secs(1)));

        // Wait for debounce window
        std::thread::sleep(Duration::from_millis(50));
        assert!(timer.should_refresh(Duration::from_millis(25)));
    }

    #[test]
    fn controller_creates_with_default_debounce() {
        let controller = RefreshController::new();
        assert_eq!(controller.debounce_duration, Duration::from_millis(DEBOUNCE_MS));
    }

    #[test]
    fn controller_creates_with_custom_debounce() {
        let custom_duration = Duration::from_millis(500);
        let controller = RefreshController::with_debounce(custom_duration);
        assert_eq!(controller.debounce_duration, custom_duration);
    }
}
