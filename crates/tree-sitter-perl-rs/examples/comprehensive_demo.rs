//! Comprehensive demonstration of tree-sitter-perl capabilities
use tree_sitter_perl::{
    EnhancedFullParser, pure_rust_parser::AstNode, sexp_formatter::SexpFormatter,
};

fn main() {
    println!("=== Tree-sitter Perl Comprehensive Demo ===\n");

    // Test various Perl constructs
    let test_cases = [
        (
            "Simple subroutine",
            r#"
sub greet {
    my $name = shift;
    return "Hello, $name!";
}
"#,
        ),
        (
            "Unicode identifiers",
            r#"
my $café = "coffee shop";
my $π = 3.14159;
my $Σ = sub { my $sum = 0; $sum += $_ for @_; $sum };
print "π = $π\n";
"#,
        ),
        (
            "Modern Perl features",
            r#"
use feature 'say';
say "Hello, World!";

package Point {
    sub new {
        my ($class, $x, $y) = @_;
        bless { x => $x, y => $y }, $class;
    }
}
"#,
        ),
        (
            "Complex data structures",
            r#"
my %config = (
    database => {
        host => 'localhost',
        port => 5432,
        credentials => {
            user => 'admin',
            pass => 'secret'
        }
    },
    features => [qw(auth logging metrics)],
    version => '1.2.3'
);
"#,
        ),
        (
            "Regular expressions",
            r#"
my $text = "The year is 2024";
if ($text =~ /year is (\d+)/) {
    print "Year: $1\n";
}
$text =~ s/\d+/2025/g;
"#,
        ),
        (
            "Reference operations",
            r#"
my @array = (1, 2, 3);
my $array_ref = \@array;
my $first = $array_ref->[0];

my %hash = (key => 'value');
my $hash_ref = \%hash;
my $value = $hash_ref->{key};
"#,
        ),
    ];

    for (name, code) in test_cases {
        println!("--- {} ---", name);
        println!("Code:");
        println!("{}", code.trim());
        println!();

        let mut parser = EnhancedFullParser::new();
        match parser.parse(code) {
            Ok(ast) => {
                println!("✓ Parsed successfully!");

                // Show AST structure
                println!("\nAST Structure:");
                print_ast(&ast, 0);

                // Show S-expression format
                println!("\nS-expression:");
                let formatter = SexpFormatter::new("");
                let sexp = formatter.format(&ast);
                println!("{}", truncate(&sexp, 200));

                // Show statistics
                let stats = collect_stats(&ast);
                println!("\nStatistics:");
                println!("  Total nodes: {}", stats.total_nodes);
                println!("  Max depth: {}", stats.max_depth);
                println!("  Unique node types: {}", stats.node_types.len());
            }
            Err(e) => {
                println!("✗ Parse error: {}", e);
            }
        }
        println!("\n{}\n", "=".repeat(60));
    }
}

fn print_ast(node: &AstNode, indent: usize) {
    let prefix = "  ".repeat(indent);

    match node {
        AstNode::Program(items) => {
            println!("{}Program ({} items)", prefix, items.len());
            for item in items.iter().take(5) {
                // Limit output
                print_ast(item, indent + 1);
            }
            if items.len() > 5 {
                println!("{}... and {} more items", prefix, items.len() - 5);
            }
        }
        AstNode::Statement(content) => {
            println!("{}Statement", prefix);
            print_ast(content, indent + 1);
        }
        AstNode::SubDeclaration { name, .. } => {
            println!("{}SubDeclaration: {}", prefix, name);
        }
        AstNode::PackageDeclaration { name, .. } => {
            println!("{}Package: {}", prefix, name);
        }
        AstNode::UseStatement { module, .. } => {
            println!("{}Use: {}", prefix, module);
        }
        AstNode::VariableDeclaration { scope, variables, .. } => {
            println!("{}VarDecl: {} ({} vars)", prefix, scope, variables.len());
        }
        AstNode::BinaryOp { left, op, right } => {
            println!("{}BinaryOp: {}", prefix, op);
            print_ast(left, indent + 1);
            print_ast(right, indent + 1);
        }
        AstNode::IfStatement { condition, .. } => {
            println!("{}If", prefix);
            print_ast(condition, indent + 1);
        }
        AstNode::FunctionCall { function, args } => {
            println!("{}FunctionCall ({} args)", prefix, args.len());
            print_ast(function, indent + 1);
        }
        AstNode::HashRef(pairs) => {
            println!("{}HashRef ({} pairs)", prefix, pairs.len());
        }
        AstNode::ArrayRef(elements) => {
            println!("{}ArrayRef ({} elements)", prefix, elements.len());
        }
        AstNode::Identifier(name) => {
            println!("{}Identifier: {}", prefix, name);
        }
        AstNode::String(s) => {
            let display = if s.len() > 20 { format!("{}...", &s[..20]) } else { s.to_string() };
            println!("{}String: \"{}\"", prefix, display);
        }
        AstNode::Number(n) => {
            println!("{}Number: {}", prefix, n);
        }
        _ => {
            println!("{}[Other node type]", prefix);
        }
    }
}

fn collect_stats(node: &AstNode) -> Stats {
    let mut stats = Stats::default();
    collect_stats_recursive(node, 0, &mut stats);
    stats
}

fn collect_stats_recursive(node: &AstNode, depth: usize, stats: &mut Stats) {
    stats.total_nodes += 1;
    stats.max_depth = stats.max_depth.max(depth);

    let type_name = match node {
        AstNode::Program(_) => "Program",
        AstNode::Statement(_) => "Statement",
        AstNode::SubDeclaration { .. } => "SubDeclaration",
        AstNode::VariableDeclaration { .. } => "VariableDeclaration",
        AstNode::BinaryOp { .. } => "BinaryOp",
        AstNode::FunctionCall { .. } => "FunctionCall",
        AstNode::Identifier(_) => "Identifier",
        AstNode::String(_) => "String",
        AstNode::Number(_) => "Number",
        _ => "Other",
    };

    stats.node_types.insert(type_name.to_string());

    // Recurse into children
    match node {
        AstNode::Program(items) => {
            for item in items {
                collect_stats_recursive(item, depth + 1, stats);
            }
        }
        AstNode::Statement(content) => {
            collect_stats_recursive(content, depth + 1, stats);
        }
        AstNode::BinaryOp { left, right, .. } => {
            collect_stats_recursive(left, depth + 1, stats);
            collect_stats_recursive(right, depth + 1, stats);
        }
        AstNode::FunctionCall { function, args } => {
            collect_stats_recursive(function, depth + 1, stats);
            for arg in args {
                collect_stats_recursive(arg, depth + 1, stats);
            }
        }
        _ => {}
    }
}

fn truncate(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len { s } else { &s[..max_len.min(s.len())] }
}

#[derive(Default)]
struct Stats {
    total_nodes: usize,
    max_depth: usize,
    node_types: std::collections::HashSet<String>,
}
