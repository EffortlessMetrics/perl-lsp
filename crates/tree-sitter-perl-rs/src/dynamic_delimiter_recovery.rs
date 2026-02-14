//! Dynamic heredoc delimiter recovery system
//!
//! This module attempts to resolve heredoc delimiters that are computed
//! at runtime, using various heuristics and recovery strategies.

use regex::Regex;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::LazyLock;

#[derive(Debug, Clone)]
pub struct DynamicDelimiterRecovery {
    /// Known variable assignments in scope
    variable_values: HashMap<String, Vec<PossibleValue>>,
    /// Common delimiter patterns seen in real code
    common_delimiters: Vec<&'static str>,
    /// Recovery mode configuration
    recovery_mode: RecoveryMode,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryMode {
    /// Just mark as unparseable
    Conservative,
    /// Try common patterns and heuristics
    BestGuess,
    /// Prompt user for hint
    Interactive,
    /// Execute in sandbox (requires opt-in)
    Sandbox,
}

#[derive(Debug, Clone)]
pub struct PossibleValue {
    pub value: String,
    pub confidence: f32,
    pub source: ValueSource,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValueSource {
    Literal,        // Direct assignment
    Concatenation,  // String concatenation
    FunctionReturn, // Return value of known function
    UserHint,       // User-provided hint
    Heuristic,      // Guessed from context
}

#[derive(Debug)]
pub struct DelimiterAnalysis {
    pub delimiter: Option<String>,
    pub confidence: f32,
    pub alternatives: Vec<String>,
    pub recovery_strategy: String,
    pub warnings: Vec<String>,
}

// Enhanced patterns for delimiter variables
static SCALAR_ASSIGN_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    match Regex::new(r#"(?m)^\s*(?:my|our|local|state)\s+[\$@%](\w+)\s*=\s*["']([^"']+)["']"#) {
        Ok(re) => re,
        Err(_) => unreachable!("SCALAR_ASSIGN_PATTERN regex failed to compile"),
    }
});

static ARRAY_ASSIGN_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    match Regex::new(r#"(?m)^\s*(?:my|our|local|state)\s+@(\w+)\s*=\s*\(([^)]+)\)"#) {
        Ok(re) => re,
        Err(_) => unreachable!("ARRAY_ASSIGN_PATTERN regex failed to compile"),
    }
});

static HASH_ASSIGN_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    match Regex::new(r#"(?m)^\s*(?:my|our|local|state)\s+%(\w+)\s*=\s*\(([^)]+)\)"#) {
        Ok(re) => re,
        Err(_) => unreachable!("HASH_ASSIGN_PATTERN regex failed to compile"),
    }
});

static COMMON_DELIMITER_NAMES: LazyLock<Vec<&'static str>> =
    LazyLock::new(|| vec!["delimiter", "delim", "end", "eof", "marker", "tag", "label"]);

static STRING_FUNC_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    match Regex::new(r"^(uc|lc|ucfirst|lcfirst|reverse|chomp|chop)\s*\(\s*(.+?)\s*\)$") {
        Ok(re) => re,
        Err(_) => unreachable!("STRING_FUNC_PATTERN regex failed to compile"),
    }
});

static STR_CONV_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    match Regex::new(r"^(.+?)->(?:to_string|as_string|stringify)\(\s*\)$") {
        Ok(re) => re,
        Err(_) => unreachable!("STR_CONV_PATTERN regex failed to compile"),
    }
});

static VAR_INTERP_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    match Regex::new(r"\$\{?([a-zA-Z_]\w*)\}?") {
        Ok(re) => re,
        Err(_) => unreachable!("VAR_INTERP_PATTERN regex failed to compile"),
    }
});

static HASH_PAIR_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    match Regex::new(r#"(\w+)\s*=>\s*["']([^"']+)["']"#) {
        Ok(re) => re,
        Err(_) => unreachable!("HASH_PAIR_PATTERN regex failed to compile"),
    }
});

impl DynamicDelimiterRecovery {
    pub fn new(mode: RecoveryMode) -> Self {
        Self {
            variable_values: HashMap::new(),
            common_delimiters: vec![
                "EOF", "END", "EOT", "EOD", "DONE", "STOP", "HERE", "DATA", "TEXT", "SQL", "HTML",
                "XML", "PERL", "CODE", "SCRIPT", "TEMPLATE",
            ],
            recovery_mode: mode,
        }
    }

    /// Scan code for variable assignments that might be delimiters
    pub fn scan_for_assignments(&mut self, code: &str) {
        // Scan for scalar variable assignments
        for cap in SCALAR_ASSIGN_PATTERN.captures_iter(code) {
            if let (Some(var), Some(val)) = (cap.get(1), cap.get(2)) {
                let var_name = var.as_str();
                let value = val.as_str();

                // Higher confidence if variable name suggests delimiter
                let confidence = if COMMON_DELIMITER_NAMES
                    .iter()
                    .any(|&n| var_name.to_lowercase().contains(n))
                {
                    0.8
                } else {
                    0.5
                };

                self.variable_values.entry(var_name.to_string()).or_default().push(PossibleValue {
                    value: value.to_string(),
                    confidence,
                    source: ValueSource::Literal,
                });
            }
        }

        // Scan for array assignments
        for cap in ARRAY_ASSIGN_PATTERN.captures_iter(code) {
            if let (Some(var), Some(val)) = (cap.get(1), cap.get(2)) {
                let var_name = var.as_str();
                let values_str = val.as_str();

                // Parse array elements (simplified - handles quoted strings)
                let elements: Vec<String> = values_str
                    .split(',')
                    .filter_map(|elem| {
                        let elem = elem.trim();
                        if ((elem.starts_with('"') && elem.ends_with('"'))
                            || (elem.starts_with('\'') && elem.ends_with('\'')))
                            && elem.len() >= 2
                        {
                            Some(elem[1..elem.len() - 1].to_string())
                        } else {
                            None
                        }
                    })
                    .collect();

                // Store first element as the most likely value
                if let Some(first_elem) = elements.first() {
                    let confidence = if COMMON_DELIMITER_NAMES
                        .iter()
                        .any(|&n| var_name.to_lowercase().contains(n))
                    {
                        0.7
                    } else {
                        0.4
                    };

                    self.variable_values.entry(var_name.to_string()).or_default().push(
                        PossibleValue {
                            value: first_elem.clone(),
                            confidence,
                            source: ValueSource::Literal,
                        },
                    );
                }
            }
        }

        // Scan for hash assignments
        for cap in HASH_ASSIGN_PATTERN.captures_iter(code) {
            if let (Some(var), Some(val)) = (cap.get(1), cap.get(2)) {
                let var_name = var.as_str();
                let pairs_str = val.as_str();

                // Parse hash pairs (simplified - handles key => "value" patterns)
                for cap in HASH_PAIR_PATTERN.captures_iter(pairs_str) {
                    if let (Some(_key), Some(val)) = (cap.get(1), cap.get(2)) {
                        let value = val.as_str();

                        let confidence = if COMMON_DELIMITER_NAMES
                            .iter()
                            .any(|&n| var_name.to_lowercase().contains(n))
                        {
                            0.6
                        } else {
                            0.3
                        };

                        self.variable_values.entry(var_name.to_string()).or_default().push(
                            PossibleValue {
                                value: value.to_string(),
                                confidence,
                                source: ValueSource::Literal,
                            },
                        );
                    }
                }
            }
        }
    }

    /// Analyze a dynamic delimiter expression
    pub fn analyze_dynamic_delimiter(
        &self,
        expression: &str,
        context: &ParseContext,
    ) -> DelimiterAnalysis {
        let mut analysis = DelimiterAnalysis {
            delimiter: None,
            confidence: 0.0,
            alternatives: Vec::new(),
            recovery_strategy: String::new(),
            warnings: Vec::new(),
        };

        match self.recovery_mode {
            RecoveryMode::Conservative => {
                analysis.warnings.push(
                    "Dynamic delimiter cannot be resolved without code execution".to_string(),
                );
                analysis.recovery_strategy = "Marked as unparseable".to_string();
            }

            RecoveryMode::BestGuess => {
                // Try various heuristics
                if let Some(delimiter) = self.try_resolve_variable(expression, context) {
                    analysis.delimiter = Some(delimiter.value.clone());
                    analysis.confidence = delimiter.confidence;
                    analysis.recovery_strategy = format!("Resolved via {:?}", delimiter.source);
                } else {
                    // Fall back to common patterns
                    analysis.alternatives = self.guess_common_delimiters(expression);
                    analysis.recovery_strategy = "Guessing from common patterns".to_string();

                    if !analysis.alternatives.is_empty() {
                        analysis.delimiter = Some(analysis.alternatives[0].clone());
                        analysis.confidence = 0.3;
                    }
                }
            }

            RecoveryMode::Interactive => {
                analysis
                    .warnings
                    .push("User input required to resolve dynamic delimiter".to_string());
                analysis.recovery_strategy = "Awaiting user hint".to_string();
                // In real implementation, would trigger UI prompt
            }

            RecoveryMode::Sandbox => {
                analysis
                    .warnings
                    .push("Sandbox execution required to resolve dynamic delimiter".to_string());
                analysis.recovery_strategy = "Requires --enable-sandbox flag".to_string();
            }
        }

        // Add general warnings
        if expression.contains("$") {
            analysis.warnings.push(
                "Variable interpolation in delimiter makes static analysis unreliable".to_string(),
            );
        }

        if expression.contains("(") || expression.contains("{") {
            analysis
                .warnings
                .push("Complex expression in delimiter requires runtime evaluation".to_string());
        }

        analysis
    }

    /// Try to resolve a variable to its value with enhanced pattern support
    fn try_resolve_variable(&self, expr: &str, _context: &ParseContext) -> Option<PossibleValue> {
        let expr = expr.trim();

        // Simple variable like $delimiter
        if let Some(var_name) = expr.strip_prefix('$')
            && !expr.contains('.')
            && !expr.contains('{')
            && !expr.contains('[')
            && !expr.contains('(')
            && let Some(values) = self.variable_values.get(var_name)
        {
            return values
                .iter()
                .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(Ordering::Equal))
                .cloned();
        }

        // Braced variable ${var} or ${var[index]}
        if expr.starts_with("${") && expr.ends_with('}') {
            let inner = &expr[2..expr.len() - 1];

            // Simple braced variable ${var}
            if !inner.contains('[')
                && !inner.contains('{')
                && let Some(values) = self.variable_values.get(inner)
            {
                return values
                    .iter()
                    .max_by(|a, b| {
                        a.confidence.partial_cmp(&b.confidence).unwrap_or(Ordering::Equal)
                    })
                    .cloned();
            }

            // Array/hash subscript ${var[0]} or ${var{key}}
            if let Some(base_var) = self.extract_subscript_base(inner)
                && let Some(values) = self.variable_values.get(&base_var)
            {
                // For subscripts, return lower confidence since we can't resolve the index
                let mut best_value = values
                    .iter()
                    .max_by(|a, b| {
                        a.confidence.partial_cmp(&b.confidence).unwrap_or(Ordering::Equal)
                    })?
                    .clone();
                best_value.confidence *= 0.6; // Reduce confidence for subscripts
                best_value.source = ValueSource::Heuristic;
                return Some(best_value);
            }
        }

        // Array access like $arr[0]
        if expr.contains('[')
            && expr.contains(']')
            && let Some(base_var) = self.extract_array_base(expr)
            && let Some(values) = self.variable_values.get(&base_var)
        {
            let mut best_value = values
                .iter()
                .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(Ordering::Equal))?
                .clone();
            best_value.confidence *= 0.5; // Lower confidence for array access
            best_value.source = ValueSource::Heuristic;
            return Some(best_value);
        }

        // Hash access like $hash{key}
        if expr.contains('{')
            && expr.contains('}')
            && !expr.starts_with("${")
            && let Some(base_var) = self.extract_hash_base(expr)
            && let Some(values) = self.variable_values.get(&base_var)
        {
            let mut best_value = values
                .iter()
                .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(Ordering::Equal))?
                .clone();
            best_value.confidence *= 0.5; // Lower confidence for hash access
            best_value.source = ValueSource::Heuristic;
            return Some(best_value);
        }

        // Function calls like uc($var) or lc($delimiter)
        if let Some(resolved_func) = self.try_resolve_string_function(expr) {
            return Some(resolved_func);
        }

        // Method calls like $obj->method() or $class->new()
        if expr.contains("->")
            && let Some(resolved_method) = self.try_resolve_method_call(expr)
        {
            return Some(resolved_method);
        }

        // Concatenation: $var . "END" (supports multiple parts and new operators)
        if expr.contains('.') || expr.contains('x') {
            return self.try_resolve_concatenation(expr);
        }

        // Interpolated strings like "$var_suffix" or "${base}_postfix"
        if (expr.starts_with('"') && expr.ends_with('"'))
            || (expr.starts_with('`') && expr.ends_with('`'))
        {
            return self.try_resolve_interpolated_string(expr);
        }

        // Environment variables like $ENV{PATH}
        if expr.starts_with("$ENV{") && expr.ends_with('}') {
            return self.try_resolve_env_variable(expr);
        }

        None
    }

    /// Extract base variable name from subscript expressions like 'var[0]' or 'var{key}'
    fn extract_subscript_base(&self, expr: &str) -> Option<String> {
        if let Some(bracket_pos) = expr.find('[') {
            Some(expr[..bracket_pos].to_string())
        } else {
            expr.find('{').map(|brace_pos| expr[..brace_pos].to_string())
        }
    }

    /// Extract base variable name from array access like '$arr[0]'
    fn extract_array_base(&self, expr: &str) -> Option<String> {
        if let Some(var_name) = expr.strip_prefix('$')
            && let Some(bracket_pos) = var_name.find('[')
        {
            return Some(format!("@{}", &var_name[..bracket_pos]));
        }
        None
    }

    /// Extract base variable name from hash access like '$hash{key}'
    fn extract_hash_base(&self, expr: &str) -> Option<String> {
        if let Some(var_name) = expr.strip_prefix('$')
            && let Some(brace_pos) = var_name.find('{')
        {
            return Some(format!("%{}", &var_name[..brace_pos]));
        }
        None
    }

    /// Try to resolve function calls like uc($var), lc($delimiter), etc.
    fn try_resolve_string_function(&self, expr: &str) -> Option<PossibleValue> {
        if let Some(cap) = STRING_FUNC_PATTERN.captures(expr) {
            let func_name = cap.get(1)?.as_str();
            let arg_expr = cap.get(2)?.as_str();

            // Recursively resolve the argument
            if let Some(arg_value) = self.try_resolve_variable(
                arg_expr,
                &ParseContext {
                    current_package: None,
                    imported_modules: vec![],
                    in_subroutine: None,
                    file_type_hint: None,
                },
            ) {
                let transformed_value = match func_name {
                    "uc" => arg_value.value.to_uppercase(),
                    "lc" => arg_value.value.to_lowercase(),
                    "ucfirst" => {
                        let mut chars = arg_value.value.chars();
                        match chars.next() {
                            None => String::new(),
                            Some(first) => {
                                first.to_uppercase().collect::<String>() + chars.as_str()
                            }
                        }
                    }
                    "lcfirst" => {
                        let mut chars = arg_value.value.chars();
                        match chars.next() {
                            None => String::new(),
                            Some(first) => {
                                first.to_lowercase().collect::<String>() + chars.as_str()
                            }
                        }
                    }
                    "reverse" => arg_value.value.chars().rev().collect(),
                    "chomp" | "chop" => {
                        // These modify in-place, but for delimiters we'll return original
                        arg_value.value
                    }
                    _ => arg_value.value,
                };

                return Some(PossibleValue {
                    value: transformed_value,
                    confidence: arg_value.confidence * 0.8, // Slightly lower for function calls
                    source: ValueSource::FunctionReturn,
                });
            }
        }

        None
    }

    /// Try to resolve method calls (limited scope for delimiter resolution)
    fn try_resolve_method_call(&self, expr: &str) -> Option<PossibleValue> {
        // For now, just handle simple cases like $obj->to_string()
        if let Some(captures) = STR_CONV_PATTERN.captures(expr) {
            let obj_expr = captures.get(1)?.as_str();

            // Try to resolve the object
            if let Some(obj_value) = self.try_resolve_variable(
                obj_expr,
                &ParseContext {
                    current_package: None,
                    imported_modules: vec![],
                    in_subroutine: None,
                    file_type_hint: None,
                },
            ) {
                return Some(PossibleValue {
                    value: obj_value.value,
                    confidence: obj_value.confidence * 0.6,
                    source: ValueSource::FunctionReturn,
                });
            }
        }

        None
    }

    /// Enhanced concatenation resolution supporting x operator and complex expressions
    fn try_resolve_concatenation(&self, expr: &str) -> Option<PossibleValue> {
        let mut resolved = String::new();
        let mut confidences = Vec::new();

        // Split on both . and x operators
        let parts: Vec<&str> = if expr.contains(" x ") {
            // Handle string repetition operator
            let mut result = Vec::new();
            for part in expr.split('.') {
                if part.trim().contains(" x ") {
                    result.extend(part.split(" x "));
                } else {
                    result.push(part);
                }
            }
            result
        } else {
            expr.split('.').collect()
        };

        for (i, part) in parts.iter().enumerate() {
            let part = part.trim();
            if part.is_empty() {
                return None;
            }

            // Handle string repetition like "AB" x 3
            if i > 0
                && part.chars().all(|c| c.is_ascii_digit())
                && let Ok(repeat_count) = part.parse::<usize>()
                && repeat_count <= 100
                && repeat_count > 0
            {
                // Sanity limit
                // Get the last resolved part to repeat
                if !resolved.is_empty() {
                    let last_part = resolved.clone();
                    resolved.clear();
                    resolved.push_str(&last_part.repeat(repeat_count));
                    confidences.push(0.7);
                    continue;
                }
            }

            if ((part.starts_with('"') && part.ends_with('"'))
                || (part.starts_with('\'') && part.ends_with('\'')))
                && part.len() >= 2
            {
                resolved.push_str(&part[1..part.len() - 1]);
                confidences.push(1.0);
            } else if part.starts_with("${") && part.ends_with('}') {
                let var_name = &part[2..part.len() - 1];
                if let Some(values) = self.variable_values.get(var_name) {
                    let val = values
                        .iter()
                        .max_by(|a, b| {
                            a.confidence.partial_cmp(&b.confidence).unwrap_or(Ordering::Equal)
                        })?
                        .clone();
                    resolved.push_str(&val.value);
                    confidences.push(val.confidence);
                } else {
                    return None;
                }
            } else if let Some(var_name) = part.strip_prefix('$') {
                if let Some(values) = self.variable_values.get(var_name) {
                    let val = values
                        .iter()
                        .max_by(|a, b| {
                            a.confidence.partial_cmp(&b.confidence).unwrap_or(Ordering::Equal)
                        })?
                        .clone();
                    resolved.push_str(&val.value);
                    confidences.push(val.confidence);
                } else {
                    return None;
                }
            } else if part.chars().all(|c| c.is_ascii_digit()) {
                // Handle numeric literals in concatenation
                resolved.push_str(part);
                confidences.push(1.0);
            } else {
                return None;
            }
        }

        if !resolved.is_empty() {
            let confidence = confidences.iter().fold(1.0_f32, |acc, &c| acc.min(c));
            return Some(PossibleValue {
                value: resolved,
                confidence,
                source: ValueSource::Concatenation,
            });
        }

        None
    }

    /// Try to resolve interpolated strings like "$var_suffix" or "${base}_postfix"
    fn try_resolve_interpolated_string(&self, expr: &str) -> Option<PossibleValue> {
        let _quote_char = expr.chars().next()?;
        let content = &expr[1..expr.len() - 1];

        // Simple case: just a variable
        if content.starts_with('$') && !content[1..].contains('$') {
            return self.try_resolve_variable(
                content,
                &ParseContext {
                    current_package: None,
                    imported_modules: vec![],
                    in_subroutine: None,
                    file_type_hint: None,
                },
            );
        }

        // Complex interpolation - try to resolve all variables
        let mut resolved = content.to_string();
        let mut total_confidence: f32 = 1.0;
        let mut any_resolved = false;

        for captures in VAR_INTERP_PATTERN.captures_iter(content) {
            if let Some(var_match) = captures.get(0) {
                let var_name = captures.get(1)?.as_str();

                if let Some(values) = self.variable_values.get(var_name) {
                    let val = values
                        .iter()
                        .max_by(|a, b| {
                            a.confidence.partial_cmp(&b.confidence).unwrap_or(Ordering::Equal)
                        })?
                        .clone();

                    resolved = resolved.replace(var_match.as_str(), &val.value);
                    total_confidence = total_confidence.min(val.confidence);
                    any_resolved = true;
                }
            }
        }

        if any_resolved {
            Some(PossibleValue {
                value: resolved,
                confidence: total_confidence * 0.8, // Lower confidence for interpolation
                source: ValueSource::Concatenation,
            })
        } else {
            None
        }
    }

    /// Try to resolve environment variables like $ENV{PATH}
    fn try_resolve_env_variable(&self, expr: &str) -> Option<PossibleValue> {
        if let Some(env_name) = expr.strip_prefix("$ENV{").and_then(|s| s.strip_suffix('}')) {
            // Remove quotes if present
            let env_name = env_name.trim_matches('"').trim_matches('\'');

            // For security, only resolve common safe environment variables
            match env_name {
                "HOME" | "USER" | "PATH" | "PWD" | "SHELL" | "TERM" | "LANG" | "LC_ALL" => {
                    if let Ok(value) = std::env::var(env_name) {
                        return Some(PossibleValue {
                            value,
                            confidence: 0.9,
                            source: ValueSource::Literal,
                        });
                    }
                }
                _ => {
                    // For other env vars, return a heuristic based on name
                    let heuristic_value = match env_name {
                        name if name.to_lowercase().contains("path") => {
                            "/usr/local/bin".to_string()
                        }
                        name if name.to_lowercase().contains("url") => {
                            "http://localhost".to_string()
                        }
                        name if name.to_lowercase().contains("port") => "8080".to_string(),
                        name if name.to_lowercase().contains("host") => "localhost".to_string(),
                        name if name.to_lowercase().contains("debug") => "1".to_string(),
                        _ => "value".to_string(),
                    };

                    return Some(PossibleValue {
                        value: heuristic_value,
                        confidence: 0.3,
                        source: ValueSource::Heuristic,
                    });
                }
            }
        }

        None
    }

    /// Guess common delimiters based on context
    fn guess_common_delimiters(&self, expression: &str) -> Vec<String> {
        let mut guesses = Vec::new();

        // If variable name contains hints
        let lower = expression.to_lowercase();
        if lower.contains("sql") {
            guesses.push("SQL".to_string());
        }
        if lower.contains("end") || lower.contains("eof") {
            guesses.push("EOF".to_string());
            guesses.push("END".to_string());
        }

        // Add general common delimiters
        guesses.extend(self.common_delimiters[..5].iter().map(|&s| s.to_string()));

        guesses
    }

    /// Add a user-provided hint
    pub fn add_user_hint(&mut self, var_name: &str, value: &str) {
        self.variable_values.entry(var_name.to_string()).or_default().push(PossibleValue {
            value: value.to_string(),
            confidence: 0.9, // High confidence for user hints
            source: ValueSource::UserHint,
        });
    }
}

/// Context information for delimiter resolution
#[derive(Debug)]
pub struct ParseContext {
    pub current_package: Option<String>,
    pub imported_modules: Vec<String>,
    pub in_subroutine: Option<String>,
    pub file_type_hint: Option<FileType>,
}

#[derive(Debug)]
pub enum FileType {
    Script,
    Module,
    Test,
    Documentation,
}

/// Integration with the main parser
pub struct DynamicHeredocNode {
    pub expression: String,
    pub analysis: DelimiterAnalysis,
    pub recovery_attempted: bool,
}

impl DynamicHeredocNode {
    pub fn to_diagnostic_message(&self) -> String {
        format!(
            "Dynamic heredoc delimiter '{}' {}. {}",
            self.expression,
            if let Some(ref delim) = self.analysis.delimiter {
                format!(
                    "resolved to '{}' (confidence: {:.0}%)",
                    delim,
                    self.analysis.confidence * 100.0
                )
            } else {
                "could not be resolved".to_string()
            },
            self.analysis.recovery_strategy
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_assignment_detection() {
        let mut recovery = DynamicDelimiterRecovery::new(RecoveryMode::BestGuess);
        let code = r#"
my $delimiter = "EOF";
my $text = <<$delimiter;
Content here
EOF
"#;

        recovery.scan_for_assignments(code);
        assert!(recovery.variable_values.contains_key("delimiter"));

        let values = &recovery.variable_values["delimiter"];
        assert_eq!(values[0].value, "EOF");
        assert!(values[0].confidence > 0.7);
    }

    #[test]
    fn test_enhanced_assignment_detection() {
        let mut recovery = DynamicDelimiterRecovery::new(RecoveryMode::BestGuess);
        let code = r#"
our $global_delim = "GLOBAL_END";
local $temp_marker = "TEMP";
state $persistent_tag = "PERSIST";
my @delimiters = ("EOF", "END", "STOP");
my %markers = (sql => "SQL", perl => "PERL");
"#;

        recovery.scan_for_assignments(code);

        // Test scalar assignments with different declarators
        assert!(recovery.variable_values.contains_key("global_delim"));
        assert!(recovery.variable_values.contains_key("temp_marker"));
        assert!(recovery.variable_values.contains_key("persistent_tag"));

        let global_values = &recovery.variable_values["global_delim"];
        assert_eq!(global_values[0].value, "GLOBAL_END");

        // Test array assignments
        assert!(recovery.variable_values.contains_key("delimiters"));
        let array_values = &recovery.variable_values["delimiters"];
        assert_eq!(array_values[0].value, "EOF"); // First element

        // Test hash assignments
        assert!(recovery.variable_values.contains_key("markers"));
        let hash_values = &recovery.variable_values["markers"];
        assert_eq!(hash_values[0].value, "SQL"); // First value found
    }

    #[test]
    fn test_common_delimiter_guessing() {
        let recovery = DynamicDelimiterRecovery::new(RecoveryMode::BestGuess);
        let guesses = recovery.guess_common_delimiters("$end_marker");

        assert!(guesses.contains(&"EOF".to_string()));
        assert!(guesses.contains(&"END".to_string()));
    }

    #[test]
    fn test_analysis_modes() {
        let recovery = DynamicDelimiterRecovery::new(RecoveryMode::Conservative);
        let context = ParseContext {
            current_package: None,
            imported_modules: vec![],
            in_subroutine: None,
            file_type_hint: None,
        };

        let analysis = recovery.analyze_dynamic_delimiter("$foo", &context);
        assert!(analysis.delimiter.is_none());
        assert!(!analysis.warnings.is_empty());
    }

    #[test]
    fn test_braced_variable_resolution() {
        use perl_tdd_support::must_some;
        let mut recovery = DynamicDelimiterRecovery::new(RecoveryMode::BestGuess);
        let code = r#"my $marker = "END";"#;
        recovery.scan_for_assignments(code);
        let context = ParseContext {
            current_package: None,
            imported_modules: vec![],
            in_subroutine: None,
            file_type_hint: None,
        };

        let result = must_some(recovery.try_resolve_variable("${marker}", &context));
        assert_eq!(result.value, "END");
    }

    #[test]
    fn test_concatenation_resolution() {
        use perl_tdd_support::must_some;
        let mut recovery = DynamicDelimiterRecovery::new(RecoveryMode::BestGuess);
        let code = r#"my $base = "ST";"#;
        recovery.scan_for_assignments(code);
        let context = ParseContext {
            current_package: None,
            imported_modules: vec![],
            in_subroutine: None,
            file_type_hint: None,
        };

        let result =
            must_some(recovery.try_resolve_variable("$base . \"ART\"", &context));
        assert_eq!(result.value, "START");
        assert_eq!(result.source, ValueSource::Concatenation);
    }

    #[test]
    fn test_function_call_resolution() {
        use perl_tdd_support::must_some;
        let mut recovery = DynamicDelimiterRecovery::new(RecoveryMode::BestGuess);
        let code = r#"my $delimiter = "end";"#;
        recovery.scan_for_assignments(code);
        let context = ParseContext {
            current_package: None,
            imported_modules: vec![],
            in_subroutine: None,
            file_type_hint: None,
        };

        // Test uppercase function
        let result = must_some(recovery
            .try_resolve_variable("uc($delimiter)", &context));
        assert_eq!(result.value, "END");
        assert_eq!(result.source, ValueSource::FunctionReturn);

        // Test lowercase function
        recovery.variable_values.clear();
        recovery.scan_for_assignments(r#"my $delimiter = "END";"#);

        let result = must_some(recovery
            .try_resolve_variable("lc($delimiter)", &context));
        assert_eq!(result.value, "end");
        assert_eq!(result.source, ValueSource::FunctionReturn);
    }

    #[test]
    fn test_array_hash_access_resolution() {
        let mut recovery = DynamicDelimiterRecovery::new(RecoveryMode::BestGuess);
        let code = r#"
my @delimiters = ("EOF", "END", "STOP");
my %markers = (sql => "SQL", perl => "PERL");
"#;
        recovery.scan_for_assignments(code);

        // Simulate array values
        recovery.variable_values.insert(
            "delimiters".to_string(),
            vec![PossibleValue {
                value: "EOF".to_string(),
                confidence: 0.8,
                source: ValueSource::Literal,
            }],
        );

        recovery.variable_values.insert(
            "markers".to_string(),
            vec![PossibleValue {
                value: "SQL".to_string(),
                confidence: 0.8,
                source: ValueSource::Literal,
            }],
        );

        let context = ParseContext {
            current_package: None,
            imported_modules: vec![],
            in_subroutine: None,
            file_type_hint: None,
        };

        // Test array access
        let result = must_some(recovery
            .try_resolve_variable("$delimiters[0]", &context));
        assert_eq!(result.value, "EOF");
        assert_eq!(result.source, ValueSource::Heuristic);

        // Test hash access
        let result = must_some(recovery
            .try_resolve_variable("$markers{sql}", &context));
        assert_eq!(result.value, "SQL");
        assert_eq!(result.source, ValueSource::Heuristic);
    }

    #[test]
    fn test_interpolated_string_resolution() {
        let mut recovery = DynamicDelimiterRecovery::new(RecoveryMode::BestGuess);
        let code = r#"
my $prefix = "MY";
my $suffix = "END";
"#;
        recovery.scan_for_assignments(code);
        let context = ParseContext {
            current_package: None,
            imported_modules: vec![],
            in_subroutine: None,
            file_type_hint: None,
        };

        // Test simple interpolation
        let result = must_some(recovery
            .try_resolve_variable("\"${prefix}_${suffix}\"", &context));
        assert_eq!(result.value, "MY_END");
        assert_eq!(result.source, ValueSource::Concatenation);
    }

    #[test]
    fn test_environment_variable_resolution() {
        let recovery = DynamicDelimiterRecovery::new(RecoveryMode::BestGuess);
        let context = ParseContext {
            current_package: None,
            imported_modules: vec![],
            in_subroutine: None,
            file_type_hint: None,
        };

        // Test environment variable access
        if let Some(result) = recovery.try_resolve_variable("$ENV{HOME}", &context) {
            assert_eq!(result.source, ValueSource::Literal);
            assert!(result.confidence > 0.8);
        }

        // Test heuristic for unknown env vars
        let result = must_some(recovery
            .try_resolve_variable("$ENV{CUSTOM_DEBUG_FLAG}", &context));
        assert_eq!(result.value, "1"); // Debug heuristic
        assert_eq!(result.source, ValueSource::Heuristic);
        assert!(result.confidence < 0.5);
    }

    #[test]
    fn test_complex_expression_patterns() {
        let mut recovery = DynamicDelimiterRecovery::new(RecoveryMode::BestGuess);
        let code = r#"
my $type = "SQL";
my $counter = "1";
"#;
        recovery.scan_for_assignments(code);
        let context = ParseContext {
            current_package: None,
            imported_modules: vec![],
            in_subroutine: None,
            file_type_hint: None,
        };

        // Test numeric concatenation
        let result = must_some(recovery
            .try_resolve_variable("$type . $counter", &context));
        assert_eq!(result.value, "SQL1");
        assert_eq!(result.source, ValueSource::Concatenation);

        // Test complex braced subscript
        recovery.variable_values.insert(
            "config".to_string(),
            vec![PossibleValue {
                value: "DELIMITER".to_string(),
                confidence: 0.8,
                source: ValueSource::Literal,
            }],
        );

        let result = must_some(recovery
            .try_resolve_variable("${config[0]}", &context));
        assert_eq!(result.value, "DELIMITER");
        assert_eq!(result.source, ValueSource::Heuristic);
        assert!(result.confidence < 0.8); // Reduced confidence for subscripts
    }
}
