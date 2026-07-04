//! `PERL_LSP_TIMING` phase-1 instrumentation sink.
//!
//! This module implements the opt-in timing contract used by the Neovim
//! live-edit latency lane. It emits one JSONL event per timing span so that a
//! developer can see where `textDocument/didChange` spends its wall time
//! (lock wait, apply-changes, rope-to-string, full parse, parent-map rebuild,
//! incremental-document update, commit) plus scheduler wait spans.
//!
//! # Transport safety
//!
//! The LSP server speaks JSON-RPC over **stdout**. Writing timing data to
//! stdout would corrupt the transport and kill the session, so this sink only
//! ever writes to **stderr** or to a **configured JSONL file** — never stdout.
//!
//! # Activation (`PERL_LSP_TIMING`)
//!
//! | Value                          | Sink                       |
//! |--------------------------------|----------------------------|
//! | unset / empty / `0` / `off`    | disabled (zero overhead)   |
//! | `1` / `stderr` / `true`        | JSONL to stderr            |
//! | any other value                | JSONL appended to that path |
//!
//! The env var is read **once** (cached in a `OnceLock`); when disabled the
//! only per-call cost is a single `OnceLock` deref plus, in test builds, one
//! relaxed atomic load. Callers additionally guard span construction behind
//! [`is_enabled`] so no strings are allocated on the hot path when disabled.
//!
//! # Scope (phase 1)
//!
//! Instrumentation only. This module records timings; it does **not** change
//! the mutation boundary or any parse/scheduling behavior.

use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;

/// Resolved output sink for timing events, computed once from the environment.
enum TimingMode {
    /// Disabled — emit nothing.
    Off,
    /// Emit JSONL lines to stderr.
    Stderr,
    /// Emit JSONL lines appended to the given file path.
    File(PathBuf),
}

/// A single timing span event.
///
/// Context fields are optional so the same shape can carry document spans
/// (with `detail` = uri tail, plus `version`/`bytes`/`edits`) and scheduler
/// spans (with `detail` = method name and the numeric fields left `None`).
#[derive(Clone, Debug)]
pub(crate) struct TimingSpan {
    /// Dotted span name, e.g. `didChange.full_parse`.
    pub span: &'static str,
    /// Wall time for the span in milliseconds.
    pub ms: f64,
    /// Free-form detail — uri tail for document spans, method for scheduler spans.
    pub detail: Option<String>,
    /// Document version, when applicable.
    pub version: Option<i64>,
    /// Post-change byte length, when applicable.
    pub bytes: Option<i64>,
    /// Number of content-change edits, when applicable.
    pub edits: Option<i64>,
}

impl TimingSpan {
    /// Construct a document span carrying the full context tuple.
    pub(crate) fn document(
        span: &'static str,
        ms: f64,
        uri_tail: String,
        version: i64,
        bytes: usize,
        edits: usize,
    ) -> Self {
        TimingSpan {
            span,
            ms,
            detail: Some(uri_tail),
            version: Some(version),
            bytes: i64::try_from(bytes).ok(),
            edits: i64::try_from(edits).ok(),
        }
    }

    /// Construct a lightweight span with only a `detail` string (e.g. method).
    pub(crate) fn labeled(span: &'static str, ms: f64, detail: impl Into<String>) -> Self {
        TimingSpan {
            span,
            ms,
            detail: Some(detail.into()),
            version: None,
            bytes: None,
            edits: None,
        }
    }
}

/// Parse a `PERL_LSP_TIMING` value into a [`TimingMode`].
///
/// Pure and side-effect free so it can be unit-tested without touching global
/// state or the environment.
fn parse_mode(raw: &str) -> TimingMode {
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || trimmed == "0"
        || trimmed.eq_ignore_ascii_case("off")
        || trimmed.eq_ignore_ascii_case("false")
    {
        TimingMode::Off
    } else if trimmed == "1"
        || trimmed.eq_ignore_ascii_case("stderr")
        || trimmed.eq_ignore_ascii_case("true")
    {
        TimingMode::Stderr
    } else {
        TimingMode::File(PathBuf::from(trimmed))
    }
}

/// Return the process-wide timing mode, computing it once from the environment.
fn mode() -> &'static TimingMode {
    static MODE: OnceLock<TimingMode> = OnceLock::new();
    MODE.get_or_init(|| match std::env::var("PERL_LSP_TIMING") {
        Ok(value) => parse_mode(&value),
        Err(_) => TimingMode::Off,
    })
}

/// Cached, lazily-opened file handle for the [`TimingMode::File`] sink.
fn file_writer(path: &PathBuf) -> Option<&'static std::sync::Mutex<std::fs::File>> {
    static WRITER: OnceLock<Option<std::sync::Mutex<std::fs::File>>> = OnceLock::new();
    WRITER
        .get_or_init(|| {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .ok()
                .map(std::sync::Mutex::new)
        })
        .as_ref()
}

/// Whether timing is currently enabled (env sink on, or test capture on).
///
/// This is the cheap guard callers use before constructing a [`TimingSpan`],
/// so that no allocation happens on the hot path when timing is off.
#[inline]
pub(crate) fn is_enabled() -> bool {
    !matches!(mode(), TimingMode::Off) || capture_enabled()
}

/// Milliseconds elapsed since `start`, as `f64`.
#[inline]
pub(crate) fn elapsed_ms(start: std::time::Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1_000.0
}

/// Serialize a span to a single-line JSON string (no trailing newline).
fn format_span_json(span: &TimingSpan) -> String {
    // Round to 3 decimals for a readable, stable JSONL stream.
    let ms = (span.ms * 1_000.0).round() / 1_000.0;
    serde_json::json!({
        "t": "perl_lsp_timing",
        "span": span.span,
        "ms": ms,
        "detail": span.detail,
        "version": span.version,
        "bytes": span.bytes,
        "edits": span.edits,
    })
    .to_string()
}

/// Emit one timing span to the active sink(s).
///
/// Callers should gate on [`is_enabled`] before building the span; this
/// function re-checks so an accidental unguarded call is still cheap when off.
pub(crate) fn emit(span: TimingSpan) {
    #[cfg(any(test, feature = "expose_lsp_test_api"))]
    if capture_enabled() {
        capture::push(span.clone());
    }

    match mode() {
        TimingMode::Off => {}
        TimingMode::Stderr => {
            let line = format_span_json(&span);
            // Ignore write errors: timing is best-effort diagnostics.
            let mut err = std::io::stderr().lock();
            let _ = writeln!(err, "{line}");
        }
        TimingMode::File(path) => {
            let line = format_span_json(&span);
            if let Some(writer) = file_writer(path) {
                if let Ok(mut file) = writer.lock() {
                    let _ = writeln!(file, "{line}");
                }
            } else {
                // Fall back to stderr if the file could not be opened.
                let mut err = std::io::stderr().lock();
                let _ = writeln!(err, "{line}");
            }
        }
    }
}

/// Whether the in-process test capture sink is active.
///
/// Compiles to a constant `false` outside test / `expose_lsp_test_api` builds,
/// so production [`is_enabled`] reduces to a single env-mode check.
#[cfg(any(test, feature = "expose_lsp_test_api"))]
#[inline]
fn capture_enabled() -> bool {
    capture::is_enabled()
}

#[cfg(not(any(test, feature = "expose_lsp_test_api")))]
#[inline]
fn capture_enabled() -> bool {
    false
}

/// In-process capture sink for tests and the ranged-typing receipt.
///
/// Independent of the `PERL_LSP_TIMING` env sink so tests never race on global
/// environment state or the cached [`mode`]. Enable with [`capture::start`],
/// collect with [`capture::drain`].
#[cfg(any(test, feature = "expose_lsp_test_api"))]
pub(crate) mod capture {
    use super::TimingSpan;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Mutex, OnceLock};

    static ENABLED: AtomicBool = AtomicBool::new(false);

    fn buffer() -> &'static Mutex<Vec<TimingSpan>> {
        static BUFFER: OnceLock<Mutex<Vec<TimingSpan>>> = OnceLock::new();
        BUFFER.get_or_init(|| Mutex::new(Vec::new()))
    }

    /// Whether capture is currently enabled.
    #[inline]
    pub(crate) fn is_enabled() -> bool {
        ENABLED.load(Ordering::Relaxed)
    }

    /// Clear any buffered spans and enable capture.
    pub(crate) fn start() {
        if let Ok(mut buf) = buffer().lock() {
            buf.clear();
        }
        ENABLED.store(true, Ordering::Relaxed);
    }

    /// Push a captured span (called from [`super::emit`]).
    pub(crate) fn push(span: TimingSpan) {
        if let Ok(mut buf) = buffer().lock() {
            buf.push(span);
        }
    }

    /// Disable capture and return the buffered spans.
    pub(crate) fn drain() -> Vec<TimingSpan> {
        ENABLED.store(false, Ordering::Relaxed);
        buffer().lock().map(|mut buf| std::mem::take(&mut *buf)).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mode_off_variants() {
        assert!(matches!(parse_mode(""), TimingMode::Off));
        assert!(matches!(parse_mode("   "), TimingMode::Off));
        assert!(matches!(parse_mode("0"), TimingMode::Off));
        assert!(matches!(parse_mode("off"), TimingMode::Off));
        assert!(matches!(parse_mode("OFF"), TimingMode::Off));
        assert!(matches!(parse_mode("false"), TimingMode::Off));
    }

    #[test]
    fn parse_mode_stderr_variants() {
        assert!(matches!(parse_mode("1"), TimingMode::Stderr));
        assert!(matches!(parse_mode("stderr"), TimingMode::Stderr));
        assert!(matches!(parse_mode("STDERR"), TimingMode::Stderr));
        assert!(matches!(parse_mode("true"), TimingMode::Stderr));
    }

    #[test]
    fn parse_mode_file_path() {
        match parse_mode("/tmp/perl-lsp-timing.jsonl") {
            TimingMode::File(path) => {
                assert_eq!(path, PathBuf::from("/tmp/perl-lsp-timing.jsonl"));
            }
            _ => panic!("expected File mode for a path value"),
        }
    }

    #[test]
    fn format_span_json_document_shape() {
        let span =
            TimingSpan::document("didChange.full_parse", 12.3456, "foo.pl".to_string(), 7, 4096, 3);
        let line = format_span_json(&span);
        let value: serde_json::Value =
            serde_json::from_str(&line).expect("emitted line must be valid JSON");
        assert_eq!(value["t"], "perl_lsp_timing");
        assert_eq!(value["span"], "didChange.full_parse");
        assert_eq!(value["detail"], "foo.pl");
        assert_eq!(value["version"], 7);
        assert_eq!(value["bytes"], 4096);
        assert_eq!(value["edits"], 3);
        // Rounded to 3 decimals.
        assert_eq!(value["ms"], 12.346);
        // JSONL: exactly one line, no embedded newline.
        assert!(!line.contains('\n'));
    }

    #[test]
    fn format_span_json_labeled_shape_has_null_context() {
        let span = TimingSpan::labeled("scheduler.read_wait", 0.5, "textDocument/completion");
        let value: serde_json::Value =
            serde_json::from_str(&format_span_json(&span)).expect("valid JSON");
        assert_eq!(value["span"], "scheduler.read_wait");
        assert_eq!(value["detail"], "textDocument/completion");
        assert!(value["version"].is_null());
        assert!(value["bytes"].is_null());
        assert!(value["edits"].is_null());
    }

    #[test]
    fn capture_start_records_and_drain_disables() {
        capture::start();
        assert!(is_enabled(), "capture should make timing enabled");
        emit(TimingSpan::labeled("didChange.total", 1.0, "a.pl"));
        emit(TimingSpan::labeled("didChange.full_parse", 2.0, "a.pl"));
        let spans = capture::drain();
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].span, "didChange.total");
        assert_eq!(spans[1].span, "didChange.full_parse");
        // Drain disables capture again.
        assert!(!capture::is_enabled());
    }
}
