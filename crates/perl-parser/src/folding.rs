//! Folding range extraction for LSP textDocument/foldingRange
//!
//! This module provides folding range extraction from the Perl AST,
//! allowing editors to collapse/expand code sections.

use crate::ast::{Node, NodeKind, SourceLocation};

/// Extracts folding ranges from a Perl AST
pub struct FoldingRangeExtractor {
    ranges: Vec<FoldingRange>,
}

/// Represents a foldable region in the code
#[derive(Debug, Clone)]
pub struct FoldingRange {
    pub start_offset: usize,  // Changed from start_line to start_offset
    pub end_offset: usize,    // Changed from end_line to end_offset
    pub kind: Option<FoldingRangeKind>,
}

#[derive(Debug, Clone)]
pub enum FoldingRangeKind {
    Comment,
    Imports,
    Region,
}

impl FoldingRangeExtractor {
    pub fn new() -> Self {
        Self {
            ranges: Vec::new(),
        }
    }
    
    /// Extract all folding ranges from the AST
    pub fn extract(&mut self, ast: &Node) -> Vec<FoldingRange> {
        self.ranges.clear();
        self.visit_node(ast);
        self.ranges.clone()
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
                                    self.add_range_from_locations(start_loc, end_loc, Some(FoldingRangeKind::Imports));
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
                        self.add_range_from_locations(start_loc, end_loc, Some(FoldingRangeKind::Imports));
                    }
                }
            }
            
            NodeKind::Package { name: _, block } => {
                // Package with block is foldable
                if let Some(block_node) = block {
                    self.add_range_from_node(node, None);
                    self.visit_node(block_node);
                }
            }
            
            NodeKind::Subroutine { name: _, params: _, body, .. } |
            NodeKind::Method { name: _, params: _, body } => {
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
            
            NodeKind::If { condition: _, then_branch, elsif_branches, else_branch } => {
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
            
            NodeKind::While { condition: _, body, continue_block } => {
                self.add_range_from_node(node, None);
                self.visit_node(body);
                if let Some(cont) = continue_block {
                    self.visit_node(cont);
                }
            }
            
            NodeKind::For { init: _, condition: _, update: _, body, continue_block: _ } |
            NodeKind::Foreach { variable: _, list: _, body } => {
                self.add_range_from_node(node, None);
                self.visit_node(body);
            }
            
            NodeKind::Do { block } |
            NodeKind::Eval { block } => {
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
            
            NodeKind::PhaseBlock { phase: _, block } => {
                // BEGIN, END, CHECK, INIT blocks
                self.add_range_from_node(node, None);
                self.visit_node(block);
            }
            
            NodeKind::Class { name: _, body } => {
                self.add_range_from_node(node, None);
                self.visit_node(body);
            }
            
            // POD is typically inside strings or special constructs, not a separate NodeKind
            
            NodeKind::Heredoc { .. } => {
                // Heredocs are always foldable
                self.add_range_from_node(node, None);
            }
            
            NodeKind::StatementModifier { statement, modifier: _, condition } => {
                self.visit_node(statement);
                self.visit_node(condition);
            }
            
            NodeKind::ArrayLiteral { elements } => {
                // Arrays with multiple elements are foldable
                if elements.len() > 1 {
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
        if end_offset > start_offset + 1 {
            self.ranges.push(FoldingRange {
                start_offset,
                end_offset,
                kind,
            });
        }
    }
    
    /// Add a folding range from two locations
    fn add_range_from_locations(&mut self, start: &SourceLocation, end: &SourceLocation, kind: Option<FoldingRangeKind>) {
        let start_offset = start.start;
        let end_offset = end.end;
        
        if end_offset > start_offset + 1 {
            self.ranges.push(FoldingRange {
                start_offset,
                end_offset,
                kind,
            });
        }
    }
    
}

