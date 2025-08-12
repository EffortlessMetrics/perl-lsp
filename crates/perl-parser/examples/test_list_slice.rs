//! Test list and slice operations
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Array slices
        "@array[0..5]",
        "@array[1, 3, 5]",
        "@array[@indices]",
        // Hash slices
        "@hash{'key1', 'key2'}",
        "@hash{@keys}",
        "@hash{qw(foo bar baz)}",
        // List assignment
        "($x, $y) = (1, 2)",
        "($first, @rest) = @array",
        "($key, $value) = each %hash",
        // List context
        "my ($a, $b, $c) = @_",
        "my ($x, $y) = split /,/, $line",
        // Push/pop/shift/unshift
        "push @array, 1, 2, 3",
        "my $last = pop @array",
        "my $first = shift @array",
        "unshift @array, 0",
        // Splice
        "splice @array, 2, 1",
        "splice @array, 1, 2, 'a', 'b'",
        // Map and grep
        "my @squares = map { $_ * $_ } @numbers",
        "my @evens = grep { $_ % 2 == 0 } @numbers",
        // Sort
        "my @sorted = sort @array",
        "my @sorted = sort { $a <=> $b } @numbers",
    ];

    for test in tests {
        println!("\nTesting: {}", test);
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(ast) => {
                println!("✅ Success!");
                println!("   S-expr: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }
}
