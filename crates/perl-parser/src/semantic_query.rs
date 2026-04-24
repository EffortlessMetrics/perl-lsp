//! Read-only semantic query facade over parser + semantic + workspace layers.
//!
//! This module provides a small, intentionally incomplete integration surface for
//! consumers that need semantic lookups without coupling to crate-internal data structures.

use crate::analysis::class_model::{ClassModel, ClassModelBuilder, MethodResolutionOrder};
use crate::ast::{Node, NodeKind};
use crate::error::ParseError;
use crate::pragma_tracker::{PragmaState, PragmaTracker};
use crate::semantic::SemanticModel;
use crate::symbol::SymbolKind;
use crate::workspace::workspace_index::WorkspaceIndex;
use crate::{Parser, SourceLocation};
use std::collections::{HashMap, HashSet};
use std::ops::Range;

/// Stable identifier for a definition result.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DefinitionId(pub String);

/// Source location for a resolved definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionLocation {
    /// Optional file URI when the definition comes from workspace index.
    pub uri: Option<String>,
    /// Byte-range location when available.
    pub location: SourceLocation,
}

/// Resolved symbol returned by semantic queries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSymbol {
    /// Unqualified symbol name.
    pub name: String,
    /// Qualified symbol name.
    pub qualified_name: String,
    /// Symbol kind.
    pub kind: SymbolKind,
    /// Stable identifier for downstream caching.
    pub definition_id: DefinitionId,
    /// Definition location.
    pub definition: DefinitionLocation,
}

/// Import visible in the current file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleImport {
    /// Imported module name from `use`.
    pub module: String,
    /// Raw import arguments.
    pub args: Vec<String>,
    /// Location of the `use` statement.
    pub location: SourceLocation,
}

/// Inheritance hop for a parent-chain query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InheritedOrigin {
    /// Parent package/class name.
    pub package: String,
    /// Whether the parent is modeled in the same parsed file.
    pub same_file: bool,
}

/// Parent-chain result for a class/package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParentChain {
    /// Receiver class/package.
    pub class_name: String,
    /// Ordered parent chain (MRO-specific for modeled classes).
    pub inherited_from: Vec<InheritedOrigin>,
}

/// Effective pragma state snapshot at a byte offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectivePragmaState {
    /// Effective `strict vars`.
    pub strict_vars: bool,
    /// Effective `strict subs`.
    pub strict_subs: bool,
    /// Effective `strict refs`.
    pub strict_refs: bool,
    /// Effective `warnings`.
    pub warnings: bool,
    /// Effective `utf8`.
    pub utf8: bool,
    /// Active feature names.
    pub features: Vec<String>,
    /// Lexically imported builtin names.
    pub builtin_imports: Vec<String>,
}

/// Narrow read-only query facade for parser+semantic+workspace integration.
pub struct SemanticQueryFacade {
    ast: Node,
    semantic_model: SemanticModel,
    pragma_map: Vec<(Range<usize>, PragmaState)>,
    class_models: Vec<ClassModel>,
    workspace_index: Option<WorkspaceIndex>,
}

impl SemanticQueryFacade {
    /// Build a facade from source text by parsing and analyzing it.
    pub fn from_source(source: &str) -> Result<Self, ParseError> {
        let mut parser = Parser::new(source);
        let ast = parser.parse()?;
        let semantic_model = SemanticModel::build(&ast, source);
        let pragma_map = PragmaTracker::build(&ast);
        let class_models = ClassModelBuilder::new().build(&ast);

        Ok(Self { ast, semantic_model, pragma_map, class_models, workspace_index: None })
    }

    /// Attach a workspace index for cross-file lookups.
    #[must_use]
    pub fn with_workspace_index(mut self, workspace_index: WorkspaceIndex) -> Self {
        self.workspace_index = Some(workspace_index);
        self
    }

    /// Resolve the symbol definition at a byte position in the current file.
    pub fn resolve_symbol_at(&self, position: usize) -> Option<ResolvedSymbol> {
        let symbol = self.semantic_model.definition_at(position)?;
        let id = DefinitionId(format!(
            "{}:{}:{}:{}",
            symbol.qualified_name, symbol.location.start, symbol.location.end, symbol.scope_id
        ));

        Some(ResolvedSymbol {
            name: symbol.name.clone(),
            qualified_name: symbol.qualified_name.clone(),
            kind: symbol.kind,
            definition_id: id,
            definition: DefinitionLocation { uri: None, location: symbol.location },
        })
    }

    /// Resolve a definition location by symbol name, preferring workspace data when available.
    pub fn definition_location(&self, symbol_name: &str) -> Option<DefinitionLocation> {
        if let Some(index) = &self.workspace_index {
            if let Some(location) = index.find_definition(symbol_name) {
                return Some(DefinitionLocation {
                    uri: Some(location.uri),
                    location: SourceLocation {
                        start: location.range.start.line as usize,
                        end: location.range.end.line as usize,
                    },
                });
            }
        }

        self.semantic_model
            .symbol_table()
            .symbols
            .get(symbol_name)
            .and_then(|items| items.first())
            .map(|symbol| DefinitionLocation { uri: None, location: symbol.location })
    }

    /// Return all `use` imports visible in this parsed file.
    pub fn visible_imports(&self) -> Vec<VisibleImport> {
        let mut imports = Vec::new();
        collect_imports(&self.ast, &mut imports);
        imports
    }

    /// Return the ordered parent chain for a class/package in this file model.
    pub fn parent_chain(&self, class_name: &str) -> Option<ParentChain> {
        let models_by_name: HashMap<&str, &ClassModel> =
            self.class_models.iter().map(|model| (model.name.as_str(), model)).collect();

        let model = models_by_name.get(class_name).copied()?;
        let order = match model.mro {
            MethodResolutionOrder::Dfs => dfs_ancestor_order(class_name, &models_by_name),
            MethodResolutionOrder::C3 => c3_ancestor_order(class_name, &models_by_name),
        };

        Some(ParentChain {
            class_name: class_name.to_string(),
            inherited_from: order
                .into_iter()
                .map(|package| InheritedOrigin {
                    same_file: models_by_name.contains_key(package.as_str()),
                    package,
                })
                .collect(),
        })
    }

    /// Return effective pragma state for a byte offset.
    pub fn effective_pragma_state(&self, offset: usize) -> EffectivePragmaState {
        let state = PragmaTracker::state_for_offset(&self.pragma_map, offset);
        EffectivePragmaState {
            strict_vars: state.strict_vars,
            strict_subs: state.strict_subs,
            strict_refs: state.strict_refs,
            warnings: state.warnings,
            utf8: state.utf8,
            features: state.features.iter().map(ToString::to_string).collect(),
            builtin_imports: state.builtin_imports,
        }
    }
}

fn collect_imports(node: &Node, out: &mut Vec<VisibleImport>) {
    if let NodeKind::Use { module, args, .. } = &node.kind {
        out.push(VisibleImport {
            module: module.clone(),
            args: args.clone(),
            location: node.location,
        });
    }

    for child in node.children() {
        collect_imports(child, out);
    }
}

fn dfs_ancestor_order(
    class_name: &str,
    models_by_name: &HashMap<&str, &ClassModel>,
) -> Vec<String> {
    fn walk(
        class_name: &str,
        models_by_name: &HashMap<&str, &ClassModel>,
        seen: &mut HashSet<String>,
        out: &mut Vec<String>,
    ) {
        if let Some(model) = models_by_name.get(class_name).copied() {
            for parent in &model.parents {
                if seen.insert(parent.clone()) {
                    out.push(parent.clone());
                    walk(parent, models_by_name, seen, out);
                }
            }
        }
    }

    let mut seen = HashSet::new();
    let mut out = Vec::new();
    walk(class_name, models_by_name, &mut seen, &mut out);
    out
}

fn c3_ancestor_order(class_name: &str, models_by_name: &HashMap<&str, &ClassModel>) -> Vec<String> {
    fn linearize(
        name: &str,
        models_by_name: &HashMap<&str, &ClassModel>,
        visited: &mut HashSet<String>,
    ) -> Vec<String> {
        if !visited.insert(name.to_string()) {
            return Vec::new();
        }

        let Some(model) = models_by_name.get(name).copied() else {
            return Vec::new();
        };

        let parents = model.parents.clone();
        if parents.is_empty() {
            return vec![name.to_string()];
        }

        let mut parent_mros: Vec<Vec<String>> = parents
            .iter()
            .map(|parent| linearize(parent, models_by_name, &mut visited.clone()))
            .collect();
        parent_mros.push(parents.clone());

        let mut out = vec![name.to_string()];
        loop {
            parent_mros.retain(|list| !list.is_empty());
            if parent_mros.is_empty() {
                break;
            }

            let chosen = parent_mros.iter().find_map(|list| {
                let head = list.first()?;
                let in_tail = parent_mros
                    .iter()
                    .any(|other| other.iter().skip(1).any(|candidate| candidate == head));
                if in_tail {
                    None
                } else {
                    Some(head.clone())
                }
            });

            if let Some(head) = chosen {
                out.push(head.clone());
                for list in &mut parent_mros {
                    if list.first() == Some(&head) {
                        list.remove(0);
                    }
                }
            } else {
                for list in parent_mros {
                    for item in list {
                        if !out.contains(&item) {
                            out.push(item);
                        }
                    }
                }
                break;
            }
        }

        out
    }

    let mut visited = HashSet::new();
    let mut chain = linearize(class_name, models_by_name, &mut visited);
    if !chain.is_empty() {
        chain.remove(0);
    }
    chain
}
