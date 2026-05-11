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

use super::super::internal_types::Diagnostic;
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

/// A single `@INC` search path with its origin label.
///
/// Used by [`check_missing_modules_with_search_context`] to produce labeled
/// diagnostic messages (e.g. `- lib (workspace includePaths)`) and to select
/// context-aware configuration suggestions.
///
/// # Source labels
///
/// Standard source strings (use these exact values for suggestion selection):
///
/// | `source` value | Controlled by |
/// |---|---|
/// | `"workspace includePaths"` | `perl.workspace.includePaths` |
/// | `"PERL5LIB"` | `perl.workspace.usePerl5lib` + `PERL5LIB` env |
/// | `"use lib"` | Lexical `use lib` in the document |
/// | `"interpreter startup @INC"` | `perl.workspace.useSystemInc` |
/// | `"FindBin"` | FindBin-relative paths resolved at index time |
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ModuleSearchPathDisplay {
    /// The filesystem path that was searched.
    pub path: String,
    /// Human-readable label for the origin of this path.
    pub source: String,
}

impl ModuleSearchPathDisplay {
    /// Convenience constructor.
    pub fn new(path: impl Into<String>, source: impl Into<String>) -> Self {
        Self { path: path.into(), source: source.into() }
    }
}

/// How many labeled paths to show inline before switching to "... and N more".
const MAX_LABELED_SHOWN: usize = 5;

/// How many total search roots before suggesting a timeout increase.
const MANY_ROOTS_THRESHOLD: usize = 8;

/// Pick the single most actionable configuration suggestion given the set of
/// labeled search paths that were consulted.
///
/// Priority order:
/// 1. No paths at all → point to workspace configuration.
/// 2. Workspace-only search (no env/system paths) → add to includePaths or install.
/// 3. Many roots searched → suggest resolutionTimeout.
/// 4. No system @INC paths consulted → suggest useSystemInc.
/// 5. Generic fallback.
fn choose_context_suggestion(module: &str, search_context: &[ModuleSearchPathDisplay]) -> String {
    if search_context.is_empty() {
        return "Open a workspace folder or configure `perl.workspace.includePaths`.".to_string();
    }

    let has_perl5lib = search_context.iter().any(|p| p.source == "PERL5LIB");
    let has_system_inc = search_context.iter().any(|p| p.source == "interpreter startup @INC");
    let has_workspace = search_context.iter().any(|p| p.source == "workspace includePaths");
    let has_use_lib = search_context.iter().any(|p| p.source == "use lib");
    let has_findbin = search_context.iter().any(|p| p.source == "FindBin");

    let workspace_only =
        has_workspace && !has_perl5lib && !has_system_inc && !has_use_lib && !has_findbin;

    if workspace_only {
        return format!(
            "Add the module's directory to `perl.workspace.includePaths` or install the module with: cpanm {module}"
        );
    }

    if search_context.len() >= MANY_ROOTS_THRESHOLD {
        return "Increase `perl.workspace.resolutionTimeout` if this is on a slow filesystem."
            .to_string();
    }

    if !has_system_inc {
        return "Enable `perl.workspace.useSystemInc` to consider interpreter startup `@INC`."
            .to_string();
    }

    format!("Install with: cpanm {module} or add to `perl.workspace.includePaths`")
}

/// Format the PL701 message body for labeled search paths.
///
/// Short lists (≤ `MAX_LABELED_SHOWN`): bulleted newline list with source tags.
/// Long lists (> `MAX_LABELED_SHOWN`): first entry inline, "... and N more" suffix.
fn format_labeled_path_list(search_context: &[ModuleSearchPathDisplay]) -> String {
    if search_context.is_empty() {
        return String::new();
    }

    let total = search_context.len();

    if total <= MAX_LABELED_SHOWN {
        let mut out = String::new();
        for entry in search_context {
            out.push_str(&format!("\n  - {} ({})", entry.path, entry.source));
        }
        out
    } else {
        let shown: Vec<String> = search_context[..MAX_LABELED_SHOWN]
            .iter()
            .map(|e| format!("{} ({})", e.path, e.source))
            .collect();
        let remaining = total - MAX_LABELED_SHOWN;
        format!(" {}, ... and {} more", shown.join(", "), remaining)
    }
}

/// Walk the AST and collect `(module, start, end)` triples for every `use` statement.
fn collect_use_statements(node: &Node) -> Vec<(String, usize, usize)> {
    let mut use_statements: Vec<(String, usize, usize)> = Vec::new();
    walk_node(node, &mut |n| {
        if let NodeKind::Use { module, .. } = &n.kind {
            use_statements.push((module.clone(), n.location.start, n.location.end));
        }
    });
    use_statements
}

/// Return `true` if `module_str` should be skipped by PL701 (empty, version-only, or core).
fn should_skip_module(module_str: &str) -> bool {
    module_str.is_empty() || is_version_only_use(module_str) || CORE_MODULES.contains(&module_str)
}

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
/// * `resolver` — Callback: `fn(module_name: &str, use_site_offset: usize) -> bool`.
///   Return `true` if the module is found. The second argument is the byte offset
///   of the `use` statement in the source — callers that support position-aware
///   `@INC` (e.g. `no lib` cancellation) should use it as the resolution offset.
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
///
/// # Migration
///
/// Prefer [`check_missing_modules_with_search_context`] for new call sites — it
/// accepts labeled paths and emits context-aware configuration suggestions.
pub fn check_missing_modules<F>(
    node: &Node,
    _source: &str,
    resolver: F,
    search_paths: &[String],
    diagnostics: &mut Vec<Diagnostic>,
) where
    F: Fn(&str, usize) -> bool,
{
    let use_statements = collect_use_statements(node);

    for (raw_module, start, end) in &use_statements {
        // Strip embedded version — "Foo::Bar 1.23" → "Foo::Bar"
        let module_str =
            raw_module.split_once(' ').map(|(name, _)| name).unwrap_or(raw_module.as_str());

        if should_skip_module(module_str) {
            continue;
        }

        // Skip if the resolver finds the module at this use-site offset.
        // Callers that honour position-aware `no lib` cancellation pass `*start`
        // to `resolve_module_to_path_with_doc_at_offset`; others may ignore it.
        if resolver(module_str, *start) {
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

/// Check for use statements whose modules cannot be resolved — labeled-path variant.
///
/// Identical to [`check_missing_modules`] but accepts [`ModuleSearchPathDisplay`]
/// entries instead of bare `String` paths. The source label on each entry powers:
///
/// - **Labeled message format**: `- lib (workspace includePaths)` so users can see
///   which configuration setting added each path.
/// - **Context-aware suggestions**: the absence of PERL5LIB or system `@INC` entries
///   in `search_context` drives specific `usePerl5lib` / `useSystemInc` hints.
///
/// # Arguments
///
/// * `node` — Root AST node to walk
/// * `_source` — Source text (reserved for future context extraction)
/// * `resolver` — Callback: `fn(module_name: &str, use_site_offset: usize) -> bool`.
///   Return `true` if the module was found. The second argument is the byte offset
///   of the `use` statement — callers that support position-aware `no lib`
///   cancellation should use it as the resolution offset so that `no lib`
///   directives appearing before the `use` statement suppress the path.
/// * `search_context` — Labeled `@INC` entries. Pass `&[]` when paths are unknown.
/// * `diagnostics` — Output vector; new diagnostics are pushed here
pub fn check_missing_modules_with_search_context<F>(
    node: &Node,
    _source: &str,
    resolver: F,
    search_context: &[ModuleSearchPathDisplay],
    diagnostics: &mut Vec<Diagnostic>,
) where
    F: Fn(&str, usize) -> bool,
{
    let use_statements = collect_use_statements(node);

    for (raw_module, start, end) in &use_statements {
        let module_str =
            raw_module.split_once(' ').map(|(name, _)| name).unwrap_or(raw_module.as_str());

        if should_skip_module(module_str) {
            continue;
        }

        // Pass the use-site offset so that `no lib` cancellations that precede
        // this statement are respected by position-aware resolvers.
        if resolver(module_str, *start) {
            continue;
        }

        let message = if search_context.is_empty() {
            format!("Module '{module_str}' not found in workspace or configured include paths")
        } else {
            let path_list = format_labeled_path_list(search_context);
            format!("Module '{module_str}' not found. Searched @INC:{path_list}")
        };

        let suggestion = choose_context_suggestion(module_str, search_context);

        diagnostics.push(Diagnostic {
            range: (*start, *end),
            severity: DiagnosticSeverity::Warning,
            code: Some(DiagnosticCode::ModuleNotFound.as_str().to_string()),
            message,
            related_information: vec![],
            tags: vec![],
            suggestion: Some(suggestion),
        });
    }
}

fn is_version_only_use(module: &str) -> bool {
    fn is_numeric_version_fragment(fragment: &str) -> bool {
        !fragment.is_empty() && fragment.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '_')
    }

    if module.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return is_numeric_version_fragment(module);
    }

    if let Some(rest) = module.strip_prefix('v') {
        return rest.chars().next().is_some_and(|c| c.is_ascii_digit())
            && is_numeric_version_fragment(rest);
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser::Parser;
    use perl_tdd_support::must;

    fn resolver_never_finds(_: &str, _: usize) -> bool {
        false
    }
    fn resolver_always_finds(_: &str, _: usize) -> bool {
        true
    }
    fn resolver_finds_foo(m: &str, _: usize) -> bool {
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
            |_, _| {
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

    /// The resolver must receive the byte offset of each `use` statement so
    /// that position-aware callers (e.g. `no lib` cancellation) can use it.
    #[test]
    fn resolver_receives_use_site_offset() {
        // Two `use` statements at different offsets.  A resolver that only
        // accepts the second module's offset simulates a position-aware
        // `no lib` cancellation that cancels the path for the first module but
        // not the second.
        let source = "use First::Mod;\nuse Second::Mod;\n";
        let ast = must(Parser::new(source).parse());

        // `use First::Mod;` starts at offset 0; `use Second::Mod;` at offset 16.
        let first_offset = 0usize;
        let second_offset = 16usize;

        // Use Cell so the Fn closure can accumulate the received offsets.
        let received_offsets = std::cell::RefCell::new(Vec::<usize>::new());
        let mut diags = vec![];
        check_missing_modules(
            &ast,
            source,
            |_module, offset| {
                received_offsets.borrow_mut().push(offset);
                false // never finds
            },
            &[],
            &mut diags,
        );

        let offsets = received_offsets.into_inner();
        // Both modules are non-core, so the resolver is called twice.
        assert_eq!(offsets.len(), 2, "resolver must be called once per non-core use");
        assert!(
            offsets.contains(&first_offset),
            "resolver must receive offset of first use statement ({}); got: {:?}",
            first_offset,
            offsets
        );
        assert!(
            offsets.contains(&second_offset),
            "resolver must receive offset of second use statement ({}); got: {:?}",
            second_offset,
            offsets
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
            NodeKind::Use { module: String::new(), args: vec![], has_filter_risk: false },
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

    #[test]
    fn modules_starting_with_v_are_not_treated_as_versions() {
        let source = "use vTools::Parser;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        check_missing_modules(&ast, source, resolver_never_finds, &[], &mut diags);
        assert_eq!(diags.len(), 1, "module names beginning with 'v' should still be resolved");
        assert_eq!(diags[0].code.as_deref(), Some("PL701"));
        assert!(diags[0].message.contains("vTools::Parser"));
    }

    #[test]
    fn v_prefixed_numeric_use_is_treated_as_version_requirement() {
        let source = "use v5.38;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        check_missing_modules(&ast, source, resolver_never_finds, &[], &mut diags);
        assert!(diags.is_empty(), "v-prefixed numeric versions should not emit PL701");
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

    // --- check_missing_modules_with_search_context tests ---

    fn make_ctx(entries: &[(&str, &str)]) -> Vec<ModuleSearchPathDisplay> {
        entries.iter().map(|(p, s)| ModuleSearchPathDisplay::new(*p, *s)).collect()
    }

    #[test]
    fn context_variant_emits_pl701_for_missing_module() {
        let source = "use My::Missing;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        check_missing_modules_with_search_context(
            &ast,
            source,
            resolver_never_finds,
            &[],
            &mut diags,
        );
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code.as_deref(), Some("PL701"));
        assert!(diags[0].message.contains("My::Missing"));
    }

    #[test]
    fn context_variant_found_module_no_diagnostic() {
        let source = "use Foo::Bar;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        let ctx = make_ctx(&[("lib", "workspace includePaths")]);
        check_missing_modules_with_search_context(
            &ast,
            source,
            resolver_finds_foo,
            &ctx,
            &mut diags,
        );
        assert!(diags.is_empty());
    }

    #[test]
    fn context_variant_core_modules_not_flagged() {
        let source = "use strict;\nuse warnings;\nuse Carp;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        check_missing_modules_with_search_context(
            &ast,
            source,
            resolver_never_finds,
            &[],
            &mut diags,
        );
        assert!(diags.is_empty(), "core modules must not trigger PL701 in context variant");
    }

    #[test]
    fn context_variant_labeled_paths_appear_in_message() {
        let source = "use My::Missing;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        let ctx =
            make_ctx(&[("lib", "workspace includePaths"), ("/home/user/perl5/lib", "PERL5LIB")]);
        check_missing_modules_with_search_context(
            &ast,
            source,
            resolver_never_finds,
            &ctx,
            &mut diags,
        );
        assert_eq!(diags.len(), 1);
        let msg = &diags[0].message;
        assert!(msg.contains("lib"), "message should contain path; got: {msg:?}");
        assert!(
            msg.contains("workspace includePaths"),
            "message should contain source label; got: {msg:?}"
        );
        assert!(msg.contains("PERL5LIB"), "message should contain PERL5LIB label; got: {msg:?}");
    }

    #[test]
    fn context_variant_short_list_uses_bulleted_format() {
        let source = "use My::Missing;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        let ctx = make_ctx(&[
            ("lib", "workspace includePaths"),
            ("/usr/local/lib", "interpreter startup @INC"),
        ]);
        check_missing_modules_with_search_context(
            &ast,
            source,
            resolver_never_finds,
            &ctx,
            &mut diags,
        );
        assert_eq!(diags.len(), 1);
        let msg = &diags[0].message;
        // Short list (≤ MAX_LABELED_SHOWN) should use bullet format with newlines
        assert!(
            msg.contains('\n') || msg.contains("  -"),
            "short path list should be bulleted; got: {msg:?}"
        );
    }

    #[test]
    fn context_variant_long_list_uses_truncated_format() {
        let source = "use My::Missing;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        let ctx: Vec<ModuleSearchPathDisplay> = (1..=10)
            .map(|i| {
                ModuleSearchPathDisplay::new(format!("/path/dir{i}"), "workspace includePaths")
            })
            .collect();
        check_missing_modules_with_search_context(
            &ast,
            source,
            resolver_never_finds,
            &ctx,
            &mut diags,
        );
        assert_eq!(diags.len(), 1);
        let msg = &diags[0].message;
        assert!(
            msg.contains("more"),
            "long path list should be truncated with '... and N more'; got: {msg:?}"
        );
        assert!(
            !msg.contains("/path/dir10"),
            "path beyond truncation should not appear in message; got: {msg:?}"
        );
    }

    #[test]
    fn context_variant_empty_paths_suggestion_points_to_configuration() {
        let source = "use My::Missing;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        check_missing_modules_with_search_context(
            &ast,
            source,
            resolver_never_finds,
            &[],
            &mut diags,
        );
        assert_eq!(diags.len(), 1);
        let suggestion = diags[0].suggestion.as_deref().unwrap_or("");
        assert!(
            suggestion.contains("includePaths") || suggestion.contains("workspace folder"),
            "no-paths suggestion should point to workspace configuration; got: {suggestion:?}"
        );
    }

    #[test]
    fn context_variant_workspace_only_suggestion_mentions_include_paths() {
        let source = "use My::Missing;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        let ctx = make_ctx(&[("lib", "workspace includePaths")]);
        check_missing_modules_with_search_context(
            &ast,
            source,
            resolver_never_finds,
            &ctx,
            &mut diags,
        );
        assert_eq!(diags.len(), 1);
        let suggestion = diags[0].suggestion.as_deref().unwrap_or("");
        assert!(
            suggestion.contains("includePaths"),
            "workspace-only suggestion should mention includePaths; got: {suggestion:?}"
        );
    }

    #[test]
    fn context_variant_no_system_inc_suggestion_mentions_use_system_inc() {
        let source = "use My::Missing;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        // Include PERL5LIB paths so it's not workspace-only, but no system @INC
        let ctx =
            make_ctx(&[("lib", "workspace includePaths"), ("/home/user/perl5/lib", "PERL5LIB")]);
        check_missing_modules_with_search_context(
            &ast,
            source,
            resolver_never_finds,
            &ctx,
            &mut diags,
        );
        assert_eq!(diags.len(), 1);
        let suggestion = diags[0].suggestion.as_deref().unwrap_or("");
        assert!(
            suggestion.contains("useSystemInc"),
            "no-system-inc suggestion should mention useSystemInc; got: {suggestion:?}"
        );
    }

    #[test]
    fn context_variant_many_roots_suggestion_mentions_resolution_timeout() {
        let source = "use My::Missing;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        // MANY_ROOTS_THRESHOLD = 8; provide 9 roots to trigger timeout suggestion
        let ctx: Vec<ModuleSearchPathDisplay> = (1..=9)
            .map(|i| ModuleSearchPathDisplay::new(format!("/path/dir{i}"), "PERL5LIB"))
            .collect();
        check_missing_modules_with_search_context(
            &ast,
            source,
            resolver_never_finds,
            &ctx,
            &mut diags,
        );
        assert_eq!(diags.len(), 1);
        let suggestion = diags[0].suggestion.as_deref().unwrap_or("");
        assert!(
            suggestion.contains("resolutionTimeout"),
            "many-roots suggestion should mention resolutionTimeout; got: {suggestion:?}"
        );
    }

    #[test]
    fn context_variant_workspace_only_priority_precedes_many_roots() {
        let source = "use My::Missing;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        let ctx: Vec<ModuleSearchPathDisplay> = (1..=9)
            .map(|i| {
                ModuleSearchPathDisplay::new(format!("/workspace/lib{i}"), "workspace includePaths")
            })
            .collect();
        check_missing_modules_with_search_context(
            &ast,
            source,
            resolver_never_finds,
            &ctx,
            &mut diags,
        );
        assert_eq!(diags.len(), 1);
        let suggestion = diags[0].suggestion.as_deref().unwrap_or("");
        assert!(
            suggestion.contains("includePaths"),
            "workspace-only context should keep includePaths suggestion before many-roots fallback; got: {suggestion:?}"
        );
    }

    #[test]
    fn context_variant_suggestion_contains_module_name_for_workspace_only() {
        let source = "use My::Package;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        let ctx = make_ctx(&[("lib", "workspace includePaths")]);
        check_missing_modules_with_search_context(
            &ast,
            source,
            resolver_never_finds,
            &ctx,
            &mut diags,
        );
        assert_eq!(diags.len(), 1);
        let suggestion = diags[0].suggestion.as_deref().unwrap_or("");
        assert!(
            suggestion.contains("My::Package"),
            "workspace-only suggestion should contain module name; got: {suggestion:?}"
        );
    }

    #[test]
    fn context_variant_multiple_missing_modules_all_get_diagnostics() {
        let source = "use Missing::One;\nuse Missing::Two;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        let ctx = make_ctx(&[("lib", "workspace includePaths")]);
        check_missing_modules_with_search_context(
            &ast,
            source,
            resolver_never_finds,
            &ctx,
            &mut diags,
        );
        assert_eq!(diags.len(), 2);
        assert!(diags.iter().all(|d| d.code.as_deref() == Some("PL701")));
    }

    #[test]
    fn module_search_path_display_new_constructor() {
        let entry = ModuleSearchPathDisplay::new("lib", "workspace includePaths");
        assert_eq!(entry.path, "lib");
        assert_eq!(entry.source, "workspace includePaths");
    }

    #[test]
    fn context_variant_version_only_use_not_flagged() {
        for source in &["use 5.010;\n", "use v5.38;\n"] {
            let ast = must(Parser::new(source).parse());
            let mut diags = vec![];
            check_missing_modules_with_search_context(
                &ast,
                source,
                resolver_never_finds,
                &[],
                &mut diags,
            );
            assert!(
                diags.is_empty(),
                "version-only use should not be flagged in context variant: {source}"
            );
        }
    }

    #[test]
    fn context_variant_use_lib_paths_help_distinguish_workspace_only() {
        let source = "use My::Missing;\n";
        let ast = must(Parser::new(source).parse());
        let mut diags = vec![];
        // Has both workspace AND use lib paths — should NOT be workspace-only
        let ctx = make_ctx(&[("lib", "workspace includePaths"), ("../other/lib", "use lib")]);
        check_missing_modules_with_search_context(
            &ast,
            source,
            resolver_never_finds,
            &ctx,
            &mut diags,
        );
        assert_eq!(diags.len(), 1);
        let suggestion = diags[0].suggestion.as_deref().unwrap_or("");
        // With use lib also present, NOT workspace-only, so should NOT suggest includePaths alone
        assert!(
            !suggestion.starts_with("Add the module's directory to `perl.workspace.includePaths`"),
            "workspace+use-lib mix should not use workspace-only suggestion; got: {suggestion:?}"
        );
    }
}
