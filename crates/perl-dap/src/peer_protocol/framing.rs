//! Content-Length framing for the peer protocol, reusing the workspace's
//! `perl_lsp_rs_core` framing so the wire format is identical to DAP's base
//! protocol (decision D4).

use perl_lsp_rs_core::transport::{ContentLengthFramer, FramingError, frame};

use super::message::PeerMessage;

/// Errors decoding a framed peer message.
#[derive(Debug, thiserror::Error)]
pub enum PeerFrameError {
    /// The underlying Content-Length framing was malformed.
    #[error("framing error: {0}")]
    Framing(#[from] FramingError),
    /// A complete frame body was not valid peer-protocol JSON.
    #[error("invalid peer message JSON: {0}")]
    Json(String),
}

/// Encode a peer message into a Content-Length framed byte buffer ready to write
/// to the transport.
///
/// # Errors
/// Returns an error only if the message fails to serialize to JSON, which cannot
/// happen for the well-formed [`PeerMessage`] types but is surfaced for safety.
pub fn encode_message(msg: &PeerMessage) -> Result<Vec<u8>, PeerFrameError> {
    let body = serde_json::to_vec(msg).map_err(|e| PeerFrameError::Json(e.to_string()))?;
    Ok(frame(&body))
}

/// Incremental decoder: push transport bytes, pull complete [`PeerMessage`]s.
///
/// Wraps [`ContentLengthFramer`] and deserializes each extracted body.
#[derive(Debug, Default)]
pub struct PeerFrameDecoder {
    framer: ContentLengthFramer,
}

impl PeerFrameDecoder {
    /// Create an empty decoder.
    #[must_use]
    pub fn new() -> Self {
        Self { framer: ContentLengthFramer::new() }
    }

    /// Append raw transport bytes.
    pub fn push(&mut self, bytes: &[u8]) {
        self.framer.push(bytes);
    }

    /// Attempt to extract the next complete message.
    ///
    /// - `Ok(Some(msg))` — a full message was decoded.
    /// - `Ok(None)` — more bytes are needed.
    /// - `Err(..)` — malformed framing or JSON.
    ///
    /// # Errors
    /// Propagates framing errors and JSON deserialization failures.
    pub fn try_next(&mut self) -> Result<Option<PeerMessage>, PeerFrameError> {
        match self.framer.try_next()? {
            Some(body) => {
                let msg = serde_json::from_slice::<PeerMessage>(&body)
                    .map_err(|e| PeerFrameError::Json(e.to_string()))?;
                Ok(Some(msg))
            }
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peer_protocol::message::{PeerEvent, PeerRequest, command, event};

    #[test]
    fn encode_then_decode_round_trips() {
        let msg = PeerMessage::Request(PeerRequest {
            seq: 1,
            command: command::HELLO.to_string(),
            arguments: Some(serde_json::json!({"peer": "Devel::ptkdb"})),
        });
        let bytes = encode_message(&msg).expect("encode");
        // The frame carries a Content-Length header.
        let header = String::from_utf8_lossy(&bytes[..40]);
        assert!(header.starts_with("Content-Length: "), "got: {header}");

        let mut dec = PeerFrameDecoder::new();
        dec.push(&bytes);
        let out = dec.try_next().expect("decode").expect("one message");
        assert_eq!(out, msg);
    }

    #[test]
    fn decoder_handles_split_and_batched_frames() {
        let a = encode_message(&PeerMessage::Event(PeerEvent {
            seq: 1,
            event: event::STOPPED.to_string(),
            body: None,
        }))
        .expect("encode a");
        let b = encode_message(&PeerMessage::Event(PeerEvent {
            seq: 2,
            event: event::CONTINUED.to_string(),
            body: None,
        }))
        .expect("encode b");

        let mut dec = PeerFrameDecoder::new();
        // Feed the first frame in two chunks, then the second whole.
        let (head, tail) = a.split_at(5);
        dec.push(head);
        assert!(dec.try_next().expect("partial").is_none());
        dec.push(tail);
        dec.push(&b);

        let m1 = dec.try_next().expect("m1").expect("present");
        let m2 = dec.try_next().expect("m2").expect("present");
        assert_eq!(m1.seq(), 1);
        assert_eq!(m2.seq(), 2);
        assert!(dec.try_next().expect("drained").is_none());
    }

    #[test]
    fn malformed_json_body_is_an_error_not_a_panic() {
        // A well-framed body that is not a valid PeerMessage.
        let bytes = frame(b"{\"type\":\"bogus\"}");
        let mut dec = PeerFrameDecoder::new();
        dec.push(&bytes);
        let err = dec.try_next().expect_err("should error");
        assert!(matches!(err, PeerFrameError::Json(_)));
    }
}
