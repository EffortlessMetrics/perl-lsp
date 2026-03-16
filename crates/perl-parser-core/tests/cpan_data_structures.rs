//! CPAN Pattern Tests: Data Structures

mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn hash_of_hashes() {
    let code = r#"
my %people = (
    alice => { age => 30, city => 'NYC' },
    bob   => { age => 25, city => 'LA' },
);
"#;
    assert_clean_parse(code);
}

#[test]
fn array_of_hashes() {
    let code = "my @aoh = ({name => 'Alice', age => 30}, {name => 'Bob', age => 25});";
    assert_clean_parse(code);
}

#[test]
fn hash_of_arrays() {
    let code = "my %hoa = (fruits => ['apple', 'banana'], vegs => ['carrot']);";
    assert_clean_parse(code);
}

#[test]
fn array_of_arrays() {
    let code = "my @aoa = ([1, 2, 3], [4, 5, 6], [7, 8, 9]);";
    assert_clean_parse(code);
}

#[test]
fn nested_hashref_access() {
    let code = "my $val = $hashref->{key}{nested};";
    assert_clean_parse(code);
}

#[test]
fn nested_arrayref_access() {
    let code = "my $val = $arrayref->[0][1];";
    assert_clean_parse(code);
}

#[test]
fn mixed_dereference() {
    let code = "my $val = $data->{users}[0]{name};";
    assert_clean_parse(code);
}

#[test]
fn complex_nested_structure() {
    let code = r#"
my $config = {
    database => {
        host     => 'localhost',
        port     => 5432,
        name     => 'mydb',
        options  => { AutoCommit => 1, RaiseError => 1 },
    },
    logging => {
        level => 'info',
        file  => '/var/log/app.log',
    },
};
"#;
    assert_clean_parse(code);
}

#[test]
fn hash_slice() {
    let code = "my @vals = @hash{qw(foo bar baz)};";
    assert_clean_parse(code);
}

#[test]
fn hash_slice_assignment() {
    let code = "@config{qw(host port user pass)} = ('localhost', 3306, 'root', 'secret');";
    assert_clean_parse(code);
}

#[test]
fn dispatch_table() {
    let code = r#"
my %dispatch = (
    add => sub { $_[0] + $_[1] },
    mul => sub { $_[0] * $_[1] },
    div => sub { $_[0] / $_[1] },
);
$dispatch{$op}->($a, $b);
"#;
    assert_clean_parse(code);
}
