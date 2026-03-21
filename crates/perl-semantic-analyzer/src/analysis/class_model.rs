//! Class model for Moose/Moo/Mouse/Class::Accessor intelligence.
//!
//! Provides a structured representation of Perl OOP class declarations,
//! including attributes, methods, inheritance, and role composition.
//! Built from AST traversal, reusing existing framework detection.

use crate::SourceLocation;
use crate::ast::{Node, NodeKind};
use std::collections::HashMap;

/// Which OO framework a package uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Framework {
    /// `use Moose;`
    Moose,
    /// `use Moo;`
    Moo,
    /// `use Mouse;`
    Mouse,
    /// `use Class::Accessor;` or `use parent 'Class::Accessor';`
    ClassAccessor,
    /// `use Object::Pad;`
    ObjectPad,
    /// Native Perl OOP (bless-based)
    Native,
    /// Native Perl 5.38+ class (use feature 'class')
    NativeClass,
    /// No OO framework detected
    None,
}

/// Accessor mode from the `is` option.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessorType {
    /// `is => 'ro'`
    Ro,
    /// `is => 'rw'`
    Rw,
    /// `is => 'lazy'` (Moo shorthand for `ro` + `lazy => 1`)
    Lazy,
    /// `is => 'bare'` (no accessor generated)
    Bare,
}

/// A Moose/Moo attribute declared via `has`.
#[derive(Debug, Clone)]
pub struct Attribute {
    /// Attribute name (e.g., `name` from `has 'name' => (...)`)
    pub name: String,
    /// Accessor mode
    pub is: Option<AccessorType>,
    /// Type constraint string (e.g., `Str`, `ArrayRef[Int]`)
    pub isa: Option<String>,
    /// Whether a default value is specified
    pub default: bool,
    /// Whether `required => 1` is set
    pub required: bool,
    /// Name of the accessor method (may differ from attribute name)
    pub accessor_name: String,
    /// Source location of the `has` declaration
    pub location: SourceLocation,
    /// Builder method name. `builder => 1` derives `_build_<attr>`, a string names the method.
    pub builder: Option<String>,
    /// Whether a coercion is applied (`coerce => 1`)
    pub coerce: bool,
    /// Predicate method name. `predicate => 1` derives `has_<attr>`.
    pub predicate: Option<String>,
    /// Clearer method name. `clearer => 1` derives `clear_<attr>`.
    pub clearer: Option<String>,
    /// Whether a trigger is set (`trigger => \&sub`)
    pub trigger: bool,
}

/// Information about a method modifier (`before`, `after`, `around`).
#[derive(Debug, Clone)]
pub struct MethodModifier {
    /// Modifier type
    pub kind: ModifierKind,
    /// Name of the method being modified
    pub method_name: String,
    /// Source location
    pub location: SourceLocation,
}

/// The type of method modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModifierKind {
    /// `before 'method' => sub { ... }`
    Before,
    /// `after 'method' => sub { ... }`
    After,
    /// `around 'method' => sub { ... }`
    Around,
}

/// Information about a method (subroutine) in a class.
#[derive(Debug, Clone)]
pub struct MethodInfo {
    /// Method name
    pub name: String,
    /// Source location of the sub declaration
    pub location: SourceLocation,
}

/// Structured model of a Perl OOP class or role.
#[derive(Debug, Clone)]
pub struct ClassModel {
    /// Package name (e.g., `MyApp::User`)
    pub name: String,
    /// Detected OO framework
    pub framework: Framework,
    /// Attributes declared via `has`
    pub attributes: Vec<Attribute>,
    /// Methods declared via `sub`
    pub methods: Vec<MethodInfo>,
    /// Parent class from `extends 'Parent'`
    pub parent: Option<String>,
    /// Roles consumed via `with 'Role'`
    pub roles: Vec<String>,
    /// Method modifiers (before/after/around)
    pub modifiers: Vec<MethodModifier>,
}

impl ClassModel {
    /// Returns true if this class uses any OO framework.
    pub fn has_framework(&self) -> bool {
        !matches!(self.framework, Framework::None)
    }
}

/// Builds `ClassModel` instances by walking an AST.
pub struct ClassModelBuilder {
    models: Vec<ClassModel>,
    current_package: String,
    current_framework: Framework,
    current_attributes: Vec<Attribute>,
    current_methods: Vec<MethodInfo>,
    current_parent: Option<String>,
    current_roles: Vec<String>,
    current_modifiers: Vec<MethodModifier>,
    /// Track which packages have framework detection applied
    framework_map: HashMap<String, Framework>,
}

impl Default for ClassModelBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ClassModelBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            models: Vec::new(),
            current_package: "main".to_string(),
            current_framework: Framework::None,
            current_attributes: Vec::new(),
            current_methods: Vec::new(),
            current_parent: None,
            current_roles: Vec::new(),
            current_modifiers: Vec::new(),
            framework_map: HashMap::new(),
        }
    }

    /// Build class models from an AST.
    pub fn build(mut self, node: &Node) -> Vec<ClassModel> {
        self.visit_node(node);
        self.flush_current_package();
        self.models
    }

    /// Flush the current package's accumulated data into a ClassModel.
    fn flush_current_package(&mut self) {
        let framework = self.current_framework;
        // Only produce a ClassModel if the package uses a framework or has attributes
        if framework != Framework::None || !self.current_attributes.is_empty() {
            let model = ClassModel {
                name: self.current_package.clone(),
                framework,
                attributes: std::mem::take(&mut self.current_attributes),
                methods: std::mem::take(&mut self.current_methods),
                parent: self.current_parent.take(),
                roles: std::mem::take(&mut self.current_roles),
                modifiers: std::mem::take(&mut self.current_modifiers),
            };
            self.models.push(model);
        } else {
            // Reset accumulators even if we don't produce a model
            self.current_attributes.clear();
            self.current_methods.clear();
            self.current_parent = None;
            self.current_roles.clear();
            self.current_modifiers.clear();
        }
    }

    fn visit_node(&mut self, node: &Node) {
        match &node.kind {
            NodeKind::Program { statements } => {
                self.visit_statement_list(statements);
            }

            NodeKind::Package { name, block, .. } => {
                // Flush previous package
                self.flush_current_package();

                self.current_package = name.clone();
                self.current_framework =
                    self.framework_map.get(name).copied().unwrap_or(Framework::None);

                if let Some(block) = block {
                    self.visit_node(block);
                }
            }

            NodeKind::Block { statements, .. } => {
                self.visit_statement_list(statements);
            }

            NodeKind::Subroutine { name, body, .. } => {
                if let Some(sub_name) = name {
                    self.current_methods
                        .push(MethodInfo { name: sub_name.clone(), location: node.location });
                }
                self.visit_node(body);
            }

            NodeKind::Use { module, args, .. } => {
                self.detect_framework(module, args);
            }

            NodeKind::Class { name, body } => {
                self.flush_current_package();
                self.current_package = name.clone();
                self.current_framework = Framework::NativeClass;
                self.framework_map.insert(name.clone(), Framework::NativeClass);
                self.visit_node(body);
            }

            NodeKind::Method { name, body, .. } => {
                self.current_methods
                    .push(MethodInfo { name: name.clone(), location: node.location });
                self.visit_node(body);
            }

            _ => {
                // Recurse into children for other node types
                self.visit_children(node);
            }
        }
    }

    fn visit_children(&mut self, node: &Node) {
        match &node.kind {
            NodeKind::ExpressionStatement { expression } => {
                self.visit_node(expression);
            }
            NodeKind::Block { statements, .. } => {
                self.visit_statement_list(statements);
            }
            NodeKind::If { condition, then_branch, else_branch, .. } => {
                self.visit_node(condition);
                self.visit_node(then_branch);
                if let Some(else_node) = else_branch {
                    self.visit_node(else_node);
                }
            }
            _ => {}
        }
    }

    fn visit_statement_list(&mut self, statements: &[Node]) {
        let mut idx = 0;
        while idx < statements.len() {
            // First, check for `use` declarations to detect frameworks
            if let NodeKind::Use { module, args, .. } = &statements[idx].kind {
                self.detect_framework(module, args);
                idx += 1;
                continue;
            }

            let is_framework_package = self.current_framework != Framework::None;

            if is_framework_package {
                // Try to extract `has` declarations
                if let Some(consumed) = self.try_extract_has(statements, idx) {
                    idx += consumed;
                    continue;
                }
                // Try to extract method modifiers
                if let Some(consumed) = self.try_extract_modifier(statements, idx) {
                    idx += consumed;
                    continue;
                }
                // Try to extract extends/with
                if let Some(consumed) = self.try_extract_extends_with(statements, idx) {
                    idx += consumed;
                    continue;
                }
            }

            // Recurse into the statement for subroutines etc.
            self.visit_node(&statements[idx]);
            idx += 1;
        }
    }

    /// Detect framework from a `use` statement.
    fn detect_framework(&mut self, module: &str, args: &[String]) {
        let framework = match module {
            "Moose" | "Moose::Role" => Framework::Moose,
            "Moo" | "Moo::Role" => Framework::Moo,
            "Mouse" | "Mouse::Role" => Framework::Mouse,
            "Class::Accessor" => Framework::ClassAccessor,
            "Object::Pad" => Framework::ObjectPad,
            "base" | "parent" => {
                let has_class_accessor = args
                    .iter()
                    .any(|a| normalize_symbol_name(a).as_deref() == Some("Class::Accessor"));
                if has_class_accessor {
                    Framework::ClassAccessor
                } else {
                    return;
                }
            }
            _ => return,
        };

        self.current_framework = framework;
        self.framework_map.insert(self.current_package.clone(), framework);
    }

    /// Extract Moo/Moose `has` declarations.
    ///
    /// Mirrors the two-statement pattern from `SymbolExtractor::try_extract_moo_has_declaration`.
    fn try_extract_has(&mut self, statements: &[Node], idx: usize) -> Option<usize> {
        let first = &statements[idx];

        // Form A: two statements
        // 1) ExpressionStatement(Identifier("has"))
        // 2) ExpressionStatement(HashLiteral(...)) or ExpressionStatement(ArrayLiteral([..., HashLiteral]))
        if idx + 1 < statements.len() {
            let second = &statements[idx + 1];
            let is_has_marker = matches!(
                &first.kind,
                NodeKind::ExpressionStatement { expression }
                    if matches!(&expression.kind, NodeKind::Identifier { name } if name == "has")
            );

            if is_has_marker {
                if let NodeKind::ExpressionStatement { expression } = &second.kind {
                    let has_location =
                        SourceLocation { start: first.location.start, end: second.location.end };

                    match &expression.kind {
                        NodeKind::HashLiteral { pairs } => {
                            self.extract_has_from_pairs(pairs, has_location, false);
                            return Some(2);
                        }
                        NodeKind::ArrayLiteral { elements } => {
                            if let Some(Node { kind: NodeKind::HashLiteral { pairs }, .. }) =
                                elements.last()
                            {
                                let mut names = Vec::new();
                                for el in elements.iter().take(elements.len() - 1) {
                                    names.extend(collect_symbol_names(el));
                                }
                                if !names.is_empty() {
                                    self.extract_has_with_names(&names, pairs, has_location);
                                    return Some(2);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Form B: single statement with embedded `has` marker
        if let NodeKind::ExpressionStatement { expression } = &first.kind
            && let NodeKind::HashLiteral { pairs } = &expression.kind
        {
            let has_embedded = pairs.iter().any(|(key_node, _)| {
                matches!(
                    &key_node.kind,
                    NodeKind::Binary { op, left, .. }
                        if op == "[]" && matches!(&left.kind, NodeKind::Identifier { name } if name == "has")
                )
            });

            if has_embedded {
                self.extract_has_from_pairs(pairs, first.location, true);
                return Some(1);
            }
        }

        // Form C: FunctionCall { name: "has", args: [name_expr, HashLiteral { ... }] }
        // Produced when the parser recognises `has 'name' => (is => 'ro', ...)` as a bare call.
        if let NodeKind::ExpressionStatement { expression } = &first.kind
            && let NodeKind::FunctionCall { name, args } = &expression.kind
            && name == "has"
            && !args.is_empty()
        {
            // The last arg that is a HashLiteral holds the options.
            let options_hash_idx =
                args.iter().rposition(|a| matches!(a.kind, NodeKind::HashLiteral { .. }));
            if let Some(opts_idx) = options_hash_idx {
                if let NodeKind::HashLiteral { pairs } = &args[opts_idx].kind {
                    let names: Vec<String> =
                        args[..opts_idx].iter().flat_map(collect_symbol_names).collect();
                    if !names.is_empty() {
                        self.extract_has_with_names(&names, pairs, first.location);
                        return Some(1);
                    }
                }
            }
        }

        None
    }

    /// Extract attributes from parsed `has` key/value pairs.
    fn extract_has_from_pairs(
        &mut self,
        pairs: &[(Node, Node)],
        location: SourceLocation,
        require_embedded: bool,
    ) {
        for (attr_expr, options_expr) in pairs {
            let attr_expr = if let NodeKind::Binary { op, left, right } = &attr_expr.kind
                && op == "[]"
                && matches!(&left.kind, NodeKind::Identifier { name } if name == "has")
            {
                right.as_ref()
            } else if require_embedded {
                continue;
            } else {
                attr_expr
            };

            let names = collect_symbol_names(attr_expr);
            if names.is_empty() {
                continue;
            }

            if let NodeKind::HashLiteral { pairs: option_pairs } = &options_expr.kind {
                self.extract_has_with_names(&names, option_pairs, location);
            }
        }
    }

    /// Build Attribute structs from attribute names and option pairs.
    fn extract_has_with_names(
        &mut self,
        names: &[String],
        option_pairs: &[(Node, Node)],
        location: SourceLocation,
    ) {
        let options = extract_hash_options(option_pairs);

        let is = options.get("is").and_then(|v| match v.as_str() {
            "ro" => Some(AccessorType::Ro),
            "rw" => Some(AccessorType::Rw),
            "lazy" => Some(AccessorType::Lazy),
            "bare" => Some(AccessorType::Bare),
            _ => None,
        });

        let isa = options.get("isa").cloned();
        let default = options.contains_key("default")
            || options.contains_key("builder")
            || is == Some(AccessorType::Lazy);
        let required = options.get("required").is_some_and(|v| v == "1" || v == "true");
        let coerce = options.get("coerce").is_some_and(|v| v == "1" || v == "true");
        let trigger = options.contains_key("trigger");

        // Determine accessor name: explicit accessor/reader overrides default
        let explicit_accessor = options.get("accessor").or_else(|| options.get("reader")).cloned();

        for name in names {
            let accessor_name = explicit_accessor.clone().unwrap_or_else(|| name.clone());

            // builder => 1 derives `_build_<attr>`; a string value names the method directly
            let builder = options
                .get("builder")
                .map(|v| if v == "1" { format!("_build_{name}") } else { v.clone() });

            // predicate => 1 derives `has_<attr>`; a string value is used directly
            let predicate = options
                .get("predicate")
                .map(|v| if v == "1" { format!("has_{name}") } else { v.clone() });

            // clearer => 1 derives `clear_<attr>`; a string value is used directly
            let clearer = options
                .get("clearer")
                .map(|v| if v == "1" { format!("clear_{name}") } else { v.clone() });

            self.current_attributes.push(Attribute {
                name: name.clone(),
                is,
                isa: isa.clone(),
                default,
                required,
                accessor_name,
                location,
                builder,
                coerce,
                predicate,
                clearer,
                trigger,
            });
        }
    }

    /// Extract method modifiers (before/after/around).
    fn try_extract_modifier(&mut self, statements: &[Node], idx: usize) -> Option<usize> {
        let first = &statements[idx];

        // FunctionCall form: `before 'save' => sub { }` parsed as a bare call.
        if let NodeKind::ExpressionStatement { expression } = &first.kind
            && let NodeKind::FunctionCall { name, args } = &expression.kind
        {
            let modifier_kind = match name.as_str() {
                "before" => Some(ModifierKind::Before),
                "after" => Some(ModifierKind::After),
                "around" => Some(ModifierKind::Around),
                _ => None,
            };
            if let Some(modifier_kind) = modifier_kind {
                // args[0] is the method name (String or ArrayLiteral), rest is the impl.
                let method_names: Vec<String> =
                    args.first().map(collect_symbol_names).unwrap_or_default();
                if !method_names.is_empty() {
                    for method_name in method_names {
                        self.current_modifiers.push(MethodModifier {
                            kind: modifier_kind,
                            method_name,
                            location: first.location,
                        });
                    }
                    return Some(1);
                }
            }
        }

        // Two-statement legacy form:
        // 1) ExpressionStatement(Identifier("before"/"after"/"around"))
        // 2) ExpressionStatement(HashLiteral((method_name, Subroutine)))
        if idx + 1 >= statements.len() {
            return None;
        }
        let second = &statements[idx + 1];

        let modifier_kind = match &first.kind {
            NodeKind::ExpressionStatement { expression } => match &expression.kind {
                NodeKind::Identifier { name } => match name.as_str() {
                    "before" => Some(ModifierKind::Before),
                    "after" => Some(ModifierKind::After),
                    "around" => Some(ModifierKind::Around),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        };

        let modifier_kind = modifier_kind?;

        let NodeKind::ExpressionStatement { expression } = &second.kind else {
            return None;
        };
        let NodeKind::HashLiteral { pairs } = &expression.kind else {
            return None;
        };

        let location = SourceLocation { start: first.location.start, end: second.location.end };

        for (key_node, _) in pairs {
            let method_names = collect_symbol_names(key_node);
            for method_name in method_names {
                self.current_modifiers.push(MethodModifier {
                    kind: modifier_kind,
                    method_name,
                    location,
                });
            }
        }

        Some(2)
    }

    /// Extract `extends 'Parent'` and `with 'Role'` declarations.
    fn try_extract_extends_with(&mut self, statements: &[Node], idx: usize) -> Option<usize> {
        let first = &statements[idx];

        // Form: FunctionCall { name: "extends"/"with", args: [...] }
        // Produced when `extends 'Parent'` / `with 'Role'` are parsed as bare calls.
        if let NodeKind::ExpressionStatement { expression } = &first.kind
            && let NodeKind::FunctionCall { name, args } = &expression.kind
            && matches!(name.as_str(), "extends" | "with")
        {
            let names: Vec<String> = args.iter().flat_map(collect_symbol_names).collect();
            if !names.is_empty() {
                if name == "extends" {
                    self.current_parent = names.into_iter().next();
                } else {
                    self.current_roles.extend(names);
                }
                return Some(1);
            }
        }

        // Two-statement form (legacy parser output):
        // 1) ExpressionStatement(Identifier("extends"/"with"))
        // 2) ExpressionStatement(String/ArrayLiteral)
        if idx + 1 >= statements.len() {
            return None;
        }
        let second = &statements[idx + 1];

        let keyword = match &first.kind {
            NodeKind::ExpressionStatement { expression } => match &expression.kind {
                NodeKind::Identifier { name } if matches!(name.as_str(), "extends" | "with") => {
                    name.as_str()
                }
                _ => return None,
            },
            _ => return None,
        };

        let NodeKind::ExpressionStatement { expression } = &second.kind else {
            return None;
        };

        let names = collect_symbol_names(expression);
        if names.is_empty() {
            return None;
        }

        if keyword == "extends" {
            // Moose/Moo only supports single inheritance via `extends`
            self.current_parent = names.into_iter().next();
        } else {
            self.current_roles.extend(names);
        }

        Some(2)
    }
}

// ---- Helper functions (parallel to SymbolExtractor's private helpers) ----

fn collect_symbol_names(node: &Node) -> Vec<String> {
    match &node.kind {
        NodeKind::String { value, .. } => normalize_symbol_name(value).into_iter().collect(),
        NodeKind::Identifier { name } => normalize_symbol_name(name).into_iter().collect(),
        NodeKind::ArrayLiteral { elements } => {
            elements.iter().flat_map(collect_symbol_names).collect()
        }
        _ => Vec::new(),
    }
}

fn normalize_symbol_name(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_matches('\'').trim_matches('"').trim();
    if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
}

fn extract_hash_options(pairs: &[(Node, Node)]) -> HashMap<String, String> {
    let mut options = HashMap::new();
    for (key_node, value_node) in pairs {
        let Some(key_name) = collect_symbol_names(key_node).into_iter().next() else {
            continue;
        };
        let value_text = value_summary(value_node);
        options.insert(key_name, value_text);
    }
    options
}

fn value_summary(node: &Node) -> String {
    match &node.kind {
        NodeKind::String { value, .. } => {
            normalize_symbol_name(value).unwrap_or_else(|| value.clone())
        }
        NodeKind::Identifier { name } => name.clone(),
        NodeKind::Number { value } => value.clone(),
        _ => "expr".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use perl_tdd_support::must;
    use std::collections::HashSet;

    fn build_models(code: &str) -> Vec<ClassModel> {
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        ClassModelBuilder::new().build(&ast)
    }

    fn find_model<'a>(models: &'a [ClassModel], name: &str) -> Option<&'a ClassModel> {
        models.iter().find(|m| m.name == name)
    }

    #[test]
    fn basic_moo_class() {
        let models = build_models(
            r#"
package MyApp::User;
use Moo;

has 'name' => (is => 'ro', isa => 'Str');
has 'age' => (is => 'rw', required => 1);

sub greet { }
"#,
        );

        let model = find_model(&models, "MyApp::User");
        assert!(model.is_some(), "expected ClassModel for MyApp::User");
        let model = model.unwrap();

        assert_eq!(model.framework, Framework::Moo);
        assert_eq!(model.attributes.len(), 2);

        let name_attr = model.attributes.iter().find(|a| a.name == "name");
        assert!(name_attr.is_some());
        let name_attr = name_attr.unwrap();
        assert_eq!(name_attr.is, Some(AccessorType::Ro));
        assert_eq!(name_attr.isa.as_deref(), Some("Str"));
        assert!(!name_attr.required);
        assert_eq!(name_attr.accessor_name, "name");

        let age_attr = model.attributes.iter().find(|a| a.name == "age");
        assert!(age_attr.is_some());
        let age_attr = age_attr.unwrap();
        assert_eq!(age_attr.is, Some(AccessorType::Rw));
        assert!(age_attr.required);

        assert!(model.methods.iter().any(|m| m.name == "greet"));
    }

    #[test]
    fn moose_extends_and_with() {
        let models = build_models(
            r#"
package MyApp::Admin;
use Moose;
extends 'MyApp::User';
with 'MyApp::Printable', 'MyApp::Serializable';

has 'level' => (is => 'ro');
"#,
        );

        let model = find_model(&models, "MyApp::Admin");
        assert!(model.is_some());
        let model = model.unwrap();

        assert_eq!(model.framework, Framework::Moose);
        assert_eq!(model.parent.as_deref(), Some("MyApp::User"));
        assert_eq!(model.roles, vec!["MyApp::Printable", "MyApp::Serializable"]);
        assert_eq!(model.attributes.len(), 1);
    }

    #[test]
    fn method_modifiers() {
        let models = build_models(
            r#"
package MyApp::User;
use Moo;
before 'save' => sub { };
after 'save' => sub { };
around 'validate' => sub { };
"#,
        );

        let model = find_model(&models, "MyApp::User");
        assert!(model.is_some());
        let model = model.unwrap();

        assert_eq!(model.modifiers.len(), 3);
        assert!(
            model
                .modifiers
                .iter()
                .any(|m| m.kind == ModifierKind::Before && m.method_name == "save")
        );
        assert!(
            model
                .modifiers
                .iter()
                .any(|m| m.kind == ModifierKind::After && m.method_name == "save")
        );
        assert!(
            model
                .modifiers
                .iter()
                .any(|m| m.kind == ModifierKind::Around && m.method_name == "validate")
        );
    }

    #[test]
    fn no_model_for_plain_package() {
        let models = build_models(
            r#"
package MyApp::Utils;
sub helper { 1 }
"#,
        );

        assert!(
            find_model(&models, "MyApp::Utils").is_none(),
            "plain package should not produce a ClassModel"
        );
    }

    #[test]
    fn multiple_packages() {
        let models = build_models(
            r#"
package MyApp::User;
use Moo;
has 'name' => (is => 'ro');

package MyApp::Admin;
use Moose;
extends 'MyApp::User';
has 'level' => (is => 'rw');

package MyApp::Utils;
sub helper { 1 }
"#,
        );

        assert_eq!(models.len(), 2, "expected 2 ClassModels (User + Admin, not Utils)");
        assert!(find_model(&models, "MyApp::User").is_some());
        assert!(find_model(&models, "MyApp::Admin").is_some());
        assert!(find_model(&models, "MyApp::Utils").is_none());
    }

    #[test]
    fn qw_attribute_list() {
        let models = build_models(
            r#"
use Moo;
has [qw(first_name last_name)] => (is => 'ro');
"#,
        );

        assert_eq!(models.len(), 1);
        let model = &models[0];
        assert_eq!(model.attributes.len(), 2);

        let names: HashSet<_> = model.attributes.iter().map(|a| a.name.as_str()).collect();
        assert!(names.contains("first_name"));
        assert!(names.contains("last_name"));
    }

    #[test]
    fn has_framework_helper() {
        let models = build_models(
            r#"
package MyApp::User;
use Moo;
has 'name' => (is => 'ro');
"#,
        );

        let model = find_model(&models, "MyApp::User").unwrap();
        assert!(model.has_framework());
    }

    #[test]
    fn accessor_type_lazy() {
        let models = build_models(
            r#"
use Moo;
has 'config' => (is => 'lazy');
"#,
        );

        let model = &models[0];
        assert_eq!(model.attributes[0].is, Some(AccessorType::Lazy));
        assert!(model.attributes[0].default, "lazy implies default");
    }

    #[test]
    fn explicit_accessor_name() {
        let models = build_models(
            r#"
use Moo;
has 'name' => (is => 'ro', reader => 'get_name');
"#,
        );

        let model = &models[0];
        assert_eq!(model.attributes[0].accessor_name, "get_name");
    }

    #[test]
    fn default_via_builder_option() {
        let models = build_models(
            r#"
use Moo;
has 'config' => (is => 'ro', builder => 1);
"#,
        );

        let model = &models[0];
        assert!(model.attributes[0].default, "builder option implies default");
    }

    #[test]
    fn lazy_builder_with_string_name() {
        let models = build_models(
            r#"
use Moo;
has 'config' => (is => 'ro', lazy => 1, builder => '_build_config');
"#,
        );

        let model = &models[0];
        let attr = &model.attributes[0];
        assert_eq!(
            attr.builder.as_deref(),
            Some("_build_config"),
            "builder string should be captured"
        );
        assert!(attr.default, "named builder implies default");
    }

    #[test]
    fn lazy_builder_with_numeric_one_generates_default_name() {
        let models = build_models(
            r#"
use Moo;
has 'profile' => (is => 'ro', builder => 1);
"#,
        );

        let model = &models[0];
        let attr = &model.attributes[0];
        assert_eq!(
            attr.builder.as_deref(),
            Some("_build_profile"),
            "builder => 1 should derive builder name as '_build_<attr>'"
        );
    }

    #[test]
    fn predicate_with_string_name() {
        let models = build_models(
            r#"
use Moo;
has 'name' => (is => 'ro', predicate => 'has_name');
"#,
        );

        let model = &models[0];
        let attr = &model.attributes[0];
        assert_eq!(
            attr.predicate.as_deref(),
            Some("has_name"),
            "predicate string name should be captured"
        );
    }

    #[test]
    fn predicate_with_numeric_one_generates_default_name() {
        let models = build_models(
            r#"
use Moo;
has 'name' => (is => 'ro', predicate => 1);
"#,
        );

        let model = &models[0];
        let attr = &model.attributes[0];
        assert_eq!(
            attr.predicate.as_deref(),
            Some("has_name"),
            "predicate => 1 should derive predicate name as 'has_<attr>'"
        );
    }

    #[test]
    fn clearer_with_string_name() {
        let models = build_models(
            r#"
use Moo;
has 'name' => (is => 'rw', clearer => 'clear_name');
"#,
        );

        let model = &models[0];
        let attr = &model.attributes[0];
        assert_eq!(
            attr.clearer.as_deref(),
            Some("clear_name"),
            "clearer string name should be captured"
        );
    }

    #[test]
    fn clearer_with_numeric_one_generates_default_name() {
        let models = build_models(
            r#"
use Moo;
has 'name' => (is => 'rw', clearer => 1);
"#,
        );

        let model = &models[0];
        let attr = &model.attributes[0];
        assert_eq!(
            attr.clearer.as_deref(),
            Some("clear_name"),
            "clearer => 1 should derive clearer name as 'clear_<attr>'"
        );
    }

    #[test]
    fn coerce_flag_true() {
        let models = build_models(
            r#"
use Moose;
has 'age' => (is => 'rw', isa => 'Int', coerce => 1);
"#,
        );

        let model = &models[0];
        let attr = &model.attributes[0];
        assert!(attr.coerce, "coerce => 1 should set coerce flag");
    }

    #[test]
    fn coerce_flag_false_when_absent() {
        let models = build_models(
            r#"
use Moose;
has 'age' => (is => 'rw', isa => 'Int');
"#,
        );

        let model = &models[0];
        let attr = &model.attributes[0];
        assert!(!attr.coerce, "coerce should be false when not specified");
    }

    #[test]
    fn trigger_flag_true() {
        let models = build_models(
            r#"
use Moose;
has 'name' => (is => 'rw', trigger => \&_on_name_change);
"#,
        );

        let model = &models[0];
        let attr = &model.attributes[0];
        assert!(attr.trigger, "trigger option should set trigger flag");
    }

    #[test]
    fn trigger_flag_false_when_absent() {
        let models = build_models(
            r#"
use Moose;
has 'name' => (is => 'rw');
"#,
        );

        let model = &models[0];
        let attr = &model.attributes[0];
        assert!(!attr.trigger, "trigger should be false when not specified");
    }

    // ── Bug 4 tests: NativeClass framework (must fail before fix) ─────────

    #[test]
    fn native_class_produces_model() {
        let models = build_models(
            r#"
class MyApp::Point {
    field $x :param = 0;
    field $y :param = 0;
    method get_x { return $x; }
    method get_y { return $y; }
}
"#,
        );
        assert_eq!(models.len(), 1, "expected one ClassModel for MyApp::Point");
        let model = &models[0];
        assert_eq!(model.name, "MyApp::Point");
        assert_eq!(model.framework, Framework::NativeClass);
        assert_eq!(model.methods.len(), 2);
        assert!(model.methods.iter().any(|m| m.name == "get_x"));
        assert!(model.methods.iter().any(|m| m.name == "get_y"));
    }

    #[test]
    fn native_class_and_moo_class_do_not_interfere() {
        let models = build_models(
            r#"
class Native::Point {
    field $x :param = 0;
    method get_x { return $x; }
}

package Moo::User;
use Moo;
has 'name' => (is => 'ro');
"#,
        );
        assert_eq!(models.len(), 2, "expected 2 ClassModels: Native::Point and Moo::User");
        let native = models.iter().find(|m| m.name == "Native::Point");
        assert!(native.is_some(), "expected Native::Point model");
        let native = native.unwrap();
        assert_eq!(native.framework, Framework::NativeClass);
        let moo = models.iter().find(|m| m.name == "Moo::User");
        assert!(moo.is_some(), "expected Moo::User model");
        let moo = moo.unwrap();
        assert_eq!(moo.framework, Framework::Moo);
    }

    #[test]
    fn all_advanced_options_together() {
        let models = build_models(
            r#"
use Moo;
has 'status' => (
    is        => 'rw',
    isa       => 'Str',
    builder   => '_build_status',
    coerce    => 1,
    predicate => 'has_status',
    clearer   => 'clear_status',
    trigger   => \&_on_status_change,
);
"#,
        );

        let model = &models[0];
        let attr = &model.attributes[0];
        assert_eq!(attr.builder.as_deref(), Some("_build_status"));
        assert!(attr.coerce);
        assert_eq!(attr.predicate.as_deref(), Some("has_status"));
        assert_eq!(attr.clearer.as_deref(), Some("clear_status"));
        assert!(attr.trigger);
    }
}
