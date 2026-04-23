//! POD coverage lint for exported subroutines
//!
//! Detects exported subroutines that lack corresponding `=head2` or `=item`
//! POD documentation. Only fires when the module uses `Exporter` and declares
//! `@EXPORT` or `@EXPORT_OK`.
//!
//! # Diagnostic codes
//!
//! | Code   | Severity | Description |
//! |--------|----------|-------------|
//! | `PL304` | Hint    | Exported subroutine lacks POD documentation |

use perl_diagnostics::codes::DiagnosticCode;
use perl_parser_core::ast::{Node, NodeKind};

use super::super::internal_types::Diagnostic;
use super::super::walker::walk_node;
use perl_diagnostics::codes::DiagnosticSeverity;

/// Check for exported subroutines that lack POD documentation.
///
/// Walks the AST to find `our @EXPORT` / `our @EXPORT_OK` declarations and
/// subroutine definitions, then scans the source text for `=head2` / `=item`
/// POD sections that document each exported name.
pub fn check_pod_coverage(node: &Node, source: &str, diagnostics: &mut Vec<Diagnostic>) {
    let exported_names = collect_exported_names(node, source);
    if exported_names.is_empty() {
        return;
    }

    let documented_names = collect_documented_names(source);

    let mut sub_locations: Vec<(String, usize, usize)> = Vec::new();
    walk_node(node, &mut |n| {
        if let NodeKind::Subroutine { name: Some(name), .. } = &n.kind {
            sub_locations.push((name.clone(), n.location.start, n.location.end));
        }
    });

    for (export_name, _export_start, _export_end) in &exported_names {
        if documented_names.iter().any(|doc| doc == export_name) {
            continue;
        }

        let (range_start, range_end) = if let Some((_, start, end)) =
            sub_locations.iter().find(|(n, _, _)| n == export_name)
        {
            (*start, *end)
        } else {
            (*_export_start, *_export_end)
        };

        diagnostics.push(Diagnostic {
            range: (range_start, range_end),
            severity: DiagnosticSeverity::Hint,
            code: Some(DiagnosticCode::MissingPodCoverage.as_str().to_string()),
            message: format!("Exported subroutine '{}' has no POD documentation", export_name),
            related_information: Vec::new(),
            tags: Vec::new(),
            suggestion: Some(format!(
                "Add '=head2 {}' documentation before or near the subroutine definition",
                export_name
            )),
        });
    }
}

/// Collect names from `our @EXPORT = qw(...)` and `our @EXPORT_OK = qw(...)`.
///
/// Uses AST walking first (handles `our @EXPORT = qw(foo bar)` parsed as
/// VariableDeclaration with ArrayLiteral initializer), then falls back to
/// source text scanning for patterns the AST doesn't capture (e.g. `push
/// @EXPORT, 'foo'`).
fn collect_exported_names(node: &Node, source: &str) -> Vec<(String, usize, usize)> {
    let mut names: Vec<(String, usize, usize)> = Vec::new();
    let mut has_exporter = false;

    walk_node(node, &mut |n| {
        if let NodeKind::Use { module, .. } = &n.kind
            && (module == "Exporter" || module == "parent" || module == "base")
        {
            has_exporter = true;
        }
    });

    if !has_exporter && !source.contains("@ISA") && !source.contains("Exporter") {
        return names;
    }

    walk_node(node, &mut |n| match &n.kind {
        NodeKind::VariableDeclaration { declarator, variable, initializer: Some(init), .. }
            if declarator == "our" =>
        {
            if let NodeKind::Variable { sigil, name } = &variable.kind
                && sigil == "@"
                && (name == "EXPORT" || name == "EXPORT_OK")
            {
                collect_names_from_expr(init, &mut names);
            }
        }
        NodeKind::Assignment { lhs, rhs, .. } if is_export_variable(lhs) => {
            collect_names_from_expr(rhs, &mut names);
        }
        _ => {}
    });

    if names.is_empty() {
        collect_exported_names_from_source(source, &mut names);
    }

    names
}

fn is_export_variable(node: &Node) -> bool {
    if let NodeKind::Variable { sigil, name } = &node.kind {
        sigil == "@" && (name == "EXPORT" || name == "EXPORT_OK")
    } else {
        false
    }
}

fn collect_names_from_expr(node: &Node, names: &mut Vec<(String, usize, usize)>) {
    match &node.kind {
        NodeKind::ArrayLiteral { elements } => {
            for elem in elements {
                if let NodeKind::String { value, .. } = &elem.kind
                    && !value.is_empty()
                    && !value.starts_with('$')
                    && !value.starts_with('@')
                    && !value.starts_with('%')
                {
                    names.push((value.clone(), elem.location.start, elem.location.end));
                }
            }
        }
        NodeKind::String { value, .. } => {
            if !value.is_empty()
                && !value.starts_with('$')
                && !value.starts_with('@')
                && !value.starts_with('%')
            {
                names.push((value.clone(), node.location.start, node.location.end));
            }
        }
        NodeKind::FunctionCall { args, .. } => {
            for arg in args {
                collect_names_from_expr(arg, names);
            }
        }
        _ => {}
    }
}

/// Fallback: scan source text for `@EXPORT` / `@EXPORT_OK` assignments with `qw()`.
fn collect_exported_names_from_source(source: &str, names: &mut Vec<(String, usize, usize)>) {
    for (i, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if !trimmed.contains("@EXPORT") {
            continue;
        }

        let line_start = source.lines().take(i).map(|l| l.len() + 1).sum::<usize>();

        if let Some(qw_start) = trimmed.find("qw") {
            let after_qw = &trimmed[qw_start + 2..];
            if let Some(content) = extract_qw_content(after_qw) {
                for word in content.split_whitespace() {
                    if !word.starts_with('$') && !word.starts_with('@') && !word.starts_with('%') {
                        names.push((word.to_string(), line_start, line_start + line.len()));
                    }
                }
            }
        }
    }
}

fn extract_qw_content(s: &str) -> Option<&str> {
    let s = s.trim();
    let (open, close) = match s.chars().next()? {
        '(' => ('(', ')'),
        '[' => ('[', ']'),
        '{' => ('{', '}'),
        '<' => ('<', '>'),
        _ => return None,
    };
    let start = s.find(open)? + 1;
    let end = s.rfind(close)?;
    if start < end { Some(&s[start..end]) } else { None }
}

/// Scan source text for POD documentation sections.
///
/// Looks for `=head2 name`, `=item name`, and `=item B<name>` patterns.
fn collect_documented_names(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();

        if let Some(rest) = trimmed.strip_prefix("=head2").or_else(|| trimmed.strip_prefix("=item"))
            && let Some(name) = extract_pod_name(rest)
        {
            names.push(name);
        }
    }
    names
}

/// Extract a subroutine name from POD heading text.
///
/// Handles: `=head2 foo`, `=head2 foo()`, `=head2 B<foo>`, `=head2 C<foo()>`.
fn extract_pod_name(text: &str) -> Option<String> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    let text = if let Some(inner) = text.strip_prefix("B<").and_then(|s| s.strip_suffix('>')) {
        inner
    } else if let Some(inner) = text.strip_prefix("C<").and_then(|s| s.strip_suffix('>')) {
        inner
    } else {
        text
    };

    let name = text.split(['(', ' ', '\t']).next()?;

    if name.is_empty() || name.starts_with('=') {
        return None;
    }

    let name = name.trim_start_matches('$').trim_start_matches('@').trim_start_matches('%');

    if name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == ':') && !name.is_empty() {
        Some(name.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser_core::ast::SourceLocation;

    fn make_string_node(value: &str, start: usize, end: usize) -> Node {
        Node::new(
            NodeKind::String { value: value.to_string(), interpolated: false },
            SourceLocation { start, end },
        )
    }

    fn make_array_literal(elements: Vec<Node>, start: usize, end: usize) -> Node {
        Node::new(NodeKind::ArrayLiteral { elements }, SourceLocation { start, end })
    }

    fn make_var(sigil: &str, name: &str, start: usize, end: usize) -> Node {
        Node::new(
            NodeKind::Variable { sigil: sigil.to_string(), name: name.to_string() },
            SourceLocation { start, end },
        )
    }

    fn make_use(module: &str, start: usize, end: usize) -> Node {
        Node::new(
            NodeKind::Use { module: module.to_string(), args: Vec::new(), has_filter_risk: false },
            SourceLocation { start, end },
        )
    }

    fn make_sub(name: &str, start: usize, end: usize) -> Node {
        Node::new(
            NodeKind::Subroutine {
                name: Some(name.to_string()),
                name_span: None,
                prototype: None,
                signature: None,
                attributes: Vec::new(),
                body: Box::new(Node::new(
                    NodeKind::Block { statements: Vec::new() },
                    SourceLocation { start: start + 10, end: end - 1 },
                )),
            },
            SourceLocation { start, end },
        )
    }

    fn make_program(stmts: Vec<Node>) -> Node {
        let end = stmts.last().map_or(0, |n| n.location.end);
        Node::new(NodeKind::Program { statements: stmts }, SourceLocation { start: 0, end })
    }

    fn make_var_decl(
        declarator: &str,
        sigil: &str,
        name: &str,
        init: Option<Node>,
        start: usize,
        end: usize,
    ) -> Node {
        Node::new(
            NodeKind::VariableDeclaration {
                declarator: declarator.to_string(),
                variable: Box::new(make_var(sigil, name, start + 4, start + 4 + name.len() + 1)),
                attributes: Vec::new(),
                initializer: init.map(Box::new),
            },
            SourceLocation { start, end },
        )
    }

    #[test]
    fn given_module_with_exporter_and_undocumented_exports_then_diagnostic_emitted() {
        let source = r#"package MyModule;
use Exporter 'import';
our @EXPORT = qw(foo bar);

sub foo { 1 }
sub bar { 2 }
"#;

        let ast = make_program(vec![
            make_use("Exporter", 17, 39),
            make_var_decl(
                "our",
                "@",
                "EXPORT",
                Some(make_array_literal(
                    vec![make_string_node("foo", 56, 59), make_string_node("bar", 60, 63)],
                    52,
                    64,
                )),
                40,
                65,
            ),
            make_sub("foo", 67, 80),
            make_sub("bar", 81, 94),
        ]);

        let mut diagnostics = Vec::new();
        check_pod_coverage(&ast, source, &mut diagnostics);

        assert_eq!(diagnostics.len(), 2, "both foo and bar lack POD");
        assert!(diagnostics.iter().all(|d| d.code.as_deref() == Some("PL304")));
        assert!(diagnostics.iter().any(|d| d.message.contains("foo")));
        assert!(diagnostics.iter().any(|d| d.message.contains("bar")));
    }

    #[test]
    fn given_module_with_documented_exports_then_no_diagnostic() {
        let source = r#"package MyModule;
use Exporter 'import';
our @EXPORT = qw(foo bar);

=head2 foo

Does foo things.

=head2 bar

Does bar things.

=cut

sub foo { 1 }
sub bar { 2 }
"#;

        let ast = make_program(vec![
            make_use("Exporter", 17, 39),
            make_var_decl(
                "our",
                "@",
                "EXPORT",
                Some(make_array_literal(
                    vec![make_string_node("foo", 56, 59), make_string_node("bar", 60, 63)],
                    52,
                    64,
                )),
                40,
                65,
            ),
            make_sub("foo", 120, 133),
            make_sub("bar", 134, 147),
        ]);

        let mut diagnostics = Vec::new();
        check_pod_coverage(&ast, source, &mut diagnostics);

        assert!(diagnostics.is_empty(), "documented exports should not fire");
    }

    #[test]
    fn given_module_without_exporter_then_no_diagnostic() {
        let source = r#"package Internal;
sub helper { 1 }
"#;

        let ast = make_program(vec![make_sub("helper", 18, 34)]);

        let mut diagnostics = Vec::new();
        check_pod_coverage(&ast, source, &mut diagnostics);

        assert!(diagnostics.is_empty(), "no exporter = no lint");
    }

    #[test]
    fn given_partial_documentation_then_only_missing_reported() {
        let source = r#"package MyModule;
use Exporter 'import';
our @EXPORT_OK = qw(foo bar baz);

=head2 foo

Documented.

=cut

sub foo { 1 }
sub bar { 2 }
sub baz { 3 }
"#;

        let ast = make_program(vec![
            make_use("Exporter", 17, 39),
            make_var_decl(
                "our",
                "@",
                "EXPORT_OK",
                Some(make_array_literal(
                    vec![
                        make_string_node("foo", 59, 62),
                        make_string_node("bar", 63, 66),
                        make_string_node("baz", 67, 70),
                    ],
                    55,
                    71,
                )),
                40,
                72,
            ),
            make_sub("foo", 110, 123),
            make_sub("bar", 124, 137),
            make_sub("baz", 138, 151),
        ]);

        let mut diagnostics = Vec::new();
        check_pod_coverage(&ast, source, &mut diagnostics);

        assert_eq!(diagnostics.len(), 2, "bar and baz lack POD");
        assert!(diagnostics.iter().any(|d| d.message.contains("bar")));
        assert!(diagnostics.iter().any(|d| d.message.contains("baz")));
        assert!(!diagnostics.iter().any(|d| d.message.contains("foo")));
    }

    #[test]
    fn given_item_pod_documentation_then_recognized() {
        let source = r#"package MyModule;
use Exporter 'import';
our @EXPORT = qw(process);

=over 4

=item process()

Processes data.

=back

=cut

sub process { 1 }
"#;

        let ast = make_program(vec![
            make_use("Exporter", 17, 39),
            make_var_decl(
                "our",
                "@",
                "EXPORT",
                Some(make_array_literal(vec![make_string_node("process", 56, 63)], 52, 64)),
                40,
                65,
            ),
            make_sub("process", 120, 138),
        ]);

        let mut diagnostics = Vec::new();
        check_pod_coverage(&ast, source, &mut diagnostics);

        assert!(diagnostics.is_empty(), "=item documentation should be recognized");
    }

    #[test]
    fn given_bold_pod_markup_then_recognized() {
        let source = r#"package MyModule;
use Exporter 'import';
our @EXPORT = qw(run);

=head2 B<run>

Run the thing.

=cut

sub run { 1 }
"#;

        let ast = make_program(vec![
            make_use("Exporter", 17, 39),
            make_var_decl(
                "our",
                "@",
                "EXPORT",
                Some(make_array_literal(vec![make_string_node("run", 56, 59)], 52, 60)),
                40,
                61,
            ),
            make_sub("run", 100, 113),
        ]);

        let mut diagnostics = Vec::new();
        check_pod_coverage(&ast, source, &mut diagnostics);

        assert!(diagnostics.is_empty(), "B<run> markup should match");
    }

    #[test]
    fn given_variable_exports_then_skipped() {
        let source = r#"package MyModule;
use Exporter 'import';
our @EXPORT_OK = qw($VERSION @DATA %CONFIG);
sub something { 1 }
"#;

        let ast = make_program(vec![
            make_use("Exporter", 17, 39),
            make_var_decl(
                "our",
                "@",
                "EXPORT_OK",
                Some(make_array_literal(
                    vec![
                        make_string_node("$VERSION", 59, 67),
                        make_string_node("@DATA", 68, 73),
                        make_string_node("%CONFIG", 74, 81),
                    ],
                    55,
                    82,
                )),
                40,
                83,
            ),
            make_sub("something", 84, 103),
        ]);

        let mut diagnostics = Vec::new();
        check_pod_coverage(&ast, source, &mut diagnostics);

        assert!(diagnostics.is_empty(), "variable exports should be ignored");
    }

    #[test]
    fn extract_pod_name_handles_various_formats() {
        assert_eq!(extract_pod_name(" foo"), Some("foo".to_string()));
        assert_eq!(extract_pod_name(" foo()"), Some("foo".to_string()));
        assert_eq!(extract_pod_name(" B<foo>"), Some("foo".to_string()));
        assert_eq!(extract_pod_name(" C<foo()>"), Some("foo".to_string()));
        assert_eq!(extract_pod_name(" foo_bar"), Some("foo_bar".to_string()));
        assert_eq!(extract_pod_name(""), None);
        assert_eq!(extract_pod_name(" "), None);
    }
}
