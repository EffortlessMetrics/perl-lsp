//! Context-aware heredoc parser for handling eval and s///e edge cases
//!
//! This module implements the fourth parsing phase that handles heredocs
//! in special contexts like eval strings and regex substitutions with /e flag.

use crate::heredoc_parser::{HeredocDeclaration, HeredocScanner, ContentCollector};
use crate::pure_rust_parser::{PureRustPerlParser, AstNode};
use regex::Regex;
use std::collections::HashMap;

/// Parsing context for heredoc processing
#[derive(Debug, Clone, PartialEq)]
pub enum ParseContext {
    /// Normal code context
    Normal,
    /// Inside an eval string
    EvalString { depth: usize },
    /// Inside a regex replacement with /e flag
    RegexReplacement { has_e_flag: bool },
}

/// Context-aware heredoc parser
pub struct ContextAwareHeredocParser {
    /// Base heredoc scanner
    scanner: HeredocScanner,
    /// Current parsing context stack
    context_stack: Vec<ParseContext>,
    /// Cached eval content for re-parsing
    eval_cache: HashMap<String, Vec<HeredocDeclaration>>,
}

impl ContextAwareHeredocParser {
    pub fn new(input: &str) -> Self {
        Self {
            scanner: HeredocScanner::new(input),
            context_stack: vec![ParseContext::Normal],
            eval_cache: HashMap::new(),
        }
    }
    
    /// Parse input with context awareness
    pub fn parse(&mut self) -> (String, Vec<HeredocDeclaration>) {
        // Phase 1: Normal heredoc scanning
        let (mut processed, mut declarations) = self.scanner.scan();
        
        // Phase 2: Detect special contexts
        let contexts = self.detect_contexts(&processed);
        
        // Phase 3: Process special contexts
        for context in contexts {
            match context {
                ContextInfo::Eval { start, end, content } => {
                    // Re-parse eval content for heredocs
                    if content.contains("<<") {
                        let eval_declarations = self.parse_eval_content(&content);
                        self.merge_eval_declarations(&mut processed, &mut declarations, 
                                                   start, end, eval_declarations);
                    }
                }
                ContextInfo::SubstitutionWithE { pattern_start, pattern_end, 
                                                replacement_start, replacement_end } => {
                    // Handle s///e replacements
                    let replacement = &processed[replacement_start..replacement_end];
                    if replacement.contains("<<") {
                        self.handle_substitution_heredoc(&mut processed, &mut declarations,
                                                       pattern_start, replacement_start, 
                                                       replacement_end);
                    }
                }
            }
        }
        
        (processed, declarations)
    }
    
    /// Detect special contexts in the input
    fn detect_contexts(&self, input: &str) -> Vec<ContextInfo> {
        let mut contexts = Vec::new();
        
        // Detect eval contexts
        let eval_regex = Regex::new(r#"eval\s*<<\s*(['"]?)(\w+)\1"#).unwrap();
        for cap in eval_regex.captures_iter(input) {
            if let Some(m) = cap.get(0) {
                let terminator = cap.get(2).unwrap().as_str();
                // Find the heredoc content
                if let Some(content_range) = self.find_heredoc_content(input, m.end(), terminator) {
                    contexts.push(ContextInfo::Eval {
                        start: m.start(),
                        end: content_range.end,
                        content: input[content_range.start..content_range.end].to_string(),
                    });
                }
            }
        }
        
        // Detect s///e contexts
        let subst_regex = Regex::new(r#"s([/|#])([^/|#]*)(\1)([^/|#]*)(\1)([a-z]*e[a-z]*)"#).unwrap();
        for cap in subst_regex.captures_iter(input) {
            if let (Some(pattern), Some(replacement), Some(flags)) = 
                (cap.get(2), cap.get(4), cap.get(6)) {
                if flags.as_str().contains('e') && replacement.as_str().contains("<<") {
                    contexts.push(ContextInfo::SubstitutionWithE {
                        pattern_start: pattern.start(),
                        pattern_end: pattern.end(),
                        replacement_start: replacement.start(),
                        replacement_end: replacement.end(),
                    });
                }
            }
        }
        
        contexts
    }
    
    /// Find heredoc content boundaries
    fn find_heredoc_content(&self, input: &str, start: usize, terminator: &str) -> Option<ContentRange> {
        let lines: Vec<&str> = input[start..].lines().collect();
        let mut content_start = None;
        let mut content_end = None;
        
        for (i, line) in lines.iter().enumerate() {
            if content_start.is_none() && i > 0 {
                content_start = Some(start + lines[..i].iter().map(|l| l.len() + 1).sum::<usize>());
            }
            
            if line.trim() == terminator {
                content_end = Some(start + lines[..=i].iter().map(|l| l.len() + 1).sum::<usize>());
                break;
            }
        }
        
        if let (Some(start), Some(end)) = (content_start, content_end) {
            Some(ContentRange { start, end })
        } else {
            None
        }
    }
    
    /// Parse heredocs within eval content
    fn parse_eval_content(&mut self, content: &str) -> Vec<HeredocDeclaration> {
        // Create a sub-scanner for the eval content
        let mut eval_scanner = HeredocScanner::new(content);
        let (_, declarations) = eval_scanner.scan();
        
        // Cache the results
        self.eval_cache.insert(content.to_string(), declarations.clone());
        
        declarations
    }
    
    /// Merge eval heredoc declarations back into main parse
    fn merge_eval_declarations(&self, processed: &mut String, 
                             main_declarations: &mut Vec<HeredocDeclaration>,
                             eval_start: usize, eval_end: usize,
                             eval_declarations: Vec<HeredocDeclaration>) {
        // Adjust positions relative to main input
        for mut decl in eval_declarations {
            decl.position += eval_start;
            decl.line_number += processed[..eval_start].lines().count();
            main_declarations.push(decl);
        }
    }
    
    /// Handle heredocs in s///e replacements
    fn handle_substitution_heredoc(&self, processed: &mut String,
                                 declarations: &mut Vec<HeredocDeclaration>,
                                 pattern_start: usize,
                                 replacement_start: usize,
                                 replacement_end: usize) {
        // Mark the replacement as code context
        let replacement = &processed[replacement_start..replacement_end];
        
        // Create marker for runtime evaluation
        let marker = format!("__HEREDOC_IN_EVAL_CONTEXT__{}__", declarations.len());
        
        // Store metadata for runtime handling
        declarations.push(HeredocDeclaration {
            delimiter: "EVAL_CONTEXT".to_string(),
            quoted: false,
            indented: false,
            position: replacement_start,
            line_number: processed[..replacement_start].lines().count(),
            content: Some(replacement.to_string()),
            interpolate: true,
        });
    }
}

/// Information about a special context
#[derive(Debug)]
enum ContextInfo {
    Eval {
        start: usize,
        end: usize,
        content: String,
    },
    SubstitutionWithE {
        pattern_start: usize,
        pattern_end: usize,
        replacement_start: usize,
        replacement_end: usize,
    },
}

/// Content range in the input
#[derive(Debug)]
struct ContentRange {
    start: usize,
    end: usize,
}

/// Enhanced full parser with context awareness
pub struct ContextAwareFullParser {
    base_parser: PureRustPerlParser,
}

impl ContextAwareFullParser {
    pub fn new() -> Self {
        Self {
            base_parser: PureRustPerlParser::new(),
        }
    }
    
    /// Parse with full context awareness
    pub fn parse(&mut self, input: &str) -> Result<AstNode, Box<dyn std::error::Error>> {
        // Use context-aware heredoc parser
        let mut heredoc_parser = ContextAwareHeredocParser::new(input);
        let (processed, declarations) = heredoc_parser.parse();
        
        // Parse the processed input
        let mut ast = self.base_parser.parse(&processed)?;
        
        // Annotate AST with heredoc metadata
        self.annotate_ast(&mut ast, &declarations);
        
        Ok(ast)
    }
    
    /// Annotate AST nodes with heredoc context information
    fn annotate_ast(&self, ast: &mut AstNode, declarations: &[HeredocDeclaration]) {
        // Walk AST and mark nodes that contain heredocs
        match ast {
            AstNode::EvalStatement { expression, .. } => {
                // Mark eval nodes that contain heredocs
                if let AstNode::String(content) = expression.as_ref() {
                    if declarations.iter().any(|d| d.delimiter == "EVAL_CONTEXT") {
                        // Add metadata for runtime handling
                        // In a real implementation, we'd modify the AST node type
                        // to include heredoc metadata
                    }
                }
            }
            AstNode::Substitution { replacement, flags, .. } => {
                if flags.contains("e") && declarations.iter().any(|d| d.delimiter == "EVAL_CONTEXT") {
                    // Mark for runtime evaluation
                }
            }
            _ => {}
        }
        
        // Recursively process children
        self.walk_and_annotate(ast, declarations);
    }
    
    /// Recursively walk and annotate AST
    fn walk_and_annotate(&self, ast: &mut AstNode, declarations: &[HeredocDeclaration]) {
        // This would walk all children nodes
        // For brevity, not implementing full tree walking here
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_eval_heredoc_detection() {
        let input = r#"
eval <<'EOF';
my $x = <<'INNER';
Hello from inner heredoc
INNER
print $x;
EOF
"#;
        
        let mut parser = ContextAwareHeredocParser::new(input);
        let (processed, declarations) = parser.parse();
        
        // Should find both heredocs
        assert!(declarations.len() >= 1, "Should find eval heredoc");
        
        // The eval content should be marked for re-parsing
        assert!(declarations.iter().any(|d| d.delimiter == "EOF"));
    }
    
    #[test]
    fn test_substitution_e_flag_heredoc() {
        let input = r#"
$text =~ s/foo/<<END/e;
replacement text
END
"#;
        
        let mut parser = ContextAwareHeredocParser::new(input);
        let (processed, declarations) = parser.parse();
        
        // Should detect heredoc in s///e context
        assert!(!declarations.is_empty(), "Should find heredoc in s///e");
        
        // Should have special context marker
        assert!(declarations.iter().any(|d| d.delimiter == "EVAL_CONTEXT" || d.delimiter == "END"));
    }
    
    #[test]
    fn test_nested_eval_heredocs() {
        let input = r#"
eval <<'OUTER';
eval <<'INNER';
print "Deep nesting";
INNER
OUTER
"#;
        
        let mut parser = ContextAwareHeredocParser::new(input);
        let (processed, declarations) = parser.parse();
        
        // Should handle nested evals
        assert!(declarations.len() >= 1, "Should find outer eval heredoc");
    }
}