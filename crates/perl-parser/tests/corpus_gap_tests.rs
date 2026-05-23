// Test harness for corpus gap coverage
// These tests ensure the parser handles real-world Perl features missing from original corpus

use perl_parser::Parser;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod corpus_gap_tests {
    use super::*;

    fn resolve_corpus_path(filename: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let candidate = Path::new("test_corpus").join(filename);
        if candidate.exists() {
            return Ok(candidate);
        }

        let fallback = Path::new("../../test_corpus").join(filename);
        if fallback.exists() {
            return Ok(fallback);
        }

        Err(format!(
            "Unable to locate corpus fixture '{filename}' in test_corpus/ or ../../test_corpus/"
        )
        .into())
    }

    fn resolve_corpus_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let candidate = Path::new("test_corpus");
        if candidate.exists() {
            return Ok(candidate.to_path_buf());
        }

        let fallback = Path::new("../../test_corpus");
        if fallback.exists() {
            return Ok(fallback.to_path_buf());
        }

        Err("Unable to locate test_corpus/ directory".into())
    }

    fn discover_corpus_files() -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let corpus_dir = resolve_corpus_dir()?;
        let mut files = Vec::new();
        collect_pl_files(&corpus_dir, &mut files);
        files.sort();
        Ok(files)
    }

    fn collect_pl_files(dir: &Path, files: &mut Vec<PathBuf>) {
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !name.starts_with('.') && !name.starts_with('_') {
                    collect_pl_files(&path, files);
                }
            } else if path.extension().and_then(|e| e.to_str()) == Some("pl") {
                files.push(path);
            }
        }
    }

    fn read_corpus_file(filename: &str) -> Result<String, Box<dyn std::error::Error>> {
        let path = resolve_corpus_path(filename)?;
        Ok(fs::read_to_string(path)?)
    }

    // Helper to test a corpus file doesn't crash the parser
    fn test_corpus_file(filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = read_corpus_file(filename)?;

        let mut parser = Parser::new(&content);
        let result = parser.parse();

        assert!(result.is_ok(), "Failed to parse {}: {:?}", filename, result.err());
        Ok(())
    }

    #[test]
    fn test_source_filters() -> Result<(), Box<dyn std::error::Error>> {
        test_corpus_file("source_filters.pl")
    }

    #[test]
    fn test_xs_inline_ffi() -> Result<(), Box<dyn std::error::Error>> {
        test_corpus_file("xs_inline_ffi.pl")
    }

    #[test]
    fn test_modern_perl_features() -> Result<(), Box<dyn std::error::Error>> {
        test_corpus_file("modern_perl_features.pl")
    }

    #[test]
    fn test_advanced_regex() -> Result<(), Box<dyn std::error::Error>> {
        test_corpus_file("advanced_regex.pl")
    }

    #[test]
    fn test_data_end_sections() -> Result<(), Box<dyn std::error::Error>> {
        test_corpus_file("data_end_sections.pl")
    }

    #[test]
    fn test_end_section() -> Result<(), Box<dyn std::error::Error>> {
        test_corpus_file("end_section.pl")
    }

    #[test]
    fn test_packages_versions() -> Result<(), Box<dyn std::error::Error>> {
        test_corpus_file("packages_versions.pl")
    }

    #[test]
    fn test_legacy_syntax() -> Result<(), Box<dyn std::error::Error>> {
        test_corpus_file("legacy_syntax.pl")
    }

    #[test]
    fn test_continue_redo_statements() -> Result<(), Box<dyn std::error::Error>> {
        test_corpus_file("continue_redo_statements.pl")
    }

    #[test]
    fn test_format_statements() -> Result<(), Box<dyn std::error::Error>> {
        test_corpus_file("format_statements.pl")
    }

    #[test]
    fn test_glob_expressions() -> Result<(), Box<dyn std::error::Error>> {
        test_corpus_file("glob_expressions.pl")
    }

    #[test]
    fn test_tie_interface() -> Result<(), Box<dyn std::error::Error>> {
        test_corpus_file("tie_interface.pl")
    }

    #[test]
    fn test_regex_timeout_hardening() -> Result<(), Box<dyn std::error::Error>> {
        test_corpus_file("regex_timeout_hardening.pl")
    }

    #[test]
    fn test_parser_stress_cases() -> Result<(), Box<dyn std::error::Error>> {
        test_corpus_file("parser_stress_cases.pl")
    }

    /// Regression: anonymous sub as expression initializer (`my $c = sub { 1 };`)
    /// must produce a subroutine node inside the initializer (locks down peek_second() fix).
    #[test]
    fn test_anonymous_sub_expression() -> Result<(), Box<dyn std::error::Error>> {
        let input = "my $c = sub { 1 };";
        let mut parser = Parser::new(input);
        let ast = parser.parse()?;

        let sexp = ast.to_sexp();
        // The variable declaration's initializer should contain a subroutine node
        assert!(
            sexp.contains("subroutine") || sexp.contains("anonymous_sub") || sexp.contains("sub"),
            "expected subroutine/anonymous_sub/sub node in initializer, got: {sexp}"
        );
        Ok(())
    }

    /// Regression: `local($ENV{PATH})` inside call args should parse as a
    /// local declaration argument (not an ERROR-producing list declaration).
    #[test]
    fn test_local_parenthesized_lvalue_in_call_args() -> Result<(), Box<dyn std::error::Error>> {
        let input = "foo(local($ENV{PATH}) = '/tmp/bin', $next);";
        let mut parser = Parser::new(input);
        let ast = parser.parse()?;

        let sexp = ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "expected no ERROR nodes for local(...) call arg, got: {sexp}"
        );
        assert!(sexp.contains("local"), "expected local declaration in AST, got: {sexp}");
        assert!(sexp.contains("PATH"), "expected PATH key in AST, got: {sexp}");
        Ok(())
    }

    /// Coverage: state variables and signatures in the same snippet should
    /// parse without introducing ERROR nodes.
    #[test]
    fn test_state_and_signature_combo() -> Result<(), Box<dyn std::error::Error>> {
        let input = r#"
            use feature 'signatures';
            no warnings 'experimental::signatures';
            sub next_id ($prefix, $seed = 0) {
                state $counter = 0;
                return $prefix . ++$counter . $seed;
            }
        "#;
        let mut parser = Parser::new(input);
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();

        assert!(
            !sexp.contains("ERROR"),
            "expected no ERROR nodes for signatures/state snippet, got: {sexp}"
        );
        assert!(
            sexp.contains("state"),
            "expected state declaration to be represented, got: {sexp}"
        );
        Ok(())
    }

    /// Coverage: parses modern control-flow expression forms (postderef + defined-or).
    #[test]
    fn test_postderef_defined_or_expression() -> Result<(), Box<dyn std::error::Error>> {
        let input = "my $first = $obj->items->@*->[0] // 'fallback';";
        let mut parser = Parser::new(input);
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();

        assert!(
            !sexp.contains("ERROR"),
            "expected no ERROR nodes for postderef defined-or expression, got: {sexp}"
        );
        assert!(sexp.contains("fallback"), "expected literal fallback value, got: {sexp}");
        Ok(())
    }

    /// Gap coverage: postfix dereference chained with method invocation and hash
    /// key access should stay parseable without recovery ERROR nodes.
    #[test]
    fn test_postderef_method_hash_chain() -> Result<(), Box<dyn std::error::Error>> {
        let input = "my $name = $obj->records->@[0]->{meta}->{name} // 'unknown';";
        let mut parser = Parser::new(input);
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();

        assert!(
            !sexp.contains("ERROR"),
            "expected no ERROR nodes for postderef hash chain, got: {sexp}"
        );
        assert!(sexp.contains("unknown"), "expected fallback literal in AST, got: {sexp}");
        Ok(())
    }

    /// Gap coverage: `do { ... }` assignment expressions should parse as expression
    /// values without introducing recovery ERROR nodes.
    #[test]
    fn test_do_block_expression_assignment() -> Result<(), Box<dyn std::error::Error>> {
        let input = r#"
            my $value = do {
                my $tmp = 40;
                $tmp + 2;
            };
        "#;
        let mut parser = Parser::new(input);
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();

        assert!(
            !sexp.contains("ERROR"),
            "expected no ERROR nodes for do-block expression assignment, got: {sexp}"
        );
        assert!(sexp.contains("do"), "expected do-block in AST, got: {sexp}");
        Ok(())
    }

    /// Gap coverage: signatures with invocant syntax and defaults were under-specified
    /// in direct parser tests; ensure they parse as a single subroutine declaration.
    #[test]
    fn test_signature_with_invocant_defaults() -> Result<(), Box<dyn std::error::Error>> {
        let input = r#"
            use feature 'signatures';
            no warnings 'experimental::signatures';
            sub render ($self: $template = 'main', %opts) {
                return $template;
            }
        "#;
        let mut parser = Parser::new(input);
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();

        assert!(
            !sexp.contains("ERROR"),
            "expected no ERROR nodes for invocant signature, got: {sexp}"
        );
        assert!(sexp.contains("render"), "expected subroutine name in AST, got: {sexp}");
        Ok(())
    }

    /// Gap coverage: `given/when/default` with regex and smartmatch-like forms should
    /// remain parseable without dropping the default arm.
    #[test]
    fn test_given_when_regex_and_default() -> Result<(), Box<dyn std::error::Error>> {
        let input = r#"
            use feature 'switch';
            given ($line) {
                when (/^\s*#/ ) { next; }
                when ($_ ~~ [qw(INFO WARN ALERT)]) { say $_; }
                default { say 'unknown'; }
            }
        "#;
        let mut parser = Parser::new(input);
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();

        assert!(
            !sexp.contains("ERROR"),
            "expected no ERROR nodes for given/when/default snippet, got: {sexp}"
        );
        assert!(sexp.contains("default"), "expected default branch in AST, got: {sexp}");
        Ok(())
    }

    /// Regression: hash slices accept `qw` word lists with delimiter styles other
    /// than parentheses, so adjacent `{qw{...}}` braces must not confuse slice
    /// parsing or quote-word tokenization.
    #[test]
    fn test_hash_slice_qw_alternate_delimiters() -> Result<(), Box<dyn std::error::Error>> {
        for input in [
            "my %opts = (foo => 1, bar => 2); my @pick = @opts{qw{red blue}};",
            "my %opts = (foo => 1, bar => 2); my @pick = @opts{qw/red blue/};",
        ] {
            let mut parser = Parser::new(input);
            let ast = parser.parse()?;
            let sexp = ast.to_sexp();

            assert!(
                !sexp.contains("ERROR"),
                "expected no ERROR nodes for hash-slice qw delimiter form, got: {sexp}"
            );
            assert!(sexp.contains("\"red\""), "expected red key in qw list, got: {sexp}");
            assert!(sexp.contains("\"blue\""), "expected blue key in qw list, got: {sexp}");
        }

        Ok(())
    }

    /// Gap coverage: postfix dereference on hashrefs with an immediately chained
    /// slice should not produce recovery nodes when followed by `grep`.
    #[test]
    fn test_postderef_hash_slice_in_grep() -> Result<(), Box<dyn std::error::Error>> {
        let input = r#"
            my @enabled = grep { $_->{enabled} } $cfg->services->@*;
            my @owners = map { $_ } $cfg->meta->%*{qw(owner backup)};
        "#;
        let mut parser = Parser::new(input);
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();

        assert!(
            !sexp.contains("ERROR"),
            "expected no ERROR nodes for postfix deref in grep/map, got: {sexp}"
        );
        assert!(sexp.contains("enabled"), "expected enabled key in AST, got: {sexp}");
        assert!(sexp.contains("\"owner\""), "expected owner key in AST, got: {sexp}");
        Ok(())
    }

    /// Regression: chained substitutions with escaped delimiters and modifiers
    /// should remain parseable as expression statements in sequence.
    #[test]
    fn test_chained_substitutions_with_modifiers() -> Result<(), Box<dyn std::error::Error>> {
        let input = r#"
            $line =~ s/\s+$//r;
            $line =~ s{\\}{/}gr;
        "#;
        let mut parser = Parser::new(input);
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();

        assert!(
            !sexp.contains("ERROR"),
            "expected no ERROR nodes for chained substitutions, got: {sexp}"
        );
        assert!(sexp.contains("line"), "expected line variable in AST, got: {sexp}");
        Ok(())
    }

    /// Gap coverage: indirect-object style builtins (`open FH, ...`) paired with
    /// a `continue` block historically trigger recovery in some parser modes.
    #[test]
    fn test_indirect_open_with_continue_block() -> Result<(), Box<dyn std::error::Error>> {
        let input = r#"
            while (my $line = <STDIN>) {
                open FH, '<', $line or next;
                my $v = <FH>;
            } continue {
                close FH;
            }
        "#;
        let mut parser = Parser::new(input);
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();

        assert!(
            !sexp.contains("ERROR"),
            "expected no ERROR nodes for indirect-open continue snippet, got: {sexp}"
        );
        assert!(sexp.contains("continue"), "expected continue block in AST, got: {sexp}");
        Ok(())
    }

    /// Gap coverage: package blocks with lexical state and method signatures should
    /// parse as nested declarations without error recovery.
    #[test]
    fn test_package_block_with_method_signature() -> Result<(), Box<dyn std::error::Error>> {
        let input = r#"
            use v5.36;
            package App::Worker {
                use feature 'signatures';
                no warnings 'experimental::signatures';

                sub run ($self, $job = 'default') {
                    state $count = 0;
                    return ++$count . q{:} . $job;
                }
            }
        "#;
        let mut parser = Parser::new(input);
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();

        assert!(
            !sexp.contains("ERROR"),
            "expected no ERROR nodes for package block method signature snippet, got: {sexp}"
        );
        assert!(sexp.contains("App::Worker"), "expected package name in AST, got: {sexp}");
        assert!(sexp.contains("run"), "expected method name in AST, got: {sexp}");
        Ok(())
    }

    // Property-based test for delimiters
    #[test]
    fn test_arbitrary_delimiters() {
        let delimiters = vec![
            ('!', '!'),
            ('{', '}'),
            ('[', ']'),
            ('(', ')'),
            ('<', '>'),
            ('|', '|'),
            ('#', '#'),
            ('/', '/'),
            ('@', '@'),
        ];

        for (open, close) in delimiters {
            let code = format!("m{open}pattern{close}", open = open, close = close);
            let mut parser = Parser::new(&code);
            let result = parser.parse();
            assert!(result.is_ok(), "Failed to parse m{}{}", open, close);
        }
    }

    /// Auto-discovery test: parse every .pl file in the corpus directory.
    /// This ensures 100% corpus coverage without needing to list files individually.
    #[test]
    fn test_all_corpus_files() -> Result<(), Box<dyn std::error::Error>> {
        let files = discover_corpus_files()?;
        assert!(!files.is_empty(), "No .pl files found in test_corpus/");

        let mut failures = Vec::new();

        for path in &files {
            let content = fs::read_to_string(path)?;
            let mut parser = Parser::new(&content);

            if let Err(e) = parser.parse() {
                failures.push(format!("{}: {e}", path.display()));
            }
        }

        assert!(
            failures.is_empty(),
            "Failed to parse {} of {} corpus files:\n  {}",
            failures.len(),
            files.len(),
            failures.join("\n  ")
        );

        println!("Parsed all {} corpus files successfully", files.len());
        Ok(())
    }

    // Parse all corpus files repeatedly to keep a lightweight performance guard in default CI.
    #[test]
    fn bench_corpus_files() -> Result<(), Box<dyn std::error::Error>> {
        use std::time::Instant;

        let files = discover_corpus_files()?;
        assert!(!files.is_empty(), "No .pl files found in test_corpus/");

        const ITERATIONS: u32 = 3;
        const MAX_PER_PARSE_MS: u128 = 500;

        for path in &files {
            let content = fs::read_to_string(path)?;
            let display = path.display().to_string();

            let start = Instant::now();
            for _ in 0..ITERATIONS {
                let mut parser = Parser::new(&content);
                let parse_result = parser.parse();
                assert!(
                    parse_result.is_ok(),
                    "Failed to parse file {}: {:?}",
                    display,
                    parse_result.err()
                );
            }
            let duration = start.elapsed();
            let per_parse_ms = duration.as_millis() / u128::from(ITERATIONS);

            println!("{}: {}ms per parse", display, per_parse_ms);
            assert!(
                per_parse_ms <= MAX_PER_PARSE_MS,
                "Corpus parse regression for {}: {}ms per parse exceeds {}ms budget",
                display,
                per_parse_ms,
                MAX_PER_PARSE_MS
            );
        }

        Ok(())
    }
}
