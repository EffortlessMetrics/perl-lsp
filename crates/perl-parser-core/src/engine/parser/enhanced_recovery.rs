//! Enhanced error recovery mechanisms for the Perl parser
//!
//! This module provides advanced error recovery strategies including:
//! - Context-aware error suggestions
//! - Adaptive recovery based on error patterns
//! - Heuristic recovery for common mistakes
//! - Memory and timeout protection

use crate::{
    ast::Node,
    error::{ParseError, ParseResult},
    token_stream::{Token, TokenKind},
};
use std::time::{Duration, Instant};
use std::collections::HashMap;

/// Configuration for enhanced error recovery
#[derive(Debug, Clone)]
pub struct RecoveryConfig {
    /// Maximum time allowed for parsing (default: 5 seconds)
    pub max_parse_time: Duration,
    /// Maximum number of nodes in AST (default: 100,000)
    pub max_ast_nodes: usize,
    /// Maximum memory usage estimate in bytes (default: 50MB)
    pub max_memory_bytes: usize,
    /// Enable heuristic recovery suggestions
    pub enable_heuristics: bool,
    /// Enable adaptive recovery strategies
    pub enable_adaptive: bool,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            max_parse_time: Duration::from_secs(5),
            max_ast_nodes: 100_000,
            max_memory_bytes: 50 * 1024 * 1024, // 50MB
            enable_heuristics: true,
            enable_adaptive: true,
        }
    }
}

/// Enhanced error recovery state
pub struct EnhancedRecovery {
    config: RecoveryConfig,
    start_time: Instant,
    node_count: usize,
    memory_estimate: usize,
    error_patterns: HashMap<String, Vec<Suggestion>>,
}

/// Error suggestion with context
#[derive(Debug, Clone)]
pub(crate) struct Suggestion {
    /// The suggestion text
    pub message: String,
    /// Confidence level (0.0 to 1.0)
    pub confidence: f32,
    /// The fix to apply (if any)
    pub fix: Option<FixSuggestion>,
}

/// Fix suggestion for automatic recovery
#[derive(Debug, Clone)]
pub(crate) struct FixSuggestion {
    /// Type of fix to apply
    pub fix_type: FixType,
    /// Text to insert
    pub insert_text: String,
    /// Position to insert at
    pub position: usize,
}

/// Types of automatic fixes
#[derive(Debug, Clone)]
pub(crate) enum FixType {
    /// Insert a missing semicolon
    InsertSemicolon,
    /// Insert a missing closing brace
    InsertClosingBrace,
    /// Insert a missing closing parenthesis
    InsertClosingParen,
    /// Insert a missing closing bracket
    InsertClosingBracket,
    /// Replace a token with another
    ReplaceToken,
    /// Insert a missing operator
    InsertOperator,
}

impl EnhancedRecovery {
    /// Create a new enhanced recovery instance
    pub(crate) fn new(config: RecoveryConfig) -> Self {
        Self {
            config,
            start_time: Instant::now(),
            node_count: 0,
            memory_estimate: 0,
            error_patterns: Self::init_error_patterns(),
        }
    }

    /// Initialize error pattern mappings
    fn init_error_patterns() -> HashMap<String, Vec<Suggestion>> {
        let mut patterns = HashMap::new();

        // Missing semicolon patterns
        patterns.insert(
            "statement_without_semicolon".to_string(),
            vec![
                Suggestion {
                    message: "Missing semicolon at end of statement".to_string(),
                    confidence: 0.9,
                    fix: Some(FixSuggestion {
                        fix_type: FixType::InsertSemicolon,
                        insert_text: ";".to_string(),
                        position: 0, // Will be updated with actual position
                    }),
                },
                Suggestion {
                    message: "Statements in Perl must end with a semicolon".to_string(),
                    confidence: 0.7,
                    fix: None,
                },
            ],
        );

        // Unclosed delimiter patterns
        patterns.insert(
            "unclosed_delimiter".to_string(),
            vec![
                Suggestion {
                    message: "Unclosed delimiter - missing closing character".to_string(),
                    confidence: 0.95,
                    fix: None, // Will be determined by delimiter type
                },
                Suggestion {
                    message: "Check for matching opening/closing pairs".to_string(),
                    confidence: 0.6,
                    fix: None,
                },
            ],
        );

        // Unexpected token patterns
        patterns.insert(
            "unexpected_token".to_string(),
            vec![
                Suggestion {
                    message: "Unexpected token - check syntax".to_string(),
                    confidence: 0.5,
                    fix: None,
                },
                Suggestion {
                    message: "May be missing an operator or delimiter".to_string(),
                    confidence: 0.4,
                    fix: None,
                },
            ],
        );

        // Missing expression patterns
        patterns.insert(
            "missing_expression".to_string(),
            vec![
                Suggestion {
                    message: "Missing expression after operator".to_string(),
                    confidence: 0.8,
                    fix: None,
                },
                Suggestion {
                    message: "Incomplete assignment - check right-hand side".to_string(),
                    confidence: 0.7,
                    fix: None,
                },
            ],
        );

        patterns
    }

    /// Check if parsing should continue based on resource limits
    pub(crate) fn should_continue(&self) -> ParseResult<()> {
        // Check time limit
        if self.start_time.elapsed() > self.config.max_parse_time {
            return Err(ParseError::RecursionLimit);
        }

        // Check node count limit
        if self.node_count > self.config.max_ast_nodes {
            return Err(ParseError::NestingTooDeep {
                depth: self.node_count,
                max_depth: self.config.max_ast_nodes,
            });
        }

        // Check memory estimate (rough approximation)
        if self.memory_estimate > self.config.max_memory_bytes {
            return Err(ParseError::NestingTooDeep {
                depth: self.memory_estimate,
                max_depth: self.config.max_memory_bytes,
            });
        }

        Ok(())
    }

    /// Increment node count and update memory estimate
    pub(crate) fn track_node(&mut self) {
        self.node_count += 1;
        // Rough estimate: 200 bytes per node (including children)
        self.memory_estimate += 200;
    }

    /// Get context-aware suggestions for an error
    pub(crate) fn get_suggestions(&self, error_type: &str, context: &ErrorContext) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();

        if self.config.enable_heuristics {
            // Get base suggestions from pattern matching
            if let Some(base_suggestions) = self.error_patterns.get(error_type) {
                suggestions.extend(base_suggestions.clone());
            }

            // Add context-specific suggestions
            suggestions.extend(self.get_context_suggestions(context));
        }

        // Sort by confidence
        suggestions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));

        suggestions
    }

    /// Get context-specific suggestions
    fn get_context_suggestions(&self, context: &ErrorContext) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();

        // Check for common Perl mistakes
        if let Some(token) = &context.current_token {
            match token.kind {
                TokenKind::RightBrace => {
                    suggestions.push(Suggestion {
                        message: "Extra closing brace - may indicate unmatched braces".to_string(),
                        confidence: 0.8,
                        fix: None,
                    });
                }
                TokenKind::RightParen => {
                    suggestions.push(Suggestion {
                        message: "Extra closing parenthesis - check for opening '('".to_string(),
                        confidence: 0.8,
                        fix: None,
                    });
                }
                TokenKind::RightBracket => {
                    suggestions.push(Suggestion {
                        message: "Extra closing bracket - check for opening '['".to_string(),
                        confidence: 0.8,
                        fix: None,
                    });
                }
                _ => {}
            }
        }

        // Check for incomplete constructs
        if context.incomplete_statement {
            suggestions.push(Suggestion {
                message: "Incomplete statement - may be missing semicolon or expression".to_string(),
                confidence: 0.7,
                fix: None,
            });
        }

        suggestions
    }

    /// Apply adaptive recovery strategy
    pub(crate) fn apply_adaptive_recovery(&self, error: &ParseError) -> RecoveryStrategy {
        if !self.config.enable_adaptive {
            return RecoveryStrategy::Default;
        }

        match error {
            ParseError::UnexpectedToken { found, .. } => {
                if found == ";" {
                    RecoveryStrategy::SkipToken
                } else if found.contains("$") || found.contains("@") || found.contains("%") {
                    RecoveryStrategy::TreatAsVariable
                } else {
                    RecoveryStrategy::Default
                }
            }
            ParseError::UnclosedDelimiter { .. } => RecoveryStrategy::InsertClosing,
            ParseError::UnexpectedEof => RecoveryStrategy::InsertMissing,
            _ => RecoveryStrategy::Default,
        }
    }
}

/// Context information for error analysis
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// Current token that caused the error
    pub current_token: Option<Token>,
    /// Previous tokens for context
    pub previous_tokens: Vec<Token>,
    /// Whether we're in an incomplete statement
    pub incomplete_statement: bool,
    /// Current nesting level
    pub nesting_level: usize,
    /// Whether we're in a specific construct (if, while, etc.)
    pub construct_type: Option<String>,
}

/// Recovery strategy based on error analysis
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RecoveryStrategy {
    /// Default recovery behavior
    Default,
    /// Skip the current token and continue
    SkipToken,
    /// Treat as variable declaration
    TreatAsVariable,
    /// Insert missing closing delimiter
    InsertClosing,
    /// Insert missing syntax element
    InsertMissing,
    /// Apply heuristic corrections
    Heuristic,
}

/// Enhanced error recovery implementation
pub trait EnhancedErrorRecovery {
    /// Create enhanced recovery with default config
    fn with_enhanced_recovery() -> Self;
    
    /// Create enhanced recovery with custom config
    fn with_enhanced_recovery_config(config: RecoveryConfig) -> Self;
    
    /// Get current recovery state
    fn recovery_state(&self) -> &EnhancedRecovery;
    
    /// Get mutable recovery state
    fn recovery_state_mut(&mut self) -> &mut EnhancedRecovery;
    
    /// Create enhanced error node with suggestions
    fn create_enhanced_error_node(
        &mut self,
        error: ParseError,
        context: ErrorContext,
    ) -> Node;
    
    /// Apply adaptive recovery based on error
    fn apply_adaptive_recovery(&mut self, error: &ParseError, context: &ErrorContext) -> bool;
}

/// Helper functions for enhanced recovery
pub(crate) mod helpers {
    use super::*;
    
    /// Analyze error context from parser state
    pub(crate) fn analyze_error_context(
        current_token: Option<Token>,
        previous_tokens: Vec<Token>,
        incomplete_statement: bool,
        nesting_level: usize,
    ) -> ErrorContext {
        // Determine construct type from previous tokens
        let construct_type = previous_tokens
            .iter()
            .rev()
            .find(|t| matches!(t.kind, TokenKind::If | TokenKind::While | TokenKind::For | TokenKind::Sub))
            .map(|t| format!("{:?}", t.kind));
        
        ErrorContext {
            current_token,
            previous_tokens,
            incomplete_statement,
            nesting_level,
            construct_type,
        }
    }
    
    /// Generate fix suggestion for unclosed delimiter
    pub(crate) fn generate_delimiter_fix(delimiter: char, position: usize) -> FixSuggestion {
        let closing = match delimiter {
            '(' => ')',
            '[' => ']',
            '{' => '}',
            '<' => '>',
            // For symmetric delimiters, use the same character
            _ => delimiter,
        };
        
        FixSuggestion {
            fix_type: match delimiter {
                '(' => FixType::InsertClosingParen,
                '[' => FixType::InsertClosingBracket,
                '{' => FixType::InsertClosingBrace,
                _ => FixType::InsertOperator,
            },
            insert_text: closing.to_string(),
            position,
        }
    }
}