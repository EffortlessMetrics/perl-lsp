use crate::breakpoints::BreakpointStore;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub(crate) struct DispatcherState {
    pub(crate) breakpoint_store: BreakpointStore,
    pub(crate) response_seq: Arc<Mutex<i64>>,
    pub(crate) event_seq: Arc<Mutex<i64>>,
    pub(crate) initialized: Arc<Mutex<bool>>,
}

impl DispatcherState {
    pub(crate) fn new() -> Self {
        Self {
            breakpoint_store: BreakpointStore::new(),
            response_seq: Arc::new(Mutex::new(1)),
            event_seq: Arc::new(Mutex::new(1)),
            initialized: Arc::new(Mutex::new(false)),
        }
    }
}
