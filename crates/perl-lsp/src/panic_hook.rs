//! Global panic hook for LSP server crash recovery.
//!
//! Installs a `std::panic::set_hook` that:
//! 1. Logs the panic information (message + location) via `tracing::error!`.
//! 2. Sends a `window/showMessage` notification with `MessageType::Error` so the
//!    LSP client can display a human-readable error message.
//! 3. Calls the previously-installed panic hook (usually the default one that
//!    writes to stderr) so that existing diagnostics are preserved.
//! 4. Does **not** call `std::process::abort` — the panic continues to unwind
//!    normally, allowing `catch_unwind` boundaries to recover where possible.
//!
//! # Thread Safety
//!
//! The hook captures an [`OutboundSender`] clone, which is backed by an
//! unbounded Tokio MPSC channel.  The sender is lock-free and safe to call
//! from any thread, including a panicking one.  We deliberately avoid
//! holding any `Mutex` inside the hook to prevent deadlocks when a panic
//! occurs while a lock is held.
//!
//! # Usage
//!
//! ```no_run
//! let server = perl_lsp::LspServer::new();
//! server.install_panic_hook();
//! server.run().ok();
//! ```

use crate::runtime::outbound::OutboundSender;
use serde_json::json;

/// Install a global panic hook backed by the given outbound sender.
///
/// The hook:
/// - Logs the panic via `tracing::error!` (safe from panicking context).
/// - Sends a `window/showMessage` notification to the LSP client (best-effort).
/// - Chains to the previously-registered panic hook.
///
/// This function is idempotent with respect to the *sender*, but calling it
/// multiple times will stack hooks.  In production the server calls it once
/// during startup.
pub(crate) fn install(outbound: OutboundSender) {
    let previous_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |info| {
        // ---- 1. Build a human-readable summary of the panic ---------------
        let location = info.location().map_or_else(
            || "unknown location".to_string(),
            |loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()),
        );

        // Attempt to extract &str or String payloads; fall back to generic
        // text for other types.
        let payload_text = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "non-string panic payload".to_string()
        };

        let summary = format!("LSP server internal error at {location}: {payload_text}");

        // ---- 2. Log via tracing (non-blocking, safe from panic context) ---
        tracing::error!(
            panic.location = %location,
            panic.payload = %payload_text,
            "LSP server panic detected — attempting graceful notification"
        );

        // ---- 3. Notify the client (best-effort — ignore send errors) ------
        let client_message = format!("Perl LSP internal error: {payload_text}\n(at {location})");
        let _ = outbound.send_notification(
            "window/showMessage",
            json!({
                "type": 1,  // MessageType::Error
                "message": client_message
            }),
        );

        // ---- 4. Chain to the previous hook (usually writes to stderr) -----
        previous_hook(info);

        // We deliberately do NOT abort.  The panic will continue to unwind,
        // giving catch_unwind boundaries a chance to recover.
        let _ = summary; // suppress unused-variable warning
    }));
}
