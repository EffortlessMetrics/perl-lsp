//! Transport layer: run (stdin/stdout), run_socket, run_with_io.

use super::*;
use std::sync::mpsc::TryRecvError;

const EVENT_WRITE_BATCH_MAX: usize = 64;

fn write_framed_payload<W: Write>(writer: &mut W, payload: &[u8]) -> io::Result<()> {
    writer.write_all(b"Content-Length: ")?;
    writer.write_all(payload.len().to_string().as_bytes())?;
    writer.write_all(b"\r\n\r\n")?;
    writer.write_all(payload)
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

        thread::spawn(move || {
            while let Ok(first_msg) = rx.recv() {
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
                let mut write_failed = false;
                for payload in &payloads {
                    if let Err(e) = write_framed_payload(&mut *writer, payload) {
                        tracing::error!(error = %e, "Failed to write DAP frame in event handler");
                        write_failed = true;
                        break;
                    }
                }
                if !write_failed && let Err(e) = writer.flush() {
                    tracing::error!(error = %e, "Failed to flush DAP frame in event handler");
                }

                if disconnected {
                    break;
                }
            }
            tracing::debug!("Event handler thread terminating - channel closed");
        });

        let mut reader = BufReader::new(input);
        let mut framer = ContentLengthFramer::new();
        let mut read_buf = [0u8; 8 * 1024];

        loop {
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

                let DapMessage::Request { seq, command, arguments } = msg else {
                    continue;
                };

                let response = self.dispatch_request(seq, &command, arguments);
                let payload = match serde_json::to_vec(&response) {
                    Ok(payload) => payload,
                    Err(e) => {
                        tracing::error!(error = %e, "Failed to serialize DAP response");
                        continue;
                    }
                };

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
