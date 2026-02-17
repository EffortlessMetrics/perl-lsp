//! Runtime heredoc handler for dynamic evaluation and context tracking
//!
//! This module provides the infrastructure for evaluating heredocs at runtime,
//! tracking variables, and handling nested evaluation contexts.

use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

/// Runtime context for heredoc evaluation
#[derive(Debug, Clone)]
pub struct RuntimeHeredocContext {
    /// Variables available in the current scope
    pub variables: HashMap<String, String>,
    /// Whether interpolation is enabled
    pub interpolation: bool,
    /// Current evaluation depth
    pub eval_depth: usize,
}

impl Default for RuntimeHeredocContext {
    fn default() -> Self {
        Self { variables: HashMap::new(), interpolation: true, eval_depth: 0 }
    }
}

/// Handler for runtime heredoc processing
pub struct RuntimeHeredocHandler {
    /// Maximum allowed evaluation depth
    pub max_eval_depth: usize,
    /// Stack of evaluation contexts
    context_stack: Vec<RuntimeHeredocContext>,
}

impl RuntimeHeredocHandler {
    pub fn new() -> Self {
        Self { max_eval_depth: 10, context_stack: vec![RuntimeHeredocContext::default()] }
    }

    /// Evaluate a heredoc string in the given context
    pub fn evaluate_heredoc(
        &mut self,
        content: &str,
        context: &RuntimeHeredocContext,
    ) -> Result<String, RuntimeError> {
        if context.eval_depth >= self.max_eval_depth {
            return Err(RuntimeError::MaxEvalDepthExceeded);
        }

        // Detect nested heredocs
        #[allow(clippy::unwrap_used)]
        static HEREDOC_REGEX: LazyLock<Regex> =
            LazyLock::new(|| match Regex::new(r#"<<\s*(['"]?)(\w+)(['"]?)"#) {
                Ok(re) => re,
                Err(_) => unreachable!("HEREDOC_REGEX failed to compile"),
            });

        let mut result = content.to_string();
        for cap in HEREDOC_REGEX.captures_iter(content) {
            if let (Some(full_match), Some(delimiter)) = (cap.get(0), cap.get(2)) {
                // If it's a dynamic delimiter, we may need to resolve it
                if delimiter.as_str().starts_with('$') {
                    let evaluated = self.resolve_variable(delimiter.as_str(), context)?;
                    result = result.replace(full_match.as_str(), &format!("<<{}", evaluated));
                }
            }
        }

        // Check for nested heredocs
        if result.contains("<<") {
            result = self.handle_nested_heredocs(&result, context)?;
        }

        Ok(result)
    }

    /// Handle eval string with heredocs
    pub fn eval_with_heredoc(&mut self, eval_content: &str) -> Result<String, RuntimeError> {
        self.push_context()?;

        let result = if eval_content.contains("<<") {
            self.process_eval_heredocs(eval_content)?
        } else {
            eval_content.to_string()
        };

        self.pop_context();
        Ok(result)
    }

    /// Process heredocs in eval content
    fn process_eval_heredocs(&mut self, content: &str) -> Result<String, RuntimeError> {
        // Note: Rust regex doesn't support backreferences, so we'll handle quotes manually
        #[allow(clippy::unwrap_used)]
        static HEREDOC_REGEX: LazyLock<Regex> =
            LazyLock::new(|| match Regex::new(r#"<<\s*(['"]?)(\w+)(['"]?)"#) {
                Ok(re) => re,
                Err(_) => unreachable!("HEREDOC_REGEX failed to compile"),
            });
        let mut processed = content.to_string();
        let mut offset = 0;

        for cap in HEREDOC_REGEX.captures_iter(content) {
            if let (Some(full_match), Some(delimiter)) = (cap.get(0), cap.get(2)) {
                // Check that opening and closing quotes match
                let open_quote = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                let close_quote = cap.get(3).map(|m| m.as_str()).unwrap_or("");
                if open_quote != close_quote {
                    continue; // Skip mismatched quotes
                }

                let match_start = full_match.start() + offset;
                let match_end = full_match.end() + offset;

                // Extract content for this heredoc (simplified)
                if let Some(heredoc_content) =
                    self.extract_heredoc_content(&processed[match_end..], delimiter.as_str())
                {
                    let context = self.current_context()?;
                    let evaluated = Self::evaluate_heredoc_static(
                        &heredoc_content,
                        context,
                        &context.variables,
                    )?;

                    processed.replace_range(match_start..match_end, &evaluated);
                    let diff = evaluated.len() as isize - (match_end - match_start) as isize;
                    offset = (offset as isize + diff) as usize;
                }
            }
        }

        Ok(processed)
    }

    /// Static version of evaluate_heredoc for use in closures
    fn evaluate_heredoc_static(
        content: &str,
        _context: &RuntimeHeredocContext,
        variables: &HashMap<String, String>,
    ) -> Result<String, RuntimeError> {
        let mut result = content.to_string();

        // Perform variable interpolation
        for (name, value) in variables {
            let pattern = format!("${}", name);
            result = result.replace(&pattern, value);
        }

        Ok(result)
    }

    /// Resolve variable value in context
    fn resolve_variable(
        &self,
        name: &str,
        context: &RuntimeHeredocContext,
    ) -> Result<String, RuntimeError> {
        let var_name = name.strip_prefix('$').unwrap_or(name);
        context
            .variables
            .get(var_name)
            .cloned()
            .ok_or_else(|| RuntimeError::HeredocError(format!("Unresolved variable: {}", name)))
    }

    /// Extract heredoc content based on delimiter
    fn extract_heredoc_content(&self, input: &str, terminator: &str) -> Option<String> {
        let mut content = Vec::new();
        let mut found = false;

        for line in input.lines() {
            if line.trim() == terminator {
                found = true;
                break;
            }
            content.push(line);
        }

        if found { Some(content.join("\n")) } else { None }
    }

    /// Handle nested heredoc evaluation
    fn handle_nested_heredocs(
        &mut self,
        content: &str,
        context: &RuntimeHeredocContext,
    ) -> Result<String, RuntimeError> {
        let mut handler = RuntimeHeredocHandler::new();
        handler.max_eval_depth = self.max_eval_depth;
        handler.context_stack = vec![context.clone()];

        handler.push_context()?;
        let result = handler.evaluate_heredoc(content, context);
        handler.pop_context();

        result
    }

    /// Push a new context onto the stack
    fn push_context(&mut self) -> Result<(), RuntimeError> {
        let mut new_context = self.current_context()?.clone();
        new_context.eval_depth += 1;
        self.context_stack.push(new_context);
        Ok(())
    }

    /// Pop context from the stack
    fn pop_context(&mut self) {
        if self.context_stack.len() > 1 {
            self.context_stack.pop();
        }
    }

    /// Get current context
    fn current_context(&self) -> Result<&RuntimeHeredocContext, RuntimeError> {
        self.context_stack.last().ok_or(RuntimeError::ContextStackUnderflow)
    }
}

/// Runtime errors
#[derive(Debug, thiserror::Error, Clone)]
pub enum RuntimeError {
    #[error("Context stack underflow")]
    ContextStackUnderflow,
    #[error("Maximum eval depth exceeded")]
    MaxEvalDepthExceeded,

    #[error("Not in eval context")]
    NotEvalContext,

    #[error("Regex error: {0}")]
    RegexError(String),

    #[error("Heredoc parsing error: {0}")]
    HeredocError(String),
}

impl Default for RuntimeHeredocHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_evaluation() {
        use perl_tdd_support::must;
        let mut handler = RuntimeHeredocHandler::new();
        let mut context = RuntimeHeredocContext::default();
        context.variables.insert("name".to_string(), "World".to_string());

        let content = "Hello, $name!";
        let result = must(handler.evaluate_heredoc(content, &context));
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_max_eval_depth() {
        let mut handler = RuntimeHeredocHandler::new();
        let context = RuntimeHeredocContext { eval_depth: 10, ..Default::default() };

        let result = handler.evaluate_heredoc("test", &context);
        assert!(matches!(result, Err(RuntimeError::MaxEvalDepthExceeded)));
    }

    #[test]
    fn test_substitution_with_heredoc() {
        let _handler = RuntimeHeredocHandler::new();
        let _text = "foo bar foo";
        // Test logic for substitution with heredocs
    }
}
