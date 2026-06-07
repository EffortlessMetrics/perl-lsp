//! Import-spec extractor for workspace-level `ImportExportIndex` population.
//!
//! Recognizes `use`, `require`, `require + Module->import(...)`, and
//! standalone `ClassName->import(@names)` patterns in an AST and produces
//! [`ImportSpec`] entries for each.
//!
//! # Placement note — circular dependency debt
//!
//! This extractor lives in `perl-workspace` rather than
//! `perl-semantic-analyzer` because of a circular dependency:
//! `perl-semantic-analyzer/Cargo.toml` declares `perl-workspace` as a
//! dependency, so moving any producer into `perl-semantic-analyzer` would
//! create a cycle.
//!
//! This is **temporary architectural debt**. The correct long-term placement
//! is `perl-semantic-analyzer`, which owns the semantic production layer.
//! The blocker is the current `perl-semantic-analyzer → perl-workspace` dep
//! arc.
//!
//! **Follow-up**: invert or remove the `perl-semantic-analyzer → perl-workspace`
//! dependency (possibly by introducing a `perl-workspace-types` leaf crate for
//! the fact types), then consolidate this extractor into `perl-semantic-analyzer`.
//! Track as a follow-up after the dynamic-boundary suppression PRs merge.
//!
//! # Supported patterns
//!
//! | Perl source                                | `ImportKind`        | `ImportSymbols`          |
//! |--------------------------------------------|---------------------|--------------------------|
//! | `use Module qw(a b)`                       | `UseExplicitList`   | `Explicit(["a","b"])`    |
//! | `use Module ()`                            | `UseEmpty`          | `None`                   |
//! | `use Module ':tag'`                        | `UseTag`            | `Tags(["tag"])`          |
//! | `use Module` (bare)                        | `Use`               | `Default`                |
//! | `use constant { FOO => 1 }`                | `UseConstant`       | `Explicit(["FOO"])`      |
//! | `use constant PI => 3.14`                  | `UseConstant`       | `Explicit(["PI"])`       |
//! | `require Module`                           | `Require`           | `Default`                |
//! | `require Module; Module->import(...)`      | `RequireThenImport` | per args                 |
//! | `require $var`                             | `DynamicRequire`    | `Dynamic`                |
//! | `Foo->import(@names)` (standalone)         | `ManualImport`      | `Dynamic`                |

use crate::ast::{Node, NodeKind};
use perl_semantic_facts::{
    AnchorId, Confidence, FileId, ImportKind, ImportSpec, ImportSymbols, Provenance, UseLibFact,
};

/// Walk the AST and return one [`ImportSpec`] per import site.
///
/// Each spec carries the supplied `file_id` and an `anchor_id` derived from
/// the statement's byte-offset (for incremental invalidation).
///
/// See the module-level doc for the full list of recognised patterns.
pub fn extract_import_specs(ast: &Node, file_id: FileId) -> Vec<ImportSpec> {
    let mut out = Vec::new();
    walk(ast, file_id, &mut out);
    out
}

/// Walk the AST and return one [`UseLibFact`] per static `use lib`/`no lib` entry.
///
/// Dynamic args (`use lib $var`, `use lib @dirs`) are skipped — no fact is emitted.
/// Double-quoted strings and `qq` quote operators that contain `$`, `@`, or `%`
/// are treated as dynamic (they interpolate at runtime) and are also skipped.
/// Single-quoted strings, `q` quote operators, and interpolating strings that
/// contain no interpolation sigils produce a
/// `Provenance::ExactAst` / `Confidence::High` fact.
///
/// `is_active` is `true` for `use lib` entries and `false` for `no lib` entries.
pub fn extract_use_lib_facts(ast: &Node, file_id: FileId) -> Vec<UseLibFact> {
    let mut out = Vec::new();
    walk_use_lib(ast, file_id, &mut out);
    out
}

// ── UseLibFact walker ────────────────────────────────────────────────────────

fn walk_use_lib(node: &Node, file_id: FileId, out: &mut Vec<UseLibFact>) {
    match &node.kind {
        NodeKind::Use { module, args, .. } if module == "lib" => {
            collect_use_lib_facts(args, true, file_id, node, out);
        }
        NodeKind::No { module, args, .. } if module == "lib" => {
            collect_use_lib_facts(args, false, file_id, node, out);
        }
        _ => {}
    }

    for child in node.children() {
        walk_use_lib(child, file_id, out);
    }
}

/// Inspect the argument list of a `use lib` / `no lib` statement and push a
/// [`UseLibFact`] for each static (quoted-string or `qw(...)`) argument.
///
/// Dynamic arguments (variables `$var`, arrays `@arr`, and anything else that
/// is not a static string literal or `qw(...)` list) are silently skipped.
fn collect_use_lib_facts(
    args: &[String],
    is_active: bool,
    file_id: FileId,
    node: &Node,
    out: &mut Vec<UseLibFact>,
) {
    let anchor_id = anchor_from_node(node);

    for arg in args {
        let trimmed = arg.trim();

        // qw(...) list — emit one fact per word.
        if let Some(inner) = parse_qw_content(trimmed) {
            for word in inner.split_whitespace() {
                out.push(UseLibFact::new(
                    word.to_string(),
                    is_active,
                    file_id,
                    Some(anchor_id),
                    Provenance::ExactAst,
                    Confidence::High,
                ));
            }
            continue;
        }

        if let Some(literal) = parse_use_lib_literal(trimmed) {
            if literal.interpolates
                && (literal.body.contains('$')
                    || literal.body.contains('@')
                    || literal.body.contains('%'))
            {
                continue;
            }
            out.push(UseLibFact::new(
                literal.body.to_string(),
                is_active,
                file_id,
                Some(anchor_id),
                Provenance::ExactAst,
                Confidence::High,
            ));
            continue;
        }

        // Dynamic argument ($var, @arr, or anything else) — skip, emit nothing.
    }
}

struct UseLibLiteral<'a> {
    body: &'a str,
    interpolates: bool,
}

fn parse_use_lib_literal(s: &str) -> Option<UseLibLiteral<'_>> {
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        return Some(UseLibLiteral { body: unquote(s), interpolates: true });
    }
    if s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2 {
        return Some(UseLibLiteral { body: unquote(s), interpolates: false });
    }
    if let Some(body) = parse_quote_operator_content(s, "qq") {
        return Some(UseLibLiteral { body, interpolates: true });
    }
    if let Some(body) = parse_quote_operator_content(s, "q") {
        return Some(UseLibLiteral { body, interpolates: false });
    }
    None
}

// ── AST walker ──────────────────────────────────────────────────────────────

fn walk(node: &Node, file_id: FileId, out: &mut Vec<ImportSpec>) {
    // Handle `use` statements.
    if let NodeKind::Use { module, args, .. } = &node.kind {
        if let Some(spec) = classify_use(module, args, file_id, node) {
            out.push(spec);
        }
    }

    // Detect standalone `ClassName->import(@names)` method calls where the
    // object is a static identifier (not a variable). These are NOT preceded
    // by a `require` statement. The exported symbol list is often dynamic
    // (e.g. `Foo->import(@names)`), so we emit `ImportSymbols::Dynamic`
    // conservatively.
    if let Some(spec) = try_classify_standalone_class_import(node, file_id) {
        out.push(spec);
    }

    // For statement-list containers, scan consecutive statements to detect
    // `require Module; Module->import(...)` pairs and standalone `require`s.
    match &node.kind {
        NodeKind::Program { statements } | NodeKind::Block { statements } => {
            walk_statements(statements, file_id, out);
        }
        NodeKind::Package { block: Some(block), .. } => {
            if let NodeKind::Block { statements } = &block.kind {
                walk_statements(statements, file_id, out);
            }
        }
        _ => {}
    }

    for child in node.children() {
        walk(child, file_id, out);
    }
}

// ── Standalone ClassName->import(@names) detection ──────────────────────────

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

    // Only emit when the argument list is dynamic — explicit lists are handled
    // precisely elsewhere (require+import pair or use statement).
    let symbols = extract_import_call_symbols(args);
    if !matches!(symbols, ImportSymbols::Dynamic) {
        return None;
    }

    let anchor_id = anchor_from_node(node);
    Some(ImportSpec {
        module: class_name.to_string(),
        // ManualImport distinguishes this from `use Foo` — it is a
        // `Class->import(...)` method call, not a `use` declaration.
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

// ── Statement-list scanner for require patterns ──────────────────────────────

fn walk_statements(statements: &[Node], file_id: FileId, out: &mut Vec<ImportSpec>) {
    let mut consumed: std::collections::HashSet<usize> = std::collections::HashSet::new();

    for (i, stmt) in statements.iter().enumerate() {
        if consumed.contains(&i) {
            continue;
        }

        let expr = unwrap_expression_statement(stmt);

        let (require_node, require_args) = match &expr.kind {
            NodeKind::FunctionCall { name, args } if name == "require" => (stmt, args),
            _ => continue,
        };

        // Dynamic require: `require $var`
        if is_dynamic_require(require_args) {
            out.push(make_dynamic_require(file_id, require_node));
            consumed.insert(i);
            continue;
        }

        // Static require: extract module name.
        let module_name = match extract_require_module_name(require_args) {
            Some(name) => name,
            None => continue,
        };

        // Look ahead for `Module->import(...)`.
        let import_spec = statements.get(i + 1).and_then(|next_stmt| {
            let next_expr = unwrap_expression_statement(next_stmt);
            try_match_import_call(next_expr, &module_name)
        });

        if let Some((symbols, _import_node)) = import_spec {
            let anchor_id = anchor_from_node(require_node);
            let confidence = confidence_for_symbols(&symbols);
            out.push(ImportSpec {
                module: module_name,
                kind: ImportKind::RequireThenImport,
                symbols,
                provenance: Provenance::ExactAst,
                confidence,
                file_id: Some(file_id),
                anchor_id: Some(anchor_id),
                scope_id: None,
                span_start_byte: Some(require_node.location.start as u32),
            });
            consumed.insert(i);
            consumed.insert(i + 1);
        } else {
            let anchor_id = anchor_from_node(require_node);
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

// ── require helpers ──────────────────────────────────────────────────────────

fn unwrap_expression_statement(node: &Node) -> &Node {
    match &node.kind {
        NodeKind::ExpressionStatement { expression } => expression,
        _ => node,
    }
}

fn is_dynamic_require(args: &[Node]) -> bool {
    matches!(args.first(), Some(arg) if matches!(&arg.kind, NodeKind::Variable { .. }))
}

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

fn make_dynamic_require(file_id: FileId, node: &Node) -> ImportSpec {
    let anchor_id = anchor_from_node(node);
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

    let obj_name = match &object.kind {
        NodeKind::Identifier { name } => name.as_str(),
        _ => return None,
    };

    if obj_name != expected_module {
        return None;
    }

    let symbols = extract_import_call_symbols(args);
    Some((symbols, node))
}

// ── Symbol extraction from import() argument lists ───────────────────────────

fn extract_import_call_symbols(args: &[Node]) -> ImportSymbols {
    if args.is_empty() {
        return ImportSymbols::Default;
    }

    let mut names: Vec<String> = Vec::new();
    let mut tags: Vec<String> = Vec::new();
    let mut has_dynamic_arg = false;

    for arg in args {
        has_dynamic_arg |= collect_import_arg_symbols(arg, &mut names, &mut tags);
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

/// Returns `true` when the argument is dynamic (prevents exact symbol list).
fn collect_import_arg_symbols(arg: &Node, names: &mut Vec<String>, tags: &mut Vec<String>) -> bool {
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
            if let Some(inner) = parse_qw_content(name) {
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
        NodeKind::Variable { .. } => true, // `Foo->import(@names)` → dynamic
        NodeKind::ArrayLiteral { elements } => {
            let mut has_dyn = false;
            for el in elements {
                has_dyn |= collect_import_arg_symbols(el, names, tags);
            }
            has_dyn
        }
        _ => true,
    }
}

// ── use-statement classification ─────────────────────────────────────────────

fn classify_use(module: &str, args: &[String], file_id: FileId, node: &Node) -> Option<ImportSpec> {
    if is_version_pragma(module) {
        return None;
    }

    let anchor_id = anchor_from_node(node);

    if module == "constant" {
        return Some(classify_use_constant(args, file_id, anchor_id));
    }

    let (kind, symbols) = classify_args(args, module, node);

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

fn classify_args(args: &[String], module: &str, node: &Node) -> (ImportKind, ImportSymbols) {
    if args.is_empty() {
        let bare_len = "use ".len() + module.len() + 1; // +1 for ';'
        let span_len = node.location.end.saturating_sub(node.location.start);
        if span_len > bare_len {
            return (ImportKind::UseEmpty, ImportSymbols::None);
        }
        return (ImportKind::Use, ImportSymbols::Default);
    }

    let mut explicit_names: Vec<String> = Vec::new();
    let mut tags: Vec<String> = Vec::new();

    for arg in args {
        let trimmed = arg.trim();

        if let Some(inner) = parse_qw_content(trimmed) {
            for word in inner.split_whitespace() {
                if let Some(tag) = word.strip_prefix(':') {
                    tags.push(tag.to_string());
                } else {
                    explicit_names.push(word.to_string());
                }
            }
            continue;
        }

        let unquoted = unquote(trimmed);
        if let Some(tag) = unquoted.strip_prefix(':') {
            tags.push(tag.to_string());
            continue;
        }

        if trimmed == "=>" || trimmed == "," || trimmed == "\\" {
            continue;
        }

        if looks_like_symbol_name(trimmed) {
            explicit_names.push(unquote(trimmed).to_string());
        }
    }

    if explicit_names.is_empty() && tags.is_empty() && !args.is_empty() {
        let has_any_symbol = args.iter().any(|a| {
            let t = a.trim();
            looks_like_symbol_name(t) || parse_qw_content(t).is_some()
        });
        if !has_any_symbol {
            return (ImportKind::UseEmpty, ImportSymbols::None);
        }
    }

    if !tags.is_empty() && explicit_names.is_empty() {
        return (ImportKind::UseTag, ImportSymbols::Tags(tags));
    }

    if !tags.is_empty() && !explicit_names.is_empty() {
        return (ImportKind::UseExplicitList, ImportSymbols::Mixed { tags, names: explicit_names });
    }

    if !explicit_names.is_empty() {
        return (ImportKind::UseExplicitList, ImportSymbols::Explicit(explicit_names));
    }

    (ImportKind::Use, ImportSymbols::Default)
}

fn classify_use_constant(args: &[String], file_id: FileId, anchor_id: AnchorId) -> ImportSpec {
    let mut constant_names: Vec<String> = Vec::new();

    if args.is_empty() {
        return ImportSpec {
            module: "constant".to_string(),
            kind: ImportKind::UseConstant,
            symbols: ImportSymbols::None,
            provenance: Provenance::ExactAst,
            confidence: Confidence::High,
            file_id: Some(file_id),
            anchor_id: Some(anchor_id),
            scope_id: None,
            span_start_byte: None,
        };
    }

    if args.first().map(|a| a.as_str()) == Some("{") {
        let mut i = 1;
        while i < args.len() {
            let token = args[i].trim();
            if token == "}" || token == "=>" || token == "," {
                i += 1;
                continue;
            }
            if i + 1 < args.len() && args[i + 1].trim() == "=>" {
                constant_names.push(token.to_string());
                i += 3;
            } else {
                i += 1;
            }
        }
    } else if let Some(inner) = args.first().and_then(|a| parse_qw_content(a.trim())) {
        constant_names.extend(inner.split_whitespace().map(|w| w.to_string()));
    } else if let Some(name) = args.first() {
        let trimmed = name.trim();
        if looks_like_constant_name(trimmed) {
            constant_names.push(trimmed.to_string());
        }
    }

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
        span_start_byte: None,
    }
}

// ── Utility helpers ──────────────────────────────────────────────────────────

fn anchor_from_node(node: &Node) -> AnchorId {
    AnchorId(node.location.start as u64)
}

fn is_version_pragma(module: &str) -> bool {
    if module.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return true;
    }
    if module.starts_with('v')
        && module.len() > 1
        && module[1..].chars().all(|c| c.is_ascii_digit() || c == '.')
    {
        return true;
    }
    false
}

fn parse_qw_content(s: &str) -> Option<&str> {
    parse_quote_operator_content(s, "qw")
}

fn parse_quote_operator_content<'a>(s: &'a str, operator: &str) -> Option<&'a str> {
    // Perl allows whitespace between a quote-like operator and its opening
    // delimiter, e.g. `qw [a b]` or `q (x)`.  Trim it before reading the
    // delimiter so the space-before-delimiter form is handled the same as the
    // compact form.  (Mirrors `perl_parser_core::hir::model::parse_qw_content`,
    // which already trims, and the `WorkspaceIndex` constant/module extractors.)
    let rest = s.strip_prefix(operator)?.trim_start();
    let mut chars = rest.chars();
    let open = chars.next()?;
    if open.is_ascii_alphanumeric() || open == '_' {
        return None;
    }
    let close = match open {
        '(' => ')',
        '{' => '}',
        '[' => ']',
        '<' => '>',
        other => other,
    };
    if !rest.ends_with(close) {
        return None;
    }
    let start = open.len_utf8();
    let end = rest.len().checked_sub(close.len_utf8())?;
    if end < start {
        return None;
    }
    Some(&rest[start..end])
}

fn unquote(s: &str) -> &str {
    if (s.starts_with('\'') && s.ends_with('\'')) || (s.starts_with('"') && s.ends_with('"')) {
        if s.len() >= 2 {
            return &s[1..s.len() - 1];
        }
    }
    s
}

fn looks_like_symbol_name(s: &str) -> bool {
    let s = unquote(s);
    if s.is_empty() {
        return false;
    }
    if s.starts_with(':') {
        return true;
    }
    if s.starts_with('$')
        || s.starts_with('@')
        || s.starts_with('%')
        || s.starts_with('&')
        || s.starts_with('*')
    {
        return true;
    }
    s.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
}

fn looks_like_constant_name(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    s.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
}

fn confidence_for_symbols(symbols: &ImportSymbols) -> Confidence {
    if matches!(symbols, ImportSymbols::Dynamic) { Confidence::Low } else { Confidence::High }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;

    fn parse_and_extract(code: &str) -> Vec<ImportSpec> {
        let mut parser = Parser::new(code);
        let ast = match parser.parse() {
            Ok(a) => a,
            Err(_) => return Vec::new(),
        };
        extract_import_specs(&ast, FileId(1))
    }

    fn parse_and_extract_use_lib(
        code: &str,
    ) -> Result<Vec<UseLibFact>, Box<dyn std::error::Error>> {
        let mut parser = Parser::new(code);
        let ast =
            parser.parse().map_err(|error| format!("parse failed for {code:?}: {error:?}"))?;
        Ok(extract_use_lib_facts(&ast, FileId(2)))
    }

    #[test]
    fn use_lib_extractor_returns_empty_vec_without_lib_statement()
    -> Result<(), Box<dyn std::error::Error>> {
        let facts = parse_and_extract_use_lib("use strict;")?;

        assert!(facts.is_empty(), "non-lib imports must not emit UseLibFact values");
        Ok(())
    }

    #[test]
    fn use_lib_extractor_collects_active_and_inactive_facts()
    -> Result<(), Box<dyn std::error::Error>> {
        let facts = parse_and_extract_use_lib("use lib 'active'; no lib 'inactive';")?;

        assert_eq!(
            facts,
            vec![
                UseLibFact::new(
                    "active".to_string(),
                    true,
                    FileId(2),
                    Some(AnchorId(0)),
                    Provenance::ExactAst,
                    Confidence::High,
                ),
                UseLibFact::new(
                    "inactive".to_string(),
                    false,
                    FileId(2),
                    Some(AnchorId(18)),
                    Provenance::ExactAst,
                    Confidence::High,
                ),
            ]
        );
        Ok(())
    }

    #[test]
    fn use_lib_extractor_walks_ast_children_for_later_statement()
    -> Result<(), Box<dyn std::error::Error>> {
        let facts = parse_and_extract_use_lib("use strict; use lib 'later';")?;
        let fact = facts.first().ok_or("expected UseLibFact from later child")?;

        assert_eq!(fact.path, "later");
        assert_eq!(fact.file_id, FileId(2));
        assert_eq!(fact.anchor_id, Some(AnchorId(12)));
        Ok(())
    }

    #[test]
    fn use_lib_extractor_assigns_anchor_to_collected_fact() -> Result<(), Box<dyn std::error::Error>>
    {
        let facts = parse_and_extract_use_lib("use lib 'anchored';")?;
        let fact = facts.first().ok_or("expected anchored UseLibFact")?;

        assert_eq!(fact.path, "anchored");
        assert_eq!(fact.anchor_id, Some(AnchorId(0)));
        Ok(())
    }

    #[test]
    fn use_lib_extractor_emits_qw_words_with_static_metadata()
    -> Result<(), Box<dyn std::error::Error>> {
        let facts = parse_and_extract_use_lib("use lib qw(lib vendor);")?;

        assert_eq!(
            facts,
            vec![
                UseLibFact::new(
                    "lib".to_string(),
                    true,
                    FileId(2),
                    Some(AnchorId(0)),
                    Provenance::ExactAst,
                    Confidence::High,
                ),
                UseLibFact::new(
                    "vendor".to_string(),
                    true,
                    FileId(2),
                    Some(AnchorId(0)),
                    Provenance::ExactAst,
                    Confidence::High,
                ),
            ]
        );
        Ok(())
    }

    #[test]
    fn use_lib_extractor_emits_literal_with_static_metadata()
    -> Result<(), Box<dyn std::error::Error>> {
        let facts = parse_and_extract_use_lib("no lib 'old';")?;
        let fact = facts.first().ok_or("expected literal UseLibFact")?;

        assert_eq!(
            fact,
            &UseLibFact::new(
                "old".to_string(),
                false,
                FileId(2),
                Some(AnchorId(0)),
                Provenance::ExactAst,
                Confidence::High,
            )
        );
        Ok(())
    }

    #[test]
    fn use_bare_module_produces_use_default() -> Result<(), Box<dyn std::error::Error>> {
        let specs = parse_and_extract("use strict;");
        let spec = specs.first().ok_or("expected ImportSpec")?;
        assert_eq!(spec.module, "strict");
        assert_eq!(spec.kind, ImportKind::Use);
        assert_eq!(spec.symbols, ImportSymbols::Default);
        Ok(())
    }

    /// Regression: `qw` with whitespace before the delimiter (`qw [a b]`) must
    /// extract the explicit import list the same as the compact form `qw(a b)`.
    /// Previously the leading space was treated as the delimiter, so no symbols
    /// were extracted. See `parse_quote_operator_content`.
    #[test]
    fn use_explicit_list_qw_space_before_delimiter() -> Result<(), Box<dyn std::error::Error>> {
        let specs = parse_and_extract("use List::Util qw [first any];");
        let spec = specs.first().ok_or("expected ImportSpec")?;
        assert_eq!(spec.module, "List::Util");
        assert_eq!(spec.kind, ImportKind::UseExplicitList);
        if let ImportSymbols::Explicit(names) = &spec.symbols {
            assert!(names.contains(&"first".to_string()), "got {:?}", spec.symbols);
            assert!(names.contains(&"any".to_string()), "got {:?}", spec.symbols);
        } else {
            return Err(format!("expected Explicit, got {:?}", spec.symbols).into());
        }
        Ok(())
    }

    /// Regression: `use lib qw [..]` (space before delimiter) must still emit a
    /// `UseLibFact` per word.
    #[test]
    fn use_lib_extractor_emits_qw_words_space_before_delimiter()
    -> Result<(), Box<dyn std::error::Error>> {
        let facts = parse_and_extract_use_lib("use lib qw [lib vendor];")?;
        let paths: Vec<&str> = facts.iter().map(|f| f.path.as_str()).collect();
        assert!(paths.contains(&"lib"), "got {paths:?}");
        assert!(paths.contains(&"vendor"), "got {paths:?}");
        Ok(())
    }

    /// `parse_quote_operator_content` unit coverage: leading whitespace before
    /// the delimiter is tolerated for `qw`, `q`, and `qq`; a word character
    /// after the operator is still rejected (it is not a delimiter).
    /// Covers space, tab, and newline — all `trim_start` whitespace variants.
    #[test]
    fn parse_quote_operator_content_tolerates_leading_space() {
        // Space before delimiter.
        assert_eq!(parse_quote_operator_content("qw [a b]", "qw"), Some("a b"));
        assert_eq!(parse_quote_operator_content("qw(a b)", "qw"), Some("a b"));
        assert_eq!(parse_quote_operator_content("q (x)", "q"), Some("x"));
        assert_eq!(parse_quote_operator_content("qq {y}", "qq"), Some("y"));
        // Tab before delimiter (`trim_start` trims tabs too).
        assert_eq!(parse_quote_operator_content("qw\t[a b]", "qw"), Some("a b"));
        assert_eq!(parse_quote_operator_content("q\t(x)", "q"), Some("x"));
        // Newline before delimiter (e.g. heredoc-adjacent or multi-line use).
        assert_eq!(parse_quote_operator_content("qw\n[a b]", "qw"), Some("a b"));
        // Multiple mixed whitespace before delimiter.
        assert_eq!(parse_quote_operator_content("qw  \t [a b]", "qw"), Some("a b"));
        // A word char after the operator is a bareword, not a delimiter.
        assert_eq!(parse_quote_operator_content("qq foo", "qq"), None);
    }

    #[test]
    fn use_explicit_list_qw() -> Result<(), Box<dyn std::error::Error>> {
        let specs = parse_and_extract("use List::Util qw(first any);");
        let spec = specs.first().ok_or("expected ImportSpec")?;
        assert_eq!(spec.module, "List::Util");
        assert_eq!(spec.kind, ImportKind::UseExplicitList);
        if let ImportSymbols::Explicit(names) = &spec.symbols {
            assert!(names.contains(&"first".to_string()));
            assert!(names.contains(&"any".to_string()));
        } else {
            return Err(format!("expected Explicit, got {:?}", spec.symbols).into());
        }
        Ok(())
    }

    #[test]
    fn version_pragma_skipped() -> Result<(), Box<dyn std::error::Error>> {
        let specs = parse_and_extract("use 5.036;");
        assert!(specs.is_empty(), "version pragma must not produce ImportSpec");
        Ok(())
    }

    #[test]
    fn standalone_class_dynamic_import_produces_manual_import()
    -> Result<(), Box<dyn std::error::Error>> {
        let specs = parse_and_extract("Foo->import(@names);");
        let spec = specs
            .iter()
            .find(|s| s.module == "Foo" && matches!(s.symbols, ImportSymbols::Dynamic))
            .ok_or("expected Dynamic ImportSpec for Foo")?;
        assert_eq!(spec.kind, ImportKind::ManualImport);
        assert_eq!(spec.provenance, Provenance::DynamicBoundary);
        assert_eq!(spec.confidence, Confidence::Low);
        Ok(())
    }

    #[test]
    fn require_then_import_pair_produces_require_then_import()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = "require Foo::Bar;\nFoo::Bar->import(qw(alpha beta));";
        let specs = parse_and_extract(code);
        let spec =
            specs.iter().find(|s| s.module == "Foo::Bar").ok_or("expected Foo::Bar ImportSpec")?;
        assert_eq!(spec.kind, ImportKind::RequireThenImport);
        if let ImportSymbols::Explicit(names) = &spec.symbols {
            assert!(names.contains(&"alpha".to_string()));
            assert!(names.contains(&"beta".to_string()));
        } else {
            return Err(format!("expected Explicit, got {:?}", spec.symbols).into());
        }
        Ok(())
    }

    #[test]
    fn require_dynamic_variable_produces_dynamic_require() -> Result<(), Box<dyn std::error::Error>>
    {
        let specs = parse_and_extract("require $mod;");
        let spec = specs
            .iter()
            .find(|s| s.kind == ImportKind::DynamicRequire)
            .ok_or("expected DynamicRequire ImportSpec")?;
        assert_eq!(spec.symbols, ImportSymbols::Dynamic);
        assert_eq!(spec.provenance, Provenance::DynamicBoundary);
        Ok(())
    }

    #[test]
    fn span_start_byte_is_populated_for_use() -> Result<(), Box<dyn std::error::Error>> {
        let specs = parse_and_extract("use Foo;");
        let spec = specs.first().ok_or("expected ImportSpec")?;
        assert!(spec.span_start_byte.is_some(), "span_start_byte must be set for use statements");
        Ok(())
    }

    #[test]
    fn standalone_explicit_class_import_not_emitted_as_dynamic()
    -> Result<(), Box<dyn std::error::Error>> {
        // `Foo->import('bar')` — static arg list should NOT produce a Dynamic spec.
        let specs = parse_and_extract("Foo->import('bar');");
        let dynamic_specs: Vec<_> =
            specs.iter().filter(|s| matches!(s.symbols, ImportSymbols::Dynamic)).collect();
        assert!(
            dynamic_specs.is_empty(),
            "explicit import args must not produce a Dynamic spec, got: {dynamic_specs:#?}"
        );
        Ok(())
    }
}
