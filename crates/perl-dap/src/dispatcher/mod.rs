//! DAP Message Dispatcher
#![allow(deprecated)]

use crate::breakpoints::BreakpointStore;
use crate::protocol::{Event, Request, Response};
use serde_json::Value;
use std::sync::{Arc, Mutex};

mod handlers;
mod response;

#[deprecated(
    since = "0.2.0",
    note = "Use DebugAdapter directly; DapDispatcher will be removed in a future release"
)]
pub struct DispatchResult {
    pub response: Response,
    pub events: Vec<Event>,
}

#[deprecated(
    since = "0.2.0",
    note = "Use DebugAdapter directly; DapDispatcher will be removed in a future release"
)]
#[derive(Debug, Clone)]
pub struct DapDispatcher {
    breakpoint_store: BreakpointStore,
    response_seq: Arc<Mutex<i64>>,
    event_seq: Arc<Mutex<i64>>,
    initialized: Arc<Mutex<bool>>,
}

impl DapDispatcher {
    pub fn new() -> Self {
        Self {
            breakpoint_store: BreakpointStore::new(),
            response_seq: Arc::new(Mutex::new(1)),
            event_seq: Arc::new(Mutex::new(1)),
            initialized: Arc::new(Mutex::new(false)),
        }
    }

    pub fn dispatch(&self, request: &Request) -> Response {
        self.dispatch_with_events(request).response
    }

    pub fn dispatch_with_events(&self, request: &Request) -> DispatchResult {
        let result = self.dispatch_inner(request);
        let success = result.is_ok();
        let command = request.command.as_str();
        let response = self.create_response(request, result);

        let events = match (command, success) {
            ("initialize", true) => vec![self.create_initialized_event()],
            _ => Vec::new(),
        };

        DispatchResult { response, events }
    }

    #[cfg(test)]
    pub fn breakpoint_store(&self) -> &BreakpointStore {
        &self.breakpoint_store
    }
}

impl Default for DapDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
