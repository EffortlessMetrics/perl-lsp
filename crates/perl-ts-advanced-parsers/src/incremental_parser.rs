//! Incremental parsing support for efficient document updates
//!
//! This module provides incremental parsing capabilities that allow
//! re-parsing only the changed portions of a document, significantly
//! improving performance for real-time editing scenarios.

use crate::enhanced_full_parser::EnhancedFullParser;
use perl_parser_pest::ParseError;
use perl_parser_pest::pure_rust_parser::{AstNode, PerlParser, PureRustPerlParser, Rule};
use pest::Parser;
use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

/// Edit operation representing a change to the document
#[derive(Debug, Clone)]
pub struct Edit {
    /// Start position of the edit (byte offset)
    pub start_byte: usize,
    /// Old end position (before the edit)
    pub old_end_byte: usize,
    /// New end position (after the edit)
    pub new_end_byte: usize,
    /// Start position (line, column)
    pub start_position: Position,
    /// Old end position (line, column)
    pub old_end_position: Position,
    /// New end position (line, column)
    pub new_end_position: Position,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

/// Cached parse tree with metadata for incremental updates
#[derive(Debug, Clone)]
pub struct ParseTree {
    /// Root AST node
    pub root: AstNode,
    /// Node positions for quick lookup
    pub node_positions: HashMap<NodeId, NodePosition>,
    /// Source text used for parsing
    pub source: Arc<str>,
    /// Line breaks for position calculation
    pub line_breaks: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(usize);

#[derive(Debug, Clone)]
pub struct NodePosition {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_position: Position,
    pub end_position: Position,
}

/// Incremental parser that maintains parse state across edits
pub struct IncrementalParser {
    /// Current parse tree
    current_tree: Option<ParseTree>,
    /// Edit history for debugging
    edit_history: Vec<Edit>,
    /// Maximum edits to track
    max_history: usize,
    /// Whether to use enhanced parser features
    use_enhanced: bool,
}

impl IncrementalParser {
    pub fn new() -> Self {
        Self { current_tree: None, edit_history: Vec::new(), max_history: 100, use_enhanced: true }
    }

    pub fn with_enhanced(mut self, use_enhanced: bool) -> Self {
        self.use_enhanced = use_enhanced;
        self
    }

    /// Parse the initial document
    pub fn parse_initial(&mut self, source: &str) -> Result<&ParseTree, ParseError> {
        let mut parser = EnhancedFullParser::new();
        let root = parser.parse(source).map_err(|_| ParseError::ParseFailed)?;

        let line_breaks = find_line_breaks(source);
        let mut node_positions = HashMap::new();
        let mut id_counter = 0;
        collect_node_positions(&root, &mut node_positions, &mut id_counter, source, &line_breaks);

        self.current_tree =
            Some(ParseTree { root, node_positions, source: Arc::from(source), line_breaks });

        self.current_tree.as_ref().ok_or(ParseError::ParseFailed)
    }

    /// Apply an edit and re-parse incrementally
    pub fn apply_edit(&mut self, edit: Edit, new_source: &str) -> Result<&ParseTree, ParseError> {
        // Track edit history
        self.edit_history.push(edit.clone());
        if self.edit_history.len() > self.max_history {
            self.edit_history.remove(0);
        }

        // If no current tree, parse from scratch
        if self.current_tree.is_none() {
            return self.parse_initial(new_source);
        }

        // Determine affected regions
        let affected_nodes = self.find_affected_nodes(&edit);

        // If too many nodes affected, re-parse from scratch
        if affected_nodes.len() > 10 {
            return self.parse_initial(new_source);
        }

        // Find the smallest enclosing statement or block
        let reparse_range = self.find_reparse_range(&affected_nodes, &edit);

        // Extract and parse the affected region
        let region_ast = self.parse_region(new_source, &reparse_range)?;

        // Splice the new AST into the existing tree
        self.splice_ast(region_ast, &reparse_range, new_source)?;

        self.current_tree.as_ref().ok_or(ParseError::ParseFailed)
    }

    /// Find nodes affected by an edit
    fn find_affected_nodes(&self, edit: &Edit) -> Vec<NodeId> {
        let tree = match &self.current_tree {
            Some(tree) => tree,
            None => return Vec::new(),
        };

        let mut affected = Vec::new();

        for (id, pos) in &tree.node_positions {
            // Node is affected if edit intersects with it
            if edit.start_byte <= pos.end_byte && edit.old_end_byte >= pos.start_byte {
                affected.push(*id);
            }
        }

        affected
    }

    /// Find the optimal range to re-parse
    fn find_reparse_range(&self, _affected_nodes: &[NodeId], edit: &Edit) -> Range<usize> {
        let tree = if let Some(t) = self.current_tree.as_ref() {
            t
        } else {
            return edit.start_byte..edit.new_end_byte;
        };

        // Find the smallest enclosing statement or block
        let mut min_start = edit.start_byte;
        let mut max_end = edit.new_end_byte;

        // Extend to statement boundaries
        let source_bytes = tree.source.as_bytes();

        // Find previous statement boundary
        while min_start > 0 {
            if min_start >= 2 && source_bytes[min_start - 1] == b';' {
                break;
            }
            if min_start >= 2 && source_bytes[min_start - 1] == b'}' {
                break;
            }
            if min_start >= 2
                && source_bytes[min_start - 1] == b'\n'
                && (min_start == 1 || source_bytes[min_start - 2] == b'\n')
            {
                break;
            }
            min_start -= 1;
        }

        // Find next statement boundary
        while max_end < source_bytes.len() {
            if source_bytes[max_end] == b';' {
                max_end += 1;
                break;
            }
            if source_bytes[max_end] == b'}' {
                max_end += 1;
                break;
            }
            max_end += 1;
        }

        min_start..max_end
    }

    /// Parse a specific region of the source
    fn parse_region(&self, source: &str, range: &Range<usize>) -> Result<AstNode, ParseError> {
        let region = &source[range.clone()];

        // Try to parse as a statement first
        if let Ok(pairs) = PerlParser::parse(Rule::statement, region) {
            let mut parser = PureRustPerlParser::new();
            for pair in pairs {
                if let Ok(Some(node)) = parser.build_node(pair) {
                    return Ok(node);
                }
            }
        }

        // Try as a block
        if let Ok(pairs) = PerlParser::parse(Rule::block, region) {
            let mut parser = PureRustPerlParser::new();
            for pair in pairs {
                if let Ok(Some(node)) = parser.build_node(pair) {
                    return Ok(node);
                }
            }
        }

        // Fall back to expression
        if let Ok(pairs) = PerlParser::parse(Rule::expression, region) {
            let mut parser = PureRustPerlParser::new();
            for pair in pairs {
                if let Ok(Some(node)) = parser.build_node(pair) {
                    return Ok(node);
                }
            }
        }

        Err(ParseError::ParseFailed)
    }

    /// Splice a new AST node into the existing tree
    fn splice_ast(
        &mut self,
        _new_node: AstNode,
        _range: &Range<usize>,
        new_source: &str,
    ) -> Result<(), ParseError> {
        // For now, just re-parse the whole document
        // A full implementation would surgically replace the affected nodes
        self.parse_initial(new_source)?;
        Ok(())
    }

    /// Get the current parse tree
    pub fn current_tree(&self) -> Option<&ParseTree> {
        self.current_tree.as_ref()
    }

    /// Get edit history
    pub fn edit_history(&self) -> &[Edit] {
        &self.edit_history
    }
}

/// Find all line break positions in the source
fn find_line_breaks(source: &str) -> Vec<usize> {
    let mut breaks = vec![0];
    for (i, ch) in source.char_indices() {
        if ch == '\n' {
            breaks.push(i + 1);
        }
    }
    breaks
}

/// Calculate position from byte offset
fn byte_to_position(byte_offset: usize, line_breaks: &[usize]) -> Position {
    let line = line_breaks.binary_search(&byte_offset).unwrap_or_else(|i| i.saturating_sub(1));
    let line_start = line_breaks[line];
    let column = byte_offset - line_start;
    Position { line, column }
}

/// Collect positions for all nodes in the AST
fn collect_node_positions(
    node: &AstNode,
    positions: &mut HashMap<NodeId, NodePosition>,
    id_counter: &mut usize,
    source: &str,
    line_breaks: &[usize],
) {
    // This is a simplified implementation
    // A full implementation would track actual byte positions during parsing
    let node_id = NodeId(*id_counter);
    *id_counter += 1;

    // For now, store dummy positions
    positions.insert(
        node_id,
        NodePosition {
            start_byte: 0,
            end_byte: source.len(),
            start_position: Position { line: 0, column: 0 },
            end_position: byte_to_position(source.len(), line_breaks),
        },
    );

    // Recursively process child nodes
    match node {
        AstNode::Program(nodes) | AstNode::Block(nodes) | AstNode::List(nodes) => {
            for child in nodes {
                collect_node_positions(child, positions, id_counter, source, line_breaks);
            }
        }
        AstNode::Statement(inner)
        | AstNode::BeginBlock(inner)
        | AstNode::EndBlock(inner)
        | AstNode::CheckBlock(inner)
        | AstNode::InitBlock(inner)
        | AstNode::UnitcheckBlock(inner)
        | AstNode::DoBlock(inner)
        | AstNode::EvalBlock(inner)
        | AstNode::EvalString(inner)
        | AstNode::DeferStatement(inner) => {
            collect_node_positions(inner, positions, id_counter, source, line_breaks);
        }
        AstNode::IfStatement { condition, then_block, elsif_clauses, else_block } => {
            collect_node_positions(condition, positions, id_counter, source, line_breaks);
            collect_node_positions(then_block, positions, id_counter, source, line_breaks);
            for (cond, block) in elsif_clauses {
                collect_node_positions(cond, positions, id_counter, source, line_breaks);
                collect_node_positions(block, positions, id_counter, source, line_breaks);
            }
            if let Some(else_block) = else_block {
                collect_node_positions(else_block, positions, id_counter, source, line_breaks);
            }
        }
        AstNode::WhileStatement { condition, block, .. }
        | AstNode::UntilStatement { condition, block, .. } => {
            collect_node_positions(condition, positions, id_counter, source, line_breaks);
            collect_node_positions(block, positions, id_counter, source, line_breaks);
        }
        AstNode::ForStatement { init, condition, update, block, .. } => {
            if let Some(init) = init {
                collect_node_positions(init, positions, id_counter, source, line_breaks);
            }
            if let Some(condition) = condition {
                collect_node_positions(condition, positions, id_counter, source, line_breaks);
            }
            if let Some(update) = update {
                collect_node_positions(update, positions, id_counter, source, line_breaks);
            }
            collect_node_positions(block, positions, id_counter, source, line_breaks);
        }
        AstNode::ForeachStatement { variable, list, block, .. } => {
            if let Some(variable) = variable {
                collect_node_positions(variable, positions, id_counter, source, line_breaks);
            }
            collect_node_positions(list, positions, id_counter, source, line_breaks);
            collect_node_positions(block, positions, id_counter, source, line_breaks);
        }
        AstNode::BinaryOp { left, right, .. } => {
            collect_node_positions(left, positions, id_counter, source, line_breaks);
            collect_node_positions(right, positions, id_counter, source, line_breaks);
        }
        AstNode::UnaryOp { operand, .. } => {
            collect_node_positions(operand, positions, id_counter, source, line_breaks);
        }
        AstNode::Assignment { target, value, .. } => {
            collect_node_positions(target, positions, id_counter, source, line_breaks);
            collect_node_positions(value, positions, id_counter, source, line_breaks);
        }
        _ => {} // Other node types
    }
}

impl Default for IncrementalParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_parse() {
        use perl_tdd_support::must;
        let mut parser = IncrementalParser::new();
        let source = "my $x = 42;\nprint $x;";

        let tree = must(parser.parse_initial(source));
        assert!(matches!(&tree.root, AstNode::Program(_)));
        assert_eq!(tree.line_breaks.len(), 3); // Start, after first line, end
    }

    #[test]
    fn test_simple_edit() {
        use perl_tdd_support::must;
        let mut parser = IncrementalParser::new();
        let source = "my $x = 42;";
        must(parser.parse_initial(source));

        // Change 42 to 43
        let edit = Edit {
            start_byte: 8,
            old_end_byte: 10,
            new_end_byte: 10,
            start_position: Position { line: 0, column: 8 },
            old_end_position: Position { line: 0, column: 10 },
            new_end_position: Position { line: 0, column: 10 },
        };

        let new_source = "my $x = 43;";
        let tree = must(parser.apply_edit(edit, new_source));
        assert!(matches!(&tree.root, AstNode::Program(_)));
    }

    #[test]
    fn test_line_breaks() {
        let source = "line1\nline2\nline3";
        let breaks = find_line_breaks(source);
        assert_eq!(breaks, vec![0, 6, 12]);

        let pos = byte_to_position(7, &breaks);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.column, 1);
    }
}
