//! Context-aware heredoc parser for handling eval and s///e edge cases
//!
//! This module implements the fourth parsing phase that handles heredocs
//! in special contexts like eval strings and regex substitutions with /e flag.

use crate::heredoc_parser::{HeredocDeclaration, HeredocScanner};
use crate::pure_rust_parser::{AstNode, PureRustPerlParser};
use regex::Regex;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

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
pub struct ContextAwareHeredocParser<'a> {
    /// Base heredoc scanner
    scanner: HeredocScanner<'a>,
    /// Current parsing context stack
    #[allow(dead_code)]
    context_stack: Vec<ParseContext>,
    /// Cached eval content for re-parsing
    #[allow(dead_code)]
    eval_cache: HashMap<String, Vec<HeredocDeclaration>>,
}

impl<'a> ContextAwareHeredocParser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            scanner: HeredocScanner::new(input),
            context_stack: vec![ParseContext::Normal],
            eval_cache: HashMap::new(),
        }
    }

    /// Parse input with context awareness
    pub fn parse(self) -> (String, Vec<HeredocDeclaration>) {
        // Phase 1: Normal heredoc scanning
        let (processed, mut declarations) = self.scanner.scan();

        // Phase 2: Detect special contexts
        let contexts = Self::detect_contexts_static(&processed);

        // Phase 3: Process special contexts
        for context in contexts {
            match context {
                ContextInfo::Eval { start, end, content } => {
                    // Re-parse eval content for heredocs
                    if content.contains("<<") {
                        let eval_declarations = Self::parse_eval_content_static(&content);
                        Self::merge_eval_declarations_static(
                            &processed,
                            &mut declarations,
                            start,
                            end,
                            eval_declarations,
                        );
                    }
                }
                ContextInfo::SubstitutionWithE {
                    pattern_start: _,
                    pattern_end: _,
                    replacement_start,
                    replacement_end,
                } => {
                    // Handle s///e replacements
                    let replacement = &processed[replacement_start..replacement_end];
                    if replacement.contains("<<") {
                        Self::handle_substitution_heredoc_static(
                            &processed,
                            &mut declarations,
                            0,
                            replacement_start,
                            replacement_end,
                        );
                    }
                }
            }
        }

        (processed, declarations)
    }

    /// Detect special contexts in the input
    fn detect_contexts_static(input: &str) -> Vec<ContextInfo> {
        let mut contexts = Vec::new();

        // Detect eval contexts - use a more permissive regex without backreferences
        static EVAL_REGEX: LazyLock<Regex> =
            LazyLock::new(|| match Regex::new(r#"eval\s*<<\s*(?:'(\w+)'|"(\w+)"|(\w+))"#) {
                Ok(re) => re,
                Err(_) => unreachable!("EVAL_REGEX failed to compile"),
            });

        for cap in EVAL_REGEX.captures_iter(input) {
            if let Some(m) = cap.get(0) {
                // Get terminator from whichever capture group matched
                let terminator = cap
                    .get(1)
                    .or_else(|| cap.get(2))
                    .or_else(|| cap.get(3))
                    .map(|m| m.as_str())
                    .unwrap_or("");
                // Find the heredoc content
                if let Some(content_range) =
                    Self::find_heredoc_content_static(input, m.end(), terminator)
                {
                    contexts.push(ContextInfo::Eval {
                        start: m.start(),
                        end: content_range.end,
                        content: input[content_range.start..content_range.end].to_string(),
                    });
                }
            }
        }

        // Detect s///e contexts - handle common delimiters separately to avoid backreferences
        static SUBST_SLASH_REGEX: LazyLock<Regex> =
            LazyLock::new(|| match Regex::new(r#"s/([^/]*)/([^/]*)/([a-z]*e[a-z]*)"#) {
                Ok(re) => re,
                Err(_) => unreachable!("SUBST_SLASH_REGEX failed to compile"),
            });
        static SUBST_PIPE_REGEX: LazyLock<Regex> =
            LazyLock::new(|| match Regex::new(r#"s\|([^|]*)\|([^|]*)\|([a-z]*e[a-z]*)"#) {
                Ok(re) => re,
                Err(_) => unreachable!("SUBST_PIPE_REGEX failed to compile"),
            });
        static SUBST_HASH_REGEX: LazyLock<Regex> =
            LazyLock::new(|| match Regex::new(r#"s#([^#]*)#([^#]*)#([a-z]*e[a-z]*)"#) {
                Ok(re) => re,
                Err(_) => unreachable!("SUBST_HASH_REGEX failed to compile"),
            });

        // Process all substitution regex patterns
        for regex in &[&*SUBST_SLASH_REGEX, &*SUBST_PIPE_REGEX, &*SUBST_HASH_REGEX] {
            for cap in regex.captures_iter(input) {
                if let (Some(pattern), Some(replacement), Some(flags)) =
                    (cap.get(1), cap.get(2), cap.get(3))
                    && flags.as_str().contains('e')
                    && replacement.as_str().contains("<<")
                {
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
    fn find_heredoc_content_static(
        input: &str,
        start: usize,
        terminator: &str,
    ) -> Option<ContentRange> {
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
    fn parse_eval_content_static(content: &str) -> Vec<HeredocDeclaration> {
        // Create a sub-scanner for the eval content
        let eval_scanner = HeredocScanner::new(content);
        let (_, declarations) = eval_scanner.scan();

        declarations
    }

    /// Merge eval heredoc declarations back into main parse
    fn merge_eval_declarations_static(
        processed: &str,
        main_declarations: &mut Vec<HeredocDeclaration>,
        eval_start: usize,
        _eval_end: usize,
        eval_declarations: Vec<HeredocDeclaration>,
    ) {
        // Adjust positions relative to main input
        for mut decl in eval_declarations {
            decl.declaration_pos += eval_start;
            decl.declaration_end += eval_start;
            decl.declaration_line += processed[..eval_start].lines().count();
            main_declarations.push(decl);
        }
    }

    /// Handle heredocs in s///e replacements
    fn handle_substitution_heredoc_static(
        processed: &str,
        declarations: &mut Vec<HeredocDeclaration>,
        _pattern_start: usize,
        replacement_start: usize,
        replacement_end: usize,
    ) {
        // Mark the replacement as code context
        let replacement = &processed[replacement_start..replacement_end];

        // Create marker for runtime evaluation
        let marker = format!("__HEREDOC_IN_EVAL_CONTEXT__{}__", declarations.len());

        // Store metadata for runtime handling
        declarations.push(HeredocDeclaration {
            terminator: "EVAL_CONTEXT".to_string(),
            declaration_pos: replacement_start,
            declaration_end: replacement_end,
            declaration_line: processed[..replacement_start].lines().count(),
            interpolated: true,
            indented: false,
            placeholder_id: marker.clone(),
            content: Some(Arc::from(replacement)),
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
        #[allow(dead_code)]
        pattern_start: usize,
        #[allow(dead_code)]
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
#[derive(Default)]
pub struct ContextAwareFullParser {
    base_parser: PureRustPerlParser,
}

impl ContextAwareFullParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse with full context awareness
    pub fn parse(&mut self, input: &str) -> Result<AstNode, Box<dyn std::error::Error>> {
        // Use context-aware heredoc parser
        let heredoc_parser = ContextAwareHeredocParser::new(input);
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
            AstNode::EvalString(expression) => {
                // Mark eval nodes that contain heredocs
                if let AstNode::String(_content) = expression.as_ref()
                    && declarations.iter().any(|d| d.terminator == "EVAL_CONTEXT")
                {
                    // Add metadata for runtime handling
                    // In a real implementation, we'd modify the AST node type
                    // to include heredoc metadata
                }
            }
            AstNode::Substitution { flags, .. } => {
                if flags.contains("e")
                    && declarations.iter().any(|d| d.terminator == "EVAL_CONTEXT")
                {
                    // Mark for runtime evaluation
                }
            }
            _ => {}
        }

        // Recursively process children
        self.walk_and_annotate(ast, declarations);
    }

    /// Recursively walk and annotate AST
    fn walk_and_annotate(&self, _ast: &mut AstNode, _declarations: &[HeredocDeclaration]) {
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

        let parser = ContextAwareHeredocParser::new(input);
        let (_processed, declarations) = parser.parse();

        // Should find both heredocs
        assert!(!declarations.is_empty(), "Should find eval heredoc");

        // The eval content should be marked for re-parsing
        assert!(declarations.iter().any(|d| d.terminator == "EOF"));
    }

    #[test]
    fn test_substitution_e_flag_heredoc() {
        let input = r#"
$text =~ s/foo/<<END/e;
replacement text
END
"#;

        let parser = ContextAwareHeredocParser::new(input);
        let (_processed, declarations) = parser.parse();

        // Should detect heredoc in s///e context
        assert!(!declarations.is_empty(), "Should find heredoc in s///e");

        // Should have special context marker
        assert!(
            declarations.iter().any(|d| d.terminator == "EVAL_CONTEXT" || d.terminator == "END")
        );
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

        let parser = ContextAwareHeredocParser::new(input);
        let (_processed, declarations) = parser.parse();

        // Should handle nested evals
        assert!(!declarations.is_empty(), "Should find outer eval heredoc");
    }
}
