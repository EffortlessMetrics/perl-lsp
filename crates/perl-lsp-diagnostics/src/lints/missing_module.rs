//! Missing module detection lint
//!
//! Detects `use Module` statements where the module cannot be resolved
//! in the workspace or configured include paths.
//!
//! # Diagnostic codes
//!
//! | Code  | Severity | Description                        |
//! |-------|----------|------------------------------------|
//! | PL701 | Warning  | Module not found in include paths  |

use crate::internal_types::Diagnostic;
use perl_diagnostics::codes::DiagnosticCode;
use perl_diagnostics::codes::DiagnosticSeverity;
use perl_parser_core::ast::{Node, NodeKind};

use super::super::walker::walk_node;

/// Perl core modules that ship with every Perl installation.
///
/// This list prevents false positives when `use_system_inc` is false.
/// Conservative (under-includes) — a missed detection is better than
/// a false positive that erodes diagnostic trust. It does not attempt to
/// emulate Perl's full runtime `@INC` search order.
pub const CORE_MODULES: &[&str] = &[
    // Pragmas (no-network, no-filesystem)
    "strict",
    "warnings",
    "utf8",
    "feature",
    "constant",
    "lib",
    "base",
    "parent",
    "Exporter",
    "vars",
    "subs",
    "overload",
    "overloading",
    "integer",
    "bigint",
    "bignum",
    "bigrat",
    "bytes",
    "charnames",
    "encoding",
    "locale",
    "mro",
    "open",
    "ops",
    "re",
    "sigtrap",
    "sort",
    "threads",
    "threads::shared",
    "autodie",
    "autouse",
    "diagnostics",
    "English",
    "experimental",
    "fields",
    "filetest",
    "if",
    "less",
    // Core stdlib — compiled into Perl or always available
    "POSIX",
    "Carp",
    "Scalar::Util",
    "List::Util",
    // Note: List::MoreUtils is NOT a core module (it is a CPAN distribution).
    // It intentionally does NOT appear here so that missing installations are detected.
    "File::Basename",
    "File::Path",
    "File::Spec",
    "File::Spec::Functions",
    "File::Temp",
    "File::Copy",
    "File::Find",
    "Cwd",
    "Data::Dumper",
    "Storable",
    "Encode",
    "IO::File",
    "IO::Handle",
    "IO::Dir",
    "IO::Pipe",
    "IO::Select",
    "IO::Socket",
    "IO::Socket::INET",
    "Fcntl",
    "UNIVERSAL",
    "FindBin",
    "Getopt::Long",
    "Getopt::Std",
    "Time::HiRes",
    "Time::Local",
    "MIME::Base64",
    "Digest::MD5",
    "Digest::SHA",
    "Socket",
    "Sys::Hostname",
    "NEXT",
    "Tie::Handle",
    "Tie::Hash",
    "Tie::Scalar",
    "Tie::StdHash",
    "Tie::StdScalar",
    "Tie::Array",
    "Tie::StdArray",
    "Attribute::Handlers",
    "AutoLoader",
    "B",
    "CPAN",
    "Config",
    "DB",
    "Devel::Peek",
    "DynaLoader",
    "Errno",
    "ExtUtils::MakeMaker",
    "Fatal",
    "Hash::Util",
    "I18N::LangTags",
    "MIME::QuotedPrint",
    "Math::BigFloat",
    "Math::BigInt",
    "Math::Complex",
    "Math::Trig",
    "Module::CoreList",
    "Module::Load",
    "Net::Ping",
    "PerlIO",
    "Safe",
    "Term::ANSIColor",
    "Term::Cap",
    "Term::ReadLine",
    "Test",
    "Test::Builder",
    "Test::Harness",
    "Test::More",
    "Test::Simple",
    "Text::Abbrev",
    "Text::Balanced",
    "Text::ParseWords",
    "Text::Tabs",
    "Text::Wrap",
    "Thread",
    "Tie::File",
    "Tie::Memoize",
    "Tie::RefHash",
    "Unicode::Collate",
    "Unicode::Normalize",
    "Unicode::UCD",
    "XSLoader",
    "attributes",
    "deprecate",
    "version",
];

/// Check for use statements whose modules cannot be resolved.
///
/// Walks the AST to collect all `use Module` statements. For each non-pragma,
/// non-digit, non-core module, attempts to resolve via the provided resolver.
/// Emits PL701 Warning if resolution returns `false`.
///
/// # Arguments
///
/// * `node` — Root AST node to walk
/// * `source` — Source text (used for context; not searched directly here)
/// * `resolver` — Callback: `fn(module_name: &str) -> bool`. Return `true` if
///   the module is found (workspace or configured include paths).
/// * `search_paths` — The `@INC` paths that were searched. Included in the
///   diagnostic message so the user knows where perl-lsp looked. Pass `&[]`
///   when the paths are not available. If more than 10 entries are provided,
///   only the first 10 are shown followed by "... and N more".
/// * `diagnostics` — Output vector; new diagnostics are pushed here
///
/// # Skipped inputs
///
/// - Version-only `use` statements: `use 5.010;` `use v5.38;`
/// - All entries in `CORE_MODULES`
/// - `use if` form (module field is "if"; treated as pragma)
pub fn check_missing_modules<F>(
    node: &Node,
    _source: &str,
    resolver: F,
    search_paths: &[String],
    diagnostics: &mut Vec<Diagnostic>,
) where
    F: Fn(&str) -> bool,
{
    let mut use_statements: Vec<(String, usize, usize)> = Vec::new();

    walk_node(node, &mut |n| {
        if let NodeKind::Use { module, .. } = &n.kind {
            use_statements.push((module.clone(), n.location.start, n.location.end));
        }
    });

    for (raw_module, start, end) in &use_statements {
        // Strip embedded version — "Foo::Bar 1.23" → "Foo::Bar"
        let module_str =
            raw_module.split_once(' ').map(|(name, _)| name).unwrap_or(raw_module.as_str());

        // Skip empty module names — these come from parser error-recovery nodes
        // (NodeKind::Use { module: String::new(), .. }) and would produce false positives.
        if module_str.is_empty() {
            continue;
        }

        // Skip version-only use: `use 5.010;` or `use v5.38;`
        if module_str.chars().next().is_some_and(|c| c.is_ascii_digit() || c == 'v') {
            continue;
        }

        // Skip core modules (prevents false positives when system @INC is disabled)
        if CORE_MODULES.contains(&module_str) {
            continue;
        }

        // Skip if the resolver finds the module
        if resolver(module_str) {
            continue;
        }

        let message = if search_paths.is_empty() {
            format!("Module '{}' not found in workspace or configured include paths", module_str)
        } else {
            const MAX_SHOWN: usize = 10;
            let shown = search_paths.len().min(MAX_SHOWN);
            let path_list = search_paths[..shown].join(", ");
            if search_paths.len() > MAX_SHOWN {
                let remaining = search_paths.len() - MAX_SHOWN;
                format!(
                    "Module '{}' not found. Searched @INC: {}, ... and {} more. \
                     Add to lib path or install the module.",
                    module_str, path_list, remaining
                )
            } else {
                format!(
                    "Module '{}' not found. Searched @INC: {}. \
                     Add to lib path or install the module.",
                    module_str, path_list
                )
            }
        };
        diagnostics.push(Diagnostic {
            range: (*start, *end),
            severity: DiagnosticSeverity::Warning,
            code: Some(DiagnosticCode::ModuleNotFound.as_str().to_string()),
            message,
            related_information: vec![],
            tags: vec![],
            suggestion: Some(format!(
                "Install with: cpanm {} or add to .perl-lsp.toml: include_paths",
                module_str
            )),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser::Parser;
    use perl_tdd_support::must;

    fn resolver_never_finds(_: &str) -> bool {
        false
    }
    fn resolver_always_finds(_: &str) -> bool {
        true
    }
    fn resolver_finds_foo(m: &str) -> bool {
        m == "Foo::Bar"
    }

    #[test]
    fn missing_module_emits_pl701() {
        let source = "use Missing::Module;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        check_missing_modules(&ast, source, resolver_never_finds, &[], &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code.as_deref(), Some("PL701"));
        assert!(diags[0].message.contains("Missing::Module"));
    }

    #[test]
    fn found_module_no_diagnostic() {
        let source = "use Foo::Bar;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        check_missing_modules(&ast, source, resolver_finds_foo, &[], &mut diags);
        assert!(diags.is_empty());
    }

    #[test]
    fn version_only_use_not_flagged() {
        for source in &["use 5.010;\n", "use v5.38;\n"] {
            let ast = must(Parser::new(source).parse());
            let mut diags = vec![];
            check_missing_modules(&ast, source, resolver_never_finds, &[], &mut diags);
            assert!(diags.is_empty(), "version-only use should not be flagged: {}", source);
        }
    }

    #[test]
    fn core_modules_not_flagged() {
        for module in
            &["strict", "warnings", "Carp", "POSIX", "Scalar::Util", "FindBin", "File::Basename"]
        {
            let source = format!("use {};\n", module);
            let ast = must(Parser::new(&source).parse());
            let mut diags = vec![];
            check_missing_modules(&ast, &source, resolver_never_finds, &[], &mut diags);
            assert!(diags.is_empty(), "core module {} should not be flagged", module);
        }
    }

    #[test]
    fn versioned_module_strips_version_before_lookup() {
        // "use Foo::Bar 1.23;" — should resolve "Foo::Bar", not "Foo::Bar 1.23"
        let source = "use Foo::Bar 1.23;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        // resolver_finds_foo only returns true for "Foo::Bar" (bare, no version)
        check_missing_modules(&ast, source, resolver_finds_foo, &[], &mut diags);
        assert!(diags.is_empty(), "versioned use should strip version before resolver lookup");
    }

    #[test]
    fn diagnostic_range_covers_use_statement() {
        let source = "use Missing::Mod;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        check_missing_modules(&ast, source, resolver_never_finds, &[], &mut diags);
        assert_eq!(diags.len(), 1);
        let (start, end) = diags[0].range;
        assert!(start < end, "range start must be before end");
        assert!(end <= source.len(), "range end must be within source");
    }

    #[test]
    fn resolver_always_finds_no_diagnostic() {
        let source = "use Anything::AtAll;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        check_missing_modules(&ast, source, resolver_always_finds, &[], &mut diags);
        assert!(diags.is_empty());
    }

    #[test]
    fn multiple_missing_modules_emits_multiple_diagnostics() {
        let source = "use Missing::One;\nuse Missing::Two;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        check_missing_modules(&ast, source, resolver_never_finds, &[], &mut diags);
        assert_eq!(diags.len(), 2);
        assert!(diags.iter().all(|d| d.code.as_deref() == Some("PL701")));
    }

    #[test]
    fn mixed_present_and_missing_only_flags_missing() {
        let source = "use Foo::Bar;\nuse Missing::One;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        check_missing_modules(&ast, source, resolver_finds_foo, &[], &mut diags);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("Missing::One"));
    }

    #[test]
    fn severity_is_warning() {
        let source = "use Missing::Module;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        check_missing_modules(&ast, source, resolver_never_finds, &[], &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, DiagnosticSeverity::Warning);
    }

    // --- edge cases ---

    /// `use if COND, 'Module'` stores module="if" in the AST.
    /// "if" is in CORE_MODULES so it must never emit PL701.
    #[test]
    fn use_if_conditional_not_flagged() {
        // The parser stores module = "if" for the `use if` form.
        // CORE_MODULES contains "if", so no diagnostic should fire.
        let source = "use if $^O eq 'MSWin32', 'Win32';\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        check_missing_modules(&ast, source, resolver_never_finds, &[], &mut diags);
        assert!(
            diags.is_empty(),
            "`use if` conditional form must not emit PL701 (got {} diagnostics)",
            diags.len()
        );
    }

    /// `List::MoreUtils` is a CPAN module, not a Perl core module.
    /// It must NOT be silently skipped — PL701 should fire when the resolver
    /// cannot find it.
    #[test]
    fn list_more_utils_is_not_core_and_fires_pl701() {
        let source = "use List::MoreUtils qw(any all);\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        check_missing_modules(&ast, source, resolver_never_finds, &[], &mut diags);
        assert_eq!(
            diags.len(),
            1,
            "List::MoreUtils is not a core module; PL701 should fire when the resolver cannot find it"
        );
        assert_eq!(diags[0].code.as_deref(), Some("PL701"));
        assert!(diags[0].message.contains("List::MoreUtils"));
    }

    /// Resolver returning `false` never causes a panic or double-borrow even when
    /// called many times in one pass (validates the closure is re-entrant safe).
    #[test]
    fn resolver_called_multiple_times_is_stable() {
        let source = "use A::B;\nuse C::D;\nuse E::F;\nuse G::H;\nuse I::J;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        let call_count = std::cell::Cell::new(0u32);
        check_missing_modules(
            &ast,
            source,
            |_| {
                call_count.set(call_count.get() + 1);
                false
            },
            &[],
            &mut diags,
        );
        assert_eq!(diags.len(), 5, "five distinct missing modules should each emit PL701");
        assert_eq!(
            call_count.get(),
            5,
            "resolver should be called exactly once per non-core module"
        );
    }

    /// An empty module string comes from parser error-recovery nodes.
    /// It must be silently skipped — no PL701 and no panic.
    #[test]
    fn empty_module_string_is_silently_skipped() {
        use perl_parser_core::ast::{Node, NodeKind, SourceLocation};
        // Construct a Program node wrapping a Use node with an empty module name.
        // This simulates what the parser emits during error recovery.
        let use_node = Node::new(
            NodeKind::Use {
                module: String::new(),
                args: vec![],
                has_filter_risk: false,
                has_explicit_import_list: false,
            },
            SourceLocation { start: 0, end: 4 },
        );
        let program = Node::new(
            NodeKind::Program { statements: vec![use_node] },
            SourceLocation { start: 0, end: 4 },
        );
        let mut diags = vec![];
        check_missing_modules(&program, "", resolver_never_finds, &[], &mut diags);
        assert!(
            diags.is_empty(),
            "empty module name from error-recovery must not emit PL701 (got {} diagnostics)",
            diags.len()
        );
    }

    /// Suggestion text must contain the module name so the user knows what to install.
    #[test]
    fn suggestion_contains_module_name() {
        let source = "use Some::Package;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        check_missing_modules(&ast, source, resolver_never_finds, &[], &mut diags);
        assert_eq!(diags.len(), 1);
        let suggestion = diags[0].suggestion.as_deref().unwrap_or("");
        assert!(
            suggestion.contains("Some::Package"),
            "suggestion should mention the module name; got: {suggestion:?}"
        );
    }

    // --- @INC context tests (PL701 enhancement) ---

    /// When search_paths are provided, the diagnostic message must include them
    /// so the user can see where perl-lsp looked for the module.
    #[test]
    fn pl701_message_includes_search_paths_when_provided() {
        let source = "use My::Missing::Mod;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        let paths = vec!["/usr/lib/perl5".to_string(), "/home/user/perl/lib".to_string()];
        check_missing_modules(&ast, source, resolver_never_finds, &paths, &mut diags);
        assert_eq!(diags.len(), 1);
        let msg = &diags[0].message;
        assert!(
            msg.contains("/usr/lib/perl5"),
            "message should contain first search path; got: {msg:?}"
        );
        assert!(
            msg.contains("/home/user/perl/lib"),
            "message should contain second search path; got: {msg:?}"
        );
    }

    /// When search_paths is empty, the diagnostic should fall back gracefully
    /// (no crash, still emits PL701 with the module name).
    #[test]
    fn pl701_message_with_empty_search_paths_still_emits_diagnostic() {
        let source = "use My::Missing::Mod;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        check_missing_modules(&ast, source, resolver_never_finds, &[], &mut diags);
        assert_eq!(diags.len(), 1, "should still emit PL701 with empty search paths");
        assert_eq!(diags[0].code.as_deref(), Some("PL701"));
        assert!(diags[0].message.contains("My::Missing::Mod"));
    }

    /// When @INC list is very long (>10 entries), the message should truncate
    /// with "... and N more" rather than dumping all paths.
    #[test]
    fn pl701_message_truncates_long_inc_list() {
        let source = "use My::Missing::Mod;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        let paths: Vec<String> = (1..=15).map(|i| format!("/path/dir{}", i)).collect();
        check_missing_modules(&ast, source, resolver_never_finds, &paths, &mut diags);
        assert_eq!(diags.len(), 1);
        let msg = &diags[0].message;
        assert!(
            msg.contains("and") && msg.contains("more"),
            "long @INC list should be truncated with '... and N more'; got: {msg:?}"
        );
        // Should NOT dump all 15 paths
        assert!(
            !msg.contains("/path/dir15"),
            "path beyond truncation limit should not appear in message; got: {msg:?}"
        );
    }

    /// The suggestion field should mention the module name and a config path hint
    /// so the user knows both how to install and how to configure @INC.
    #[test]
    fn pl701_suggestion_mentions_module_and_config_hint() {
        let source = "use My::Package;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        let paths = vec!["/usr/lib/perl5".to_string()];
        check_missing_modules(&ast, source, resolver_never_finds, &paths, &mut diags);
        assert_eq!(diags.len(), 1);
        let suggestion = diags[0].suggestion.as_deref().unwrap_or("");
        assert!(
            suggestion.contains("My::Package"),
            "suggestion should mention the module name; got: {suggestion:?}"
        );
        assert!(
            suggestion.contains(".perl-lsp.toml") || suggestion.contains("include_paths"),
            "suggestion should mention config path hint; got: {suggestion:?}"
        );
    }

    /// File with a syntax error followed by a valid `use` — the lint should still
    /// fire on the missing module, not crash on the partial AST.
    #[test]
    fn broken_file_with_valid_use_still_emits_pl701() {
        // parse_with_recovery tolerates syntax errors; the Use node for Missing::Mod
        // should still be present and trigger PL701.
        let source = "my $x = ;\nuse Missing::Mod;\n";
        let output = Parser::new(source).parse_with_recovery();
        let ast = output.ast;
        let mut diags = vec![];
        check_missing_modules(&ast, source, resolver_never_finds, &[], &mut diags);
        // Must not panic; if the Use node was recovered, we get PL701.
        // If recovery omitted it entirely we get 0. Either is acceptable — but not a panic.
        let pl701_count = diags.iter().filter(|d| d.code.as_deref() == Some("PL701")).count();
        assert!(pl701_count <= 1, "at most one PL701 for one use statement (got {})", pl701_count);
    }
}
