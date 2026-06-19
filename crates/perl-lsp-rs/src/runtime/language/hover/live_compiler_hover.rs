use super::*;
use perl_lsp_rs_core::providers::navigation::hover_shadow::{
    HoverCutoverOutcome, HoverCutoverResult, hover_cutover,
};
use perl_semantic_facts::ProviderFactSourceKind;

#[derive(Debug, Clone)]
pub(super) struct LiveHoverCompilerContext {
    uri: String,
    symbol: String,
    byte_offset: u32,
}

impl LspServer {
    pub(super) fn live_hover_compiler_context(
        uri: &str,
        text: &str,
        offset: usize,
    ) -> Option<LiveHoverCompilerContext> {
        let symbol = Self::get_token_at_position_static(text, offset);
        if symbol.is_empty() {
            return None;
        }

        let byte_offset = u32::try_from(offset).ok()?;
        Some(LiveHoverCompilerContext { uri: uri.to_string(), symbol, byte_offset })
    }

    pub(super) fn try_live_compiler_hover(
        &self,
        legacy_value: Option<&Value>,
        context: Option<&LiveHoverCompilerContext>,
    ) -> Option<Value> {
        let context = context?;
        let legacy_text = legacy_value.and_then(Self::hover_value_markdown);

        #[cfg(not(all(feature = "workspace", not(target_arch = "wasm32"))))]
        {
            let _ = (legacy_text, context);
            None
        }

        #[cfg(all(feature = "workspace", not(target_arch = "wasm32")))]
        {
            let workspace_index = self.workspace_index()?;
            let outcome = workspace_index.with_semantic_queries_for_uri(
                &context.uri,
                |file_id, queries| {
                    hover_cutover(
                        legacy_text.clone(),
                        &queries,
                        &context.symbol,
                        file_id,
                        context.byte_offset,
                        None,
                    )
                },
            )?;

            if !Self::hover_outcome_uses_live_compiler_facts(&outcome) {
                return None;
            }

            match outcome.result {
                HoverCutoverResult::Exact(explanation)
                | HoverCutoverResult::Ambiguous(explanation)
                | HoverCutoverResult::DynamicBoundary(explanation) => {
                    if Self::should_preserve_legacy_hover(
                        legacy_text.as_deref(),
                        &explanation.markdown,
                    ) {
                        return None;
                    }
                    Some(Self::hover_markdown_value(explanation.markdown))
                }
                HoverCutoverResult::LegacyFallback(_) => None,
            }
        }
    }

    fn should_preserve_legacy_hover(legacy_text: Option<&str>, compiler_markdown: &str) -> bool {
        let Some(legacy_text) = legacy_text else {
            return false;
        };

        legacy_text.contains("**Moo/Moose Attribute Accessor**")
            && !compiler_markdown.contains("Moo/Moose Attribute Accessor")
    }

    fn hover_outcome_uses_live_compiler_facts(outcome: &HoverCutoverOutcome) -> bool {
        outcome.receipt.fact_source_traces.iter().any(|trace| {
            matches!(
                trace.source,
                ProviderFactSourceKind::CompilerFact
                    | ProviderFactSourceKind::FrameworkAdapter
                    | ProviderFactSourceKind::DynamicBoundary
            )
        })
    }

    fn hover_value_markdown(value: &Value) -> Option<String> {
        let contents = value.get("contents")?;
        if let Some(markdown) = contents.as_str() {
            return Some(markdown.to_string());
        }
        contents.get("value").and_then(Value::as_str).map(str::to_string)
    }

    fn hover_markdown_value(markdown: String) -> Value {
        json!({
            "contents": {
                "kind": "markdown",
                "value": markdown,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_richer_moo_accessor_hover_over_generated_compiler_card()
    -> Result<(), Box<dyn std::error::Error>> {
        let legacy = "**Moo/Moose Attribute Accessor**\n\n**Attribute**: `email`\n**Type**: `Str`";
        let compiler =
            "**Symbol** `email` (generated)\n\nSource: framework adapter / framework synthesis";

        if !LspServer::should_preserve_legacy_hover(Some(legacy), compiler) {
            return Err("expected rich Moo/Moose accessor hover to be preserved".into());
        }

        Ok(())
    }

    #[test]
    fn uses_compiler_hover_when_it_keeps_accessor_attribution()
    -> Result<(), Box<dyn std::error::Error>> {
        let legacy = "**Moo/Moose Attribute Accessor**\n\n**Attribute**: `email`";
        let compiler = "**Moo/Moose Attribute Accessor**\n\n**Attribute**: `email`";

        if LspServer::should_preserve_legacy_hover(Some(legacy), compiler) {
            return Err("expected compiler hover with accessor attribution to be usable".into());
        }

        Ok(())
    }
}
