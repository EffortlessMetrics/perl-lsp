//! Request latency instrumentation for first-use LSP hot paths.

use super::*;
use std::time::Instant;

pub(crate) const FIRST_USE_LATENCY_METHODS: &[&str] = &[
    "initialize",
    "textDocument/didOpen",
    "textDocument/didChange",
    "textDocument/completion",
    "textDocument/hover",
    "textDocument/definition",
    "textDocument/references",
    "textDocument/signatureHelp",
    "textDocument/semanticTokens/full",
];

impl LspServer {
    pub(crate) fn record_lsp_request_latency(&self, method: &str, start: Instant) {
        let duration_ms = duration_ms_since(start);
        let index_state = self.request_latency_index_state();
        let first_use_hot_path = FIRST_USE_LATENCY_METHODS.contains(&method);
        tracing::debug!(
            target: "perl_lsp::latency",
            method,
            duration_ms,
            index_state,
            first_use_hot_path,
            "lsp request latency"
        );
    }

    pub(crate) fn request_latency_index_state(&self) -> &'static str {
        #[cfg(feature = "workspace")]
        {
            use perl_parser::workspace_index::IndexState;

            match self.coordinator().map(|coordinator| coordinator.state()) {
                Some(IndexState::Building { .. }) => "building",
                Some(IndexState::Ready { .. }) => "ready",
                Some(IndexState::Degraded { .. }) => "degraded",
                None => "none",
            }
        }

        #[cfg(not(feature = "workspace"))]
        {
            "unavailable"
        }
    }
}

fn duration_ms_since(start: Instant) -> u64 {
    let millis = start.elapsed().as_millis();
    u64::try_from(millis).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_use_latency_methods_cover_issue_contract() {
        assert_eq!(
            FIRST_USE_LATENCY_METHODS,
            [
                "initialize",
                "textDocument/didOpen",
                "textDocument/didChange",
                "textDocument/completion",
                "textDocument/hover",
                "textDocument/definition",
                "textDocument/references",
                "textDocument/signatureHelp",
                "textDocument/semanticTokens/full",
            ]
        );
    }

    #[test]
    fn duration_ms_since_reports_non_negative_milliseconds() {
        let elapsed = duration_ms_since(Instant::now());

        assert!(elapsed < 1_000, "fresh latency sample should stay small");
    }

    #[test]
    fn request_latency_index_state_reports_known_bucket() {
        let server = LspServer::new();
        let index_state = server.request_latency_index_state();

        assert!(
            ["building", "ready", "degraded", "none", "unavailable"].contains(&index_state),
            "unexpected index state bucket `{index_state}`"
        );
    }

    #[cfg(feature = "workspace")]
    #[test]
    fn request_latency_index_state_reports_degraded_bucket()
    -> Result<(), Box<dyn std::error::Error>> {
        use perl_parser::workspace_index::DegradationReason;

        let server = LspServer::new();
        let Some(coordinator) = server.coordinator() else {
            return Err(std::io::Error::other(
                "workspace feature must install an index coordinator",
            )
            .into());
        };
        coordinator.transition_to_degraded(DegradationReason::IoError {
            message: "coverage test".to_string(),
        });

        assert_eq!(server.request_latency_index_state(), "degraded");
        Ok(())
    }

    #[cfg(feature = "workspace")]
    #[test]
    fn request_latency_index_state_reports_none_without_coordinator() {
        let mut server = LspServer::new();
        server.index_coordinator = None;

        assert_eq!(server.request_latency_index_state(), "none");
    }
}
