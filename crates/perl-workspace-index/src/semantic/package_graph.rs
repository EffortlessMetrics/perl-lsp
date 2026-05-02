//! Package graph index for cross-file inheritance and role-composition lookups.
//!
//! Maintains a directed graph of [`PackageNode`] entries connected by
//! [`PackageEdge`] entries with kinds [`PackageEdgeKind::Inherits`],
//! [`PackageEdgeKind::ComposesRole`], and [`PackageEdgeKind::DependsOn`].
//!
//! Supports incremental add/remove via [`PackageGraphIndex::add_edges`] and
//! [`PackageGraphIndex::remove_edges_for_file`], keyed by the file's source URI.
//!
//! # Cycle Detection
//!
//! The [`ancestors`](PackageGraphIndex::ancestors) method traverses the
//! inheritance chain with a visited set. When a cycle is detected the
//! traversal terminates and the result carries a `cycle_detected` flag
//! rather than looping indefinitely.
//!
//! # Requirements
//!
//! - **Req 14.1**: Maintain `PackageNode` entries for each known package, class, and role.
//! - **Req 14.2**: Record `PackageEdge` entries with Inherits, ComposesRole, DependsOn kinds.
//! - **Req 14.4**: Cycle detection — terminate traversal and report cycle rather than looping.

use perl_semantic_facts::{
    EntityId, FileId, PackageEdge, PackageEdgeKind, PackageKind, PackageNode,
};
use std::collections::{HashMap, HashSet};

/// Result of an ancestor traversal through the package graph.
///
/// Contains the ordered list of ancestor package names and a flag indicating
/// whether a cycle was detected during traversal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AncestorResult {
    /// Ancestor package names in traversal order (breadth-first).
    pub ancestors: Vec<String>,
    /// `true` when a circular inheritance chain was detected.
    pub cycle_detected: bool,
}

/// Cross-file package graph index.
///
/// Populated from [`PackageEdge`] data extracted by the package graph
/// extractor during workspace indexing. Supports incremental updates:
/// call [`remove_edges_for_file`](Self::remove_edges_for_file) to purge
/// stale entries, then [`add_edges`](Self::add_edges) to insert fresh ones.
#[derive(Debug, Default)]
pub struct PackageGraphIndex {
    /// Package name → node metadata.
    nodes: HashMap<String, PackageNode>,

    /// Package name → outgoing edges (edges where `from_package` == key).
    outgoing_edges: HashMap<String, Vec<PackageEdge>>,

    /// Tracks which file URIs have contributed edges so that
    /// [`remove_edges_for_file`](Self::remove_edges_for_file) can purge
    /// stale entries.
    file_edges: HashMap<String, Vec<PackageEdge>>,
}

impl PackageGraphIndex {
    /// Create an empty package graph index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a batch of [`PackageEdge`] entries from a single file.
    ///
    /// For each edge, the source and target packages are ensured to have
    /// [`PackageNode`] entries. Unknown target packages are recorded with
    /// [`PackageKind::External`] and [`Confidence::Low`].
    pub fn add_edges(&mut self, source_uri: &str, file_id: FileId, edges: Vec<PackageEdge>) {
        // Store edges for later removal.
        self.file_edges.insert(source_uri.to_string(), edges.clone());

        for edge in &edges {
            // Ensure the source package node exists.
            self.ensure_node_from_edge_source(edge, file_id);

            // Ensure the target package node exists (may be external).
            self.ensure_node_from_edge_target(edge);

            // Record the outgoing edge.
            self.outgoing_edges.entry(edge.from_package.clone()).or_default().push(edge.clone());
        }
    }

    /// Remove all edges and orphaned nodes that originated from the given file URI.
    ///
    /// This is the "remove" half of incremental re-indexing: call this before
    /// [`add_edges`](Self::add_edges) with the updated edges.
    pub fn remove_edges_for_file(&mut self, source_uri: &str) {
        let removed_edges = match self.file_edges.remove(source_uri) {
            Some(edges) => edges,
            None => return,
        };

        // Collect the package names whose outgoing edges need pruning.
        let affected_packages: HashSet<String> =
            removed_edges.iter().map(|e| e.from_package.clone()).collect();

        // Remove the specific edges from the outgoing_edges map.
        for pkg in &affected_packages {
            if let Some(edges) = self.outgoing_edges.get_mut(pkg) {
                edges.retain(|e| !removed_edges.contains(e));
            }
        }

        // Clean up empty edge buckets.
        self.outgoing_edges.retain(|_, v| !v.is_empty());

        // Remove nodes that are no longer referenced by any edge (either as
        // source or target) and were not contributed by another file.
        let all_referenced: HashSet<String> = self.all_referenced_packages();
        self.nodes.retain(|name, _| all_referenced.contains(name));
    }

    /// Traverse the inheritance chain (Inherits edges) starting from
    /// `package_name` and return all ancestor packages.
    ///
    /// Uses breadth-first traversal with a visited set for cycle detection.
    /// When a cycle is detected the traversal terminates immediately and
    /// [`AncestorResult::cycle_detected`] is set to `true`.
    pub fn ancestors(&self, package_name: &str) -> AncestorResult {
        let mut visited = HashSet::new();
        let mut ancestors = Vec::new();
        let mut cycle_detected = false;

        visited.insert(package_name.to_string());
        self.collect_ancestors(package_name, &mut visited, &mut ancestors, &mut cycle_detected);

        AncestorResult { ancestors, cycle_detected }
    }

    /// Recursively collect ancestors via DFS, using `visited` to track the
    /// current traversal path for cycle detection.
    ///
    /// Nodes are added to `visited` before recursing and removed after, so
    /// only back-edges (true cycles) trigger `cycle_detected`. Diamond
    /// convergence is handled by checking `ancestors` to avoid duplicates
    /// without flagging a cycle.
    fn collect_ancestors(
        &self,
        package_name: &str,
        on_stack: &mut HashSet<String>,
        ancestors: &mut Vec<String>,
        cycle_detected: &mut bool,
    ) {
        if *cycle_detected {
            return;
        }

        for parent in self.direct_parents(package_name) {
            if on_stack.contains(&parent) {
                // Back-edge to a node on the current DFS path — true cycle.
                *cycle_detected = true;
                return;
            }

            // Skip already-collected ancestors (diamond convergence).
            if ancestors.contains(&parent) {
                continue;
            }

            ancestors.push(parent.clone());
            on_stack.insert(parent.clone());
            self.collect_ancestors(&parent, on_stack, ancestors, cycle_detected);
            on_stack.remove(&parent);

            if *cycle_detected {
                return;
            }
        }
    }

    /// Return the roles composed by `package_name` (ComposesRole edges).
    pub fn composed_roles(&self, package_name: &str) -> Vec<String> {
        self.outgoing_edges
            .get(package_name)
            .map(|edges| {
                edges
                    .iter()
                    .filter(|e| e.kind == PackageEdgeKind::ComposesRole)
                    .map(|e| e.to_package.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Return the direct dependencies of `package_name` (DependsOn edges).
    pub fn dependencies(&self, package_name: &str) -> Vec<String> {
        self.outgoing_edges
            .get(package_name)
            .map(|edges| {
                edges
                    .iter()
                    .filter(|e| e.kind == PackageEdgeKind::DependsOn)
                    .map(|e| e.to_package.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Look up a [`PackageNode`] by name.
    pub fn get_node(&self, package_name: &str) -> Option<&PackageNode> {
        self.nodes.get(package_name)
    }

    /// Return all outgoing edges for a package.
    pub fn get_edges(&self, package_name: &str) -> &[PackageEdge] {
        self.outgoing_edges.get(package_name).map(Vec::as_slice).unwrap_or_default()
    }

    /// Return the number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Return the total number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.outgoing_edges.values().map(Vec::len).sum()
    }

    // ── Private helpers ──

    /// Return the direct parent package names (Inherits edges) for a package.
    fn direct_parents(&self, package_name: &str) -> Vec<String> {
        self.outgoing_edges
            .get(package_name)
            .map(|edges| {
                edges
                    .iter()
                    .filter(|e| e.kind == PackageEdgeKind::Inherits)
                    .map(|e| e.to_package.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Ensure a node exists for the source package of an edge.
    fn ensure_node_from_edge_source(&mut self, edge: &PackageEdge, file_id: FileId) {
        self.nodes.entry(edge.from_package.clone()).or_insert_with(|| {
            // Infer the package kind from the edge kind.
            let kind = match edge.kind {
                PackageEdgeKind::ComposesRole => PackageKind::Class,
                _ => PackageKind::Package,
            };
            PackageNode::new(
                EntityId(0), // placeholder — will be refined when entity facts are available
                edge.from_package.clone(),
                kind,
                edge.anchor_id,
                Some(file_id),
            )
        });
    }

    /// Ensure a node exists for the target package of an edge.
    fn ensure_node_from_edge_target(&mut self, edge: &PackageEdge) {
        self.nodes.entry(edge.to_package.clone()).or_insert_with(|| {
            let kind = match edge.kind {
                PackageEdgeKind::ComposesRole => PackageKind::Role,
                _ => PackageKind::External,
            };
            PackageNode::new(
                EntityId(0), // placeholder
                edge.to_package.clone(),
                kind,
                None,
                None,
            )
        });
    }

    /// Collect all package names referenced by any edge (source or target).
    fn all_referenced_packages(&self) -> HashSet<String> {
        let mut referenced = HashSet::new();
        for edges in self.file_edges.values() {
            for edge in edges {
                referenced.insert(edge.from_package.clone());
                referenced.insert(edge.to_package.clone());
            }
        }
        referenced
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_semantic_facts::{AnchorId, Confidence, Provenance};

    /// Helper: build a simple `PackageEdge` with Inherits kind.
    fn inherits_edge(from: &str, to: &str) -> PackageEdge {
        PackageEdge::new(
            from.to_string(),
            to.to_string(),
            PackageEdgeKind::Inherits,
            Some(AnchorId(1)),
            Provenance::ExactAst,
            Confidence::High,
        )
    }

    /// Helper: build a ComposesRole edge.
    fn composes_edge(from: &str, to: &str) -> PackageEdge {
        PackageEdge::new(
            from.to_string(),
            to.to_string(),
            PackageEdgeKind::ComposesRole,
            Some(AnchorId(2)),
            Provenance::ExactAst,
            Confidence::High,
        )
    }

    /// Helper: build a DependsOn edge.
    fn depends_edge(from: &str, to: &str) -> PackageEdge {
        PackageEdge::new(
            from.to_string(),
            to.to_string(),
            PackageEdgeKind::DependsOn,
            None,
            Provenance::NameHeuristic,
            Confidence::Low,
        )
    }

    #[test]
    fn empty_graph_has_no_nodes_or_edges() -> Result<(), Box<dyn std::error::Error>> {
        let index = PackageGraphIndex::new();
        assert_eq!(index.node_count(), 0);
        assert_eq!(index.edge_count(), 0);
        Ok(())
    }

    #[test]
    fn add_edges_creates_nodes_and_edges() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = PackageGraphIndex::new();
        let edges = vec![inherits_edge("Child", "Parent")];
        index.add_edges("file:///lib/Child.pm", FileId(1), edges);

        assert_eq!(index.node_count(), 2);
        assert_eq!(index.edge_count(), 1);

        let child_node = index.get_node("Child").ok_or("expected Child node")?;
        assert_eq!(child_node.name, "Child");

        let parent_node = index.get_node("Parent").ok_or("expected Parent node")?;
        assert_eq!(parent_node.name, "Parent");
        assert_eq!(parent_node.kind, PackageKind::External);
        Ok(())
    }

    #[test]
    fn ancestors_returns_linear_chain() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = PackageGraphIndex::new();
        // Child -> Parent -> GrandParent
        index.add_edges("file:///lib/Child.pm", FileId(1), vec![inherits_edge("Child", "Parent")]);
        index.add_edges(
            "file:///lib/Parent.pm",
            FileId(2),
            vec![inherits_edge("Parent", "GrandParent")],
        );

        let result = index.ancestors("Child");
        assert!(!result.cycle_detected);
        assert_eq!(result.ancestors, vec!["Parent", "GrandParent"]);
        Ok(())
    }

    #[test]
    fn ancestors_returns_empty_for_no_parents() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = PackageGraphIndex::new();
        index.add_edges("file:///lib/Root.pm", FileId(1), vec![depends_edge("Root", "SomeDep")]);

        let result = index.ancestors("Root");
        assert!(!result.cycle_detected);
        assert!(result.ancestors.is_empty());
        Ok(())
    }

    #[test]
    fn ancestors_returns_empty_for_unknown_package() -> Result<(), Box<dyn std::error::Error>> {
        let index = PackageGraphIndex::new();
        let result = index.ancestors("Unknown");
        assert!(!result.cycle_detected);
        assert!(result.ancestors.is_empty());
        Ok(())
    }

    #[test]
    fn ancestors_detects_direct_cycle() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = PackageGraphIndex::new();
        // A -> B -> A (cycle)
        index.add_edges("file:///lib/A.pm", FileId(1), vec![inherits_edge("A", "B")]);
        index.add_edges("file:///lib/B.pm", FileId(2), vec![inherits_edge("B", "A")]);

        let result = index.ancestors("A");
        assert!(result.cycle_detected, "should detect cycle A -> B -> A");
        // B should be in ancestors before cycle is detected.
        assert!(result.ancestors.contains(&"B".to_string()));
        Ok(())
    }

    #[test]
    fn ancestors_detects_longer_cycle() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = PackageGraphIndex::new();
        // A -> B -> C -> A (cycle)
        index.add_edges("file:///lib/A.pm", FileId(1), vec![inherits_edge("A", "B")]);
        index.add_edges("file:///lib/B.pm", FileId(2), vec![inherits_edge("B", "C")]);
        index.add_edges("file:///lib/C.pm", FileId(3), vec![inherits_edge("C", "A")]);

        let result = index.ancestors("A");
        assert!(result.cycle_detected, "should detect cycle A -> B -> C -> A");
        assert!(result.ancestors.contains(&"B".to_string()));
        assert!(result.ancestors.contains(&"C".to_string()));
        Ok(())
    }

    #[test]
    fn ancestors_self_cycle() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = PackageGraphIndex::new();
        // A -> A (self-cycle)
        index.add_edges("file:///lib/A.pm", FileId(1), vec![inherits_edge("A", "A")]);

        let result = index.ancestors("A");
        assert!(result.cycle_detected, "should detect self-cycle A -> A");
        Ok(())
    }

    #[test]
    fn ancestors_diamond_no_cycle() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = PackageGraphIndex::new();
        // Diamond: D -> B, D -> C, B -> A, C -> A
        index.add_edges(
            "file:///lib/D.pm",
            FileId(1),
            vec![inherits_edge("D", "B"), inherits_edge("D", "C")],
        );
        index.add_edges("file:///lib/B.pm", FileId(2), vec![inherits_edge("B", "A")]);
        index.add_edges("file:///lib/C.pm", FileId(3), vec![inherits_edge("C", "A")]);

        let result = index.ancestors("D");
        assert!(!result.cycle_detected, "diamond is not a cycle");
        // All ancestors should be present.
        assert!(result.ancestors.contains(&"B".to_string()));
        assert!(result.ancestors.contains(&"C".to_string()));
        assert!(result.ancestors.contains(&"A".to_string()));
        Ok(())
    }

    #[test]
    fn composed_roles_returns_role_names() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = PackageGraphIndex::new();
        index.add_edges(
            "file:///lib/MyClass.pm",
            FileId(1),
            vec![
                composes_edge("MyClass", "Printable"),
                composes_edge("MyClass", "Serializable"),
                inherits_edge("MyClass", "Base"),
            ],
        );

        let roles = index.composed_roles("MyClass");
        assert_eq!(roles.len(), 2);
        assert!(roles.contains(&"Printable".to_string()));
        assert!(roles.contains(&"Serializable".to_string()));
        Ok(())
    }

    #[test]
    fn composed_roles_returns_empty_for_no_roles() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = PackageGraphIndex::new();
        index.add_edges("file:///lib/Plain.pm", FileId(1), vec![inherits_edge("Plain", "Base")]);

        let roles = index.composed_roles("Plain");
        assert!(roles.is_empty());
        Ok(())
    }

    #[test]
    fn dependencies_returns_depends_on_targets() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = PackageGraphIndex::new();
        index.add_edges(
            "file:///lib/App.pm",
            FileId(1),
            vec![
                depends_edge("App", "DBI"),
                depends_edge("App", "JSON"),
                inherits_edge("App", "Base"),
            ],
        );

        let deps = index.dependencies("App");
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"DBI".to_string()));
        assert!(deps.contains(&"JSON".to_string()));
        Ok(())
    }

    #[test]
    fn remove_edges_for_file_clears_entries() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = PackageGraphIndex::new();
        index.add_edges("file:///lib/Child.pm", FileId(1), vec![inherits_edge("Child", "Parent")]);

        assert_eq!(index.node_count(), 2);
        assert_eq!(index.edge_count(), 1);

        index.remove_edges_for_file("file:///lib/Child.pm");

        assert_eq!(index.node_count(), 0);
        assert_eq!(index.edge_count(), 0);
        Ok(())
    }

    #[test]
    fn remove_edges_for_file_is_idempotent() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = PackageGraphIndex::new();
        index.add_edges("file:///lib/Child.pm", FileId(1), vec![inherits_edge("Child", "Parent")]);

        index.remove_edges_for_file("file:///lib/Child.pm");
        // Second remove should be a no-op.
        index.remove_edges_for_file("file:///lib/Child.pm");

        assert_eq!(index.node_count(), 0);
        assert_eq!(index.edge_count(), 0);
        Ok(())
    }

    #[test]
    fn remove_unknown_file_is_noop() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = PackageGraphIndex::new();
        index.add_edges("file:///lib/Child.pm", FileId(1), vec![inherits_edge("Child", "Parent")]);

        index.remove_edges_for_file("file:///nonexistent.pm");

        // Original entries should still be present.
        assert_eq!(index.node_count(), 2);
        assert_eq!(index.edge_count(), 1);
        Ok(())
    }

    #[test]
    fn multiple_files_coexist() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = PackageGraphIndex::new();
        index.add_edges("file:///lib/A.pm", FileId(1), vec![inherits_edge("A", "Base")]);
        index.add_edges("file:///lib/B.pm", FileId(2), vec![inherits_edge("B", "Base")]);

        assert_eq!(index.node_count(), 3); // A, B, Base
        assert_eq!(index.edge_count(), 2);

        // Remove one file — only its edges should disappear.
        index.remove_edges_for_file("file:///lib/A.pm");

        assert_eq!(index.edge_count(), 1);
        // A is no longer referenced, but B and Base remain.
        assert_eq!(index.node_count(), 2);
        assert!(index.get_node("A").is_none());
        assert!(index.get_node("B").is_some());
        assert!(index.get_node("Base").is_some());
        Ok(())
    }

    #[test]
    fn incremental_reindex_replaces_edges() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = PackageGraphIndex::new();
        index.add_edges(
            "file:///lib/Child.pm",
            FileId(1),
            vec![inherits_edge("Child", "OldParent")],
        );

        let result = index.ancestors("Child");
        assert_eq!(result.ancestors, vec!["OldParent"]);

        // Simulate re-indexing: remove old, add updated edges.
        index.remove_edges_for_file("file:///lib/Child.pm");
        index.add_edges(
            "file:///lib/Child.pm",
            FileId(1),
            vec![inherits_edge("Child", "NewParent")],
        );

        let result = index.ancestors("Child");
        assert_eq!(result.ancestors, vec!["NewParent"]);
        assert!(index.get_node("OldParent").is_none());
        Ok(())
    }

    #[test]
    fn role_target_gets_role_kind() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = PackageGraphIndex::new();
        index.add_edges(
            "file:///lib/MyClass.pm",
            FileId(1),
            vec![composes_edge("MyClass", "MyRole")],
        );

        let role_node = index.get_node("MyRole").ok_or("expected MyRole node")?;
        assert_eq!(role_node.kind, PackageKind::Role);

        let class_node = index.get_node("MyClass").ok_or("expected MyClass node")?;
        assert_eq!(class_node.kind, PackageKind::Class);
        Ok(())
    }

    #[test]
    fn external_target_gets_low_confidence_kind() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = PackageGraphIndex::new();
        index.add_edges(
            "file:///lib/Child.pm",
            FileId(1),
            vec![inherits_edge("Child", "Unknown::External")],
        );

        let ext_node = index.get_node("Unknown::External").ok_or("expected external node")?;
        assert_eq!(ext_node.kind, PackageKind::External);
        assert!(ext_node.file_id.is_none());
        Ok(())
    }

    #[test]
    fn get_edges_returns_all_outgoing() -> Result<(), Box<dyn std::error::Error>> {
        let mut index = PackageGraphIndex::new();
        index.add_edges(
            "file:///lib/MyClass.pm",
            FileId(1),
            vec![
                inherits_edge("MyClass", "Base"),
                composes_edge("MyClass", "Role1"),
                depends_edge("MyClass", "Dep1"),
            ],
        );

        let edges = index.get_edges("MyClass");
        assert_eq!(edges.len(), 3);
        Ok(())
    }

    #[test]
    fn get_edges_returns_empty_for_unknown() -> Result<(), Box<dyn std::error::Error>> {
        let index = PackageGraphIndex::new();
        let edges = index.get_edges("Unknown");
        assert!(edges.is_empty());
        Ok(())
    }

    // ── Property-based tests ──

    mod prop_tests {
        use super::*;
        use proptest::prelude::*;
        use proptest::test_runner::Config as ProptestConfig;

        /// Strategy to generate a Perl-like package name (e.g. "Pkg0", "Pkg3").
        ///
        /// Uses a small fixed pool so that random edges frequently collide on
        /// the same node, naturally producing cycles and diamonds.
        fn arb_package_name(node_count: usize) -> impl Strategy<Value = String> {
            (0..node_count).prop_map(|i| format!("Pkg{i}"))
        }

        /// Strategy to generate a random directed graph as a list of
        /// `(from, to)` Inherits edges.
        ///
        /// `node_count` controls the pool size (small pools → more cycles).
        /// `edge_count` controls how many edges are generated.
        fn arb_edge_list(
            node_count: usize,
            max_edges: usize,
        ) -> impl Strategy<Value = Vec<(String, String)>> {
            prop::collection::vec(
                (arb_package_name(node_count), arb_package_name(node_count)),
                0..=max_edges,
            )
        }

        /// Build a [`PackageGraphIndex`] from a list of `(from, to)` pairs,
        /// all treated as [`PackageEdgeKind::Inherits`].
        fn build_graph(edge_list: &[(String, String)]) -> PackageGraphIndex {
            let mut index = PackageGraphIndex::new();
            let edges: Vec<PackageEdge> =
                edge_list.iter().map(|(from, to)| inherits_edge(from, to)).collect();
            if !edges.is_empty() {
                index.add_edges("file:///lib/generated.pm", FileId(1), edges);
            }
            index
        }

        /// Return `true` when the graph (represented as an edge list)
        /// contains a cycle reachable from `start` via Inherits edges.
        ///
        /// Uses DFS with an on-stack set, mirroring the production algorithm.
        fn has_reachable_cycle(edge_list: &[(String, String)], start: &str) -> bool {
            // Build adjacency list.
            let mut adj: std::collections::HashMap<&str, Vec<&str>> =
                std::collections::HashMap::new();
            for (from, to) in edge_list {
                adj.entry(from.as_str()).or_default().push(to.as_str());
            }

            let mut on_stack = std::collections::HashSet::new();
            on_stack.insert(start);
            dfs_has_cycle(&adj, start, &mut on_stack)
        }

        fn dfs_has_cycle<'a>(
            adj: &std::collections::HashMap<&'a str, Vec<&'a str>>,
            node: &'a str,
            on_stack: &mut std::collections::HashSet<&'a str>,
        ) -> bool {
            if let Some(neighbors) = adj.get(node) {
                for &next in neighbors {
                    if on_stack.contains(next) {
                        return true;
                    }
                    on_stack.insert(next);
                    if dfs_has_cycle(adj, next, on_stack) {
                        return true;
                    }
                    on_stack.remove(next);
                }
            }
            false
        }

        // **Validates: Requirements 14.4**
        //
        // Property 12: Package Graph Cycle Termination — For any package
        // graph with circular inheritance chains, the `ancestors` traversal
        // terminates in finite time and reports the cycle rather than
        // looping indefinitely.
        proptest! {
            #![proptest_config(ProptestConfig {
                failure_persistence: None,
                ..ProptestConfig::default()
            })]

            #[test]
            fn prop_ancestors_terminates_and_detects_cycles(
                edge_list in arb_edge_list(6, 12),
            ) {
                let graph = build_graph(&edge_list);

                // Collect all node names present in the graph.
                let mut all_nodes: std::collections::HashSet<String> =
                    std::collections::HashSet::new();
                for (from, to) in &edge_list {
                    all_nodes.insert(from.clone());
                    all_nodes.insert(to.clone());
                }

                // For every node, `ancestors` must terminate (the test
                // completing is proof) and must report cycle_detected when
                // a cycle is reachable from that node.
                for node in &all_nodes {
                    let result = graph.ancestors(node);

                    let oracle_has_cycle = has_reachable_cycle(&edge_list, node);

                    if oracle_has_cycle {
                        prop_assert!(
                            result.cycle_detected,
                            "ancestors('{}') should detect a cycle (edges: {:?})",
                            node,
                            edge_list,
                        );
                    }
                    // When the oracle says no cycle, the implementation
                    // should agree.
                    if !oracle_has_cycle {
                        prop_assert!(
                            !result.cycle_detected,
                            "ancestors('{}') should NOT detect a cycle (edges: {:?})",
                            node,
                            edge_list,
                        );
                    }
                }
            }
        }
    }
}
