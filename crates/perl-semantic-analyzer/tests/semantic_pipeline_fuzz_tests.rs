//! Fuzz-style stress tests for the parse → semantic-analysis pipeline.
//!
//! This suite targets the highest-impact path (editor open/change events):
//! parse with recovery, symbol extraction, scope analysis, and semantic token generation.

use perl_semantic_analyzer::analysis::scope_analyzer::ScopeAnalyzer;
use perl_semantic_analyzer::analysis::semantic::SemanticAnalyzer;
use perl_semantic_analyzer::symbol::SymbolExtractor;
use perl_semantic_analyzer::Parser;

#[derive(Debug, Clone)]
struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.state = value;
        value
    }

    fn next_usize(&mut self, upper_bound: usize) -> usize {
        if upper_bound == 0 {
            return 0;
        }
        (self.next_u64() as usize) % upper_bound
    }

    fn next_ascii_char(&mut self) -> char {
        const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789$@%_{}[]();,:#'\"/\\\n\t ";
        ALPHABET[self.next_usize(ALPHABET.len())] as char
    }
}

fn random_noise(rng: &mut XorShift64, max_len: usize) -> String {
    let len = rng.next_usize(max_len + 1);
    let mut out = String::with_capacity(len);
    for _ in 0..len {
        out.push(rng.next_ascii_char());
    }
    out
}

fn random_snippet(rng: &mut XorShift64) -> String {
    const FRAGMENTS: &[&str] = &[
        "package App::Service;",
        "use strict;",
        "use warnings;",
        "sub run { my ($x) = @_; return $x }",
        "my $value = shift;",
        "our $VERSION = '1.0';",
        "my @items = qw/a b c/;",
        "my %h = (a => 1, b => 2);",
        "if ($x) { print $x; } else { warn $x; }",
        "for my $i (0..3) { $i += 1; }",
        "eval { die 'boom' if rand() > 2; };",
        "sub weird { my ($a, $b) = @_; $a ? $b : $a }",
        "method size :lvalue ($self) { $self->{size} }",
        "state $counter = 0;",
        "given ($x) { when (1) { 1 } default { 0 } }",
        "use constant PI => 3.14;",
        "=pod\nTest pod\n=cut",
        "s{foo}{bar}g; tr/a-z/A-Z/;",
        "my $unicode = \"λ\";",
        "${\"dynamic\"};",
    ];

    let mut snippet = String::new();
    let fragment_count = 1 + rng.next_usize(8);

    for _ in 0..fragment_count {
        snippet.push_str(FRAGMENTS[rng.next_usize(FRAGMENTS.len())]);
        snippet.push('\n');

        if rng.next_usize(4) == 0 {
            snippet.push_str(&random_noise(rng, 40));
            snippet.push('\n');
        }
    }

    if rng.next_usize(3) == 0 {
        snippet.push_str(&random_noise(rng, 200));
    }

    snippet
}

#[test]
fn fuzz_semantic_pipeline_preserves_location_invariants() -> Result<(), Box<dyn std::error::Error>>
{
    let mut rng = XorShift64::new(0x5EED_CAFE_D00D_F00D);

    for _case_idx in 0..256 {
        let code = random_snippet(&mut rng);
        let len = code.len();

        let mut parser = Parser::new(&code);
        let parse_output = parser.parse_with_recovery();

        let symbol_table = SymbolExtractor::new_with_source(&code).extract(&parse_output.ast);
        let semantic = SemanticAnalyzer::analyze_with_source(&parse_output.ast, &code);
        let scope_issues = ScopeAnalyzer::new().analyze(&parse_output.ast, &code, &[]);

        for symbols in symbol_table.symbols.values() {
            for symbol in symbols {
                assert!(symbol.location.start <= symbol.location.end);
                assert!(symbol.location.end <= len);
            }
        }

        for references in symbol_table.references.values() {
            for reference in references {
                assert!(reference.location.start <= reference.location.end);
                assert!(reference.location.end <= len);
            }
        }

        for token in semantic.semantic_tokens() {
            assert!(token.location.start <= token.location.end);
            assert!(token.location.end <= len);
        }

        for issue in scope_issues {
            assert!(issue.range.0 <= issue.range.1);
            assert!(issue.range.1 <= len);
        }
    }

    Ok(())
}
