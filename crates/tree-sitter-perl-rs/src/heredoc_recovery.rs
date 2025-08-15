//! Enhanced heredoc recovery system with lexer integration
//!
//! This module provides sophisticated recovery strategies for heredocs
//! with dynamic delimiters, integrating directly with the Perl lexer.

use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;

use crate::dynamic_delimiter_recovery::{DynamicDelimiterRecovery, ParseContext, RecoveryMode};
use crate::perl_lexer::{Token, TokenType};

/// Enhanced heredoc recovery with multiple strategies
pub struct HeredocRecovery {
    /// Dynamic delimiter recovery engine
    delimiter_recovery: DynamicDelimiterRecovery,
    /// Cache of resolved delimiters
    delimiter_cache: HashMap<String, Arc<str>>,
    /// Pattern matchers for common heredoc constructs
    matchers: HeredocMatchers,
    /// Configuration
    config: RecoveryConfig,
}

#[derive(Debug, Clone)]
pub struct RecoveryConfig {
    pub enable_heuristics: bool,
    pub enable_pattern_matching: bool,
    pub enable_context_analysis: bool,
    pub max_lookahead: usize,
    pub confidence_threshold: f32,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            enable_heuristics: true,
            enable_pattern_matching: true,
            enable_context_analysis: true,
            max_lookahead: 100,
            confidence_threshold: 0.6,
        }
    }
}

/// Collection of regex patterns for heredoc detection
struct HeredocMatchers {
    /// Pattern for <<$var syntax
    dynamic_delimiter: Regex,
    /// Pattern for <<${expr} syntax
    expr_delimiter: Regex,
    /// Pattern for << $var (with space)
    spaced_delimiter: Regex,
    /// Pattern for method calls like <<$obj->method()
    method_delimiter: Regex,
    /// Pattern for concatenated delimiters like <<($var . "END")
    concat_delimiter: Regex,
}

impl Default for HeredocMatchers {
    fn default() -> Self {
        Self {
            dynamic_delimiter: Regex::new(r"<<\$(\w+)").unwrap(),
            expr_delimiter: Regex::new(r"<<\$\{([^}]+)\}").unwrap(),
            spaced_delimiter: Regex::new(r"<<\s+\$(\w+)").unwrap(),
            method_delimiter: Regex::new(r"<<\$(\w+)->(\w+)\(\)").unwrap(),
            concat_delimiter: Regex::new(r"<<\(([^)]+)\)").unwrap(),
        }
    }
}

/// Result of heredoc recovery attempt
#[derive(Debug)]
pub struct RecoveryResult {
    /// The recovered delimiter (if successful)
    pub delimiter: Option<Arc<str>>,
    /// Confidence level (0.0 to 1.0)
    pub confidence: f32,
    /// Recovery method used
    pub method: RecoveryMethod,
    /// Alternative delimiters to try
    pub alternatives: Vec<Arc<str>>,
    /// Diagnostic information
    pub diagnostics: Vec<String>,
    /// Whether to generate an error node
    pub error_node: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryMethod {
    /// Found via static analysis
    StaticAnalysis,
    /// Found via pattern matching
    PatternMatch,
    /// Found via heuristic
    Heuristic,
    /// Found in delimiter cache
    Cached,
    /// Context-based recovery
    ContextAnalysis,
    /// Failed to recover
    Failed,
}

impl HeredocRecovery {
    pub fn new(config: RecoveryConfig) -> Self {
        Self {
            delimiter_recovery: DynamicDelimiterRecovery::new(RecoveryMode::BestGuess),
            delimiter_cache: HashMap::new(),
            matchers: HeredocMatchers::default(),
            config,
        }
    }

    /// Attempt to recover a heredoc with dynamic delimiter
    pub fn recover_dynamic_heredoc(
        &mut self,
        input: &str,
        position: usize,
        tokens: &[Token],
    ) -> RecoveryResult {
        let mut result = RecoveryResult {
            delimiter: None,
            confidence: 0.0,
            method: RecoveryMethod::Failed,
            alternatives: Vec::new(),
            diagnostics: Vec::new(),
            error_node: true,
        };

        // Extract the heredoc expression
        let expr_end = self.find_expression_end(input, position);
        let expression = &input[position..expr_end];

        result.diagnostics.push(format!("Attempting recovery for: {}", expression));

        // Try multiple recovery strategies in order of confidence

        // 1. Check cache first
        if let Some(delimiter) = self.delimiter_cache.get(expression) {
            result.delimiter = Some(delimiter.clone());
            result.confidence = 0.95;
            result.method = RecoveryMethod::Cached;
            result.error_node = false;
            return result;
        }

        // 2. Try static analysis with lookahead
        if self.config.enable_heuristics {
            if let Some((delimiter, confidence)) = self.try_static_analysis(input, position, tokens)
            {
                if confidence >= self.config.confidence_threshold {
                    result.delimiter = Some(delimiter.clone());
                    result.confidence = confidence;
                    result.method = RecoveryMethod::StaticAnalysis;
                    result.error_node = false;
                    self.delimiter_cache.insert(expression.to_string(), delimiter);
                    return result;
                }
            }
        }

        // 3. Try pattern matching
        if self.config.enable_pattern_matching {
            if let Some((delimiter, confidence)) = self.try_pattern_matching(expression) {
                if confidence >= self.config.confidence_threshold {
                    result.delimiter = Some(delimiter.clone());
                    result.confidence = confidence;
                    result.method = RecoveryMethod::PatternMatch;
                    result.error_node = false;

                    // Still collect alternatives even if we found a good match
                    let alternatives = self.apply_heuristics(expression);
                    for alt in alternatives {
                        if alt.as_ref() != delimiter.as_ref()
                            && !result.alternatives.iter().any(|a| a.as_ref() == alt.as_ref())
                        {
                            result.alternatives.push(alt);
                        }
                    }

                    self.delimiter_cache.insert(expression.to_string(), delimiter);
                    return result;
                } else {
                    // Add as alternative
                    result.alternatives.push(delimiter);
                }
            }
        }

        // 4. Try context analysis
        if self.config.enable_context_analysis {
            let context = self.build_context(tokens, position);
            let analysis = self.delimiter_recovery.analyze_dynamic_delimiter(expression, &context);

            if let Some(delim) = analysis.delimiter {
                if analysis.confidence >= self.config.confidence_threshold {
                    let delimiter: Arc<str> = Arc::from(delim);
                    result.delimiter = Some(delimiter.clone());
                    result.confidence = analysis.confidence;
                    result.method = RecoveryMethod::ContextAnalysis;
                    result.error_node = false;
                    self.delimiter_cache.insert(expression.to_string(), delimiter);
                    return result;
                }
            }

            // Add alternatives from analysis
            for alt in analysis.alternatives {
                result.alternatives.push(Arc::from(alt));
            }

            result.diagnostics.extend(analysis.warnings);
        }

        // 5. Try heuristics based on common patterns
        if self.config.enable_heuristics {
            let heuristic_delims = self.apply_heuristics(expression);
            if !heuristic_delims.is_empty() {
                result.delimiter = Some(heuristic_delims[0].clone());

                // Special variables get higher confidence
                let expr = expression.strip_prefix("<<").unwrap_or(expression).trim();
                if expr == "$_" || expr == "$@" || expr == "$!" || expr == "$?" {
                    result.confidence = 0.7; // High enough to succeed
                    result.error_node = false; // We're confident in the recovery
                } else {
                    result.confidence = 0.3;
                }

                result.method = RecoveryMethod::Heuristic;
                result.alternatives.extend(heuristic_delims.into_iter().skip(1));
            }
        }

        result
    }

    /// Find the end of the heredoc expression
    pub fn find_expression_end(&self, input: &str, start: usize) -> usize {
        let bytes = input.as_bytes();
        let mut pos = start;
        let mut paren_depth = 0;
        let mut brace_depth = 0;

        // Skip the << prefix
        if pos + 1 < bytes.len() && bytes[pos] == b'<' && bytes[pos + 1] == b'<' {
            pos += 2;
        }

        // Skip whitespace
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }

        // Now find the end of the expression
        while pos < bytes.len() {
            match bytes[pos] {
                b'(' => paren_depth += 1,
                b')' => {
                    paren_depth -= 1;
                    if paren_depth == 0 && brace_depth == 0 {
                        return pos + 1;
                    }
                }
                b'{' => brace_depth += 1,
                b'}' => {
                    brace_depth -= 1;
                    if paren_depth == 0 && brace_depth == 0 {
                        return pos + 1;
                    }
                }
                b';' | b'\n' if paren_depth == 0 && brace_depth == 0 => return pos,
                _ => {}
            }
            pos += 1;
        }

        pos
    }

    /// Try static analysis by looking for variable assignments
    fn try_static_analysis(
        &mut self,
        input: &str,
        position: usize,
        tokens: &[Token],
    ) -> Option<(Arc<str>, f32)> {
        // Extract variable info from heredoc expression
        let expr_end = self.find_expression_end(input, position);
        let expression = &input[position..expr_end];

        // Check for array element access like $markers[1]
        let array_pattern = Regex::new(r"\$(\w+)\[(\d+)\]").unwrap();
        if let Some(cap) = array_pattern.captures(expression) {
            let var_name = cap.get(1)?.as_str();
            let index: usize = cap.get(2)?.as_str().parse().ok()?;

            // Look for array assignment
            for i in (0..tokens.len()).rev() {
                if let TokenType::Identifier(name) = &tokens[i].token_type {
                    if name.as_ref() == format!("@{}", var_name) {
                        // Found array variable, look for list assignment
                        if i + 2 < tokens.len()
                            && matches!(tokens[i + 1].token_type, TokenType::Operator(ref op) if op.as_ref() == "=")
                        {
                            // Look for the list values
                            let mut list_values = Vec::new();
                            let mut j = i + 2;
                            let mut in_list = false;

                            while j < tokens.len() {
                                match &tokens[j].token_type {
                                    TokenType::LeftParen => in_list = true,
                                    TokenType::RightParen => break,
                                    TokenType::StringLiteral if in_list => {
                                        if let Some(value) =
                                            self.extract_string_literal(&tokens[j].text)
                                        {
                                            list_values.push(value);
                                        }
                                    }
                                    _ => {}
                                }
                                j += 1;
                            }

                            // Return the value at the requested index
                            if index < list_values.len() {
                                return Some((Arc::from(list_values[index].clone()), 0.9));
                            }
                        }
                    }
                }
            }
        }

        // Package-qualified variable pattern like $Package::var
        let pkg_var_pattern = Regex::new(r"\$((?:\w+::)*\w+)").unwrap();
        if let Some(cap) = pkg_var_pattern.captures(expression) {
            let full_var = cap.get(1)?.as_str();
            let var_name = full_var.split("::").last()?;

            // Look for 'our' declaration or direct assignment
            for i in (0..tokens.len()).rev() {
                if let TokenType::Identifier(name) = &tokens[i].token_type {
                    if name.as_ref() == format!("${}", var_name) {
                        // Check if next tokens form an assignment
                        if i + 2 < tokens.len()
                            && matches!(tokens[i + 1].token_type, TokenType::Operator(ref op) if op.as_ref() == "=")
                            && matches!(tokens[i + 2].token_type, TokenType::StringLiteral)
                        {
                            // Extract the string value
                            let text = tokens[i + 2].text.as_ref();
                            if let Some(delimiter) = self.extract_string_literal(text) {
                                return Some((Arc::from(delimiter), 0.9));
                            }
                        }
                    }
                }
            }
        }

        // Brace-delimited variable pattern like ${var} or ${${var}}
        let brace_var_pattern = Regex::new(r"\$\{(.+)\}").unwrap();
        if let Some(cap) = brace_var_pattern.captures(expression) {
            let inner = cap.get(1)?.as_str();

            // Check if it's a nested ${} expression
            if inner.starts_with("${") && inner.ends_with('}') {
                // Extract the innermost variable name
                let innermost = inner.trim_start_matches("${").trim_end_matches('}');
                if let Some(value) = self.resolve_variable_value(innermost, tokens) {
                    return Some((Arc::from(value), 0.9));
                }
            } else {
                // Simple ${var} case
                if let Some(value) = self.resolve_variable_value(inner, tokens) {
                    return Some((Arc::from(value), 0.9));
                }
            }
        }

        // Regular scalar variable pattern
        let var_pattern = Regex::new(r"\$(\w+)").unwrap();
        if let Some(cap) = var_pattern.captures(expression) {
            let var_name = cap.get(1)?.as_str();

            // Look for assignment in previous tokens
            for i in (0..tokens.len()).rev() {
                if let TokenType::Identifier(name) = &tokens[i].token_type {
                    if name.as_ref() == format!("${}", var_name) {
                        // Check if next tokens form an assignment
                        if i + 2 < tokens.len()
                            && matches!(tokens[i + 1].token_type, TokenType::Operator(ref op) if op.as_ref() == "=")
                            && matches!(tokens[i + 2].token_type, TokenType::StringLiteral)
                        {
                            // Extract the string value
                            let text = tokens[i + 2].text.as_ref();
                            if let Some(delimiter) = self.extract_string_literal(text) {
                                return Some((Arc::from(delimiter), 0.9));
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Extract string literal value from quoted string
    fn extract_string_literal(&self, text: &str) -> Option<String> {
        let text = text.trim();
        // Use idiomatic strip_prefix/strip_suffix instead of manual indexing
        text.strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .or_else(|| text.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
            .map(|s| s.to_string())
    }

    /// Resolve a variable value by following assignment chains
    fn resolve_variable_value(&self, var_name: &str, tokens: &[Token]) -> Option<String> {
        let mut current_var = var_name;
        let mut visited = std::collections::HashSet::new();

        // Follow assignment chains up to a reasonable depth
        for _ in 0..5 {
            if visited.contains(current_var) {
                break; // Circular reference
            }
            visited.insert(current_var);

            // Look for assignment of current variable
            for i in (0..tokens.len()).rev() {
                if let TokenType::Identifier(name) = &tokens[i].token_type {
                    if name.as_ref() == format!("${}", current_var) {
                        // Check if next tokens form an assignment
                        if i + 2 < tokens.len()
                            && matches!(tokens[i + 1].token_type, TokenType::Operator(ref op) if op.as_ref() == "=")
                        {
                            match &tokens[i + 2].token_type {
                                TokenType::StringLiteral => {
                                    // Found a string literal value
                                    let text = tokens[i + 2].text.as_ref();
                                    return self.extract_string_literal(text);
                                }
                                TokenType::Identifier(next_var) => {
                                    // Variable assigned to another variable
                                    if let Some(stripped) = next_var.strip_prefix('$') {
                                        current_var = stripped;
                                        break; // Continue with the new variable
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Try pattern matching on the expression
    fn try_pattern_matching(&self, expression: &str) -> Option<(Arc<str>, f32)> {
        // Try different patterns
        if let Some(cap) = self.matchers.dynamic_delimiter.captures(expression) {
            let var_name = cap.get(1)?.as_str();
            // Common delimiter variable names
            if var_name.to_lowercase().contains("delim")
                || var_name.to_lowercase().contains("end")
                || var_name.to_lowercase().contains("eof")
            {
                return Some((Arc::from("EOF"), 0.7));
            }
        }

        if let Some(cap) = self.matchers.method_delimiter.captures(expression) {
            let _obj = cap.get(1)?.as_str();
            let method = cap.get(2)?.as_str();
            // Common pattern: $config->delimiter()
            if method.contains("delim") {
                return Some((Arc::from("END"), 0.6));
            }
        }

        None
    }

    /// Build parse context from tokens
    fn build_context(&self, _tokens: &[Token], _position: usize) -> ParseContext {
        let context = ParseContext {
            current_package: None,
            imported_modules: Vec::new(),
            in_subroutine: None,
            file_type_hint: None,
        };

        // Scan tokens for context clues
        // (simplified - could scan for package/sub keywords if needed)

        context
    }

    /// Apply heuristics to guess common delimiters
    fn apply_heuristics(&self, expression: &str) -> Vec<Arc<str>> {
        let mut delimiters = Vec::new();

        // Most common Perl heredoc delimiters
        let common = ["EOF", "END", "EOT", "EOD", "HERE", "DATA", "TEXT"];

        // Special handling for special variables
        // Strip << prefix if present
        let expr = if expression.starts_with("<<") { &expression[2..].trim() } else { expression };

        if expr == "$_" || expr == "$@" || expr == "$!" || expr == "$?" {
            // For special variables, return common delimiters in priority order
            delimiters.push(Arc::from("EOF"));
            delimiters.push(Arc::from("END"));
            delimiters.push(Arc::from("EOT"));
            delimiters.push(Arc::from("EOD"));
            delimiters.push(Arc::from("DONE"));
            return delimiters;
        }

        // If expression contains hints, prioritize those
        let lower = expression.to_lowercase();

        // Check for specific patterns
        if lower.contains("eof") {
            delimiters.push(Arc::from("EOF"));
            // Also add END as it's commonly confused with EOF
            delimiters.push(Arc::from("END"));
        }
        if lower.contains("end") && !lower.contains("eof") {
            delimiters.push(Arc::from("END"));
            delimiters.push(Arc::from("EOF"));
        }
        if lower.contains("sql") {
            delimiters.push(Arc::from("SQL"));
        }

        // Check other delimiters
        for delim in &common {
            let delim_lower = delim.to_lowercase();
            if delim_lower != "eof" && delim_lower != "end" && lower.contains(&delim_lower) {
                delimiters.push(Arc::from(*delim));
            }
        }

        // Add remaining common delimiters
        for delim in &common {
            if !delimiters.iter().any(|d: &Arc<str>| d.as_ref() == *delim) {
                delimiters.push(Arc::from(*delim));
            }
        }

        delimiters
    }

    /// Generate error token for unrecoverable heredoc
    pub fn generate_error_token(
        &self,
        input: &str,
        start: usize,
        result: &RecoveryResult,
    ) -> Token {
        let end = self.find_expression_end(input, start);
        let text = &input[start..end];

        let mut error_msg = format!("Unresolved dynamic heredoc delimiter: {}", text);
        if !result.diagnostics.is_empty() {
            error_msg.push_str(&format!(" ({})", result.diagnostics.join("; ")));
        }
        if !result.alternatives.is_empty() {
            error_msg.push_str(&format!(
                " - possible delimiters: {}",
                result
                    .alternatives
                    .iter()
                    .map(|d| format!("'{}'", d))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        Token {
            token_type: TokenType::Error(Arc::from(error_msg)),
            text: Arc::from(text),
            start,
            end,
        }
    }

    /// Parse a complete delimiter expression, handling nested structures
    pub fn parse_delimiter_expression(
        &self,
        input: &str,
        start_pos: usize,
    ) -> Option<(String, usize)> {
        let bytes = input.as_bytes();
        let mut pos = start_pos;
        let mut brace_depth = 0;
        let mut bracket_depth = 0;
        let mut paren_depth = 0;
        let mut in_method_call = false;

        // Skip leading sigil if present ($ or @)
        if pos < bytes.len() && (bytes[pos] == b'$' || bytes[pos] == b'@' || bytes[pos] == b'%') {
            pos += 1;

            // Handle special variables like $_ and $@
            if pos < bytes.len() {
                match bytes[pos] {
                    b'_' | b'@' | b'!' | b'?' | b'$' | b'*' | b'#' | b'[' | b']' => {
                        pos += 1;
                        return Some((input[start_pos..pos].to_string(), pos));
                    }
                    _ => {}
                }
            }
        }

        while pos < bytes.len() {
            match bytes[pos] {
                b'{' => brace_depth += 1,
                b'}' => {
                    if brace_depth > 0 {
                        brace_depth -= 1;
                    } else if brace_depth == 0 && bracket_depth == 0 && paren_depth == 0 {
                        break;
                    }
                }
                b'[' => bracket_depth += 1,
                b']' => {
                    if bracket_depth > 0 {
                        bracket_depth -= 1;
                    } else if brace_depth == 0 && bracket_depth == 0 && paren_depth == 0 {
                        break;
                    }
                }
                b'(' => paren_depth += 1,
                b')' => {
                    if paren_depth > 0 {
                        paren_depth -= 1;
                    } else if brace_depth == 0 && bracket_depth == 0 && paren_depth == 0 {
                        break;
                    }
                }
                b'-' if pos + 1 < bytes.len() && bytes[pos + 1] == b'>' => {
                    // Method call arrow
                    in_method_call = true;
                    pos += 1; // Will increment again at loop end
                }
                b':' if pos + 1 < bytes.len() && bytes[pos + 1] == b':' => {
                    // Package separator
                    pos += 1; // Will increment again at loop end
                }
                // Stop at semicolon, newline or whitespace if we're not inside any delimiters
                b';' | b'\n' | b' ' | b'\t'
                    if brace_depth == 0
                        && bracket_depth == 0
                        && paren_depth == 0
                        && !in_method_call =>
                {
                    break;
                }
                // For method calls, reset the flag after the identifier
                b' ' | b';' | b'\n' if in_method_call => {
                    in_method_call = false;
                    if brace_depth == 0 && bracket_depth == 0 && paren_depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            pos += 1;
        }

        if pos > start_pos { Some((input[start_pos..pos].to_string(), pos)) } else { None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expression_end_detection() {
        let recovery = HeredocRecovery::new(RecoveryConfig::default());

        // Simple variable
        let input = "<<$delimiter;";
        let end = recovery.find_expression_end(input, 0);
        assert_eq!(&input[0..end], "<<$delimiter");

        // Expression with braces
        let input = "<<${foo};";
        let end = recovery.find_expression_end(input, 0);
        assert_eq!(&input[0..end], "<<${foo}");

        // Method call
        let input = "<<$obj->method();";
        let end = recovery.find_expression_end(input, 0);
        assert_eq!(&input[0..end], "<<$obj->method()");
    }

    #[test]
    fn test_heuristics() {
        let recovery = HeredocRecovery::new(RecoveryConfig::default());

        let delims = recovery.apply_heuristics("$end_delimiter");
        assert!(!delims.is_empty());
        assert_eq!(delims[0].as_ref(), "END");

        let delims = recovery.apply_heuristics("$eof");
        assert!(!delims.is_empty());
        assert_eq!(delims[0].as_ref(), "EOF");
    }
}
