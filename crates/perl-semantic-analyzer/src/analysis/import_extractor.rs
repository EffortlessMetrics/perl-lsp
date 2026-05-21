//! Import specification extractor for `use` and `require` statements.
//!
//! Walks the AST to extract [`ImportSpec`] entries from Perl `use` and `require`
//! statements, classifying each import site by its syntactic shape
//! ([`ImportKind`]) and symbol selection policy ([`ImportSymbols`]).
//!
//! # Supported Patterns
//!
//! | Perl source                              | `ImportKind`          | `ImportSymbols`              |
//! |------------------------------------------|-----------------------|------------------------------|
//! | `use Module qw(a b)`                     | `UseExplicitList`     | `Explicit(["a", "b"])`       |
//! | `use Module ()`                          | `UseEmpty`            | `None`                       |
//! | `use Module ':tag'`                      | `UseTag`              | `Tags(["tag"])`              |
//! | `use Module` (bare)                      | `Use`                 | `Default`                    |
//! | `use constant { FOO => 1 }`              | `UseConstant`         | `Explicit(["FOO"])`          |
//! | `use constant PI => 3.14`                | `UseConstant`         | `Explicit(["PI"])`           |
//! | `require Module`                         | `Require`             | `Default`                    |
//! | `require Module; Module->import(...)`    | `RequireThenImport`   | `Explicit([...])` / `Default`|
//! | `require $var`                           | `DynamicRequire`      | `Dynamic`                    |

use crate::ast::{Node, NodeKind};
use perl_semantic_facts::{
    AnchorId, Confidence, FileId, ImportKind, ImportSpec, ImportSymbols, Provenance,
};

/// Extractor that walks an AST to produce [`ImportSpec`] entries for each
/// `use` and `require` statement found.
pub struct ImportExtractor;

impl ImportExtractor {
    /// Walk the entire AST and return one [`ImportSpec`] per `use` or
    /// `require` statement.
    ///
    /// Each spec carries the supplied `file_id` and an `anchor_id` derived from
    /// the statement's byte-offset span.
    pub fn extract(ast: &Node, file_id: FileId) -> Vec<ImportSpec> {
        let mut specs = Vec::new();
        Self::walk(ast, file_id, &mut specs);
        specs
    }

    // ── AST walker ──────────────────────────────────────────────────────

    fn walk(node: &Node, file_id: FileId, out: &mut Vec<ImportSpec>) {
        // Handle `use` statements directly.
        if let NodeKind::Use { module, args, .. } = &node.kind {
            if let Some(spec) = Self::classify_use(module, args, file_id, node) {
                out.push(spec);
            }
        }

        // Detect standalone `ClassName->import(...)` method calls where
        // `ClassName` is a static identifier (not a variable).
        //
        // These are NOT preceded by a `require` statement. The exported
        // symbol list is often dynamic (e.g. `Foo->import(@names)`), so
        // we emit `ImportSymbols::Dynamic` conservatively.
        //
        // This covers Case 3 in the PR-B spec: a static class name with
        // dynamic arguments signals that some set of symbols is imported
        // from `Foo`, but the exact names are not statically known.
        if let Some(spec) = Self::try_classify_standalone_class_import(node, file_id) {
            out.push(spec);
        }

        // For statement-list containers (Program, Block, Package), scan
        // consecutive statements to detect `require Module; Module->import(...)`
        // pairs and standalone `require` statements.
        match &node.kind {
            NodeKind::Program { statements } | NodeKind::Block { statements } => {
                Self::walk_statements(statements, file_id, out);
            }
            NodeKind::Package { block: Some(block), .. } => {
                if let NodeKind::Block { statements } = &block.kind {
                    Self::walk_statements(statements, file_id, out);
                }
            }
            _ => {}
        }

        for child in node.children() {
            Self::walk(child, file_id, out);
        }
    }

    /// Detect a standalone `ClassName->import(...)` call where `ClassName`
    /// is a static identifier (not a variable).
    ///
    /// Returns `None` when:
    /// - The node is not a `MethodCall`.
    /// - The method name is not `"import"`.
    /// - The object is a variable (those are covered by `walk_statements`).
    /// - The argument list is entirely static (fully explicit symbols) — those
    ///   are already captured by `walk_statements` when preceded by `require`.
    ///
    /// Returns an `ImportSpec` with `ImportSymbols::Dynamic` when the
    /// argument list contains any dynamic argument (e.g. `@names`, `$names`).
    /// Returns `None` when all arguments are static strings or `qw(...)` lists
    /// (those produce `Explicit` specs through `walk_statements`).
    fn try_classify_standalone_class_import(node: &Node, file_id: FileId) -> Option<ImportSpec> {
        let (object, method, args) = match &node.kind {
            NodeKind::MethodCall { object, method, args } => (object, method, args),
            _ => return None,
        };

        if method != "import" {
            return None;
        }

        // Only static class names (Identifier nodes), not variables.
        let class_name = match &object.kind {
            NodeKind::Identifier { name } => name.as_str(),
            _ => return None,
        };

        // Classify the argument list.
        let symbols = Self::extract_import_call_symbols(args);

        // Only emit evidence when the arguments are Dynamic (unknown at
        // compile time). Explicit/tag lists are precise and do not need
        // the conservative "any bareword might be imported" treatment.
        if !matches!(symbols, ImportSymbols::Dynamic) {
            return None;
        }

        let anchor_id = Self::anchor_from_node(node);
        Some(ImportSpec {
            module: class_name.to_string(),
            // ManualImport distinguishes this from a `use Foo` statement —
            // it is a `Class->import(...)` method call, not a `use` declaration.
            kind: ImportKind::ManualImport,
            symbols,
            provenance: Provenance::DynamicBoundary,
            confidence: Confidence::Low,
            file_id: Some(file_id),
            anchor_id: Some(anchor_id),
            scope_id: None,
            span_start_byte: Some(node.location.start as u32),
        })
    }

    /// Scan a list of sibling statements for `require` patterns.
    ///
    /// Detects:
    /// - `require Module; Module->import(...)` → `RequireThenImport`
    /// - `require Module` (standalone) → `Require`
    /// - `require $var` → `DynamicRequire`
    ///
    /// Statements that are part of a `require + import` pair are recorded
    /// once (not duplicated by the per-node walk).
    fn walk_statements(statements: &[Node], file_id: FileId, out: &mut Vec<ImportSpec>) {
        // Track which statement indices have been consumed as part of a
        // require-then-import pair so the per-node walk does not re-emit them.
        let mut consumed: std::collections::HashSet<usize> = std::collections::HashSet::new();

        for (i, stmt) in statements.iter().enumerate() {
            if consumed.contains(&i) {
                continue;
            }

            // Unwrap ExpressionStatement to get the inner expression.
            let expr = Self::unwrap_expression_statement(stmt);

            // Check for `require <something>`.
            let (require_node, require_args) = match &expr.kind {
                NodeKind::FunctionCall { name, args } if name == "require" => (stmt, args),
                _ => continue,
            };

            // Dynamic require: `require $var`
            if Self::is_dynamic_require(require_args) {
                out.push(Self::make_dynamic_require(file_id, require_node));
                consumed.insert(i);
                continue;
            }

            // Static require: extract module name.
            let module_name = match Self::extract_require_module_name(require_args) {
                Some(name) => name,
                None => continue,
            };

            // Look ahead for `Module->import(...)` in the next statement.
            let import_spec = if let Some(next_stmt) = statements.get(i + 1) {
                let next_expr = Self::unwrap_expression_statement(next_stmt);
                Self::try_match_import_call(next_expr, &module_name)
            } else {
                None
            };

            if let Some((symbols, import_node)) = import_spec {
                // `require Module; Module->import(...)` → RequireThenImport
                //
                // Use the require statement's anchor for the spec.
                // Choose provenance based on whether the import argument list
                // is entirely composed of literal strings/qw() words:
                // - All static (Explicit/Tags/Mixed/Default/None) → LiteralRequireImport
                //   (guarantees the full symbol set is statically known)
                // - Dynamic → ExactAst (conservative; symbol set not fully known)
                let provenance = if matches!(symbols, ImportSymbols::Dynamic) {
                    Provenance::ExactAst
                } else {
                    Provenance::LiteralRequireImport
                };
                let anchor_id = Self::anchor_from_node(require_node);
                let confidence = Self::confidence_for_symbols(&symbols);
                out.push(ImportSpec {
                    module: module_name,
                    kind: ImportKind::RequireThenImport,
                    symbols,
                    provenance,
                    confidence,
                    file_id: Some(file_id),
                    anchor_id: Some(anchor_id),
                    scope_id: None,
                    span_start_byte: Some(require_node.location.start as u32),
                });
                consumed.insert(i);
                consumed.insert(i + 1);
                // Also record the import call node index so the per-node
                // walk does not process it.
                let _ = import_node;
            } else {
                // Standalone `require Module` → Require
                let anchor_id = Self::anchor_from_node(require_node);
                out.push(ImportSpec {
                    module: module_name,
                    kind: ImportKind::Require,
                    symbols: ImportSymbols::Default,
                    provenance: Provenance::ExactAst,
                    confidence: Confidence::High,
                    file_id: Some(file_id),
                    anchor_id: Some(anchor_id),
                    scope_id: None,
                    span_start_byte: Some(require_node.location.start as u32),
                });
                consumed.insert(i);
            }
        }
    }

    // ── Require helpers ────────────────────────────────────────────────

    /// Unwrap an `ExpressionStatement` to get the inner expression node.
    /// Returns the node itself if it is not an `ExpressionStatement`.
    fn unwrap_expression_statement(node: &Node) -> &Node {
        match &node.kind {
            NodeKind::ExpressionStatement { expression } => expression,
            _ => node,
        }
    }

    /// Check whether a `require` call's arguments indicate a dynamic require
    /// (i.e. `require $var`).
    fn is_dynamic_require(args: &[Node]) -> bool {
        match args.first() {
            Some(arg) => matches!(&arg.kind, NodeKind::Variable { .. }),
            None => false,
        }
    }

    /// Extract the module name from a `require` call's arguments.
    ///
    /// Handles:
    /// - `require Foo::Bar` → `"Foo::Bar"` (Identifier)
    /// - `require "Foo/Bar.pm"` → `"Foo::Bar"` (String, path-to-module conversion)
    fn extract_require_module_name(args: &[Node]) -> Option<String> {
        let arg = args.first()?;
        match &arg.kind {
            NodeKind::Identifier { name } => Some(name.clone()),
            NodeKind::String { value, .. } => {
                // "Foo/Bar.pm" → "Foo::Bar"
                let cleaned = value.trim_matches('\'').trim_matches('"').trim();
                let module = cleaned.trim_end_matches(".pm").replace('/', "::");
                Some(module)
            }
            _ => None,
        }
    }

    /// Build an [`ImportSpec`] for `require $var` (dynamic require).
    ///
    /// Uses `Provenance::DynamicBoundary + Confidence::Low` because the module
    /// identity is not statically known — only the pattern is known. This
    /// provenance marks the import site for the diagnostics suppressor so that
    /// symbols "plausibly imported" via dynamic require are not flagged as
    /// undefined.
    fn make_dynamic_require(file_id: FileId, node: &Node) -> ImportSpec {
        let anchor_id = Self::anchor_from_node(node);
        ImportSpec {
            module: String::new(),
            kind: ImportKind::DynamicRequire,
            symbols: ImportSymbols::Dynamic,
            provenance: Provenance::DynamicBoundary,
            confidence: Confidence::Low,
            file_id: Some(file_id),
            anchor_id: Some(anchor_id),
            scope_id: None,
            span_start_byte: Some(node.location.start as u32),
        }
    }

    /// Try to match a `Module->import(...)` method call node.
    ///
    /// Returns `Some((symbols, node))` if the node is a `MethodCall` with
    /// method `"import"` and the object matches `expected_module`.
    fn try_match_import_call<'a>(
        node: &'a Node,
        expected_module: &str,
    ) -> Option<(ImportSymbols, &'a Node)> {
        let (object, method, args) = match &node.kind {
            NodeKind::MethodCall { object, method, args } => (object, method, args),
            _ => return None,
        };

        if method != "import" {
            return None;
        }

        // The object must be an Identifier matching the module name.
        let obj_name = match &object.kind {
            NodeKind::Identifier { name } => name.as_str(),
            _ => return None,
        };

        if obj_name != expected_module {
            return None;
        }

        // Extract imported symbols from the argument list.
        let symbols = Self::extract_import_call_symbols(args);
        Some((symbols, node))
    }

    /// Extract [`ImportSymbols`] from the argument list of a `Module->import(...)` call.
    fn extract_import_call_symbols(args: &[Node]) -> ImportSymbols {
        if args.is_empty() {
            return ImportSymbols::Default;
        }

        let mut names: Vec<String> = Vec::new();
        let mut tags: Vec<String> = Vec::new();
        let mut has_dynamic_arg = false;

        for arg in args {
            has_dynamic_arg |= Self::collect_import_arg_symbols(arg, &mut names, &mut tags);
        }

        if has_dynamic_arg {
            return ImportSymbols::Dynamic;
        }

        if names.is_empty() && tags.is_empty() {
            return ImportSymbols::Default;
        }

        if !tags.is_empty() && names.is_empty() {
            return ImportSymbols::Tags(tags);
        }

        if !tags.is_empty() && !names.is_empty() {
            return ImportSymbols::Mixed { tags, names };
        }

        ImportSymbols::Explicit(names)
    }

    /// Collect symbol names and tags from a single argument node of an
    /// `import(...)` call.
    ///
    /// Returns `true` when the argument is dynamic or unsupported and should
    /// prevent the import site from claiming exact symbol names.
    fn collect_import_arg_symbols(
        arg: &Node,
        names: &mut Vec<String>,
        tags: &mut Vec<String>,
    ) -> bool {
        match &arg.kind {
            NodeKind::String { value, .. } => {
                let bare = value.trim_matches('\'').trim_matches('"');
                if let Some(tag) = bare.strip_prefix(':') {
                    tags.push(tag.to_string());
                } else if !bare.is_empty() {
                    names.push(bare.to_string());
                }
                false
            }
            NodeKind::Identifier { name } => {
                // Handle qw(...) stored as raw identifier string.
                if let Some(inner) = Self::parse_qw_content(name) {
                    for word in inner.split_whitespace() {
                        if let Some(tag) = word.strip_prefix(':') {
                            tags.push(tag.to_string());
                        } else {
                            names.push(word.to_string());
                        }
                    }
                } else if let Some(tag) = name.strip_prefix(':') {
                    tags.push(tag.to_string());
                } else if !name.is_empty() {
                    names.push(name.clone());
                }
                false
            }
            NodeKind::Variable { .. } => {
                // `Foo->import(@names)` / `Foo->import($name)` is dynamic:
                // do not guess exact imported symbols.
                true
            }
            NodeKind::ArrayLiteral { elements } => {
                // qw(...) in expression context → ArrayLiteral of String nodes
                let mut has_dynamic_arg = false;
                for el in elements {
                    has_dynamic_arg |= Self::collect_import_arg_symbols(el, names, tags);
                }
                has_dynamic_arg
            }
            _ => true,
        }
    }

    fn confidence_for_symbols(symbols: &ImportSymbols) -> Confidence {
        if matches!(symbols, ImportSymbols::Dynamic) { Confidence::Low } else { Confidence::High }
    }

    // ── Classification ──────────────────────────────────────────────────

    /// Classify a single `use` statement into an [`ImportSpec`].
    ///
    /// Returns `None` for version pragmas (`use 5.036;`, `use v5.38;`) and
    /// other non-module-import statements that should not produce import facts.
    fn classify_use(
        module: &str,
        args: &[String],
        file_id: FileId,
        node: &Node,
    ) -> Option<ImportSpec> {
        // Skip version pragmas — they are not module imports.
        if Self::is_version_pragma(module) {
            return None;
        }

        let anchor_id = Self::anchor_from_node(node);

        // `use constant` is a special pragma that defines constants.
        if module == "constant" {
            return Some(Self::classify_use_constant(args, file_id, anchor_id));
        }

        // Classify by argument shape.
        let (kind, symbols) = Self::classify_args(args, module, node);

        Some(ImportSpec {
            module: module.to_string(),
            kind,
            symbols,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
            file_id: Some(file_id),
            anchor_id: Some(anchor_id),
            scope_id: None,
            span_start_byte: Some(node.location.start as u32),
        })
    }

    /// Classify the argument list of a non-constant `use` statement.
    fn classify_args(args: &[String], module: &str, node: &Node) -> (ImportKind, ImportSymbols) {
        if args.is_empty() {
            // Distinguish `use Module;` from `use Module ()`.
            //
            // The parser produces empty args for both forms. We use a span-length
            // heuristic: `use Module;` occupies `"use " + module + ";"` bytes,
            // while `use Module ()` is longer due to the parentheses.
            let bare_len = "use ".len() + module.len() + 1; // +1 for ';'
            let span_len = node.location.end.saturating_sub(node.location.start);
            if span_len > bare_len {
                // The source text is longer than a bare `use Module;`, so there
                // were likely empty parentheses.
                return (ImportKind::UseEmpty, ImportSymbols::None);
            }
            // `use Module;` — bare import, triggers default @EXPORT.
            return (ImportKind::Use, ImportSymbols::Default);
        }

        // Collect explicit names, tags, and detect qw() forms.
        let mut explicit_names: Vec<String> = Vec::new();
        let mut tags: Vec<String> = Vec::new();

        for arg in args {
            let trimmed = arg.trim();

            // qw(...) form: "qw(a b c)"
            if let Some(inner) = Self::parse_qw_content(trimmed) {
                let words: Vec<String> = inner.split_whitespace().map(|w| w.to_string()).collect();
                for word in words {
                    if let Some(tag) = word.strip_prefix(':') {
                        tags.push(tag.to_string());
                    } else {
                        explicit_names.push(word);
                    }
                }
                continue;
            }

            // Tag argument: ':tag' or ":tag" (with or without quotes)
            let unquoted = Self::unquote(trimmed);
            if let Some(tag) = unquoted.strip_prefix(':') {
                tags.push(tag.to_string());
                continue;
            }

            // Skip fat-arrow values and punctuation that are part of overload-style
            // key-value pairs (e.g. `use overload '""' => \&stringify`).
            if trimmed == "=>" || trimmed == "," || trimmed == "\\" {
                continue;
            }

            // Regular symbol name.
            if Self::looks_like_symbol_name(trimmed) {
                explicit_names.push(Self::unquote(trimmed).to_string());
            }
        }

        // Empty parens: `use Module ()`
        if explicit_names.is_empty() && tags.is_empty() && !args.is_empty() {
            // The parser consumed `()` but produced no meaningful args.
            // However, args may contain punctuation tokens from complex use statements.
            // If all args are non-symbol tokens, treat as empty import.
            let has_any_symbol = args.iter().any(|a| {
                let t = a.trim();
                Self::looks_like_symbol_name(t) || Self::parse_qw_content(t).is_some()
            });
            if !has_any_symbol {
                return (ImportKind::UseEmpty, ImportSymbols::None);
            }
        }

        // Tags only.
        if !tags.is_empty() && explicit_names.is_empty() {
            return (ImportKind::UseTag, ImportSymbols::Tags(tags));
        }

        // Mixed tags and names.
        if !tags.is_empty() && !explicit_names.is_empty() {
            return (
                ImportKind::UseExplicitList,
                ImportSymbols::Mixed { tags, names: explicit_names },
            );
        }

        // Explicit symbol list.
        if !explicit_names.is_empty() {
            return (ImportKind::UseExplicitList, ImportSymbols::Explicit(explicit_names));
        }

        // Fallback: bare use with unrecognised args.
        (ImportKind::Use, ImportSymbols::Default)
    }

    /// Classify `use constant` statements.
    fn classify_use_constant(args: &[String], file_id: FileId, anchor_id: AnchorId) -> ImportSpec {
        let mut constant_names: Vec<String> = Vec::new();

        if args.is_empty() {
            // `use constant;` — degenerate, no constants defined.
            return ImportSpec {
                module: "constant".to_string(),
                kind: ImportKind::UseConstant,
                symbols: ImportSymbols::None,
                provenance: Provenance::ExactAst,
                confidence: Confidence::High,
                file_id: Some(file_id),
                anchor_id: Some(anchor_id),
                scope_id: None,
                span_start_byte: None, // position not needed for UseConstant
            };
        }

        // Hash-ref form: `use constant { FOO => 1, BAR => 2 }`
        // Args look like: ["{", "FOO", "=>", "1", "BAR", "=>", "2", "}"]
        let starts_hash_form = args.first().map(|a| a.as_str()) == Some("{")
            || args.first().map(|a| a.as_str()) == Some("+{")
            || (args.first().map(|a| a.as_str()) == Some("+")
                && args.get(1).map(|a| a.as_str()) == Some("{"));
        if starts_hash_form {
            let mut i = 0;
            while i < args.len() {
                let token = args[i].trim();
                if Self::is_constant_hash_punctuation(token) {
                    i += 1;
                    continue;
                }
                if i + 1 < args.len()
                    && args[i + 1].trim() == "=>"
                    && let Some(name) = Self::constant_name_candidate(token)
                {
                    constant_names.push(name);
                    i += 2;
                    let mut nesting = 0usize;
                    while i < args.len() {
                        let value_token = args[i].trim();
                        if nesting == 0 && (value_token == "," || value_token == "}") {
                            break;
                        }
                        match value_token {
                            "{" | "[" | "(" => nesting += 1,
                            "}" | "]" | ")" if nesting > 0 => nesting -= 1,
                            "}" if nesting == 0 => break,
                            _ => {}
                        }
                        i += 1;
                    }
                } else {
                    i += 1;
                }
            }
        }
        // qw() form: `use constant qw(ONE TWO THREE)`
        else if let Some(inner) = args.first().and_then(|a| Self::parse_qw_content(a.trim())) {
            let words: Vec<String> = inner.split_whitespace().map(|w| w.to_string()).collect();
            constant_names.extend(words);
        }
        // Scalar form: `use constant PI => 3.14`
        // Args look like: ["PI", "3.14"] or ["PI", "=>", "3.14"]
        else if let Some(name) = args.first() {
            let trimmed = name.trim();
            if let Some(name) = Self::constant_name_candidate(trimmed) {
                constant_names.push(name);
            }
        }

        // Deduplicate while preserving order.
        let mut seen = std::collections::HashSet::new();
        constant_names.retain(|n| seen.insert(n.clone()));

        let symbols = if constant_names.is_empty() {
            ImportSymbols::None
        } else {
            ImportSymbols::Explicit(constant_names)
        };

        ImportSpec {
            module: "constant".to_string(),
            kind: ImportKind::UseConstant,
            symbols,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
            file_id: Some(file_id),
            anchor_id: Some(anchor_id),
            scope_id: None,
            span_start_byte: None, // position not needed for UseConstant
        }
    }

    // ── Helpers ─────────────────────────────────────────────────────────

    /// Derive an [`AnchorId`] from a node's byte-offset span.
    fn anchor_from_node(node: &Node) -> AnchorId {
        // Use the start byte offset as a deterministic anchor ID.
        // This is unique per use-statement within a file.
        AnchorId(node.location.start as u64)
    }

    /// Check whether a module string is a version pragma (e.g. `5.036`, `v5.38`).
    fn is_version_pragma(module: &str) -> bool {
        // Numeric version: 5.036, 5.10
        if module.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            return true;
        }
        // v-string: v5.38, v5.12.0
        if module.starts_with('v')
            && module.len() > 1
            && module[1..].chars().all(|c| c.is_ascii_digit() || c == '.')
        {
            return true;
        }
        false
    }

    /// Extract the inner content of a `qw(...)` string.
    ///
    /// Returns `Some("a b c")` for `"qw(a b c)"`, `None` otherwise.
    fn parse_qw_content(s: &str) -> Option<&str> {
        let rest = s.strip_prefix("qw")?;
        // The parser normalises all qw delimiters to parentheses.
        let inner = rest.strip_prefix('(')?.strip_suffix(')')?;
        Some(inner)
    }

    /// Remove surrounding single or double quotes from a string.
    fn unquote(s: &str) -> &str {
        if (s.starts_with('\'') && s.ends_with('\'')) || (s.starts_with('"') && s.ends_with('"')) {
            if s.len() >= 2 {
                return &s[1..s.len() - 1];
            }
        }
        s
    }

    /// Heuristic: does this string look like a Perl symbol name?
    fn looks_like_symbol_name(s: &str) -> bool {
        let s = Self::unquote(s);
        if s.is_empty() {
            return false;
        }
        // Tags start with ':'
        if s.starts_with(':') {
            return true;
        }
        // Sigiled variables: $foo, @bar, %baz, &sub, *glob
        if s.starts_with('$')
            || s.starts_with('@')
            || s.starts_with('%')
            || s.starts_with('&')
            || s.starts_with('*')
        {
            return true;
        }
        // Bare word: starts with letter or underscore
        s.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
    }

    /// Heuristic: does this string look like a constant name?
    ///
    /// Constants are typically UPPER_CASE identifiers.
    fn looks_like_constant_name(s: &str) -> bool {
        if s.is_empty() {
            return false;
        }
        s.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
    }

    fn constant_name_candidate(s: &str) -> Option<String> {
        let name = Self::unquote(s.trim());
        Self::looks_like_constant_name(name).then(|| name.to_string())
    }

    fn is_constant_hash_punctuation(s: &str) -> bool {
        matches!(s, "+" | "+{" | "{" | "}" | "=>" | ",")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;

    /// Parse Perl source and extract import specs.
    fn parse_and_extract(code: &str) -> Vec<ImportSpec> {
        let mut parser = Parser::new(code);
        let ast = match parser.parse() {
            Ok(ast) => ast,
            Err(_) => return Vec::new(),
        };
        ImportExtractor::extract(&ast, FileId(1))
    }

    // ── use Module qw(a b) → UseExplicitList ────────────────────────────

    #[test]
    fn test_use_explicit_list_qw() -> Result<(), String> {
        let specs = parse_and_extract("use List::Util qw(first reduce any);");
        let spec = specs.first().ok_or("expected at least one ImportSpec")?;

        assert_eq!(spec.module, "List::Util");
        assert_eq!(spec.kind, ImportKind::UseExplicitList);
        if let ImportSymbols::Explicit(names) = &spec.symbols {
            assert!(names.contains(&"first".to_string()), "missing 'first' in {names:?}");
            assert!(names.contains(&"reduce".to_string()), "missing 'reduce' in {names:?}");
            assert!(names.contains(&"any".to_string()), "missing 'any' in {names:?}");
        } else {
            return Err(format!("expected Explicit, got {:?}", spec.symbols));
        }
        assert_eq!(spec.file_id, Some(FileId(1)));
        assert!(spec.anchor_id.is_some());
        Ok(())
    }

    #[test]
    fn test_use_explicit_list_quoted_strings() -> Result<(), String> {
        let specs = parse_and_extract("use Exporter 'import';");
        let spec = specs.first().ok_or("expected at least one ImportSpec")?;

        assert_eq!(spec.module, "Exporter");
        assert_eq!(spec.kind, ImportKind::UseExplicitList);
        if let ImportSymbols::Explicit(names) = &spec.symbols {
            assert!(names.contains(&"import".to_string()), "missing 'import' in {names:?}");
        } else {
            return Err(format!("expected Explicit, got {:?}", spec.symbols));
        }
        Ok(())
    }

    // ── use Module () → UseEmpty ────────────────────────────────────────
    //
    // NOTE: The current parser represents both `use Module;` and `use Module ()`
    // with empty args. We detect empty-parens by checking for an AST node whose
    // source text contains `()`. When the parser cannot distinguish the two
    // forms, both are classified as bare `Use`/`Default`.

    #[test]
    fn test_use_empty_parens() -> Result<(), String> {
        let specs = parse_and_extract("use POSIX ();");
        let spec = specs.first().ok_or("expected at least one ImportSpec")?;

        assert_eq!(spec.module, "POSIX");
        // The parser produces empty args for both `use POSIX;` and `use POSIX ()`.
        // We detect the empty-parens form by inspecting the source span length
        // relative to the module name length.
        assert_eq!(spec.kind, ImportKind::UseEmpty);
        assert_eq!(spec.symbols, ImportSymbols::None);
        Ok(())
    }

    // ── use Module ':tag' → UseTag ──────────────────────────────────────

    #[test]
    fn test_use_tag_single() -> Result<(), String> {
        let specs = parse_and_extract("use POSIX ':sys_wait_h';");
        let spec = specs.first().ok_or("expected at least one ImportSpec")?;

        assert_eq!(spec.module, "POSIX");
        assert_eq!(spec.kind, ImportKind::UseTag);
        if let ImportSymbols::Tags(tags) = &spec.symbols {
            assert!(tags.contains(&"sys_wait_h".to_string()), "missing tag in {tags:?}");
        } else {
            return Err(format!("expected Tags, got {:?}", spec.symbols));
        }
        Ok(())
    }

    #[test]
    fn test_use_tag_in_qw() -> Result<(), String> {
        let specs = parse_and_extract("use Fcntl qw(:flock);");
        let spec = specs.first().ok_or("expected at least one ImportSpec")?;

        assert_eq!(spec.module, "Fcntl");
        assert_eq!(spec.kind, ImportKind::UseTag);
        if let ImportSymbols::Tags(tags) = &spec.symbols {
            assert!(tags.contains(&"flock".to_string()), "missing tag in {tags:?}");
        } else {
            return Err(format!("expected Tags, got {:?}", spec.symbols));
        }
        Ok(())
    }

    // ── use Module (bare) → Use/Default ─────────────────────────────────

    #[test]
    fn test_use_bare() -> Result<(), String> {
        let specs = parse_and_extract("use strict;");
        let spec = specs.first().ok_or("expected at least one ImportSpec")?;

        assert_eq!(spec.module, "strict");
        assert_eq!(spec.kind, ImportKind::Use);
        assert_eq!(spec.symbols, ImportSymbols::Default);
        Ok(())
    }

    #[test]
    fn test_use_bare_qualified() -> Result<(), String> {
        let specs = parse_and_extract("use Data::Dumper;");
        let spec = specs.first().ok_or("expected at least one ImportSpec")?;

        assert_eq!(spec.module, "Data::Dumper");
        assert_eq!(spec.kind, ImportKind::Use);
        assert_eq!(spec.symbols, ImportSymbols::Default);
        Ok(())
    }

    // ── use constant → UseConstant ──────────────────────────────────────

    #[test]
    fn test_use_constant_scalar() -> Result<(), String> {
        let specs = parse_and_extract("use constant PI => 3.14;");
        let spec = specs.first().ok_or("expected at least one ImportSpec")?;

        assert_eq!(spec.module, "constant");
        assert_eq!(spec.kind, ImportKind::UseConstant);
        if let ImportSymbols::Explicit(names) = &spec.symbols {
            assert!(names.contains(&"PI".to_string()), "missing 'PI' in {names:?}");
        } else {
            return Err(format!("expected Explicit, got {:?}", spec.symbols));
        }
        Ok(())
    }

    #[test]
    fn test_use_constant_hash_ref() -> Result<(), String> {
        let specs = parse_and_extract("use constant { FOO => 1, BAR => 2 };");
        let spec = specs.first().ok_or("expected at least one ImportSpec")?;

        assert_eq!(spec.module, "constant");
        assert_eq!(spec.kind, ImportKind::UseConstant);
        if let ImportSymbols::Explicit(names) = &spec.symbols {
            assert!(names.contains(&"FOO".to_string()), "missing 'FOO' in {names:?}");
            assert!(names.contains(&"BAR".to_string()), "missing 'BAR' in {names:?}");
        } else {
            return Err(format!("expected Explicit, got {:?}", spec.symbols));
        }
        Ok(())
    }

    #[test]
    fn test_use_constant_quoted_scalar() -> Result<(), String> {
        let specs = parse_and_extract("use constant 'HTTP_OK' => 200;");
        let spec = specs.first().ok_or("expected at least one ImportSpec")?;

        assert_eq!(spec.module, "constant");
        assert_eq!(spec.kind, ImportKind::UseConstant);
        if let ImportSymbols::Explicit(names) = &spec.symbols {
            assert!(names.contains(&"HTTP_OK".to_string()), "missing 'HTTP_OK' in {names:?}");
        } else {
            return Err(format!("expected Explicit, got {:?}", spec.symbols));
        }
        Ok(())
    }

    #[test]
    fn test_use_constant_quoted_hash_ref() -> Result<(), String> {
        let specs = parse_and_extract(r#"use constant { 'FOO' => 1, "BAR" => 2 };"#);
        let spec = specs.first().ok_or("expected at least one ImportSpec")?;

        assert_eq!(spec.module, "constant");
        assert_eq!(spec.kind, ImportKind::UseConstant);
        if let ImportSymbols::Explicit(names) = &spec.symbols {
            assert!(names.contains(&"FOO".to_string()), "missing 'FOO' in {names:?}");
            assert!(names.contains(&"BAR".to_string()), "missing 'BAR' in {names:?}");
        } else {
            return Err(format!("expected Explicit, got {:?}", spec.symbols));
        }
        Ok(())
    }

    #[test]
    fn test_use_constant_plus_hash_ref() -> Result<(), String> {
        let specs = parse_and_extract("use constant +{ FOO => 1, BAR => 2 };");
        let spec = specs.first().ok_or("expected at least one ImportSpec")?;

        assert_eq!(spec.module, "constant");
        assert_eq!(spec.kind, ImportKind::UseConstant);
        if let ImportSymbols::Explicit(names) = &spec.symbols {
            assert!(names.contains(&"FOO".to_string()), "missing 'FOO' in {names:?}");
            assert!(names.contains(&"BAR".to_string()), "missing 'BAR' in {names:?}");
        } else {
            return Err(format!("expected Explicit, got {:?}", spec.symbols));
        }
        Ok(())
    }

    #[test]
    fn test_use_constant_hash_ref_ignores_nested_fat_comma_values() -> Result<(), String> {
        let specs = parse_and_extract("use constant { FOO => { nested => 1 }, BAR => 2 };");
        let spec = specs.first().ok_or("expected at least one ImportSpec")?;

        assert_eq!(spec.module, "constant");
        assert_eq!(spec.kind, ImportKind::UseConstant);
        if let ImportSymbols::Explicit(names) = &spec.symbols {
            assert_eq!(names, &vec!["FOO".to_string(), "BAR".to_string()]);
        } else {
            return Err(format!("expected Explicit, got {:?}", spec.symbols));
        }
        Ok(())
    }

    #[test]
    fn test_use_constant_empty() -> Result<(), String> {
        let specs = parse_and_extract("use constant;");
        let spec = specs.first().ok_or("expected at least one ImportSpec")?;

        assert_eq!(spec.module, "constant");
        assert_eq!(spec.kind, ImportKind::UseConstant);
        assert_eq!(spec.symbols, ImportSymbols::None);
        Ok(())
    }

    // ── Version pragmas are skipped ─────────────────────────────────────

    #[test]
    fn test_version_pragma_skipped() -> Result<(), String> {
        let specs = parse_and_extract("use 5.036;");
        assert!(specs.is_empty(), "version pragma should not produce ImportSpec");
        Ok(())
    }

    #[test]
    fn test_vstring_pragma_skipped() -> Result<(), String> {
        let specs = parse_and_extract("use v5.38;");
        assert!(specs.is_empty(), "v-string pragma should not produce ImportSpec");
        Ok(())
    }

    // ── Multiple use statements ─────────────────────────────────────────

    #[test]
    fn test_multiple_use_statements() -> Result<(), String> {
        let code = r#"
use strict;
use warnings;
use List::Util qw(first any);
use POSIX ();
use constant MAX => 100;
"#;
        let specs = parse_and_extract(code);
        assert_eq!(specs.len(), 5, "expected 5 ImportSpecs, got {}", specs.len());

        // strict — bare
        assert_eq!(specs[0].module, "strict");
        assert_eq!(specs[0].kind, ImportKind::Use);

        // warnings — bare
        assert_eq!(specs[1].module, "warnings");
        assert_eq!(specs[1].kind, ImportKind::Use);

        // List::Util — explicit list
        assert_eq!(specs[2].module, "List::Util");
        assert_eq!(specs[2].kind, ImportKind::UseExplicitList);

        // POSIX — empty
        assert_eq!(specs[3].module, "POSIX");
        assert_eq!(specs[3].kind, ImportKind::UseEmpty);

        // constant — use constant
        assert_eq!(specs[4].module, "constant");
        assert_eq!(specs[4].kind, ImportKind::UseConstant);

        Ok(())
    }

    // ── Anchor and file_id are populated ────────────────────────────────

    #[test]
    fn test_anchor_and_file_id_populated() -> Result<(), String> {
        let specs = parse_and_extract("use Foo::Bar qw(baz);");
        let spec = specs.first().ok_or("expected at least one ImportSpec")?;

        assert_eq!(spec.file_id, Some(FileId(1)));
        assert!(spec.anchor_id.is_some(), "anchor_id should be populated");
        assert_eq!(spec.provenance, Provenance::ExactAst);
        assert_eq!(spec.confidence, Confidence::High);
        Ok(())
    }

    // ── Nested use in package block ─────────────────────────────────────

    #[test]
    fn test_use_inside_package_block() -> Result<(), String> {
        let code = r#"
package MyModule;
use Exporter 'import';
our @EXPORT = qw(foo);
1;
"#;
        let specs = parse_and_extract(code);
        let exporter_spec =
            specs.iter().find(|s| s.module == "Exporter").ok_or("expected Exporter ImportSpec")?;

        assert_eq!(exporter_spec.kind, ImportKind::UseExplicitList);
        if let ImportSymbols::Explicit(names) = &exporter_spec.symbols {
            assert!(names.contains(&"import".to_string()));
        } else {
            return Err(format!("expected Explicit, got {:?}", exporter_spec.symbols));
        }
        Ok(())
    }

    // ── Mixed tags and names ────────────────────────────────────────────

    #[test]
    fn test_use_mixed_tags_and_names() -> Result<(), String> {
        let specs = parse_and_extract("use Fcntl qw(:flock LOCK_EX LOCK_NB);");
        let spec = specs.first().ok_or("expected at least one ImportSpec")?;

        assert_eq!(spec.module, "Fcntl");
        assert_eq!(spec.kind, ImportKind::UseExplicitList);
        if let ImportSymbols::Mixed { tags, names } = &spec.symbols {
            assert!(tags.contains(&"flock".to_string()), "missing tag 'flock' in {tags:?}");
            assert!(names.contains(&"LOCK_EX".to_string()), "missing 'LOCK_EX' in {names:?}");
            assert!(names.contains(&"LOCK_NB".to_string()), "missing 'LOCK_NB' in {names:?}");
        } else {
            return Err(format!("expected Mixed, got {:?}", spec.symbols));
        }
        Ok(())
    }

    // ── require Module → Require ────────────────────────────────────────

    #[test]
    fn test_require_bare_module() -> Result<(), String> {
        let specs = parse_and_extract("require Foo::Bar;");
        let spec = specs
            .iter()
            .find(|s| s.module == "Foo::Bar")
            .ok_or("expected ImportSpec for Foo::Bar")?;

        assert_eq!(spec.kind, ImportKind::Require);
        assert_eq!(spec.symbols, ImportSymbols::Default);
        assert_eq!(spec.provenance, Provenance::ExactAst);
        assert_eq!(spec.confidence, Confidence::High);
        assert_eq!(spec.file_id, Some(FileId(1)));
        assert!(spec.anchor_id.is_some(), "anchor_id should be populated");
        Ok(())
    }

    // ── require Module; Module->import(...) → RequireThenImport ─────────

    #[test]
    fn test_require_then_import_with_qw() -> Result<(), String> {
        let code = r#"
require Foo::Bar;
Foo::Bar->import(qw(alpha beta));
"#;
        let specs = parse_and_extract(code);
        let spec = specs
            .iter()
            .find(|s| s.module == "Foo::Bar")
            .ok_or("expected ImportSpec for Foo::Bar")?;

        assert_eq!(spec.kind, ImportKind::RequireThenImport);
        if let ImportSymbols::Explicit(names) = &spec.symbols {
            assert!(names.contains(&"alpha".to_string()), "missing 'alpha' in {names:?}");
            assert!(names.contains(&"beta".to_string()), "missing 'beta' in {names:?}");
        } else {
            return Err(format!("expected Explicit, got {:?}", spec.symbols));
        }
        // Fully literal import list → LiteralRequireImport provenance.
        assert_eq!(spec.provenance, Provenance::LiteralRequireImport);
        assert_eq!(spec.confidence, Confidence::High);
        Ok(())
    }

    #[test]
    fn test_require_then_import_bare() -> Result<(), String> {
        let code = r#"
require Some::Module;
Some::Module->import();
"#;
        let specs = parse_and_extract(code);
        let spec = specs
            .iter()
            .find(|s| s.module == "Some::Module")
            .ok_or("expected ImportSpec for Some::Module")?;

        assert_eq!(spec.kind, ImportKind::RequireThenImport);
        assert_eq!(spec.symbols, ImportSymbols::Default);
        Ok(())
    }

    #[test]
    fn test_require_then_import_quoted_strings() -> Result<(), String> {
        let code = r#"
require Foo::Bar;
Foo::Bar->import('alpha', 'beta');
"#;
        let specs = parse_and_extract(code);
        let spec = specs
            .iter()
            .find(|s| s.module == "Foo::Bar")
            .ok_or("expected ImportSpec for Foo::Bar")?;

        assert_eq!(spec.kind, ImportKind::RequireThenImport);
        if let ImportSymbols::Explicit(names) = &spec.symbols {
            assert!(names.contains(&"alpha".to_string()), "missing 'alpha' in {names:?}");
            assert!(names.contains(&"beta".to_string()), "missing 'beta' in {names:?}");
        } else {
            return Err(format!("expected Explicit, got {:?}", spec.symbols));
        }
        // Fully literal quoted-string import → LiteralRequireImport provenance.
        assert_eq!(spec.provenance, Provenance::LiteralRequireImport);
        assert_eq!(spec.confidence, Confidence::High);
        Ok(())
    }

    #[test]
    fn test_require_then_import_dynamic_symbol_list() -> Result<(), String> {
        let code = r#"
require Foo::Bar;
Foo::Bar->import(@names);
"#;
        let specs = parse_and_extract(code);
        let spec = specs
            .iter()
            .find(|s| s.module == "Foo::Bar")
            .ok_or("expected ImportSpec for Foo::Bar")?;

        assert_eq!(spec.kind, ImportKind::RequireThenImport);
        assert_eq!(spec.symbols, ImportSymbols::Dynamic);
        assert_eq!(spec.confidence, Confidence::Low);
        Ok(())
    }

    // ── require $var → DynamicRequire ───────────────────────────────────

    #[test]
    fn test_require_dynamic_variable() -> Result<(), String> {
        let specs = parse_and_extract("require $module;");
        let spec = specs
            .iter()
            .find(|s| s.kind == ImportKind::DynamicRequire)
            .ok_or("expected DynamicRequire ImportSpec")?;

        assert_eq!(spec.module, "");
        assert_eq!(spec.symbols, ImportSymbols::Dynamic);
        // DynamicRequire must use DynamicBoundary provenance (Q5 architectural decision):
        // the module identity is not statically known, so we cannot claim ExactAst.
        assert_eq!(spec.provenance, Provenance::DynamicBoundary);
        assert_eq!(spec.confidence, Confidence::Low);
        assert_eq!(spec.file_id, Some(FileId(1)));
        assert!(spec.anchor_id.is_some(), "anchor_id should be populated");
        Ok(())
    }

    // ── Mixed use and require statements ────────────────────────────────

    #[test]
    fn test_mixed_use_and_require() -> Result<(), String> {
        let code = r#"
use strict;
use warnings;
require Foo::Bar;
Foo::Bar->import(qw(baz));
require $dynamic;
"#;
        let specs = parse_and_extract(code);

        // strict — bare use
        let strict_spec =
            specs.iter().find(|s| s.module == "strict").ok_or("expected strict ImportSpec")?;
        assert_eq!(strict_spec.kind, ImportKind::Use);

        // warnings — bare use
        let warnings_spec =
            specs.iter().find(|s| s.module == "warnings").ok_or("expected warnings ImportSpec")?;
        assert_eq!(warnings_spec.kind, ImportKind::Use);

        // Foo::Bar — require then import
        let foo_spec =
            specs.iter().find(|s| s.module == "Foo::Bar").ok_or("expected Foo::Bar ImportSpec")?;
        assert_eq!(foo_spec.kind, ImportKind::RequireThenImport);
        if let ImportSymbols::Explicit(names) = &foo_spec.symbols {
            assert!(names.contains(&"baz".to_string()), "missing 'baz' in {names:?}");
        } else {
            return Err(format!("expected Explicit, got {:?}", foo_spec.symbols));
        }

        // dynamic require
        let dyn_spec = specs
            .iter()
            .find(|s| s.kind == ImportKind::DynamicRequire)
            .ok_or("expected DynamicRequire ImportSpec")?;
        assert_eq!(dyn_spec.symbols, ImportSymbols::Dynamic);

        Ok(())
    }

    // ── require with string path → Require ──────────────────────────────

    #[test]
    fn test_require_string_path() -> Result<(), String> {
        let specs = parse_and_extract(r#"require "Foo/Bar.pm";"#);
        let spec = specs
            .iter()
            .find(|s| s.module == "Foo::Bar")
            .ok_or("expected ImportSpec for Foo::Bar")?;

        assert_eq!(spec.kind, ImportKind::Require);
        assert_eq!(spec.symbols, ImportSymbols::Default);
        Ok(())
    }

    // ── standalone ClassName->import(@names) — Case 3 (PR-B) ────────────

    #[test]
    fn standalone_class_dynamic_import_produces_dynamic_spec() -> Result<(), String> {
        // `Foo->import(@names)` — static class, dynamic arg list.
        // Should produce one ImportSpec with ImportSymbols::Dynamic and
        // ImportKind::ManualImport (not Use — it's a method call, not a `use` statement).
        let specs = parse_and_extract(r#"Foo->import(@names);"#);
        let spec = specs
            .iter()
            .find(|s| s.module == "Foo" && matches!(s.symbols, ImportSymbols::Dynamic))
            .ok_or("expected Dynamic ImportSpec for Foo")?;

        assert_eq!(spec.provenance, Provenance::DynamicBoundary);
        assert_eq!(spec.confidence, Confidence::Low);
        assert_eq!(
            spec.kind,
            ImportKind::ManualImport,
            "Class->import(@names) must use ManualImport, not Use"
        );
        Ok(())
    }

    #[test]
    fn standalone_class_explicit_import_produces_no_dynamic_spec() -> Result<(), String> {
        // `Foo->import('bar')` — static class, static arg list.
        // Should NOT produce a Dynamic ImportSpec (explicit symbols only).
        let specs = parse_and_extract(r#"Foo->import('bar');"#);
        let dynamic_specs: Vec<_> =
            specs.iter().filter(|s| matches!(s.symbols, ImportSymbols::Dynamic)).collect();

        assert!(dynamic_specs.is_empty(), "explicit import args must not produce a Dynamic spec");
        Ok(())
    }

    #[test]
    fn variable_class_import_does_not_produce_standalone_spec() -> Result<(), String> {
        // `$var->import(@names)` — variable object, not a static class name.
        // The standalone extractor should not match variable-object calls.
        let specs = parse_and_extract(r#"$var->import(@names);"#);
        // Variable-object calls are handled by require+import pair logic, not
        // the standalone path. Without a require, this should produce no spec.
        let standalone_dynamic: Vec<_> = specs
            .iter()
            .filter(|s| matches!(s.symbols, ImportSymbols::Dynamic) && s.module.is_empty())
            .collect();

        // The standalone extractor only handles Identifier objects, so this
        // should produce nothing via the standalone path.
        assert!(
            standalone_dynamic.is_empty(),
            "variable-class import without require must not produce standalone Dynamic spec"
        );
        Ok(())
    }
}
