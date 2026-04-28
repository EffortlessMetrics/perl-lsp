//! Built-in snippet completions for common Perl patterns.

use super::{context::CompletionContext, items::CompletionItem};

struct Snippet {
    trigger: &'static str,
    label: &'static str,
    body: &'static str,
    detail: &'static str,
    doc: &'static str,
}

const SNIPPETS: &[Snippet] = &[
    Snippet {
        trigger: "subr",
        label: "subr",
        body: "sub ${1:name} {\n    my (${2:\\$self}) = @_;\n    ${3:# body}\n    return ${0};\n}",
        detail: "subroutine (with return)",
        doc: "Subroutine with parameter unpacking and return value.",
    },
    Snippet {
        trigger: "submethod",
        label: "submethod",
        body: "sub ${1:method_name} {\n    my ($self${2:, @args}) = @_;\n    $0\n}",
        detail: "method subroutine",
        doc: "Method subroutine with `$self` unpacking.",
    },
    Snippet {
        trigger: "ife",
        label: "ife",
        body: "if (${1:condition}) {\n    ${2:# then}\n} else {\n    ${0:# else}\n}",
        detail: "if/else block",
        doc: "If/else conditional block.",
    },
    Snippet {
        trigger: "ifeif",
        label: "ifeif",
        body: "if (${1:condition}) {\n    ${2:# then}\n} elsif (${3:condition}) {\n    ${0:# elsif}\n}",
        detail: "if/elsif block",
        doc: "If/elsif conditional block.",
    },
    Snippet {
        trigger: "until",
        label: "until",
        body: "until (${1:condition}) {\n    $0\n}",
        detail: "until loop",
        doc: "Loop that runs until condition is true.",
    },
    Snippet {
        trigger: "evaldo",
        label: "evaldo",
        body: "eval {\n    ${1:# try}\n} or do {\n    my \\$err = $@;\n    ${0:# catch}\n};",
        detail: "eval/do (try/catch)",
        doc: "Perl try/catch idiom using `eval { } or do { }`.",
    },
    Snippet {
        trigger: "evaldie",
        label: "evaldie",
        body: "eval {\n    ${1:# code}\n    1;\n} or die \"${0:Error}: $@\";",
        detail: "eval or die",
        doc: "Eval block with die on failure.",
    },
    Snippet {
        trigger: "usesw",
        label: "usesw",
        body: "use strict;\nuse warnings;\n$0",
        detail: "use strict/warnings",
        doc: "Standard Perl boilerplate.",
    },
    Snippet {
        trigger: "usev536",
        label: "usev536",
        body: "use v5.36;\n$0",
        detail: "use v5.36",
        doc: "Modern Perl: enables strict, warnings, signatures.",
    },
    Snippet {
        trigger: "usev540",
        label: "usev540",
        body: "use v5.40;\n$0",
        detail: "use v5.40",
        doc: "Latest stable Perl: class syntax, defer.",
    },
    Snippet {
        trigger: "selfshift",
        label: "selfshift",
        body: "my \\$self = shift;\n$0",
        detail: "my $self = shift",
        doc: "Method preamble.",
    },
    Snippet {
        trigger: "selfargs",
        label: "selfargs",
        body: "my (\\$self${1:, @args}) = @_;\n$0",
        detail: "my ($self, ...) = @_",
        doc: "Method preamble with args.",
    },
    Snippet {
        trigger: "classshift",
        label: "classshift",
        body: "my \\$class = shift;\n$0",
        detail: "my $class = shift",
        doc: "Constructor preamble.",
    },
    Snippet {
        trigger: "newctor",
        label: "newctor",
        body: "sub new {\n    my (\\$class, %args) = @_;\n    my \\$self = bless {%args}, \\$class;\n    return \\$self;\n}\n$0",
        detail: "constructor (new)",
        doc: "Basic Perl constructor using `bless`.",
    },
    Snippet {
        trigger: "mooclass",
        label: "mooclass",
        body: "package ${1:MyClass};\nuse Moo;\n\n${0}\n\n1;",
        detail: "Moo class",
        doc: "Moo class boilerplate.",
    },
    Snippet {
        trigger: "mooseclass",
        label: "mooseclass",
        body: "package ${1:MyClass};\nuse Moose;\n\n${0}\n\n__PACKAGE__->meta->make_immutable;\n1;",
        detail: "Moose class",
        doc: "Moose class boilerplate.",
    },
    Snippet {
        trigger: "hasattr",
        label: "hasattr",
        body: "has '${1:attr}' => (\n    is  => '${2:ro}',\n    isa => '${3:Str}',\n    ${0}\n);",
        detail: "has attribute",
        doc: "Moo/Moose attribute declaration.",
    },
    Snippet {
        trigger: "openfile",
        label: "openfile",
        body: "open my \\$${1:fh}, '<', ${2:\\$file} or die \"Cannot open ${2:\\$file}: $!\";\n$0",
        detail: "open file (read)",
        doc: "Open file for reading.",
    },
    Snippet {
        trigger: "openwrite",
        label: "openwrite",
        body: "open my \\$${1:fh}, '>', ${2:\\$file} or die \"Cannot open ${2:\\$file}: $!\";\n$0",
        detail: "open file (write)",
        doc: "Open file for writing.",
    },
    Snippet {
        trigger: "readline",
        label: "readline",
        body: "while (my \\$line = <${1:\\$fh}>) {\n    chomp \\$line;\n    $0\n}",
        detail: "read lines",
        doc: "Read file line by line.",
    },
    Snippet {
        trigger: "slurp",
        label: "slurp",
        body: "open my \\$${1:fh}, '<', ${2:\\$file} or die \"Cannot open ${2:\\$file}: $!\";\nmy \\$${3:content} = do { local $/; <\\$${1:fh}> };\nclose \\$${1:fh};\n$0",
        detail: "slurp file",
        doc: "Read entire file into scalar.",
    },
    Snippet {
        trigger: "maplist",
        label: "maplist",
        body: "map { ${1:expr} } @${0:list}",
        detail: "map { } @list",
        doc: "Transform list with `map`.",
    },
    Snippet {
        trigger: "greplist",
        label: "greplist",
        body: "grep { ${1:condition} } @${0:list}",
        detail: "grep { } @list",
        doc: "Filter list with `grep`.",
    },
    Snippet {
        trigger: "sortlist",
        label: "sortlist",
        body: "sort { ${1:\\$a cmp \\$b} } @${0:list}",
        detail: "sort { } @list",
        doc: "Sort list with custom comparison.",
    },
    Snippet {
        trigger: "beginblock",
        label: "beginblock",
        body: "BEGIN {\n    $0\n}",
        detail: "BEGIN block",
        doc: "Code executed at compile time.",
    },
    Snippet {
        trigger: "endblock",
        label: "endblock",
        body: "END {\n    $0\n}",
        detail: "END block",
        doc: "Code executed at exit.",
    },
    Snippet {
        trigger: "datablock",
        label: "datablock",
        body: "__DATA__\n$0",
        detail: "__DATA__ section",
        doc: "Inline data section.",
    },
    Snippet {
        trigger: "endblock_file",
        label: "endblock_file",
        body: "__END__\n$0",
        detail: "__END__ section",
        doc: "Marks end of code.",
    },
    Snippet {
        trigger: "testfile",
        label: "testfile",
        body: "use strict;\nuse warnings;\nuse Test::More;\n\n${0}\n\ndone_testing();",
        detail: "test file boilerplate",
        doc: "Basic .t test file skeleton.",
    },
    Snippet {
        trigger: "subtest",
        label: "subtest",
        body: "subtest '${1:description}' => sub {\n    $0\n};",
        detail: "subtest block",
        doc: "Test::More subtest.",
    },
    Snippet {
        trigger: "qwlist",
        label: "qwlist",
        body: "qw(${0})",
        detail: "qw() word list",
        doc: "Quote-words list.",
    },
    Snippet {
        trigger: "modfile",
        label: "modfile",
        body: "package ${1:My::Module};\n\nuse strict;\nuse warnings;\n\n${0}\n\n1;",
        detail: "module file",
        doc: "Standard Perl module skeleton.",
    },
    Snippet {
        trigger: "modexporter",
        label: "modexporter",
        body: "package ${1:My::Module};\n\nuse strict;\nuse warnings;\nuse Exporter 'import';\n\nour @EXPORT_OK = qw(${2:func1 func2});\n\n${0}\n\n1;",
        detail: "module with Exporter",
        doc: "Module skeleton with Exporter.",
    },
    Snippet {
        trigger: "hashslice",
        label: "hashslice",
        body: "@${1:hash}{@${0:keys}}",
        detail: "hash slice",
        doc: "Extract multiple hash values.",
    },
    Snippet {
        trigger: "hashref",
        label: "hashref",
        body: "my \\$${1:href} = {\n    ${2:key} => ${3:value},\n    $0\n};",
        detail: "hash reference",
        doc: "Anonymous hash reference.",
    },
    Snippet {
        trigger: "arrayref",
        label: "arrayref",
        body: "my \\$${1:aref} = [\n    ${0}\n];",
        detail: "array reference",
        doc: "Anonymous array reference.",
    },
    Snippet {
        trigger: "forline",
        label: "forline",
        body: "while (<${1:STDIN}>) {\n    chomp;\n    $0\n}",
        detail: "while (<>) loop",
        doc: "Read and process input line by line.",
    },
    Snippet {
        trigger: "printfh",
        label: "printfh",
        body: "print ${1:\\$fh} ${0:\"data\\n\"};",
        detail: "print to filehandle",
        doc: "Print to a specific filehandle.",
    },
    Snippet {
        trigger: "sayfh",
        label: "sayfh",
        body: "say ${1:\\$fh} ${0:\"data\"};",
        detail: "say to filehandle",
        doc: "Say to a specific filehandle.",
    },
    // ── Regex operators ──────────────────────────────────────────────────────
    Snippet {
        trigger: "mbasic",
        label: "mbasic",
        body: "m/${1:pattern}/${0:g}",
        detail: "match operator",
        doc: "Basic match with optional flags (g=global, i=ignore case, m=multiline).",
    },
    Snippet {
        trigger: "mregex",
        label: "mregex",
        body: "m/${1:pattern}/${0:gi}",
        detail: "match regex",
        doc: "Perl match operator with flags.",
    },
    Snippet {
        trigger: "ssubst",
        label: "ssubst",
        body: "s/${1:pattern}/${2:replacement}/${0:g}",
        detail: "substitution regex",
        doc: "Perl substitution operator.",
    },
    Snippet {
        trigger: "qrpat",
        label: "qrpat",
        body: "qr/${1:pattern}/${0:i}",
        detail: "compiled regex",
        doc: "Compile regex into a qr// object.",
    },
    Snippet {
        trigger: "sbasic",
        label: "sbasic",
        body: "s/${1:pattern}/${2:replacement}/${0:g}",
        detail: "substitution operator",
        doc: "Replace first (or all with /g) occurrences of pattern.",
    },
    Snippet {
        trigger: "qrbasic",
        label: "qrbasic",
        body: "qr/${1:pattern}/${0}",
        detail: "compiled regex",
        doc: "Compile regex for reuse. Returns a regex object.",
    },
    // ── Regex: captures ──────────────────────────────────────────────────────
    Snippet {
        trigger: "namedcap",
        label: "namedcap",
        body: "/(?<${1:name}>${2:pattern})/${0}",
        detail: "named capture group",
        doc: "Named capture group (Perl 5.10+). Access via $+{name}.",
    },
    // ── Regex: lookahead / lookbehind ────────────────────────────────────────
    Snippet {
        trigger: "lookahead",
        label: "lookahead",
        body: "/(?=${1:pattern})${0}/",
        detail: "positive lookahead",
        doc: "Assert that pattern follows without consuming characters.",
    },
    Snippet {
        trigger: "neglookahead",
        label: "neglookahead",
        body: "/(?!${1:pattern})${0}/",
        detail: "negative lookahead",
        doc: "Assert that pattern does NOT follow.",
    },
    Snippet {
        trigger: "lookbehind",
        label: "lookbehind",
        body: "/(?<=${1:pattern})${0}/",
        detail: "positive lookbehind",
        doc: "Assert that pattern precedes without consuming characters.",
    },
    Snippet {
        trigger: "neglookbehind",
        label: "neglookbehind",
        body: "/(?<!${1:pattern})${0}/",
        detail: "negative lookbehind",
        doc: "Assert that pattern does NOT precede.",
    },
    // ── Regex: flags ─────────────────────────────────────────────────────────
    Snippet {
        trigger: "rxglobal",
        label: "rxglobal",
        body: "/${1:pattern}/g${0}",
        detail: "flag: /g (global)",
        doc: "Match all occurrences, not just the first.",
    },
    Snippet {
        trigger: "rxcase",
        label: "rxcase",
        body: "/${1:pattern}/i${0}",
        detail: "flag: /i (ignore case)",
        doc: "Case-insensitive match.",
    },
    Snippet {
        trigger: "rxmulti",
        label: "rxmulti",
        body: "/${1:pattern}/m${0}",
        detail: "flag: /m (multiline)",
        doc: "^ and $ match line boundaries, not just string boundaries.",
    },
    Snippet {
        trigger: "rxdots",
        label: "rxdots",
        body: "/${1:pattern}/s${0}",
        detail: "flag: /s (dotall)",
        doc: "Dot (.) matches newlines.",
    },
    Snippet {
        trigger: "rxverbose",
        label: "rxverbose",
        body: "m/\n    ${1:pattern}  # ${2:comment}\n    ${0}\n/x",
        detail: "flag: /x (verbose)",
        doc: "Allow whitespace and comments in pattern for readability.",
    },
];

/// Add snippet completions to the completion list.
pub fn add_snippet_completions(completions: &mut Vec<CompletionItem>, context: &CompletionContext) {
    for snippet in SNIPPETS {
        if context.prefix.is_empty() || snippet.trigger.starts_with(&context.prefix) {
            completions.push(CompletionItem {
                label: snippet.label.to_string(),
                kind: super::items::CompletionItemKind::Snippet,
                detail: Some(snippet.detail.to_string()),
                documentation: Some(snippet.doc.to_string()),
                insert_text: Some(snippet.body.to_string()),
                sort_text: Some(format!("3_{}", snippet.trigger)),
                filter_text: Some(snippet.trigger.to_string()),
                additional_edits: vec![],
                text_edit_range: Some((context.prefix_start, context.position)),
                commit_characters: None,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::context::CompletionContext;
    use super::*;
    use perl_semantic_analyzer::symbol::SymbolTable;

    fn make_context(prefix: &str) -> CompletionContext {
        let st = SymbolTable::default();
        CompletionContext::new(&st, prefix.len(), None, false, false, false, prefix.to_string(), 0)
    }

    #[test]
    fn snippets_returned_with_empty_prefix() {
        let ctx = make_context("");
        let mut items = Vec::new();
        add_snippet_completions(&mut items, &ctx);
        assert!(items.len() >= 50, "expected >=50, got {}", items.len());
    }

    #[test]
    fn snippet_filter_by_prefix() {
        let ctx = make_context("eval");
        let mut items = Vec::new();
        add_snippet_completions(&mut items, &ctx);
        assert!(items.iter().any(|i| i.label == "evaldo"));
        assert!(!items.iter().any(|i| i.label == "openfile"));
    }

    #[test]
    fn all_snippets_have_kind_snippet() {
        let ctx = make_context("");
        let mut items = Vec::new();
        add_snippet_completions(&mut items, &ctx);
        for item in &items {
            assert_eq!(
                item.kind,
                super::super::items::CompletionItemKind::Snippet,
                "{}",
                item.label
            );
        }
    }

    #[test]
    fn all_snippets_have_docs_and_tab_stop() {
        let ctx = make_context("");
        let mut items = Vec::new();
        add_snippet_completions(&mut items, &ctx);
        for item in &items {
            assert!(item.documentation.is_some(), "{}", item.label);
            let t = item.insert_text.as_deref();
            assert!(t.is_some_and(|t| t.contains("$0") || t.contains("${0")), "{}", item.label);
        }
    }

    #[test]
    fn no_duplicate_triggers() {
        let mut triggers: Vec<&str> = SNIPPETS.iter().map(|s| s.trigger).collect();
        triggers.sort();
        let before = triggers.len();
        triggers.dedup();
        assert_eq!(before, triggers.len(), "duplicate triggers");
    }

    #[test]
    fn regex_operator_snippets_exist() {
        let ctx = make_context("");
        let mut items = Vec::new();
        add_snippet_completions(&mut items, &ctx);
        let triggers: Vec<&str> = items.iter().filter_map(|i| i.filter_text.as_deref()).collect();
        // Basic operators
        assert!(triggers.contains(&"mbasic"), "mbasic snippet missing");
        assert!(triggers.contains(&"sbasic"), "sbasic snippet missing");
        assert!(triggers.contains(&"qrbasic"), "qrbasic snippet missing");
        // Named captures
        assert!(triggers.contains(&"namedcap"), "namedcap snippet missing");
        // Lookahead / lookbehind
        assert!(triggers.contains(&"lookahead"), "lookahead snippet missing");
        assert!(triggers.contains(&"neglookahead"), "neglookahead snippet missing");
        assert!(triggers.contains(&"lookbehind"), "lookbehind snippet missing");
        assert!(triggers.contains(&"neglookbehind"), "neglookbehind snippet missing");
        // Flag variants
        assert!(triggers.contains(&"rxglobal"), "rxglobal snippet missing");
        assert!(triggers.contains(&"rxcase"), "rxcase snippet missing");
        assert!(triggers.contains(&"rxmulti"), "rxmulti snippet missing");
        assert!(triggers.contains(&"rxdots"), "rxdots snippet missing");
        assert!(triggers.contains(&"rxverbose"), "rxverbose snippet missing");
    }

    #[test]
    fn regex_snippet_filter_by_prefix() {
        let ctx = make_context("mba");
        let mut items = Vec::new();
        add_snippet_completions(&mut items, &ctx);
        assert!(items.iter().any(|i| i.label == "mbasic"), "mbasic not returned for prefix 'mba'");
        assert!(!items.iter().any(|i| i.label == "sbasic"), "sbasic should not match 'mba'");
    }

    #[test]
    fn regex_snippet_bodies_are_correct() {
        // Verify the actual regex syntax inside snippet bodies is spelled correctly.
        // Existence tests confirm triggers fire; this test confirms the body text
        // delivered to the LSP client contains valid Perl regex constructs.
        let body = |trigger: &str| -> String {
            SNIPPETS
                .iter()
                .find(|s| s.trigger == trigger)
                .map(|s| s.body.to_string())
                .unwrap_or_else(|| format!("SNIPPET_MISSING:{trigger}"))
        };
        assert!(body("namedcap").contains("(?<"), "namedcap must contain named-capture opener (?<");
        assert!(body("lookahead").contains("(?="), "lookahead must contain positive lookahead (?=");
        assert!(
            body("neglookahead").contains("(?!"),
            "neglookahead must contain negative lookahead (?!"
        );
        assert!(
            body("lookbehind").contains("(?<="),
            "lookbehind must contain positive lookbehind (?<="
        );
        assert!(
            body("neglookbehind").contains("(?<!"),
            "neglookbehind must contain negative lookbehind (?<!"
        );
        assert!(body("rxglobal").contains("/g"), "rxglobal must embed /g flag");
        assert!(body("rxcase").contains("/i"), "rxcase must embed /i flag");
        assert!(body("rxmulti").contains("/m"), "rxmulti must embed /m flag");
        assert!(body("rxdots").contains("/s"), "rxdots must embed /s flag");
        assert!(body("rxverbose").contains("/x"), "rxverbose must embed /x flag");
        assert!(body("mbasic").starts_with("m/"), "mbasic body must start with m/");
        assert!(body("sbasic").starts_with("s/"), "sbasic body must start with s/");
        assert!(body("qrbasic").starts_with("qr/"), "qrbasic body must start with qr/");
    }
}
