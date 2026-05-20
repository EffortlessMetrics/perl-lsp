/// Stored data breakpoint record for watchpoint management
#[derive(Debug, Clone)]
pub(super) struct DataBreakpointRecord {
    // Retained for future watchpoint lifecycle correlation.
    #[allow(dead_code)]
    pub(super) data_id: String,
    // Retained to preserve the full DAP data-breakpoint request contract.
    #[allow(dead_code)]
    pub(super) access_type: Option<String>,
    // Retained to preserve conditional-watchpoint configuration.
    #[allow(dead_code)]
    pub(super) condition: Option<String>,
}
