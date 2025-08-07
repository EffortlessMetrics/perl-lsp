//! Pragma tracker for Perl code analysis
//! 
//! Tracks `use` and `no` pragmas throughout the codebase to determine
//! effective pragma state at any point in the code.

use std::collections::{HashMap, HashSet};
use crate::ast::Node;

/// Categories of strict pragma
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StrictCategory {
    Refs,
    Subs,
    Vars,
}

/// Pragma state at a given scope
#[derive(Debug, Clone, Default)]
pub struct PragmaState {
    /// Enabled strict categories
    strict: HashSet<StrictCategory>,
    /// Enabled warnings
    warnings: bool,
    /// Other pragmas (feature, experimental, etc.)
    features: HashSet<String>,
}

impl PragmaState {
    /// Check if a specific strict category is enabled
    pub fn is_strict(&self, category: StrictCategory) -> bool {
        self.strict.contains(&category)
    }
    
    /// Check if any strict mode is enabled
    pub fn has_strict(&self) -> bool {
        !self.strict.is_empty()
    }
    
    /// Check if warnings are enabled
    pub fn has_warnings(&self) -> bool {
        self.warnings
    }
}

/// Tracks pragma state throughout a Perl file
pub struct PragmaTracker {
    /// Stack of pragma states (for nested scopes)
    state_stack: Vec<PragmaState>,
    /// Current effective state
    current_state: PragmaState,
}

impl PragmaTracker {
    pub fn new() -> Self {
        Self {
            state_stack: Vec::new(),
            current_state: PragmaState::default(),
        }
    }
    
    /// Analyze an AST to build pragma state
    pub fn analyze(&mut self, ast: &Node) -> HashMap<usize, PragmaState> {
        let mut pragma_map = HashMap::new();
        self.visit_node(ast, &mut pragma_map);
        pragma_map
    }
    
    /// Get effective pragma state at a line
    pub fn get_state_at_line(&self, line: usize, pragma_map: &HashMap<usize, PragmaState>) -> PragmaState {
        // Find the most recent pragma state before this line
        let mut effective_state = PragmaState::default();
        
        for (&pragma_line, state) in pragma_map.iter() {
            if pragma_line <= line {
                effective_state = state.clone();
            }
        }
        
        effective_state
    }
    
    fn visit_node(&mut self, node: &Node, pragma_map: &mut HashMap<usize, PragmaState>) {
        // For now, since AST doesn't have explicit use/no statement nodes,
        // we'll need to detect them from FunctionCall nodes
        match &node.kind {
            crate::ast::NodeKind::FunctionCall { name, args } if name == "use" || name == "require" => {
                // Extract module name from args if possible
                if !args.is_empty() {
                    if let crate::ast::NodeKind::String { value, .. } = &args[0].kind {
                        let line = node.location.start; // Use byte offset as approximation
                        self.handle_use_statement(Some(value.as_str()), None, line, pragma_map);
                    }
                }
            }
            crate::ast::NodeKind::FunctionCall { name, args } if name == "no" => {
                // Extract module name from args if possible
                if !args.is_empty() {
                    if let crate::ast::NodeKind::String { value, .. } = &args[0].kind {
                        let line = node.location.start; // Use byte offset as approximation
                        self.handle_no_statement(Some(value.as_str()), None, line, pragma_map);
                    }
                }
            }
            crate::ast::NodeKind::Block { statements } => {
                // Push current state for new scope
                self.state_stack.push(self.current_state.clone());
                
                // Visit children
                for stmt in statements {
                    self.visit_node(stmt, pragma_map);
                }
                
                // Pop state when leaving scope
                if let Some(prev_state) = self.state_stack.pop() {
                    self.current_state = prev_state;
                }
            }
            crate::ast::NodeKind::Program { statements } => {
                for stmt in statements {
                    self.visit_node(stmt, pragma_map);
                }
            }
            _ => {
                // Most node types don't have direct children we need to visit
            }
        }
    }
    
    fn handle_use_statement(&mut self, module: Option<&str>, args: Option<&str>, line: usize, pragma_map: &mut HashMap<usize, PragmaState>) {
        if let Some(module_name) = module {
            match module_name {
                "strict" => {
                    if let Some(categories) = args {
                        // Parse specific categories like 'refs', 'subs', 'vars'
                        if categories.contains("refs") {
                            self.current_state.strict.insert(StrictCategory::Refs);
                        }
                        if categories.contains("subs") {
                            self.current_state.strict.insert(StrictCategory::Subs);
                        }
                        if categories.contains("vars") {
                            self.current_state.strict.insert(StrictCategory::Vars);
                        }
                    } else {
                        // No args means all categories
                        self.current_state.strict.insert(StrictCategory::Refs);
                        self.current_state.strict.insert(StrictCategory::Subs);
                        self.current_state.strict.insert(StrictCategory::Vars);
                    }
                }
                "warnings" => {
                    self.current_state.warnings = true;
                }
                _ => {
                    // Track other features
                    self.current_state.features.insert(module_name.to_string());
                }
            }
            
            pragma_map.insert(line, self.current_state.clone());
        }
    }
    
    fn handle_no_statement(&mut self, module: Option<&str>, args: Option<&str>, line: usize, pragma_map: &mut HashMap<usize, PragmaState>) {
        if let Some(module_name) = module {
            match module_name {
                "strict" => {
                    if let Some(categories) = args {
                        // Remove specific categories
                        if categories.contains("refs") {
                            self.current_state.strict.remove(&StrictCategory::Refs);
                        }
                        if categories.contains("subs") {
                            self.current_state.strict.remove(&StrictCategory::Subs);
                        }
                        if categories.contains("vars") {
                            self.current_state.strict.remove(&StrictCategory::Vars);
                        }
                    } else {
                        // No args means remove all categories
                        self.current_state.strict.clear();
                    }
                }
                "warnings" => {
                    self.current_state.warnings = false;
                }
                _ => {
                    // Remove other features
                    self.current_state.features.remove(module_name);
                }
            }
            
            pragma_map.insert(line, self.current_state.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pragma_tracking() {
        // TODO: Add tests once AST structure is confirmed
    }
}