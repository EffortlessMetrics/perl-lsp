//! Transport layer: run (stdin/stdout), run_socket, run_with_io.

use super::*;

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
        eprintln!("DAP socket transport listening on 127.0.0.1:{port}");

        let (stream, peer_addr) = listener.accept()?;
        eprintln!("DAP socket client connected: {peer_addr}");

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
            while let Ok(msg) = rx.recv() {
                let framed = match serde_json::to_vec(&msg) {
                    Ok(payload) => frame(&payload),
                    Err(e) => {
                        eprintln!("Failed to serialize DAP message: {} - {:#?}", e, msg);
                        continue;
                    }
                };

                let mut writer = lock_or_recover(&event_writer, "event_writer");
                if let Err(e) = writer.write_all(&framed) {
                    eprintln!("Failed to write DAP frame in event handler: {}", e);
                    continue;
                }
                if let Err(e) = writer.flush() {
                    eprintln!("Failed to flush DAP frame in event handler: {}", e);
                }
            }
            eprintln!("Event handler thread terminating - channel closed");
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
                        eprintln!("Failed to parse DAP transport frame: {error}");
                        continue;
                    }
                };

                let msg = match serde_json::from_slice::<DapMessage>(&body) {
                    Ok(msg) => msg,
                    Err(_) => {
                        eprintln!(
                            "Failed to parse DAP message: {}",
                            String::from_utf8_lossy(&body)
                        );
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
                        eprintln!("Failed to serialize DAP response: {}", e);
                        continue;
                    }
                };

                let framed = frame(&payload);
                let mut writer = lock_or_recover(&shared_writer, "response_writer");
                writer.write_all(&framed)?;
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
