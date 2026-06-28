//! Transport layer: run (stdin/stdout), run_socket, run_with_io.

use super::*;
use std::sync::atomic::Ordering;
use std::sync::mpsc::TryRecvError;

const EVENT_WRITE_BATCH_MAX: usize = 64;
const WRITE_FAILURE_THRESHOLD: usize = 3;

fn write_framed_payload<W: Write>(writer: &mut W, payload: &[u8]) -> io::Result<()> {
    writer.write_all(b"Content-Length: ")?;
    writer.write_all(payload.len().to_string().as_bytes())?;
    writer.write_all(b"\r\n\r\n")?;
    writer.write_all(payload)
}

fn record_event_write_failure(
    consecutive_write_failures: &mut usize,
    transport_broken: &AtomicBool,
) -> bool {
    *consecutive_write_failures += 1;
    if *consecutive_write_failures >= WRITE_FAILURE_THRESHOLD {
        transport_broken.store(true, Ordering::Release);
        true
    } else {
        false
    }
}

fn record_event_write_success(consecutive_write_failures: &mut usize) {
    *consecutive_write_failures = 0;
}

fn write_event_payloads<W: Write>(
    writer: &mut W,
    payloads: &[Vec<u8>],
    consecutive_write_failures: &mut usize,
    transport_broken: &AtomicBool,
) -> bool {
    let mut write_failed = false;
    let mut transport_marked_broken = false;
    for payload in payloads {
        if let Err(e) = write_framed_payload(writer, payload) {
            tracing::error!(error = %e, "Failed to write DAP frame in event handler");
            write_failed = true;
            transport_marked_broken =
                record_event_write_failure(consecutive_write_failures, transport_broken);
            break;
        }
    }
    if !write_failed {
        if let Err(e) = writer.flush() {
            tracing::error!(error = %e, "Failed to flush DAP frame in event handler");
            transport_marked_broken =
                record_event_write_failure(consecutive_write_failures, transport_broken);
        } else {
            record_event_write_success(consecutive_write_failures);
        }
    }
    transport_marked_broken
}

impl DebugAdapter {
    /// Run the debug adapter server
    pub(crate) fn run(&mut self) -> io::Result<()> {
        self.run_with_io(io::stdin(), io::stdout())
    }

    /// Run the debug adapter over a TCP socket transport.
    ///
    /// This binds to `127.0.0.1:<port>`, accepts one client connection, and
    /// serves the DAP session on that stream.
    pub(crate) fn run_socket(&mut self, port: u16) -> io::Result<()> {
        let listener = TcpListener::bind(("127.0.0.1", port))?;
        tracing::info!(port, "DAP socket transport listening on 127.0.0.1");

        let (stream, peer_addr) = listener.accept()?;
        tracing::info!(peer_addr = %peer_addr, "DAP socket client connected");

        let reader_stream = stream.try_clone()?;
        self.run_with_io(reader_stream, stream)
    }

    /// Shared DAP transport loop used by stdio and socket modes.
    pub(super) fn run_with_io<R, W>(&mut self, input: R, output: W) -> io::Result<()>
    where
        R: Read,
        W: Write + Send + 'static,
    {
        // Create a shared writer to prevent interleaving between the main loop
        // and the event handler thread.
        let shared_writer: Arc<Mutex<W>> = Arc::new(Mutex::new(output));
        let event_writer = Arc::clone(&shared_writer);

        // Create channel for asynchronous events.
        let (tx, rx) = channel::<DapMessage>();
        self.event_sender = Some(tx.clone());

        // Clone transport_broken flag to pass to the event handler thread.
        let transport_broken = Arc::clone(&self.transport_broken);

        thread::spawn(move || {
            let mut consecutive_write_failures = 0;

            while let Ok(first_msg) = rx.recv() {
                // Check if transport is already marked broken
                if transport_broken.load(Ordering::Acquire) {
                    break;
                }

                let mut batch = Vec::with_capacity(EVENT_WRITE_BATCH_MAX);
                batch.push(first_msg);

                let mut disconnected = false;
                while batch.len() < EVENT_WRITE_BATCH_MAX {
                    match rx.try_recv() {
                        Ok(msg) => batch.push(msg),
                        Err(TryRecvError::Empty) => break,
                        Err(TryRecvError::Disconnected) => {
                            disconnected = true;
                            break;
                        }
                    }
                }

                let mut payloads = Vec::with_capacity(batch.len());
                for msg in batch {
                    match serde_json::to_vec(&msg) {
                        Ok(payload) => payloads.push(payload),
                        Err(e) => {
                            tracing::error!(
                                error = %e,
                                message = ?msg,
                                "Failed to serialize DAP message"
                            );
                        }
                    }
                }

                if payloads.is_empty() {
                    if disconnected {
                        break;
                    }
                    continue;
                }

                let mut writer = lock_or_recover(&event_writer, "event_writer");
                if write_event_payloads(
                    &mut *writer,
                    &payloads,
                    &mut consecutive_write_failures,
                    &transport_broken,
                ) {
                    tracing::error!(
                        failure_count = consecutive_write_failures,
                        threshold = WRITE_FAILURE_THRESHOLD,
                        "Event handler detected persistent write failure; marking transport broken"
                    );
                    break;
                }

                if disconnected {
                    break;
                }
            }
            tracing::debug!("Event handler thread terminating");
        });

        let mut reader = BufReader::new(input);
        let mut framer = ContentLengthFramer::new();
        let mut read_buf = [0u8; 8 * 1024];

        loop {
            // Check if transport has been marked broken by the event handler
            if self.transport_broken.load(Ordering::Acquire) {
                tracing::error!(
                    "Transport is broken; event handler detected persistent write failure"
                );
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "Event handler detected persistent write failure; transport is broken",
                ));
            }

            let bytes_read = reader.read(&mut read_buf)?;
            if bytes_read == 0 {
                return Ok(());
            }

            framer.push(&read_buf[..bytes_read]);

            loop {
                let body = match framer.try_next() {
                    Ok(Some(body)) => body,
                    Ok(None) => break,
                    Err(error) => {
                        tracing::warn!(%error, "Failed to parse DAP transport frame");
                        continue;
                    }
                };

                let msg = match serde_json::from_slice::<DapMessage>(&body) {
                    Ok(msg) => msg,
                    Err(_) => {
                        tracing::warn!(body = %String::from_utf8_lossy(&body), "Failed to parse DAP message");
                        continue;
                    }
                };

                let (seq, command, arguments) = match msg {
                    DapMessage::Request { seq, command, arguments } => (seq, command, arguments),
                    DapMessage::Response {
                        seq, request_seq, command, success, message, ..
                    } => {
                        // Log reception of response messages from client (for potential future
                        // server-initiated requests that expect responses). Currently the adapter
                        // does not initiate requests, so these are unexpected but valid per DAP spec.
                        tracing::debug!(
                            seq,
                            request_seq,
                            command,
                            success,
                            message = ?message,
                            "Received Response message from client (not yet handled)"
                        );
                        continue;
                    }
                    DapMessage::Event { seq, event, body } => {
                        // Log reception of event messages from client. The DAP protocol permits
                        // bidirectional event flow for advanced features. Currently these are
                        // unexpected, but we handle them gracefully by logging.
                        tracing::debug!(
                            seq,
                            event,
                            body = ?body,
                            "Received Event message from client (not yet handled)"
                        );
                        continue;
                    }
                };

                let response = self.dispatch_request(seq, &command, arguments);
                let payload = serde_json::to_vec(&response).map_err(io::Error::other)?;

                let mut writer = lock_or_recover(&shared_writer, "response_writer");
                write_framed_payload(&mut *writer, &payload)?;
                writer.flush()?;

                // DAP requires this event only after initialize response is sent.
                if command == "initialize"
                    && Self::response_succeeded_for_command(&response, "initialize")
                {
                    self.send_event("initialized", None);
                }
            }
        }
    }
}

/// Transport supervision tests — placed inside this module to access the
/// `pub(super)` `run_with_io` without widening its visibility.
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::sync::atomic::{AtomicUsize, Ordering as AOrdering};
    use std::time::Duration;

    // ── Minimal Write impl that always fails ──────────────────────────────────

    struct FailingWriter {
        fail_after_writes: usize,
        write_count: Arc<AtomicUsize>,
    }

    impl FailingWriter {
        fn always_failing() -> Self {
            Self { fail_after_writes: 0, write_count: Arc::new(AtomicUsize::new(0)) }
        }

        fn fail_after(n: usize) -> Self {
            Self { fail_after_writes: n, write_count: Arc::new(AtomicUsize::new(0)) }
        }
    }

    impl Write for FailingWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let n = self.write_count.fetch_add(1, AOrdering::AcqRel);
            if n >= self.fail_after_writes {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "mock write failure"));
            }
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            let n = self.write_count.load(AOrdering::Acquire);
            if n >= self.fail_after_writes {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "mock flush failure"));
            }
            Ok(())
        }
    }

    #[derive(Default)]
    struct FlushFailingWriter {
        bytes: Vec<u8>,
    }

    impl Write for FlushFailingWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.bytes.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "mock flush failure"))
        }
    }

    #[derive(Clone, Default)]
    struct SharedWriter {
        bytes: Arc<Mutex<Vec<u8>>>,
    }

    impl Write for SharedWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let mut bytes =
                self.bytes.lock().map_err(|_| io::Error::other("writer buffer mutex poisoned"))?;
            bytes.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    // ── Frame builder ─────────────────────────────────────────────────────────

    fn framed_request(seq: i64, command: &str) -> Vec<u8> {
        let body = serde_json::to_vec(&serde_json::json!({
            "type": "request",
            "seq": seq,
            "command": command,
        }))
        .unwrap_or_default();
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        let mut frame = header.into_bytes();
        frame.extend_from_slice(&body);
        frame
    }

    fn framed_message(message: &DapMessage) -> Result<Vec<u8>, serde_json::Error> {
        let body = serde_json::to_vec(message)?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        let mut frame = header.into_bytes();
        frame.extend_from_slice(&body);
        Ok(frame)
    }

    // ── Tests ─────────────────────────────────────────────────────────────────

    /// When the output writer fails on the very first write (e.g. the client closed
    /// the socket immediately), `run_with_io` must return an I/O error rather than
    /// hanging or panicking.
    #[test]
    fn test_run_with_io_returns_error_on_immediate_write_failure() {
        let mut adapter = DebugAdapter::new();
        let input = Cursor::new(framed_request(1, "initialize"));
        let writer = FailingWriter::always_failing();
        let result = adapter.run_with_io(input, writer);
        assert!(result.is_err(), "run_with_io must return Err when writer is broken immediately");
    }

    #[test]
    fn test_failing_writer_fails_at_configured_boundary() {
        let mut writer = FailingWriter::fail_after(1);

        let first = writer.write(b"a");
        assert!(matches!(first, Ok(1)), "first write should succeed before the boundary");
        assert_eq!(
            writer.write_count.load(AOrdering::Acquire),
            writer.fail_after_writes,
            "write count should sit exactly on the configured failure boundary"
        );

        let second = writer.write(b"b");
        assert!(
            matches!(second, Err(ref error) if error.kind() == io::ErrorKind::BrokenPipe),
            "write at the configured boundary must return BrokenPipe"
        );
        assert_eq!(
            writer.write_count.load(AOrdering::Acquire),
            writer.fail_after_writes + 1,
            "failed boundary write must still be counted"
        );
    }

    #[test]
    fn test_run_with_io_handles_non_request_messages_without_output()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut input_bytes = framed_message(&DapMessage::Response {
            seq: 1,
            request_seq: 99,
            success: true,
            command: "serverInitiatedRequest".to_string(),
            body: None,
            message: Some("client response".to_string()),
        })?;
        input_bytes.extend_from_slice(&framed_message(&DapMessage::Event {
            seq: 2,
            event: "clientEvent".to_string(),
            body: Some(serde_json::json!({"source": "client"})),
        })?);

        let mut adapter = DebugAdapter::new();
        let writer = SharedWriter::default();
        let written = writer.bytes.clone();

        let result = adapter.run_with_io(Cursor::new(input_bytes), writer);

        assert!(result.is_ok(), "non-request messages must not fail the transport loop");
        let bytes = written.lock().map_err(|_| "writer buffer mutex poisoned")?;
        assert!(bytes.is_empty(), "non-request messages must not emit adapter output");
        Ok(())
    }

    /// A writer that succeeds for a few writes then fails permanently triggers the
    /// supervision path: the event-handler sets `transport_broken`, and the main
    /// loop detects it on the next iteration and returns `BrokenPipe`.
    ///
    /// Regression test for #1609: before this fix the event handler would log errors
    /// forever and the main loop would never notice the broken transport.
    #[test]
    fn test_transport_broken_flag_triggers_main_loop_exit() {
        // Allow enough writes for the initialize-response framing to complete, then
        // fail everything.  Each Content-Length response involves ~3 write calls
        // (header prefix, length, \r\n\r\n, body) — 6 successes is sufficient for
        // one response while ensuring event writes fail.
        let mut adapter = DebugAdapter::new();

        // Two requests queued: initialize (triggers initialized event write which
        // will fail) + a second request so the main loop iterates again and can
        // detect the broken flag.
        let mut input_bytes = framed_request(1, "initialize");
        input_bytes.extend_from_slice(&framed_request(2, "stackTrace"));
        let input = Cursor::new(input_bytes);
        let writer = FailingWriter::fail_after(6);

        let result = adapter.run_with_io(input, writer);
        // Either the event-writer flag fires or the main-loop write fails — either
        // way the function must not return Ok while the transport is broken.
        assert!(result.is_err(), "run_with_io must return Err when output is persistently broken");
    }

    /// The event-handler thread must exit in bounded time when writes fail
    /// permanently.  This guards against infinite retry loops.
    #[test]
    fn test_event_handler_exits_in_bounded_time_after_write_failure() {
        let (done_tx, done_rx) = std::sync::mpsc::channel::<io::Result<()>>();
        thread::spawn(move || {
            let mut adapter = DebugAdapter::new();
            let input = Cursor::new(framed_request(1, "initialize"));
            let writer = FailingWriter::always_failing();
            let _ = done_tx.send(adapter.run_with_io(input, writer));
        });
        let result = done_rx.recv_timeout(Duration::from_secs(5));
        assert!(
            result.is_ok(),
            "run_with_io must complete within 5 s after persistent write failure"
        );
    }

    /// The `transport_broken` flag starts as `false` on a fresh adapter and is
    /// not set by a successful run (clean EOF on the input side).
    #[test]
    fn test_transport_broken_flag_clear_on_clean_run() {
        let mut adapter = DebugAdapter::new();
        // Empty input → immediate EOF → clean Ok(()) return.
        let input = Cursor::new(vec![]);
        // Writer that always succeeds (Vec<u8>).
        let result = adapter.run_with_io(input, Vec::<u8>::new());
        assert!(result.is_ok(), "clean EOF must return Ok");
        // Flag must remain false.
        assert!(
            !adapter.transport_broken.load(AOrdering::Acquire),
            "transport_broken must remain false after a clean run"
        );
    }

    #[test]
    fn test_event_write_failure_waits_until_threshold() {
        let transport_broken = AtomicBool::new(false);
        let mut consecutive = 0usize;

        for _ in 1..WRITE_FAILURE_THRESHOLD {
            let threshold_hit = record_event_write_failure(&mut consecutive, &transport_broken);
            assert!(!threshold_hit, "transport must not be marked broken before the threshold");
            assert!(
                !transport_broken.load(AOrdering::Acquire),
                "transport_broken must stay false before the threshold"
            );
        }

        assert_eq!(consecutive, WRITE_FAILURE_THRESHOLD - 1);
    }

    #[test]
    fn test_event_write_failure_sets_transport_broken_at_threshold() {
        let transport_broken = AtomicBool::new(false);
        let mut consecutive = WRITE_FAILURE_THRESHOLD - 1;

        let threshold_hit = record_event_write_failure(&mut consecutive, &transport_broken);

        assert!(threshold_hit, "threshold failure must mark the transport broken");
        assert_eq!(consecutive, WRITE_FAILURE_THRESHOLD);
        assert!(
            transport_broken.load(AOrdering::Acquire),
            "transport_broken must be visible after the release store"
        );
    }

    #[test]
    fn test_event_write_success_resets_failure_counter() {
        let mut consecutive = WRITE_FAILURE_THRESHOLD - 1;

        record_event_write_success(&mut consecutive);

        assert_eq!(consecutive, 0, "successful event writes must reset failures");
    }

    #[test]
    fn test_write_event_payloads_successful_flush_resets_counter() {
        let mut writer = Vec::<u8>::new();
        let transport_broken = AtomicBool::new(false);
        let mut consecutive = WRITE_FAILURE_THRESHOLD - 1;
        let payloads = vec![b"{}".to_vec()];

        let threshold_hit =
            write_event_payloads(&mut writer, &payloads, &mut consecutive, &transport_broken);

        assert!(!threshold_hit, "successful event write must not mark the transport broken");
        assert_eq!(consecutive, 0, "successful flush must reset failure count");
        assert!(
            !transport_broken.load(AOrdering::Acquire),
            "transport_broken must remain false after successful flush"
        );
        assert!(
            String::from_utf8_lossy(&writer).starts_with("Content-Length:"),
            "event payload must be written as a DAP frame"
        );
    }

    #[test]
    fn test_write_event_payloads_flush_failure_marks_transport_broken_at_threshold() {
        let mut writer = FlushFailingWriter::default();
        let transport_broken = AtomicBool::new(false);
        let mut consecutive = WRITE_FAILURE_THRESHOLD - 1;
        let payloads = vec![b"{}".to_vec()];

        let threshold_hit =
            write_event_payloads(&mut writer, &payloads, &mut consecutive, &transport_broken);

        assert!(threshold_hit, "flush failure at threshold must mark the transport broken");
        assert_eq!(consecutive, WRITE_FAILURE_THRESHOLD);
        assert!(
            transport_broken.load(AOrdering::Acquire),
            "transport_broken must be set after threshold flush failure"
        );
        assert!(
            String::from_utf8_lossy(&writer.bytes).starts_with("Content-Length:"),
            "flush failure must occur after writing the DAP frame"
        );
    }

    #[test]
    fn test_run_with_io_returns_broken_pipe_when_flag_is_already_set() {
        let mut adapter = DebugAdapter::new();
        adapter.transport_broken.store(true, AOrdering::Release);

        let result = adapter.run_with_io(Cursor::new(Vec::<u8>::new()), Vec::<u8>::new());

        assert!(
            matches!(result, Err(ref error) if error.kind() == io::ErrorKind::BrokenPipe),
            "pre-marked broken transport must return BrokenPipe"
        );
    }
}

#[cfg(test)]
mod framing_tests {
    //! Stdio transport framing edge-case coverage for `run_with_io`.
    //!
    //! Each test drives the transport loop with an in-memory `std::io::Cursor`
    //! reader and a shared `Arc<Mutex<Vec<u8>>>` writer, asserting NO PANIC and
    //! graceful recovery or clean shutdown on EOF. These live in-crate (rather
    //! than in `tests/`) because `run_with_io` is `pub(super)` — keeping the
    //! transport loop crate-private while still covering its framing seam, with
    //! no production test-only shim.
    //!
    //! Tested at the loop level: behaviour (no panic, skip or continue), not
    //! internal framing error types.

    use super::*;
    use serde_json::json;
    use std::io::Cursor;

    // ── shared output buffer ──────────────────────────────────────────────────

    /// A clonable, thread-safe write buffer satisfying `Write + Send + 'static`.
    ///
    /// Because `run_with_io` requires `W: Write + Send + 'static` (the writer is
    /// moved into the event-handler thread), we cannot pass a `&mut Vec<u8>`.
    /// Instead we wrap the buffer in `Arc<Mutex<_>>` and implement `Write` on the
    /// wrapper so we can inspect the bytes after the loop completes.
    #[derive(Clone, Default)]
    struct SharedBuf(Arc<Mutex<Vec<u8>>>);

    impl SharedBuf {
        fn new() -> Self {
            Self(Arc::new(Mutex::new(Vec::new())))
        }

        fn bytes_snapshot(&self) -> Vec<u8> {
            self.0.lock().unwrap_or_else(|e| e.into_inner()).clone()
        }
    }

    impl io::Write for SharedBuf {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let mut guard = self.0.lock().unwrap_or_else(|e| e.into_inner());
            guard.write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    // ── helpers ────────────────────────────────────────────────────────────────

    /// Build a well-formed Content-Length framed DAP request.
    fn framed_request(seq: i64, command: &str, arguments: Option<serde_json::Value>) -> Vec<u8> {
        let args_part = match arguments {
            Some(v) => v.to_string(),
            None => "null".to_string(),
        };
        let body = format!(
            r#"{{"type":"request","seq":{seq},"command":"{command}","arguments":{args_part}}}"#
        );
        let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
        frame.extend_from_slice(body.as_bytes());
        frame
    }

    /// Build a Content-Length framed payload from arbitrary bytes (may not be valid JSON).
    fn framed_raw(body: &[u8]) -> Vec<u8> {
        let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
        frame.extend_from_slice(body);
        frame
    }

    /// Extract the body bytes of the first framed message from `buf`.
    ///
    /// The event-handler thread may append an "initialized" event to the shared
    /// writer *after* `run_with_io` returns (the thread is not joined before the
    /// function exits). Parsing `written[separator..end]` as JSON therefore fails
    /// non-deterministically because the slice contains the initialize response
    /// JSON followed by a second Content-Length frame.
    ///
    /// This helper parses the `Content-Length` header of the first frame and
    /// returns only those bytes, avoiding the race.
    fn first_frame_body(buf: &[u8]) -> io::Result<&[u8]> {
        // Locate the header/body separator.
        let sep_pos = buf.windows(4).position(|w| w == b"\r\n\r\n").ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "no CRLF separator in output")
        })?;
        let header = &buf[..sep_pos];
        let body_start = sep_pos + 4;

        // Parse Content-Length from the header.
        let cl_prefix = b"Content-Length: ";
        let cl_start =
            header.windows(cl_prefix.len()).position(|w| w == cl_prefix).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "no Content-Length in header")
            })? + cl_prefix.len();
        let cl_end = header[cl_start..]
            .iter()
            .position(|&b| b == b'\r' || b == b'\n')
            .map(|p| cl_start + p)
            .unwrap_or(header.len());
        let cl_str = std::str::from_utf8(&header[cl_start..cl_end])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let cl: usize = cl_str.trim().parse().map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("bad Content-Length: {e}"))
        })?;

        let body_end = body_start + cl;
        buf.get(body_start..body_end).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "buffer truncated before body end")
        })
    }

    // ── 1. Missing Content-Length header ──────────────────────────────────────

    #[test]
    fn test_transport_missing_content_length_header_no_panic() -> io::Result<()> {
        let input = b"X-Custom: foo\r\n\r\n".to_vec();
        let mut adapter = DebugAdapter::new();
        let result = adapter.run_with_io(Cursor::new(input), SharedBuf::new());
        match result {
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }

    // ── 2. Non-numeric Content-Length ──────────────────────────────────────────

    #[test]
    fn test_transport_non_numeric_content_length_no_panic() -> io::Result<()> {
        let input = b"Content-Length: notanumber\r\n\r\n".to_vec();
        let mut adapter = DebugAdapter::new();
        let result = adapter.run_with_io(Cursor::new(input), SharedBuf::new());
        let _ = result;
        Ok(())
    }

    // ── 3. Negative Content-Length ─────────────────────────────────────────────

    #[test]
    fn test_transport_negative_content_length_no_panic() -> io::Result<()> {
        let input = b"Content-Length: -1\r\n\r\n".to_vec();
        let mut adapter = DebugAdapter::new();
        let result = adapter.run_with_io(Cursor::new(input), SharedBuf::new());
        let _ = result;
        Ok(())
    }

    // ── 4. Valid header + malformed JSON body ──────────────────────────────────

    #[test]
    fn test_transport_valid_header_malformed_json_no_panic() -> io::Result<()> {
        let bad_body = b"this is not json at all!";
        let input = framed_raw(bad_body);
        let mut adapter = DebugAdapter::new();
        let result = adapter.run_with_io(Cursor::new(input), SharedBuf::new());
        let _ = result;
        Ok(())
    }

    // ── 5. Multiple messages in one buffer (both processed) ───────────────────

    #[test]
    fn test_transport_two_messages_in_one_buffer_no_panic() -> io::Result<()> {
        let mut input =
            framed_request(1, "initialize", Some(json!({"clientID": "test", "adapterID": "perl"})));
        input.extend(framed_request(2, "disconnect", None));

        let output = SharedBuf::new();
        let mut adapter = DebugAdapter::new();
        adapter.run_with_io(Cursor::new(input), output.clone())?;

        let written = output.bytes_snapshot();
        assert!(
            written.starts_with(b"Content-Length:"),
            "expected at least one framed response in output, got {} bytes",
            written.len()
        );
        Ok(())
    }

    // ── 6. EOF mid-header ──────────────────────────────────────────────────────

    #[test]
    fn test_transport_eof_mid_header_no_panic() -> io::Result<()> {
        let input = b"Content-Leng".to_vec();
        let mut adapter = DebugAdapter::new();
        let result = adapter.run_with_io(Cursor::new(input), SharedBuf::new());
        match result {
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }

    // ── 7. EOF mid-body ────────────────────────────────────────────────────────

    #[test]
    fn test_transport_eof_mid_body_no_panic() -> io::Result<()> {
        let input = b"Content-Length: 1000\r\n\r\nshort".to_vec();
        let mut adapter = DebugAdapter::new();
        let result = adapter.run_with_io(Cursor::new(input), SharedBuf::new());
        match result {
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }

    // ── 8. Extra / duplicate headers ───────────────────────────────────────────

    #[test]
    fn test_transport_extra_headers_no_panic() -> io::Result<()> {
        let body_str = r#"{"type":"request","seq":1,"command":"initialize","arguments":{"clientID":"test","adapterID":"perl"}}"#;
        let mut input = format!(
            "Content-Type: application/vscode-jsonrpc; charset=utf-8\r\nContent-Length: {}\r\n\r\n",
            body_str.len()
        )
        .into_bytes();
        input.extend_from_slice(body_str.as_bytes());

        let output = SharedBuf::new();
        let mut adapter = DebugAdapter::new();
        adapter.run_with_io(Cursor::new(input), output.clone())?;

        let written = output.bytes_snapshot();
        assert!(!written.is_empty(), "expected a response to the initialize request");
        Ok(())
    }

    // ── 9. LF-only separator instead of CRLF ──────────────────────────────────

    #[test]
    fn test_transport_lf_only_separator_no_panic() -> io::Result<()> {
        let body_str = r#"{"type":"request","seq":1,"command":"initialize","arguments":null}"#;
        let mut input = format!("Content-Length: {}\n\n", body_str.len()).into_bytes();
        input.extend_from_slice(body_str.as_bytes());

        let mut adapter = DebugAdapter::new();
        adapter.run_with_io(Cursor::new(input), SharedBuf::new())?;
        Ok(())
    }

    // ── 10. Malformed frame followed by well-formed one (recovery) ────────────

    #[test]
    fn test_transport_recovers_after_malformed_frame() -> io::Result<()> {
        let bad_body = b"not-json!!!";
        let mut input = framed_raw(bad_body);
        input.extend(framed_request(
            1,
            "initialize",
            Some(json!({"clientID": "test", "adapterID": "perl"})),
        ));

        let output = SharedBuf::new();
        let mut adapter = DebugAdapter::new();
        adapter.run_with_io(Cursor::new(input), output.clone())?;

        let written = output.bytes_snapshot();

        assert!(
            written.starts_with(b"Content-Length:"),
            "expected a framed response after recovery from malformed frame, got {} bytes",
            written.len()
        );

        let body = first_frame_body(&written)?;
        let parsed: serde_json::Value = serde_json::from_slice(body)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        assert_eq!(parsed["command"], "initialize", "response must be for initialize");
        assert_eq!(parsed["type"], "response", "must be a response message");
        Ok(())
    }

    // ── 11. Empty input (immediate EOF) ───────────────────────────────────────

    #[test]
    fn test_transport_empty_input_clean_shutdown() -> io::Result<()> {
        let mut adapter = DebugAdapter::new();
        adapter.run_with_io(Cursor::new(Vec::<u8>::new()), SharedBuf::new())?;
        Ok(())
    }
}
