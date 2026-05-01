use super::DapDispatcher;
use crate::protocol::{Event, Request, Response};
use anyhow::Result;
use serde_json::Value;

impl DapDispatcher {
    pub(super) fn create_initialized_event(&self) -> Event {
        let mut seq = self.event_seq.lock().unwrap_or_else(|e| e.into_inner());
        let event_seq = *seq;
        *seq += 1;

        if let Ok(mut init) = self.initialized.lock() {
            *init = true;
        }

        Event {
            seq: event_seq,
            msg_type: "event".to_string(),
            event: "initialized".to_string(),
            body: None,
        }
    }

    pub(super) fn create_response(&self, request: &Request, result: Result<Value>) -> Response {
        let mut seq = self.response_seq.lock().unwrap_or_else(|e| e.into_inner());
        let response_seq = *seq;
        *seq += 1;

        match result {
            Ok(body) => Response {
                seq: response_seq,
                msg_type: "response".to_string(),
                request_seq: request.seq,
                success: true,
                command: request.command.clone(),
                message: None,
                body: Some(body),
            },
            Err(err) => Response {
                seq: response_seq,
                msg_type: "response".to_string(),
                request_seq: request.seq,
                success: false,
                command: request.command.clone(),
                message: Some(err.to_string()),
                body: None,
            },
        }
    }
}
