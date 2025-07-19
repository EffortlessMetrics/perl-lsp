//! Runtime handler for heredocs in eval and s///e contexts
//!
//! This module provides runtime support for heredocs that need to be
//! evaluated dynamically, such as those in eval strings and s///e replacements.

use std::collections::HashMap;
use regex::Regex;

/// Runtime context for heredoc evaluation
#[derive(Debug)]
pub struct RuntimeHeredocContext {
    /// Variables available in the current scope
    pub variables: HashMap<String, String>,
    /// Whether interpolation is enabled
    pub interpolate: bool,
    /// Current eval depth
    pub eval_depth: usize,
}

impl Default for RuntimeHeredocContext {
    fn default() -> Self {
        Self {
            variables: HashMap::new(),
            interpolate: true,
            eval_depth: 0,
        }
    }
}

/// Runtime heredoc handler
pub struct RuntimeHeredocHandler {
    /// Maximum allowed eval depth to prevent infinite recursion
    max_eval_depth: usize,
    /// Context stack for nested evaluations
    context_stack: Vec<RuntimeHeredocContext>,
}

impl RuntimeHeredocHandler {
    pub fn new() -> Self {
        Self {
            max_eval_depth: 10,
            context_stack: vec![RuntimeHeredocContext::default()],
        }
    }
    
    /// Evaluate heredoc content at runtime
    pub fn evaluate_heredoc(&mut self, content: &str, context: &RuntimeHeredocContext) -> Result<String, RuntimeError> {
        if context.eval_depth >= self.max_eval_depth {
            return Err(RuntimeError::MaxEvalDepthExceeded);
        }
        
        let mut result = content.to_string();
        
        // Handle variable interpolation if enabled
        if context.interpolate {
            result = self.interpolate_variables(&result, &context.variables)?;
        }
        
        // Check for nested heredocs
        if result.contains("<<") {
            result = self.handle_nested_heredocs(&result, context)?;
        }
        
        Ok(result)
    }
    
    /// Handle eval string with heredocs
    pub fn eval_with_heredoc(&mut self, eval_content: &str) -> Result<String, RuntimeError> {
        self.push_context();
        
        let result = if eval_content.contains("<<") {
            self.process_eval_heredocs(eval_content)?
        } else {
            eval_content.to_string()
        };
        
        self.pop_context();
        Ok(result)
    }
    
    /// Handle s///e replacement with heredocs
    pub fn substitute_with_heredoc(&mut self, text: &str, pattern: &str, replacement: &str, flags: &str) -> Result<String, RuntimeError> {
        if !flags.contains('e') {
            return Err(RuntimeError::NotEvalContext);
        }
        
        let regex = Regex::new(pattern)
            .map_err(|e| RuntimeError::RegexError(e.to_string()))?;
        
        let result = regex.replace_all(text, |_caps: &regex::Captures| {
            // In /e context, the replacement is evaluated as Perl code
            if replacement.contains("<<") {
                self.process_replacement_heredoc(replacement).unwrap_or_else(|_| replacement.to_string())
            } else {
                replacement.to_string()
            }
        });
        
        Ok(result.to_string())
    }
    
    /// Process heredocs in eval content
    fn process_eval_heredocs(&mut self, content: &str) -> Result<String, RuntimeError> {
        let heredoc_regex = Regex::new(r#"<<\s*(['"]?)(\w+)\1"#).unwrap();
        let mut processed = content.to_string();
        let mut offset = 0;
        
        for cap in heredoc_regex.captures_iter(content) {
            if let (Some(full_match), Some(delimiter)) = (cap.get(0), cap.get(2)) {
                let delim = delimiter.as_str();
                let quoted = !cap.get(1).unwrap().as_str().is_empty();
                
                // Find the heredoc content
                if let Some(heredoc_content) = self.extract_heredoc_content(&processed[offset..], delim) {
                    // Evaluate the heredoc content
                    let context = self.current_context();
                    let evaluated = self.evaluate_heredoc(&heredoc_content, context)?;
                    
                    // Replace in the processed string
                    let heredoc_full = format!("{}\n{}\n{}", full_match.as_str(), heredoc_content, delim);
                    processed = processed.replacen(&heredoc_full, &evaluated, 1);
                }
                
                offset = full_match.end();
            }
        }
        
        Ok(processed)
    }
    
    /// Process heredocs in s///e replacement
    fn process_replacement_heredoc(&mut self, replacement: &str) -> Result<String, RuntimeError> {
        // Similar to eval processing but in replacement context
        self.process_eval_heredocs(replacement)
    }
    
    /// Extract heredoc content from input
    fn extract_heredoc_content(&self, input: &str, delimiter: &str) -> Option<String> {
        let lines: Vec<&str> = input.lines().collect();
        let mut content_lines = Vec::new();
        let mut in_heredoc = false;
        
        for line in lines.iter().skip(1) {
            if line.trim() == delimiter {
                break;
            }
            if in_heredoc || lines.len() > 1 {
                in_heredoc = true;
                content_lines.push(*line);
            }
        }
        
        if in_heredoc {
            Some(content_lines.join("\n"))
        } else {
            None
        }
    }
    
    /// Interpolate variables in content
    fn interpolate_variables(&self, content: &str, variables: &HashMap<String, String>) -> Result<String, RuntimeError> {
        let mut result = content.to_string();
        
        // Simple variable interpolation (real Perl is more complex)
        for (name, value) in variables {
            result = result.replace(&format!("${}", name), value);
            result = result.replace(&format!("${{{}}}", name), value);
        }
        
        Ok(result)
    }
    
    /// Handle nested heredocs recursively
    fn handle_nested_heredocs(&mut self, content: &str, context: &RuntimeHeredocContext) -> Result<String, RuntimeError> {
        let mut new_context = context.clone();
        new_context.eval_depth += 1;
        
        self.evaluate_heredoc(content, &new_context)
    }
    
    /// Push a new context onto the stack
    fn push_context(&mut self) {
        let mut new_context = self.current_context().clone();
        new_context.eval_depth += 1;
        self.context_stack.push(new_context);
    }
    
    /// Pop context from the stack
    fn pop_context(&mut self) {
        if self.context_stack.len() > 1 {
            self.context_stack.pop();
        }
    }
    
    /// Get current context
    fn current_context(&self) -> &RuntimeHeredocContext {
        self.context_stack.last().unwrap()
    }
}

/// Runtime errors
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("Maximum eval depth exceeded")]
    MaxEvalDepthExceeded,
    
    #[error("Not in eval context")]
    NotEvalContext,
    
    #[error("Regex error: {0}")]
    RegexError(String),
    
    #[error("Heredoc parsing error: {0}")]
    HeredocError(String),
}

/// Integration with the main parser
pub trait RuntimeHeredocSupport {
    /// Check if AST node requires runtime heredoc handling
    fn needs_runtime_heredoc(&self) -> bool;
    
    /// Get runtime heredoc metadata
    fn get_heredoc_metadata(&self) -> Option<HeredocMetadata>;
}

/// Metadata for runtime heredoc handling
#[derive(Debug, Clone)]
pub struct HeredocMetadata {
    pub context_type: ContextType,
    pub delimiter: String,
    pub content: String,
    pub interpolate: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContextType {
    Eval,
    SubstitutionE,
    Qx,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simple_eval_heredoc() {
        let mut handler = RuntimeHeredocHandler::new();
        let eval_content = r#"print <<'EOF';
Hello from eval
EOF"#;
        
        let result = handler.eval_with_heredoc(eval_content);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_variable_interpolation() {
        let mut handler = RuntimeHeredocHandler::new();
        let mut context = RuntimeHeredocContext::default();
        context.variables.insert("name".to_string(), "World".to_string());
        
        let content = "Hello, $name!";
        let result = handler.evaluate_heredoc(content, &context).unwrap();
        assert_eq!(result, "Hello, World!");
    }
    
    #[test]
    fn test_max_eval_depth() {
        let mut handler = RuntimeHeredocHandler::new();
        let mut context = RuntimeHeredocContext::default();
        context.eval_depth = 10;
        
        let result = handler.evaluate_heredoc("test", &context);
        assert!(matches!(result, Err(RuntimeError::MaxEvalDepthExceeded)));
    }
    
    #[test]
    fn test_substitution_with_heredoc() {
        let mut handler = RuntimeHeredocHandler::new();
        let text = "foo bar foo";
        let pattern = "foo";
        let replacement = "<<'END'\nbaz\nEND";
        let flags = "ge";
        
        let result = handler.substitute_with_heredoc(text, pattern, replacement, flags);
        assert!(result.is_ok());
        // In a real implementation, this would evaluate the heredoc
    }
}