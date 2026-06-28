#![warn(missing_docs)]
//! Folding range extraction for LSP textDocument/foldingRange
//!
//! This module provides folding range extraction from the Perl AST,
//! allowing editors to collapse/expand code sections.

use perl_lexer::{PerlLexer, TokenType};
use perl_parser_core::ast::{Node, NodeKind, SourceLocation};

/// Extracts folding ranges from a Perl AST
pub struct FoldingRangeExtractor {
    /// Accumulated folding ranges during extraction
    ranges: Vec<FoldingRange>,
}

/// Represents a foldable region in the code for LSP folding range support.
///
/// Maps to LSP `FoldingRange` with byte offset coordinates for precise
/// editor integration. Supports different fold types (comments, imports, regions)
/// with optimal editor experience.
///
/// # Performance Characteristics
/// - Memory footprint: 24 bytes per range (optimized for large files)
/// - Range calculation: <1μs per fold region
/// - LSP serialization: Direct mapping to protocol types
#[derive(Debug, Clone)]
pub struct FoldingRange {
    /// Starting byte offset of the foldable region
    pub start_offset: usize, // Changed from start_line to start_offset
    /// Ending byte offset of the foldable region
    pub end_offset: usize, // Changed from end_line to end_offset
    /// Type of folding region for editor-specific handling
    pub kind: Option<FoldingRangeKind>,
}

/// Classification of foldable regions for optimal editor experience.
///
/// Maps directly to LSP `FoldingRangeKind` enum with Perl-specific
/// semantics for different code constructs.
///
/// # LSP Integration
/// - `Comment`: Multi-line comments and POD documentation
/// - `Imports`: `use` and `require` statement blocks
/// - `Region`: Code blocks, subroutines, packages
#[derive(Debug, Clone)]
pub enum FoldingRangeKind {
    /// Multi-line comments and POD documentation
    Comment,
    /// Use and require statement blocks
    Imports,
    /// Code blocks, subroutines, and packages
    Region,
}

impl Default for FoldingRangeExtractor {
    fn default() -> Self {
        Self::new()
    }
}

/// Folding range provider (alias for `FoldingRangeExtractor`).
///
/// Exposes the same API as `FoldingRangeExtractor` under the conventional
/// `FoldingRangeProvider` name expected by consumers.
pub type FoldingRangeProvider = FoldingRangeExtractor;

impl FoldingRangeExtractor {
    /// Create a new folding range extractor
    pub fn new() -> Self {
        Self { ranges: Vec::new() }
    }

    /// Extract all folding ranges from the AST
    pub fn extract(&mut self, ast: &Node) -> Vec<FoldingRange> {
        self.ranges.clear();
        self.visit_node(ast);
        self.ranges.clone()
    }

    /// Extract heredoc folding ranges from source text using the lexer.
    ///
    /// Scans the source for heredoc bodies and returns their ranges.
    pub fn extract_heredoc_ranges(text: &str) -> Vec<FoldingRange> {
        let mut ranges = Vec::new();
        let mut lexer = PerlLexer::with_body_tokens(text);

        while let Some(token) = lexer.next_token() {
            if matches!(token.token_type, TokenType::HeredocBody(_)) {
                ranges.push(FoldingRange {
                    start_offset: token.start,
                    end_offset: token.end,
                    kind: Some(FoldingRangeKind::Region),
                });
            }

            // Stop at EOF
            if matches!(token.token_type, TokenType::EOF) {
                break;
            }
        }

        ranges
    }

    /// Extract #region/#endregion folding ranges from source text.
    ///
    /// Scans for lines matching `^\s*#\s*region\b` and `^\s*#\s*endregion\b`,
    /// matching them by nesting depth to handle nested regions correctly.
    /// Unmatched markers are ignored (no fold generated).
    pub fn extract_region_markers(text: &str) -> Vec<FoldingRange> {
        let mut ranges = Vec::new();
        let mut stack: Vec<(usize, usize)> = Vec::new(); // Stack of (start_line_byte_offset, depth)
        let mut depth = 0usize;
        let mut current_offset = 0usize;

        for line in text.lines() {
            let line_start_offset = current_offset;
            let line_end_offset = current_offset + line.len();
            let trimmed = line.trim_start();

            // Check for #region marker
            if trimmed.starts_with('#') {
                let after_hash = trimmed.strip_prefix('#').unwrap_or("").trim_start();
                if after_hash.starts_with("region") {
                    // Verify it's a word boundary (not part of another word)
                    let after_region = after_hash.strip_prefix("region").unwrap_or("");
                    if after_region.is_empty()
                        || !after_region.chars().next().unwrap_or(' ').is_alphanumeric()
                    {
                        stack.push((line_start_offset, depth));
                        depth += 1;
                    }
                }
            }

            // Check for #endregion marker
            if trimmed.starts_with('#') {
                let after_hash = trimmed.strip_prefix('#').unwrap_or("").trim_start();
                if after_hash.starts_with("endregion") {
                    // Verify it's a word boundary
                    let after_endregion = after_hash.strip_prefix("endregion").unwrap_or("");
                    if after_endregion.is_empty()
                        || !after_endregion.chars().next().unwrap_or(' ').is_alphanumeric()
                    {
                        if depth > 0 {
                            depth -= 1;
                            if let Some((start_offset, _)) = stack.pop() {
                                ranges.push(FoldingRange {
                                    start_offset,
                                    end_offset: line_end_offset,
                                    kind: Some(FoldingRangeKind::Region),
                                });
                            }
                        }
                    }
                }
            }

            // Move to next line (account for newline character)
            current_offset = line_end_offset + 1; // +1 for the newline
        }

        ranges
    }

    /// Visit a node and extract folding ranges
    fn visit_node(&mut self, node: &Node) {
        match &node.kind {
            NodeKind::Program { statements } => {
                // Group consecutive use/require statements
                let mut import_start: Option<usize> = None;
                let mut import_end: Option<usize> = None;

                for (i, stmt) in statements.iter().enumerate() {
                    match &stmt.kind {
                        NodeKind::Use { .. } | NodeKind::No { .. } => {
                            if import_start.is_none() {
                                import_start = Some(i);
                            }
                            import_end = Some(i);
                        }
                        _ => {
                            // End of import block
                            if let (Some(start_idx), Some(end_idx)) = (import_start, import_end) {
                                if end_idx > start_idx {
                                    // Multiple imports - create folding range
                                    let start_loc = &statements[start_idx].location;
                                    let end_loc = &statements[end_idx].location;
                                    self.add_range_from_locations(
                                        start_loc,
                                        end_loc,
                                        Some(FoldingRangeKind::Imports),
                                    );
                                }
                            }
                            import_start = None;
                            import_end = None;
                        }
                    }

                    // Visit each statement
                    self.visit_node(stmt);
                }

                // Handle trailing imports
                if let (Some(start_idx), Some(end_idx)) = (import_start, import_end) {
                    if end_idx > start_idx {
                        let start_loc = &statements[start_idx].location;
                        let end_loc = &statements[end_idx].location;
                        self.add_range_from_locations(
                            start_loc,
                            end_loc,
                            Some(FoldingRangeKind::Imports),
                        );
                    }
                }
            }

            NodeKind::Package { name: _, block, name_span: _ } => {
                // Package with block is foldable
                if let Some(block_node) = block {
                    self.add_range_from_node(node, None);
                    self.visit_node(block_node);
                } else {
                    // Even packages without explicit blocks could be foldable
                    // if they span multiple lines (e.g., package Foo; ... package Bar;)
                    self.add_range_from_node(node, None);
                }
            }

            NodeKind::Subroutine { name: _, prototype: _, signature: _, body, .. }
            | NodeKind::Method { name: _, signature: _, body, .. } => {
                // Subroutines and methods are foldable
                self.add_range_from_node(node, None);
                self.visit_node(body);
            }

            NodeKind::Block { statements } => {
                // Blocks are foldable if they contain statements
                if !statements.is_empty() {
                    self.add_range_from_node(node, None);
                }
                for stmt in statements {
                    self.visit_node(stmt);
                }
            }

            NodeKind::If { condition: _, then_branch, elsif_branches, else_branch, .. } => {
                // If statements with blocks are foldable
                self.add_range_from_node(node, None);
                self.visit_node(then_branch);
                for (_, branch) in elsif_branches {
                    self.visit_node(branch);
                }
                if let Some(else_br) = else_branch {
                    self.visit_node(else_br);
                }
            }

            NodeKind::While { condition: _, body, continue_block, .. } => {
                self.add_range_from_node(node, None);
                self.visit_node(body);
                if let Some(cont) = continue_block {
                    self.visit_node(cont);
                }
            }

            NodeKind::For { init: _, condition: _, update: _, body, continue_block: _ }
            | NodeKind::Foreach { variable: _, list: _, body, continue_block: _ } => {
                self.add_range_from_node(node, None);
                self.visit_node(body);
            }

            NodeKind::Do { block } | NodeKind::Eval { block } | NodeKind::Defer { block } => {
                self.add_range_from_node(node, None);
                self.visit_node(block);
            }

            NodeKind::Try { body, catch_blocks, finally_block } => {
                self.add_range_from_node(node, None);
                self.visit_node(body);
                for (_, catch_block) in catch_blocks {
                    self.visit_node(catch_block);
                }
                if let Some(finally) = finally_block {
                    self.visit_node(finally);
                }
            }

            NodeKind::Given { expr: _, body } => {
                self.add_range_from_node(node, None);
                self.visit_node(body);
            }

            NodeKind::PhaseBlock { phase: _, phase_span: _, block } => {
                // BEGIN, END, CHECK, INIT blocks
                self.add_range_from_node(node, None);
                self.visit_node(block);
            }

            NodeKind::Class { body, .. } => {
                self.add_range_from_node(node, None);
                self.visit_node(body);
            }

            // POD is typically inside strings or special constructs, not a separate NodeKind
            NodeKind::Heredoc { .. } => {
                // Heredocs are always foldable as regions
                self.add_range_from_node(node, Some(FoldingRangeKind::Region));
            }

            NodeKind::StatementModifier { statement, modifier: _, condition } => {
                self.visit_node(statement);
                self.visit_node(condition);
            }

            NodeKind::ArrayLiteral { elements } => {
                // Arrays are foldable if they have elements
                // (They'll be filtered out later if too small)
                if !elements.is_empty() {
                    self.add_range_from_node(node, None);
                }
                for elem in elements {
                    self.visit_node(elem);
                }
            }

            NodeKind::HashLiteral { pairs } => {
                // Hashes with elements are foldable
                if !pairs.is_empty() {
                    self.add_range_from_node(node, None);
                }
                for (key, value) in pairs {
                    self.visit_node(key);
                    self.visit_node(value);
                }
            }

            // ArrayRef and HashRef don't exist as separate NodeKinds, they're handled via references
            NodeKind::VariableDeclaration { initializer: Some(init), .. } => {
                self.visit_node(init);
            }

            NodeKind::DataSection { marker: _, body } => {
                // Fold the data section body as a comment
                if body.is_some() {
                    self.add_range_from_node(node, Some(FoldingRangeKind::Comment));
                }
            }

            NodeKind::LabeledStatement { label: _, statement } => {
                // Labeled loops (LABEL: while/for/foreach) fold the inner statement
                self.add_range_from_node(node, None);
                self.visit_node(statement);
            }

            NodeKind::Format { .. } => {
                // Format declarations fold as regions (like heredocs)
                self.add_range_from_node(node, Some(FoldingRangeKind::Region));
            }

            NodeKind::Tie { variable, package, args } => {
                // Tie expressions with arguments are foldable when multi-line
                self.add_range_from_node(node, None);
                self.visit_node(variable);
                self.visit_node(package);
                for arg in args {
                    self.visit_node(arg);
                }
            }

            // Other node types - visit children if any
            _ => {}
        }
    }

    /// Add a folding range from a node
    fn add_range_from_node(&mut self, node: &Node, kind: Option<FoldingRangeKind>) {
        // Use actual offsets from location
        let start_offset = node.location.start;
        let end_offset = node.location.end;

        // Only add if it's not trivial
        if end_offset > start_offset.saturating_add(1) {
            self.ranges.push(FoldingRange { start_offset, end_offset, kind });
        }
    }

    /// Add a folding range from two locations
    fn add_range_from_locations(
        &mut self,
        start: &SourceLocation,
        end: &SourceLocation,
        kind: Option<FoldingRangeKind>,
    ) {
        let start_offset = start.start;
        let end_offset = end.end;

        if end_offset > start_offset.saturating_add(1) {
            self.ranges.push(FoldingRange { start_offset, end_offset, kind });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loc(start: usize, end: usize) -> SourceLocation {
        SourceLocation { start, end }
    }

    fn empty_block(start: usize, end: usize) -> Node {
        Node::new(NodeKind::Block { statements: Vec::new() }, loc(start, end))
    }

    fn bool_node(start: usize) -> Node {
        Node::new(NodeKind::Number { value: "1".to_string() }, loc(start, start + 1))
    }

    fn use_node(start: usize, end: usize, module: &str) -> Node {
        Node::new(
            NodeKind::Use { module: module.to_string(), args: Vec::new(), has_filter_risk: false },
            loc(start, end),
        )
    }

    fn variable_statement(start: usize, end: usize) -> Node {
        let variable = Node::new(
            NodeKind::Variable { sigil: "$".to_string(), name: "value".to_string() },
            loc(start + 3, start + 9),
        );
        Node::new(
            NodeKind::VariableDeclaration {
                declarator: "my".to_string(),
                variable: Box::new(variable),
                attributes: Vec::new(),
                initializer: None,
            },
            loc(start, end),
        )
    }

    fn import_ranges(statements: Vec<Node>) -> Vec<FoldingRange> {
        let end = statements.last().map(|node| node.location.end).unwrap_or(0);
        let root = Node::new(NodeKind::Program { statements }, loc(0, end));
        let mut extractor = FoldingRangeExtractor::new();
        extractor.extract(&root)
    }

    fn import_range_count(ranges: &[FoldingRange]) -> usize {
        ranges
            .iter()
            .filter(|range| matches!(range.kind.as_ref(), Some(FoldingRangeKind::Imports)))
            .count()
    }

    #[test]
    fn extract_visits_if_and_while_with_keyword_metadata() {
        let if_node = Node::new(
            NodeKind::If {
                condition: Box::new(bool_node(1)),
                then_branch: Box::new(empty_block(4, 12)),
                elsif_branches: vec![],
                else_branch: Some(Box::new(empty_block(18, 26))),
                keyword: Some("unless".to_string()),
            },
            loc(0, 27),
        );
        let while_node = Node::new(
            NodeKind::While {
                condition: Box::new(bool_node(30)),
                body: Box::new(empty_block(34, 45)),
                continue_block: Some(Box::new(empty_block(46, 55))),
                keyword: Some("until".to_string()),
            },
            loc(29, 56),
        );
        let root =
            Node::new(NodeKind::Program { statements: vec![if_node, while_node] }, loc(0, 56));
        let mut extractor = FoldingRangeExtractor::new();

        let ranges = extractor.extract(&root);

        assert!(ranges.iter().any(|range| range.start_offset == 0 && range.end_offset == 27));
        assert!(ranges.iter().any(|range| range.start_offset == 29 && range.end_offset == 56));
    }

    #[test]
    fn program_import_block_boundary_end_idx_gt_start_idx_rejects_single_import_before_statement() {
        let ranges = import_ranges(vec![use_node(0, 10, "strict"), variable_statement(11, 20)]);

        assert_eq!(import_range_count(&ranges), 0);
    }

    #[test]
    fn program_import_block_boundary_end_idx_gt_start_idx_accepts_multiple_imports_before_statement()
     {
        let ranges = import_ranges(vec![
            use_node(0, 10, "strict"),
            use_node(11, 23, "warnings"),
            variable_statement(24, 33),
        ]);

        assert_eq!(import_range_count(&ranges), 1);
    }

    #[test]
    fn program_trailing_import_block_boundary_end_idx_gt_start_idx_rejects_single_trailing_import()
    {
        let ranges = import_ranges(vec![variable_statement(0, 9), use_node(10, 20, "strict")]);

        assert_eq!(import_range_count(&ranges), 0);
    }

    #[test]
    fn program_trailing_import_block_boundary_end_idx_gt_start_idx_accepts_multiple_trailing_imports()
     {
        let ranges = import_ranges(vec![
            variable_statement(0, 9),
            use_node(10, 20, "strict"),
            use_node(21, 33, "warnings"),
        ]);

        assert_eq!(import_range_count(&ranges), 1);
    }

    #[test]
    fn add_range_from_node_boundary_end_offset_gt_start_offset_plus_one_rejects_trivial() {
        let mut extractor = FoldingRangeExtractor::new();

        extractor.add_range_from_node(&empty_block(5, 6), Some(FoldingRangeKind::Region));

        assert!(extractor.ranges.is_empty());
    }

    #[test]
    fn add_range_from_node_boundary_rejects_saturating_start_offset() {
        let mut extractor = FoldingRangeExtractor::new();

        extractor.add_range_from_node(
            &empty_block(usize::MAX, usize::MAX),
            Some(FoldingRangeKind::Region),
        );

        assert!(extractor.ranges.is_empty());
    }

    #[test]
    fn add_range_from_node_boundary_end_offset_gt_start_offset_plus_one_accepts_multibyte_span() {
        let mut extractor = FoldingRangeExtractor::new();

        extractor.add_range_from_node(&empty_block(5, 7), Some(FoldingRangeKind::Region));

        assert_eq!(extractor.ranges.len(), 1);
        assert_eq!(extractor.ranges[0].start_offset, 5);
        assert_eq!(extractor.ranges[0].end_offset, 7);
    }

    #[test]
    fn add_range_from_locations_boundary_end_offset_gt_start_offset_plus_one_rejects_trivial() {
        let mut extractor = FoldingRangeExtractor::new();

        extractor.add_range_from_locations(&loc(5, 6), &loc(6, 6), Some(FoldingRangeKind::Imports));

        assert!(extractor.ranges.is_empty());
    }

    #[test]
    fn add_range_from_locations_boundary_rejects_saturating_start_offset() {
        let mut extractor = FoldingRangeExtractor::new();

        extractor.add_range_from_locations(
            &loc(usize::MAX, usize::MAX),
            &loc(usize::MAX, usize::MAX),
            Some(FoldingRangeKind::Imports),
        );

        assert!(extractor.ranges.is_empty());
    }

    #[test]
    fn add_range_from_locations_boundary_end_offset_gt_start_offset_plus_one_accepts_multibyte_span()
     {
        let mut extractor = FoldingRangeExtractor::new();

        extractor.add_range_from_locations(&loc(5, 6), &loc(6, 7), Some(FoldingRangeKind::Imports));

        assert_eq!(extractor.ranges.len(), 1);
        assert_eq!(extractor.ranges[0].start_offset, 5);
        assert_eq!(extractor.ranges[0].end_offset, 7);
    }

    #[test]
    fn extract_heredoc_ranges_observes_multiline_body_tokens()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = "my $sql = <<'SQL';\nselect 1\nfrom dual\nSQL\n";

        let ranges = FoldingRangeExtractor::extract_heredoc_ranges(source);
        let range = ranges.first().ok_or("expected a heredoc body range")?;
        let body = source
            .get(range.start_offset..range.end_offset)
            .ok_or("heredoc range must be a valid source slice")?;

        assert_eq!(ranges.len(), 1);
        assert!(matches!(range.kind.as_ref(), Some(FoldingRangeKind::Region)));
        assert!(body.contains("select 1"));
        assert!(body.contains("from dual"));
        assert!(!body.contains("<<"));

        Ok(())
    }

    #[test]
    fn extract_heredoc_ranges_observes_single_line_body_token_for_lsp_filtering()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = "my $text = <<TXT;\nbody\nTXT\n";

        let ranges = FoldingRangeExtractor::extract_heredoc_ranges(source);
        let range = ranges.first().ok_or("expected a heredoc body range")?;
        let body = source
            .get(range.start_offset..range.end_offset)
            .ok_or("heredoc range must be a valid source slice")?;

        assert_eq!(ranges.len(), 1);
        assert!(body.contains("body"));
        assert!(!body.contains("<<"));

        Ok(())
    }

    #[test]
    fn extract_region_markers_empty_text() {
        let source = "";
        let ranges = FoldingRangeExtractor::extract_region_markers(source);
        assert_eq!(ranges.len(), 0);
    }

    #[test]
    fn extract_region_markers_single_region() {
        let source = "# region Setup\nmy $x = 1;\n# endregion\n";
        let ranges = FoldingRangeExtractor::extract_region_markers(source);
        assert_eq!(ranges.len(), 1);
        assert!(matches!(ranges[0].kind, Some(FoldingRangeKind::Region)));
    }

    #[test]
    fn extract_region_markers_multiple_non_nested() {
        let source = "# region First\ncode\n# endregion\n\n# region Second\nmore\n# endregion\n";
        let ranges = FoldingRangeExtractor::extract_region_markers(source);
        assert_eq!(ranges.len(), 2);
        assert!(ranges.iter().all(|r| matches!(r.kind, Some(FoldingRangeKind::Region))));
    }

    #[test]
    fn extract_region_markers_nested() {
        let source = "# region Outer\n# region Inner\nnested\n# endregion\n# endregion\n";
        let ranges = FoldingRangeExtractor::extract_region_markers(source);
        assert_eq!(ranges.len(), 2);
    }

    #[test]
    fn extract_region_markers_with_names() {
        let source = "# region Helpers\nhelper()\n# endregion\n";
        let ranges = FoldingRangeExtractor::extract_region_markers(source);
        assert_eq!(ranges.len(), 1);
    }

    #[test]
    fn extract_region_markers_unmatched_region() {
        let source = "# region Unclosed\ncode\nmore code\n";
        let ranges = FoldingRangeExtractor::extract_region_markers(source);
        assert_eq!(ranges.len(), 0, "Unmatched #region should not produce a fold");
    }

    #[test]
    fn extract_region_markers_unmatched_endregion() {
        let source = "# endregion\ncode\n";
        let ranges = FoldingRangeExtractor::extract_region_markers(source);
        assert_eq!(ranges.len(), 0, "Unmatched #endregion should be ignored");
    }

    #[test]
    fn extract_region_markers_word_boundary() {
        let source = "# regioncode\n# endregionmore\ncode\n";
        let ranges = FoldingRangeExtractor::extract_region_markers(source);
        assert_eq!(ranges.len(), 0, "Should not match region/endregion without word boundary");
    }

    #[test]
    fn extract_region_markers_indented() {
        let source = "    # region Indented\n    code\n    # endregion\n";
        let ranges = FoldingRangeExtractor::extract_region_markers(source);
        assert_eq!(ranges.len(), 1, "Should support indented region markers");
    }
}
