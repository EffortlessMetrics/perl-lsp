fn main() {
    let input = r#"my $single = <<'SINGLE';
No interpolation here: $var
SINGLE
my $double = <<"DOUBLE";
Interpolation works: $var
DOUBLE
my $backtick = <<`BACKTICK`;
echo "Command execution"
BACKTICK
print($single, $double, $backtick);"#;

    for (i, line) in input.lines().enumerate() {
        println!("{}: {}", i + 1, line);
    }
}