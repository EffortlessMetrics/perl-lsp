//! Pragma tracker for Perl code analysis
//! 
//! Tracks `use` and `no` pragmas throughout the codebase to determine
//! effective pragma state at any point in the code.

use std::ops::Range;
use crate::ast::{Node, NodeKind};

/// Pragma state at a given point in the code
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PragmaState {
    /// Whether strict vars is enabled
    pub strict_vars: bool,
    /// Whether strict subs is enabled  
    pub strict_subs: bool,
    /// Whether strict refs is enabled
    pub strict_refs: bool,
    /// Whether warnings are enabled
    pub warnings: bool,
}

impl PragmaState {
    /// Create a new pragma state with all strict modes enabled
    pub fn all_strict() -> Self {
        Self {
            strict_vars: true,
            strict_subs: true,
            strict_refs: true,
            warnings: false,
        }
    }
}

/// Tracks pragma state throughout a Perl file
pub struct PragmaTracker;

impl PragmaTracker {
    /// Build a range-indexed pragma map from an AST
    pub fn build(ast: &Node) -> Vec<(Range<usize>, PragmaState)> {
        let mut ranges = Vec::new();
        let mut current_state = PragmaState::default();
        
        // Build the pragma map by walking the AST
        Self::build_ranges(ast, &mut current_state, &mut ranges);
        
        // Sort by start offset
        ranges.sort_by_key(|(range, _)| range.start);
        
        ranges
    }
    
    /// Get the pragma state at a specific byte offset
    pub fn state_for_offset(pragma_map: &[(Range<usize>, PragmaState)], offset: usize) -> PragmaState {
        // Find the last pragma state that starts before this offset
        let mut effective_state = PragmaState::default();
        
        for (range, state) in pragma_map {
            if range.start <= offset {
                effective_state = state.clone();
            } else {
                break;
            }
        }
        
        effective_state
    }
    
    fn build_ranges(node: &Node, current_state: &mut PragmaState, ranges: &mut Vec<(Range<usize>, PragmaState)>) {
        match &node.kind {
            NodeKind::Use { module, args } => {
                // Handle use statements
                match module.as_str() {
                    "strict" => {
                        if args.is_empty() {
                            // use strict; enables all categories
                            current_state.strict_vars = true;
                            current_state.strict_subs = true;
                            current_state.strict_refs = true;
                        } else {
                            // Parse specific categories
                            for arg in args {
                                match arg.as_str() {
                                    "vars" | "'vars'" | "\"vars\"" => current_state.strict_vars = true,
                                    "subs" | "'subs'" | "\"subs\"" => current_state.strict_subs = true,
                                    "refs" | "'refs'" | "\"refs\"" => current_state.strict_refs = true,
                                    _ => {}
                                }
                            }
                        }
                        
                        // Record the state change at this location
                        ranges.push((
                            node.location.start..node.location.end,
                            current_state.clone()
                        ));
                    }
                    "warnings" => {
                        current_state.warnings = true;
                        ranges.push((
                            node.location.start..node.location.end,
                            current_state.clone()
                        ));
                    }
                    _ => {}
                }
            }
            NodeKind::No { module, args } => {
                // Handle no statements
                match module.as_str() {
                    "strict" => {
                        if args.is_empty() {
                            // no strict; disables all categories
                            current_state.strict_vars = false;
                            current_state.strict_subs = false;
                            current_state.strict_refs = false;
                        } else {
                            // Parse specific categories
                            for arg in args {
                                match arg.as_str() {
                                    "vars" | "'vars'" | "\"vars\"" => current_state.strict_vars = false,
                                    "subs" | "'subs'" | "\"subs\"" => current_state.strict_subs = false,
                                    "refs" | "'refs'" | "\"refs\"" => current_state.strict_refs = false,
                                    _ => {}
                                }
                            }
                        }
                        
                        // Record the state change at this location
                        ranges.push((
                            node.location.start..node.location.end,
                            current_state.clone()
                        ));
                    }
                    "warnings" => {
                        current_state.warnings = false;
                        ranges.push((
                            node.location.start..node.location.end,
                            current_state.clone()
                        ));
                    }
                    _ => {}
                }
            }
            NodeKind::Block { statements } => {
                // Save current state
                let saved_state = current_state.clone();
                
                // Process statements in the block
                for stmt in statements {
                    Self::build_ranges(stmt, current_state, ranges);
                }
                
                // Restore state after block
                *current_state = saved_state;
            }
            NodeKind::Program { statements } => {
                // Process all top-level statements
                for stmt in statements {
                    Self::build_ranges(stmt, current_state, ranges);
                }
            }
            _ => {
                // Recursively process children for other node types
                for child in node.children() {
                    Self::build_ranges(child, current_state, ranges);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;
    
    #[test]
    fn test_use_strict_enables_all() {
        let source = "use strict;\nmy $x = FOO;";
        let mut parser = Parser::new(source);
        let ast = parser.parse();
        
        let pragma_map = PragmaTracker::build(&ast);
        
        // After "use strict", all strict modes should be enabled
        let state = PragmaTracker::state_for_offset(&pragma_map, 12); // After "use strict;"
        assert!(state.strict_vars);
        assert!(state.strict_subs);
        assert!(state.strict_refs);
    }
    
    #[test]
    fn test_use_strict_specific_category() {
        let source = "use strict 'subs';\nmy $x = FOO;";
        let mut parser = Parser::new(source);
        let ast = parser.parse();
        
        let pragma_map = PragmaTracker::build(&ast);
        
        // After "use strict 'subs'", only strict subs should be enabled
        let state = PragmaTracker::state_for_offset(&pragma_map, 19); // After "use strict 'subs';"
        assert!(!state.strict_vars);
        assert!(state.strict_subs);
        assert!(!state.strict_refs);
    }
    
    #[test]
    fn test_no_strict_disables() {
        let source = "use strict;\nno strict 'subs';\nmy $x = FOO;";
        let mut parser = Parser::new(source);
        let ast = parser.parse();
        
        let pragma_map = PragmaTracker::build(&ast);
        
        // After "no strict 'subs'", strict subs should be disabled but vars/refs still enabled
        let state = PragmaTracker::state_for_offset(&pragma_map, 30); // After "no strict 'subs';"
        assert!(state.strict_vars);
        assert!(!state.strict_subs);
        assert!(state.strict_refs);
    }
}