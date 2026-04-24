//! Read-only semantic query facade spanning parser, semantic, and workspace layers.

use crate::analysis::class_model::{ClassModel, ClassModelBuilder, MethodResolutionOrder};
use crate::analysis::semantic::SemanticModel;
use crate::ast::{Node, NodeKind};
use crate::pragma_tracker::{PragmaState, PragmaTracker};
use crate::symbol::SymbolKind;
use crate::workspace_index::WorkspaceIndex;
use crate::SourceLocation;
use std::collections::{HashMap, HashSet};

/// Stable identifier for a definition that can be re-resolved in the workspace.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct DefinitionId {
    /// Canonical symbol identifier, typically package-qualified when available.
    pub symbol: String,
}

impl DefinitionId {
    /// Create a new definition identifier.
    pub fn new(symbol: impl Into<String>) -> Self {
        Self { symbol: symbol.into() }
    }
}

/// Byte-range definition location.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DefinitionLocation {
    /// Optional file URI when known.
    pub uri: Option<String>,
    /// Source location in byte offsets.
    pub location: SourceLocation,
}

/// Resolved symbol details at a source offset.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ResolvedSymbol {
    /// Symbol text name.
    pub name: String,
    /// Fully qualified symbol name where available.
    pub qualified_name: String,
    /// Stable identifier for follow-up definition queries.
    pub definition_id: DefinitionId,
    /// Symbol classification.
    pub kind: SymbolKind,
    /// Local definition location in the current document.
    pub location: DefinitionLocation,
}

/// Visible import statement in the current file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct VisibleImport {
    /// Imported module name.
    pub module: String,
    /// Import arguments from `use Module ...`.
    pub args: Vec<String>,
    /// Statement location.
    pub location: SourceLocation,
    /// Whether the import is a pragma-like lowercase module name.
    pub is_pragma_like: bool,
}

/// Parent chain for an OO package.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParentChain {
    /// Package queried.
    pub package: String,
    /// Method resolution strategy declared by the class.
    pub mro: MethodResolutionOrder,
    /// Ancestors in resolution order (local-file view).
    pub ancestors: Vec<String>,
}

/// Inherited method origin for a receiver package.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct InheritedOrigin {
    /// Receiver class/package.
    pub receiver: String,
    /// Method that was requested.
    pub method: String,
    /// Package where the method resolved.
    pub defined_in: String,
    /// Local definition location of the resolved method.
    pub location: SourceLocation,
}

/// Stable read-only pragma view at a byte offset.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct EffectivePragmaState {
    /// Effective `strict vars`.
    pub strict_vars: bool,
    /// Effective `strict subs`.
    pub strict_subs: bool,
    /// Effective `strict refs`.
    pub strict_refs: bool,
    /// Effective `warnings` global switch.
    pub warnings: bool,
    /// Effective `use utf8`.
    pub utf8: bool,
    /// Effective language features.
    pub features: Vec<String>,
    /// Builtins imported via `use builtin`.
    pub builtin_imports: Vec<String>,
}

impl From<PragmaState> for EffectivePragmaState {
    fn from(state: PragmaState) -> Self {
        Self {
            strict_vars: state.strict_vars,
            strict_subs: state.strict_subs,
            strict_refs: state.strict_refs,
            warnings: state.warnings,
            utf8: state.utf8,
            features: state.features.into_iter().map(std::string::ToString::to_string).collect(),
            builtin_imports: state.builtin_imports,
        }
    }
}

/// Narrow integration surface for read-only semantic queries.
pub struct SemanticQueryFacade<'a> {
    ast: Node,
    semantic_model: SemanticModel,
    class_models: Vec<ClassModel>,
    pragma_map: Vec<(std::ops::Range<usize>, PragmaState)>,
    workspace_index: Option<&'a WorkspaceIndex>,
}

impl<'a> SemanticQueryFacade<'a> {
    /// Build a read-only query facade from parsed AST + source.
    pub fn from_ast(ast: &Node, source: &str) -> Self {
        Self {
            ast: ast.clone(),
            semantic_model: SemanticModel::build(ast, source),
            class_models: ClassModelBuilder::new().build(ast),
            pragma_map: PragmaTracker::build(ast),
            workspace_index: None,
        }
    }

    /// Attach a workspace index for cross-file definition lookups.
    pub fn with_workspace_index(mut self, workspace_index: &'a WorkspaceIndex) -> Self {
        self.workspace_index = Some(workspace_index);
        self
    }

    /// Resolve the symbol under `offset` into a stable typed record.
    pub fn resolved_symbol_at(&self, offset: usize) -> Option<ResolvedSymbol> {
        self.semantic_model.definition_at(offset).map(|symbol| {
            let definition_id = DefinitionId::new(if symbol.qualified_name.is_empty() {
                symbol.name.clone()
            } else {
                symbol.qualified_name.clone()
            });
            ResolvedSymbol {
                name: symbol.name.clone(),
                qualified_name: symbol.qualified_name.clone(),
                kind: symbol.kind,
                location: DefinitionLocation { uri: None, location: symbol.location },
                definition_id,
            }
        })
    }

    /// Resolve a definition id to local/workspace location.
    pub fn definition_location(&self, definition_id: &DefinitionId) -> Option<DefinitionLocation> {
        if let Some(index) = self.workspace_index
            && let Some(location) = index.find_definition(&definition_id.symbol)
        {
            return Some(DefinitionLocation {
                uri: Some(location.uri),
                location: SourceLocation {
                    start: location.range.start.byte,
                    end: location.range.end.byte,
                },
            });
        }

        self.semantic_model
            .symbol_table()
            .symbols
            .get(&definition_id.symbol)
            .and_then(|symbols| symbols.first())
            .map(|symbol| DefinitionLocation { uri: None, location: symbol.location })
    }

    /// Collect `use` imports declared in this file.
    pub fn visible_imports(&self) -> Vec<VisibleImport> {
        let mut imports = Vec::new();
        Self::collect_imports(&self.ast, &mut imports);
        imports
    }

    fn collect_imports(node: &Node, imports: &mut Vec<VisibleImport>) {
        if let NodeKind::Use { module, args, .. } = &node.kind {
            imports.push(VisibleImport {
                module: module.clone(),
                args: args.clone(),
                location: node.location,
                is_pragma_like: module.chars().next().is_some_and(char::is_lowercase),
            });
        }

        for child in node.children() {
            Self::collect_imports(child, imports);
        }
    }

    /// Return local inherited parent chain for `package`.
    pub fn parent_chain(&self, package: &str) -> Option<ParentChain> {
        let map: HashMap<&str, &ClassModel> =
            self.class_models.iter().map(|model| (model.name.as_str(), model)).collect();

        let model = map.get(package).copied()?;
        let mut visited = HashSet::new();
        let mut ancestors = Vec::new();
        Self::walk_parents(package, &map, &mut visited, &mut ancestors);

        Some(ParentChain { package: package.to_string(), mro: model.mro, ancestors })
    }

    fn walk_parents(
        package: &str,
        map: &HashMap<&str, &ClassModel>,
        visited: &mut HashSet<String>,
        output: &mut Vec<String>,
    ) {
        if let Some(model) = map.get(package) {
            for parent in &model.parents {
                if visited.insert(parent.clone()) {
                    output.push(parent.clone());
                    Self::walk_parents(parent, map, visited, output);
                }
            }
        }
    }

    /// Resolve the class where `method` originates for `receiver`.
    pub fn inherited_origin(&self, receiver: &str, method: &str) -> Option<InheritedOrigin> {
        let chain = self.parent_chain(receiver)?;
        let map: HashMap<&str, &ClassModel> =
            self.class_models.iter().map(|model| (model.name.as_str(), model)).collect();

        for ancestor in chain.ancestors {
            if let Some(model) = map.get(ancestor.as_str())
                && let Some(found) = model.methods.iter().find(|candidate| candidate.name == method)
            {
                return Some(InheritedOrigin {
                    receiver: receiver.to_string(),
                    method: method.to_string(),
                    defined_in: ancestor,
                    location: found.location,
                });
            }

            if let Some(symbols) = self.semantic_model.symbol_table().symbols.get(method)
                && let Some(found) = symbols
                    .iter()
                    .find(|candidate| candidate.qualified_name == format!("{ancestor}::{method}"))
            {
                return Some(InheritedOrigin {
                    receiver: receiver.to_string(),
                    method: method.to_string(),
                    defined_in: ancestor,
                    location: found.location,
                });
            }
        }

        None
    }

    /// Return effective pragma state at `offset`.
    pub fn effective_pragma_state_at(&self, offset: usize) -> EffectivePragmaState {
        PragmaTracker::state_for_offset(&self.pragma_map, offset).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;
    use std::error::Error;
    use url::Url;

    #[test]
    fn facade_returns_visible_imports_and_pragmas() -> Result<(), Box<dyn Error>> {
        let source = "use strict;\nuse warnings;\nuse parent 'Base';\npackage Child;\n";
        let mut parser = Parser::new(source);
        let ast = parser.parse()?;

        let facade = SemanticQueryFacade::from_ast(&ast, source);
        let imports = facade.visible_imports();
        assert_eq!(imports.len(), 3);
        assert!(imports.iter().any(|item| item.module == "strict" && item.is_pragma_like));

        let state = facade.effective_pragma_state_at(source.len());
        assert!(state.strict_vars);
        assert!(state.warnings);

        Ok(())
    }

    #[test]
    fn facade_resolves_parent_chain_and_inherited_origin() -> Result<(), Box<dyn Error>> {
        let source = "package Base; sub ping { 1 }\npackage Child; use parent 'Base'; sub local { 1 }\n";
        let mut parser = Parser::new(source);
        let ast = parser.parse()?;

        let facade = SemanticQueryFacade::from_ast(&ast, source);
        let chain = facade.parent_chain("Child").ok_or("missing parent chain")?;
        assert!(chain.ancestors.iter().any(|ancestor| ancestor == "Base"));

        let origin = facade.inherited_origin("Child", "ping").ok_or("missing origin")?;
        assert_eq!(origin.defined_in, "Base");

        Ok(())
    }

    #[test]
    fn facade_resolves_workspace_definition_from_definition_id() -> Result<(), Box<dyn Error>> {
        let uri = Url::parse("file:///workspace/lib/Example.pm")?;
        let content = "package Example; sub run { return 1; } 1;";

        let index = WorkspaceIndex::new();
        index.index_file(uri.clone(), content.to_string())?;

        let mut parser = Parser::new(content);
        let ast = parser.parse()?;
        let facade = SemanticQueryFacade::from_ast(&ast, content).with_workspace_index(&index);

        let definition = facade
            .definition_location(&DefinitionId::new("Example::run"))
            .ok_or("missing definition location")?;

        assert_eq!(definition.uri.as_deref(), Some(uri.as_ref()));

        Ok(())
    }
}
