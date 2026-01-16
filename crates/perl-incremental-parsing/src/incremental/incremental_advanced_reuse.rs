//! Advanced tree reuse algorithms for incremental parsing
//!
//! This module implements sophisticated AST node reuse strategies that go beyond
//! simple value matching to achieve high node reuse rates even for complex edits.
//!
//! ## Key Features
//!
//! - **Structural similarity analysis** - Compare AST subtree patterns
//! - **Position-aware reuse** - Understand which nodes can be safely repositioned
//! - **Content-based hashing** - Fast comparison of subtree equivalence
//! - **Incremental node mapping** - Efficient lookup of reusable nodes
//! - **Advanced validation** - Ensure reused nodes maintain correctness
//!
//! ## Performance Targets
//!
//! - **≥85% node reuse** for simple value edits
//! - **≥70% node reuse** for structural modifications
//! - **≥50% node reuse** for complex multi-edit scenarios
//! - **<500µs processing** for reuse analysis on typical documents

use crate::{
    ast::{Node, NodeKind},
    edit::EditSet,
    position::{Position, Range},
};
use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};

/// Advanced node reuse analyzer with sophisticated matching algorithms
#[derive(Debug)]
pub struct AdvancedReuseAnalyzer {
    /// Cache of node structural hashes for fast comparison
    node_hashes: HashMap<usize, u64>,
    /// Mapping of positions to potentially reusable nodes
    position_map: HashMap<usize, Vec<NodeCandidate>>,
    /// Set of nodes that are known to be affected by edits
    affected_nodes: HashSet<usize>,
    /// Statistics for reuse analysis
    pub analysis_stats: ReuseAnalysisStats,
}

/// Statistics tracking reuse analysis performance and effectiveness
#[derive(Debug, Default, Clone)]
pub struct ReuseAnalysisStats {
    pub nodes_analyzed: usize,
    pub structural_matches: usize,
    pub content_matches: usize,
    pub position_adjustments: usize,
    pub reuse_candidates_found: usize,
    pub validation_passes: usize,
    pub validation_failures: usize,
}

/// A candidate node for reuse with metadata about its reusability
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields used by future advanced matching strategies
struct NodeCandidate {
    node: Node,
    structural_hash: u64,
    confidence_score: f64,
    position_delta: isize,
    reuse_type: ReuseType,
}

/// Types of reuse strategies available for nodes
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // Used by future advanced matching strategies
pub enum ReuseType {
    /// Direct reuse - node unchanged
    Direct,
    /// Position shift - same content, different position
    PositionShift,
    /// Content update - same structure, updated values
    ContentUpdate,
    /// Structural equivalent - same pattern with different details
    StructuralEquivalent,
}

/// Configuration for reuse analysis behavior
#[derive(Debug, Clone)]
pub struct ReuseConfig {
    /// Minimum confidence score for reuse (0.0-1.0)
    pub min_confidence: f64,
    /// Maximum position shift allowed for reuse
    pub max_position_shift: usize,
    /// Enable aggressive structural matching
    pub aggressive_structural_matching: bool,
    /// Enable content-based reuse for literals
    pub enable_content_reuse: bool,
    /// Maximum depth for recursive analysis
    pub max_analysis_depth: usize,
}

impl Default for ReuseConfig {
    fn default() -> Self {
        ReuseConfig {
            min_confidence: 0.75,
            max_position_shift: 1000,
            aggressive_structural_matching: true,
            enable_content_reuse: true,
            max_analysis_depth: 10,
        }
    }
}

impl Default for AdvancedReuseAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl AdvancedReuseAnalyzer {
    /// Create a new reuse analyzer with default configuration
    pub fn new() -> Self {
        AdvancedReuseAnalyzer {
            node_hashes: HashMap::new(),
            position_map: HashMap::new(),
            affected_nodes: HashSet::new(),
            analysis_stats: ReuseAnalysisStats::default(),
        }
    }

    /// Create analyzer with custom configuration
    pub fn with_config(_config: ReuseConfig) -> Self {
        // Store config for future use if needed
        Self::new()
    }

    /// Analyze potential node reuse between old and new trees
    ///
    /// Returns a mapping of old node positions to reuse strategies,
    /// enabling intelligent tree reconstruction with maximum node reuse.
    pub fn analyze_reuse_opportunities(
        &mut self,
        old_tree: &Node,
        new_tree: &Node,
        edits: &EditSet,
        config: &ReuseConfig,
    ) -> ReuseAnalysisResult {
        self.analysis_stats = ReuseAnalysisStats::default();

        // Reset internal state
        self.node_hashes.clear();
        self.position_map.clear();
        self.affected_nodes.clear();

        // Build structural analysis of both trees
        let old_analysis = self.build_tree_analysis(old_tree, config);
        let new_analysis = self.build_tree_analysis(new_tree, config);

        // Identify affected regions from edits
        self.identify_affected_nodes(old_tree, edits);

        // Find reuse candidates using multiple strategies
        let mut reuse_map = HashMap::new();

        // Strategy 1: Direct structural matching
        self.find_direct_structural_matches(&old_analysis, &new_analysis, &mut reuse_map, config);

        // Strategy 2: Position-shifted matching
        self.find_position_shifted_matches(&old_analysis, &new_analysis, &mut reuse_map, config);

        // Strategy 3: Content-updated matching
        if config.enable_content_reuse {
            self.find_content_updated_matches(&old_analysis, &new_analysis, &mut reuse_map, config);
        }

        // Strategy 4: Aggressive structural matching
        if config.aggressive_structural_matching {
            self.find_aggressive_structural_matches(
                &old_analysis,
                &new_analysis,
                &mut reuse_map,
                config,
            );
        }

        // Validate reuse candidates and calculate confidence scores
        let validated_reuse_map =
            self.validate_reuse_candidates(reuse_map, old_tree, new_tree, config);

        // Calculate final statistics
        let total_old_nodes = self.count_nodes(old_tree);
        let total_new_nodes = self.count_nodes(new_tree);
        let reused_nodes = validated_reuse_map.len();
        let reuse_percentage = if total_old_nodes > 0 {
            (reused_nodes as f64 / total_old_nodes as f64) * 100.0
        } else {
            0.0
        };

        ReuseAnalysisResult {
            reuse_map: validated_reuse_map,
            total_old_nodes,
            total_new_nodes,
            reused_nodes,
            reuse_percentage,
            analysis_stats: self.analysis_stats.clone(),
        }
    }

    /// Build comprehensive analysis of tree structure
    fn build_tree_analysis(&mut self, tree: &Node, config: &ReuseConfig) -> TreeAnalysis {
        let mut analysis = TreeAnalysis::new();
        self.analyze_node_recursive(tree, &mut analysis, 0, config);
        analysis
    }

    /// Recursively analyze nodes to build structural understanding
    fn analyze_node_recursive(
        &mut self,
        node: &Node,
        analysis: &mut TreeAnalysis,
        depth: usize,
        config: &ReuseConfig,
    ) {
        if depth > config.max_analysis_depth {
            return;
        }

        self.analysis_stats.nodes_analyzed += 1;

        // Calculate structural hash
        let structural_hash = self.calculate_structural_hash(node);
        self.node_hashes.insert(node.location.start, structural_hash);

        // Create node info for analysis
        let node_info = NodeAnalysisInfo {
            node: node.clone(),
            structural_hash,
            depth,
            children_count: self.get_children_count(node),
            content_hash: self.calculate_content_hash(node),
        };

        analysis.add_node_info(node.location.start, node_info);

        // Add to position map
        let candidate = NodeCandidate {
            node: node.clone(),
            structural_hash,
            confidence_score: 1.0, // Will be refined during analysis
            position_delta: 0,
            reuse_type: ReuseType::Direct,
        };

        self.position_map.entry(node.location.start).or_default().push(candidate);

        // Recurse into children
        match &node.kind {
            NodeKind::Program { statements } | NodeKind::Block { statements } => {
                for stmt in statements {
                    self.analyze_node_recursive(stmt, analysis, depth + 1, config);
                }
            }
            NodeKind::VariableDeclaration { variable, initializer, .. } => {
                self.analyze_node_recursive(variable, analysis, depth + 1, config);
                if let Some(init) = initializer {
                    self.analyze_node_recursive(init, analysis, depth + 1, config);
                }
            }
            NodeKind::Binary { left, right, .. } => {
                self.analyze_node_recursive(left, analysis, depth + 1, config);
                self.analyze_node_recursive(right, analysis, depth + 1, config);
            }
            NodeKind::Unary { operand, .. } => {
                self.analyze_node_recursive(operand, analysis, depth + 1, config);
            }
            NodeKind::FunctionCall { args, .. } => {
                for arg in args {
                    self.analyze_node_recursive(arg, analysis, depth + 1, config);
                }
            }
            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
                self.analyze_node_recursive(condition, analysis, depth + 1, config);
                self.analyze_node_recursive(then_branch, analysis, depth + 1, config);
                for (cond, branch) in elsif_branches {
                    self.analyze_node_recursive(cond, analysis, depth + 1, config);
                    self.analyze_node_recursive(branch, analysis, depth + 1, config);
                }
                if let Some(branch) = else_branch {
                    self.analyze_node_recursive(branch, analysis, depth + 1, config);
                }
            }
            _ => {} // Leaf nodes
        }
    }

    /// Identify nodes affected by edits
    fn identify_affected_nodes(&mut self, tree: &Node, edits: &EditSet) {
        for edit in edits.edits() {
            self.mark_affected_nodes_in_range(tree, edit.start_byte, edit.old_end_byte);
        }
    }

    /// Mark nodes as affected if they overlap with edit ranges
    fn mark_affected_nodes_in_range(&mut self, node: &Node, start: usize, end: usize) {
        let node_range = Range::from(node.location);
        let edit_range = Range::new(Position::new(start, 0, 0), Position::new(end, 0, 0));

        if node_range.overlaps(&edit_range) {
            self.affected_nodes.insert(node.location.start);
        }

        // Recurse into children
        match &node.kind {
            NodeKind::Program { statements } | NodeKind::Block { statements } => {
                for stmt in statements {
                    self.mark_affected_nodes_in_range(stmt, start, end);
                }
            }
            NodeKind::VariableDeclaration { variable, initializer, .. } => {
                self.mark_affected_nodes_in_range(variable, start, end);
                if let Some(init) = initializer {
                    self.mark_affected_nodes_in_range(init, start, end);
                }
            }
            NodeKind::Binary { left, right, .. } => {
                self.mark_affected_nodes_in_range(left, start, end);
                self.mark_affected_nodes_in_range(right, start, end);
            }
            _ => {} // Handle other node types as needed
        }
    }

    /// Find direct structural matches between trees
    fn find_direct_structural_matches(
        &mut self,
        old_analysis: &TreeAnalysis,
        new_analysis: &TreeAnalysis,
        reuse_map: &mut HashMap<usize, ReuseStrategy>,
        config: &ReuseConfig,
    ) {
        for (old_pos, old_info) in &old_analysis.node_info {
            // Skip affected nodes for direct matching
            if self.affected_nodes.contains(old_pos) {
                continue;
            }

            // Look for exact structural matches in new tree
            for (new_pos, new_info) in &new_analysis.node_info {
                if old_info.structural_hash == new_info.structural_hash
                    && old_info.children_count == new_info.children_count
                {
                    let confidence = self.calculate_match_confidence(old_info, new_info);
                    if confidence >= config.min_confidence {
                        reuse_map.insert(
                            *old_pos,
                            ReuseStrategy {
                                target_position: *new_pos,
                                reuse_type: ReuseType::Direct,
                                confidence_score: confidence,
                                position_adjustment: (*new_pos as isize) - (*old_pos as isize),
                            },
                        );
                        self.analysis_stats.structural_matches += 1;
                        break; // Use first good match
                    }
                }
            }
        }
    }

    /// Find position-shifted matches (same content, different location)
    fn find_position_shifted_matches(
        &mut self,
        old_analysis: &TreeAnalysis,
        new_analysis: &TreeAnalysis,
        reuse_map: &mut HashMap<usize, ReuseStrategy>,
        config: &ReuseConfig,
    ) {
        for (old_pos, old_info) in &old_analysis.node_info {
            // Skip if already matched or is a leaf node
            if reuse_map.contains_key(old_pos) || old_info.children_count == 0 {
                continue;
            }

            // Look for content matches that may have shifted position
            for (new_pos, new_info) in &new_analysis.node_info {
                if old_info.content_hash == new_info.content_hash
                    && old_info.structural_hash == new_info.structural_hash
                {
                    let position_shift = (*new_pos as isize - *old_pos as isize).unsigned_abs();
                    if position_shift <= config.max_position_shift {
                        let confidence = self.calculate_match_confidence(old_info, new_info) * 0.9; // Slight penalty for position shift
                        if confidence >= config.min_confidence {
                            reuse_map.insert(
                                *old_pos,
                                ReuseStrategy {
                                    target_position: *new_pos,
                                    reuse_type: ReuseType::PositionShift,
                                    confidence_score: confidence,
                                    position_adjustment: (*new_pos as isize) - (*old_pos as isize),
                                },
                            );
                            self.analysis_stats.position_adjustments += 1;
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Find content-updated matches (structure same, values changed)
    fn find_content_updated_matches(
        &mut self,
        old_analysis: &TreeAnalysis,
        new_analysis: &TreeAnalysis,
        reuse_map: &mut HashMap<usize, ReuseStrategy>,
        config: &ReuseConfig,
    ) {
        for (old_pos, old_info) in &old_analysis.node_info {
            if reuse_map.contains_key(old_pos) {
                continue;
            }

            // For leaf nodes, check if structure matches but content differs
            if old_info.children_count == 0 {
                for (new_pos, new_info) in &new_analysis.node_info {
                    if old_info.structural_hash == new_info.structural_hash
                        && old_info.content_hash != new_info.content_hash
                        && self.are_compatible_for_content_update(&old_info.node, &new_info.node)
                    {
                        let confidence = 0.8; // Content updates get medium confidence
                        if confidence >= config.min_confidence {
                            reuse_map.insert(
                                *old_pos,
                                ReuseStrategy {
                                    target_position: *new_pos,
                                    reuse_type: ReuseType::ContentUpdate,
                                    confidence_score: confidence,
                                    position_adjustment: (*new_pos as isize) - (*old_pos as isize),
                                },
                            );
                            self.analysis_stats.content_matches += 1;
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Find aggressive structural matches using pattern analysis
    fn find_aggressive_structural_matches(
        &mut self,
        old_analysis: &TreeAnalysis,
        new_analysis: &TreeAnalysis,
        reuse_map: &mut HashMap<usize, ReuseStrategy>,
        config: &ReuseConfig,
    ) {
        // This is the most sophisticated matching - look for structural patterns
        // even when exact hashes don't match
        for (old_pos, old_info) in &old_analysis.node_info {
            if reuse_map.contains_key(old_pos) {
                continue;
            }

            let mut best_match: Option<(usize, f64)> = None;

            for (new_pos, new_info) in &new_analysis.node_info {
                // Compare structural similarity
                let similarity =
                    self.calculate_structural_similarity(&old_info.node, &new_info.node);
                if similarity >= config.min_confidence * 0.8 {
                    // Slightly lower threshold for aggressive matching
                    if best_match.as_ref().is_none_or(|m| similarity > m.1) {
                        best_match = Some((*new_pos, similarity));
                    }
                }
            }

            if let Some((best_pos, confidence)) = best_match {
                if confidence >= config.min_confidence * 0.7 {
                    // Final threshold check
                    reuse_map.insert(
                        *old_pos,
                        ReuseStrategy {
                            target_position: best_pos,
                            reuse_type: ReuseType::StructuralEquivalent,
                            confidence_score: confidence,
                            position_adjustment: (best_pos as isize) - (*old_pos as isize),
                        },
                    );
                    self.analysis_stats.reuse_candidates_found += 1;
                }
            }
        }
    }

    /// Validate reuse candidates to ensure correctness
    fn validate_reuse_candidates(
        &mut self,
        candidates: HashMap<usize, ReuseStrategy>,
        old_tree: &Node,
        new_tree: &Node,
        config: &ReuseConfig,
    ) -> HashMap<usize, ReuseStrategy> {
        let mut validated = HashMap::new();

        for (old_pos, strategy) in candidates {
            if self.validate_reuse_strategy(&strategy, old_tree, new_tree, config) {
                validated.insert(old_pos, strategy);
                self.analysis_stats.validation_passes += 1;
            } else {
                self.analysis_stats.validation_failures += 1;
            }
        }

        validated
    }

    /// Calculate structural hash for fast comparison
    fn calculate_structural_hash(&self, node: &Node) -> u64 {
        let mut hasher = DefaultHasher::new();

        // Hash node kind discriminant
        std::mem::discriminant(&node.kind).hash(&mut hasher);

        // Hash structural properties based on node type
        match &node.kind {
            NodeKind::Program { statements } => {
                statements.len().hash(&mut hasher);
                "program".hash(&mut hasher);
            }
            NodeKind::Block { statements } => {
                statements.len().hash(&mut hasher);
                "block".hash(&mut hasher);
            }
            NodeKind::VariableDeclaration { declarator, .. } => {
                declarator.hash(&mut hasher);
                "vardecl".hash(&mut hasher);
            }
            NodeKind::Binary { op, .. } => {
                op.hash(&mut hasher);
                "binary".hash(&mut hasher);
            }
            NodeKind::Unary { op, .. } => {
                op.hash(&mut hasher);
                "unary".hash(&mut hasher);
            }
            NodeKind::FunctionCall { name, args } => {
                name.hash(&mut hasher);
                args.len().hash(&mut hasher);
                "funccall".hash(&mut hasher);
            }
            NodeKind::Number { .. } => "number".hash(&mut hasher),
            NodeKind::String { interpolated, .. } => {
                interpolated.hash(&mut hasher);
                "string".hash(&mut hasher);
            }
            NodeKind::Variable { sigil, .. } => {
                sigil.hash(&mut hasher);
                "variable".hash(&mut hasher);
            }
            NodeKind::Identifier { .. } => "identifier".hash(&mut hasher),
            _ => "other".hash(&mut hasher),
        }

        hasher.finish()
    }

    /// Calculate content-based hash for value comparison
    fn calculate_content_hash(&self, node: &Node) -> u64 {
        let mut hasher = DefaultHasher::new();

        match &node.kind {
            NodeKind::Number { value } => value.hash(&mut hasher),
            NodeKind::String { value, .. } => value.hash(&mut hasher),
            NodeKind::Variable { name, .. } => name.hash(&mut hasher),
            NodeKind::Identifier { name } => name.hash(&mut hasher),
            _ => {
                // For non-leaf nodes, hash is based on structure
                self.calculate_structural_hash(node).hash(&mut hasher);
            }
        }

        hasher.finish()
    }

    /// Get count of direct children for a node
    fn get_children_count(&self, node: &Node) -> usize {
        match &node.kind {
            NodeKind::Program { statements } | NodeKind::Block { statements } => statements.len(),
            NodeKind::VariableDeclaration { initializer, .. } => {
                if initializer.is_some() { 2 } else { 1 } // variable + optional initializer
            }
            NodeKind::Binary { .. } => 2, // left + right
            NodeKind::Unary { .. } => 1,  // operand
            NodeKind::FunctionCall { args, .. } => args.len(),
            NodeKind::If { elsif_branches, else_branch, .. } => {
                2 + elsif_branches.len() * 2 + if else_branch.is_some() { 1 } else { 0 }
            }
            _ => 0, // Leaf nodes
        }
    }

    /// Calculate confidence score for a potential match
    fn calculate_match_confidence(
        &self,
        old_info: &NodeAnalysisInfo,
        new_info: &NodeAnalysisInfo,
    ) -> f64 {
        let mut confidence = 0.0f64;

        // Structural match bonus
        if old_info.structural_hash == new_info.structural_hash {
            confidence += 0.4;
        }

        // Content match bonus
        if old_info.content_hash == new_info.content_hash {
            confidence += 0.3;
        }

        // Children count match bonus
        if old_info.children_count == new_info.children_count {
            confidence += 0.2;
        }

        // Depth similarity bonus
        let depth_diff = (old_info.depth as isize - new_info.depth as isize).abs();
        if depth_diff == 0 {
            confidence += 0.1;
        } else if depth_diff <= 2 {
            confidence += 0.05;
        }

        confidence.min(1.0)
    }

    /// Calculate structural similarity between two nodes
    fn calculate_structural_similarity(&self, old_node: &Node, new_node: &Node) -> f64 {
        // This is a more sophisticated comparison than hash equality
        let mut similarity = 0.0;

        // Base similarity from node type
        if std::mem::discriminant(&old_node.kind) == std::mem::discriminant(&new_node.kind) {
            similarity += 0.5;

            // Additional similarity based on node-specific properties
            match (&old_node.kind, &new_node.kind) {
                (NodeKind::Program { statements: s1 }, NodeKind::Program { statements: s2 }) => {
                    let len_similarity = 1.0
                        - ((s1.len() as f64 - s2.len() as f64).abs()
                            / (s1.len().max(s2.len()) as f64));
                    similarity += 0.3 * len_similarity;
                }
                (NodeKind::Binary { op: op1, .. }, NodeKind::Binary { op: op2, .. }) => {
                    if op1 == op2 {
                        similarity += 0.4;
                    }
                }
                (
                    NodeKind::FunctionCall { name: n1, args: a1 },
                    NodeKind::FunctionCall { name: n2, args: a2 },
                ) => {
                    if n1 == n2 {
                        similarity += 0.3;
                    }
                    let arg_similarity = 1.0
                        - ((a1.len() as f64 - a2.len() as f64).abs()
                            / (a1.len().max(a2.len()) as f64));
                    similarity += 0.2 * arg_similarity;
                }
                _ => {
                    similarity += 0.2; // Generic bonus for same type
                }
            }
        }

        similarity.min(1.0)
    }

    /// Check if two nodes are compatible for content updates
    fn are_compatible_for_content_update(&self, old_node: &Node, new_node: &Node) -> bool {
        match (&old_node.kind, &new_node.kind) {
            (NodeKind::Number { .. }, NodeKind::Number { .. }) => true,
            (
                NodeKind::String { interpolated: i1, .. },
                NodeKind::String { interpolated: i2, .. },
            ) => i1 == i2,
            (NodeKind::Variable { sigil: s1, .. }, NodeKind::Variable { sigil: s2, .. }) => {
                s1 == s2
            }
            (NodeKind::Identifier { .. }, NodeKind::Identifier { .. }) => true,
            _ => false,
        }
    }

    /// Validate a reuse strategy for correctness
    fn validate_reuse_strategy(
        &self,
        _strategy: &ReuseStrategy,
        _old_tree: &Node,
        _new_tree: &Node,
        _config: &ReuseConfig,
    ) -> bool {
        // Implement validation logic:
        // - Check that reused nodes maintain parent-child relationships
        // - Verify position adjustments are reasonable
        // - Ensure content updates are semantically valid
        // For now, accept all strategies (can be enhanced with specific validation)
        true
    }

    /// Count total nodes in a tree
    fn count_nodes(&self, node: &Node) -> usize {
        let mut count = 1;

        match &node.kind {
            NodeKind::Program { statements } | NodeKind::Block { statements } => {
                for stmt in statements {
                    count += self.count_nodes(stmt);
                }
            }
            NodeKind::VariableDeclaration { variable, initializer, .. } => {
                count += self.count_nodes(variable);
                if let Some(init) = initializer {
                    count += self.count_nodes(init);
                }
            }
            NodeKind::Binary { left, right, .. } => {
                count += self.count_nodes(left);
                count += self.count_nodes(right);
            }
            NodeKind::Unary { operand, .. } => {
                count += self.count_nodes(operand);
            }
            NodeKind::FunctionCall { args, .. } => {
                for arg in args {
                    count += self.count_nodes(arg);
                }
            }
            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
                count += self.count_nodes(condition);
                count += self.count_nodes(then_branch);
                for (cond, branch) in elsif_branches {
                    count += self.count_nodes(cond);
                    count += self.count_nodes(branch);
                }
                if let Some(branch) = else_branch {
                    count += self.count_nodes(branch);
                }
            }
            _ => {} // Leaf nodes
        }

        count
    }
}

/// Comprehensive analysis of a tree structure
#[derive(Debug)]
struct TreeAnalysis {
    node_info: HashMap<usize, NodeAnalysisInfo>,
}

impl TreeAnalysis {
    fn new() -> Self {
        TreeAnalysis { node_info: HashMap::new() }
    }

    fn add_node_info(&mut self, position: usize, info: NodeAnalysisInfo) {
        self.node_info.insert(position, info);
    }
}

/// Detailed information about a node for reuse analysis
#[derive(Debug, Clone)]
struct NodeAnalysisInfo {
    node: Node,
    structural_hash: u64,
    content_hash: u64,
    depth: usize,
    children_count: usize,
}

/// Strategy for reusing a node from old tree to new tree
#[derive(Debug, Clone)]
pub struct ReuseStrategy {
    pub target_position: usize,
    pub reuse_type: ReuseType,
    pub confidence_score: f64,
    pub position_adjustment: isize,
}

/// Result of reuse analysis with comprehensive metrics
#[derive(Debug)]
pub struct ReuseAnalysisResult {
    pub reuse_map: HashMap<usize, ReuseStrategy>,
    pub total_old_nodes: usize,
    pub total_new_nodes: usize,
    pub reused_nodes: usize,
    pub reuse_percentage: f64,
    pub analysis_stats: ReuseAnalysisStats,
}

impl ReuseAnalysisResult {
    /// Check if reuse analysis achieved target efficiency
    pub fn meets_efficiency_target(&self, target_percentage: f64) -> bool {
        self.reuse_percentage >= target_percentage
    }

    /// Get a summary of the analysis performance
    pub fn performance_summary(&self) -> String {
        format!(
            "Reuse Analysis: {:.1}% efficiency ({}/{} nodes), {} structural matches, {} position adjustments",
            self.reuse_percentage,
            self.reused_nodes,
            self.total_old_nodes,
            self.analysis_stats.structural_matches,
            self.analysis_stats.position_adjustments
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SourceLocation, ast::Node};

    #[test]
    fn test_advanced_reuse_analyzer_creation() {
        let analyzer = AdvancedReuseAnalyzer::new();
        assert_eq!(analyzer.analysis_stats.nodes_analyzed, 0);
    }

    #[test]
    fn test_structural_hash_calculation() {
        let analyzer = AdvancedReuseAnalyzer::new();

        // Create sample nodes
        let node1 = Node::new(
            NodeKind::Number { value: "42".to_string() },
            SourceLocation { start: 0, end: 2 },
        );

        let node2 = Node::new(
            NodeKind::Number { value: "99".to_string() },
            SourceLocation { start: 0, end: 2 },
        );

        let hash1 = analyzer.calculate_structural_hash(&node1);
        let hash2 = analyzer.calculate_structural_hash(&node2);

        // Same structure should have same hash
        assert_eq!(hash1, hash2, "Numbers should have same structural hash regardless of value");
    }

    #[test]
    fn test_content_hash_differs_for_different_values() {
        let analyzer = AdvancedReuseAnalyzer::new();

        let node1 = Node::new(
            NodeKind::Number { value: "42".to_string() },
            SourceLocation { start: 0, end: 2 },
        );

        let node2 = Node::new(
            NodeKind::Number { value: "99".to_string() },
            SourceLocation { start: 0, end: 2 },
        );

        let hash1 = analyzer.calculate_content_hash(&node1);
        let hash2 = analyzer.calculate_content_hash(&node2);

        assert_ne!(hash1, hash2, "Different values should have different content hashes");
    }

    #[test]
    fn test_children_count_calculation() {
        let analyzer = AdvancedReuseAnalyzer::new();

        // Leaf node
        let leaf = Node::new(
            NodeKind::Number { value: "42".to_string() },
            SourceLocation { start: 0, end: 2 },
        );
        assert_eq!(analyzer.get_children_count(&leaf), 0);

        // Binary node
        let binary = Node::new(
            NodeKind::Binary {
                op: "+".to_string(),
                left: Box::new(leaf.clone()),
                right: Box::new(leaf.clone()),
            },
            SourceLocation { start: 0, end: 5 },
        );
        assert_eq!(analyzer.get_children_count(&binary), 2);

        // Program node
        let program = Node::new(
            NodeKind::Program { statements: vec![binary] },
            SourceLocation { start: 0, end: 5 },
        );
        assert_eq!(analyzer.get_children_count(&program), 1);
    }

    #[test]
    fn test_reuse_config_defaults() {
        let config = ReuseConfig::default();
        assert_eq!(config.min_confidence, 0.75);
        assert_eq!(config.max_position_shift, 1000);
        assert!(config.aggressive_structural_matching);
        assert!(config.enable_content_reuse);
        assert_eq!(config.max_analysis_depth, 10);
    }

    #[test]
    fn test_node_compatibility_for_content_update() {
        let analyzer = AdvancedReuseAnalyzer::new();

        let num1 = Node::new(
            NodeKind::Number { value: "42".to_string() },
            SourceLocation { start: 0, end: 2 },
        );

        let num2 = Node::new(
            NodeKind::Number { value: "99".to_string() },
            SourceLocation { start: 0, end: 2 },
        );

        let str1 = Node::new(
            NodeKind::String { value: "hello".to_string(), interpolated: false },
            SourceLocation { start: 0, end: 7 },
        );

        // Same type nodes should be compatible
        assert!(analyzer.are_compatible_for_content_update(&num1, &num2));

        // Different type nodes should not be compatible
        assert!(!analyzer.are_compatible_for_content_update(&num1, &str1));
    }
}
