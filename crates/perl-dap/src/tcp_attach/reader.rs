use super::event::{DapEvent, dap_event_from_value};
use perl_lsp_rs_core::transport::framing::ContentLengthFramer;
use serde_json::Value;
use std::io::{BufReader, Read};
use std::net::TcpStream;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;

pub(crate) fn spawn_reader(
    stream: TcpStream,
    connected: Arc<Mutex<bool>>,
    event_sender: Option<Sender<DapEvent>>,
) {
    thread::spawn(move || run_reader(stream, connected, event_sender));
}

fn run_reader(
    stream: TcpStream,
    connected: Arc<Mutex<bool>>,
    event_sender: Option<Sender<DapEvent>>,
) {
    let mut reader = BufReader::new(stream);
    let mut framer = ContentLengthFramer::new();
    let mut read_buf = [0u8; 8 * 1024];

    loop {
        let bytes_read = match reader.read(&mut read_buf) {
            Ok(0) => {
                mark_disconnected(&connected);
                send_event(
                    &event_sender,
                    DapEvent::Terminated { reason: "connection_closed".to_string() },
                );
                tracing::debug!("TCP connection closed by debugger");
                return;
            }
            Ok(n) => n,
            Err(error) => {
                mark_disconnected(&connected);
                send_event(
                    &event_sender,
                    DapEvent::Error { message: format!("TCP read error: {}", error) },
                );
                tracing::error!(%error, "Error reading from TCP");
                return;
            }
        };

        framer.push(&read_buf[..bytes_read]);
        drain_frames(&mut framer, &event_sender);
    }
}

fn drain_frames(framer: &mut ContentLengthFramer, event_sender: &Option<Sender<DapEvent>>) {
    loop {
        let buffer = match framer.try_next() {
            Ok(Some(buffer)) => buffer,
            Ok(None) => break,
            Err(error) => {
                tracing::warn!(%error, "Failed to parse TCP DAP frame");
                continue;
            }
        };

        trace_frame(&buffer);
        emit_frame_event(&buffer, event_sender);
    }
}

fn trace_frame(buffer: &[u8]) {
    if let Ok(text) = std::str::from_utf8(buffer) {
        tracing::trace!(output = %text, "Received from debugger");
    } else {
        tracing::warn!(bytes = buffer.len(), "Received non-UTF8 message from debugger");
    }
}

fn emit_frame_event(buffer: &[u8], event_sender: &Option<Sender<DapEvent>>) {
    let Some(sender) = event_sender else {
        return;
    };
    let Ok(value) = serde_json::from_slice::<Value>(buffer) else {
        return;
    };
    let Some(event) = dap_event_from_value(&value) else {
        return;
    };

    let _ = sender.send(event);
}

fn send_event(event_sender: &Option<Sender<DapEvent>>, event: DapEvent) {
    if let Some(sender) = event_sender {
        let _ = sender.send(event);
    }
}

fn mark_disconnected(connected: &Arc<Mutex<bool>>) {
    *connected.lock().unwrap_or_else(|error| error.into_inner()) = false;
}
