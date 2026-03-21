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

use perl_diagnostics_codes::DiagnosticCode;
use perl_lsp_diagnostic_types::{Diagnostic, DiagnosticSeverity};
use perl_parser_core::ast::{Node, NodeKind};

use super::super::walker::walk_node;

/// Perl core modules that ship with every Perl installation.
///
/// This list prevents false positives when `use_system_inc` is false.
/// Conservative (under-includes) — a missed detection is better than
/// a false positive that erodes diagnostic trust.
pub const CORE_MODULES: &[&str] = &[
    // Pragmas (no-network, no-filesystem)
    "strict",
    "warnings",
    "utf8",
    "feature",
    "constant",
    "lib",
    "parent",
    "base",
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
    "List::MoreUtils",
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
    "Sys::Syslog",
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
    "POSIX",
    "PerlIO",
    "Safe",
    "Scalar::Util",
    "Sys::Syslog",
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
    "parent",
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

        diagnostics.push(Diagnostic {
            range: (*start, *end),
            severity: DiagnosticSeverity::Warning,
            code: Some(DiagnosticCode::ModuleNotFound.as_str().to_string()),
            message: format!(
                "Module '{}' not found in workspace or configured include paths",
                module_str
            ),
            related_information: vec![],
            tags: vec![],
            suggestion: Some(format!("Install with: cpanm {} or add to cpanfile", module_str)),
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
        check_missing_modules(&ast, source, resolver_never_finds, &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code.as_deref(), Some("PL701"));
        assert!(diags[0].message.contains("Missing::Module"));
    }

    #[test]
    fn found_module_no_diagnostic() {
        let source = "use Foo::Bar;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        check_missing_modules(&ast, source, resolver_finds_foo, &mut diags);
        assert!(diags.is_empty());
    }

    #[test]
    fn version_only_use_not_flagged() {
        for source in &["use 5.010;\n", "use v5.38;\n"] {
            let ast = must(Parser::new(source).parse());
            let mut diags = vec![];
            check_missing_modules(&ast, source, resolver_never_finds, &mut diags);
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
            check_missing_modules(&ast, &source, resolver_never_finds, &mut diags);
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
        check_missing_modules(&ast, source, resolver_finds_foo, &mut diags);
        assert!(diags.is_empty(), "versioned use should strip version before resolver lookup");
    }

    #[test]
    fn diagnostic_range_covers_use_statement() {
        let source = "use Missing::Mod;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        check_missing_modules(&ast, source, resolver_never_finds, &mut diags);
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
        check_missing_modules(&ast, source, resolver_always_finds, &mut diags);
        assert!(diags.is_empty());
    }

    #[test]
    fn multiple_missing_modules_emits_multiple_diagnostics() {
        let source = "use Missing::One;\nuse Missing::Two;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        check_missing_modules(&ast, source, resolver_never_finds, &mut diags);
        assert_eq!(diags.len(), 2);
        assert!(diags.iter().all(|d| d.code.as_deref() == Some("PL701")));
    }

    #[test]
    fn mixed_present_and_missing_only_flags_missing() {
        let source = "use Foo::Bar;\nuse Missing::One;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        check_missing_modules(&ast, source, resolver_finds_foo, &mut diags);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("Missing::One"));
    }

    #[test]
    fn severity_is_warning() {
        let source = "use Missing::Module;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        check_missing_modules(&ast, source, resolver_never_finds, &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, DiagnosticSeverity::Warning);
    }
}
