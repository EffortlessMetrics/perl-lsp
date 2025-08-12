//! Test method signatures
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Basic signatures
        "sub add ($x, $y) { return $x + $y }",
        "sub greet ($name) { say \"Hello, $name\" }",
        // Optional parameters
        "sub log ($msg, $level = 'info') { print \"[$level] $msg\" }",
        // Slurpy parameters
        "sub join_all ($sep, @items) { join $sep, @items }",
        "sub make_hash (@pairs) { return { @pairs } }",
        // Named parameters (hash slurpy)
        "sub config (%opts) { process(%opts) }",
        // Mixed parameters
        "sub complex ($required, $optional = 10, @rest) { }",
        // Type constraints (Perl 5.36+)
        "sub typed (Str $name, Int $age) { }",
        // Anonymous subs with signatures
        "my $add = sub ($x, $y) { $x + $y }",
        // Method signatures
        "method new ($class, %args) { bless \\%args, $class }",
        "method set_name ($self, $name) { $self->{name} = $name }",
        // Empty signature
        "sub no_args () { return 42 }",
        // Signature with attributes
        "sub lvalue_sub ($x) :lvalue { $x }",
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
