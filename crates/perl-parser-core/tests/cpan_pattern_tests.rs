//! CPAN Pattern Tests
//!
//! Comprehensive tests for parsing patterns commonly found in popular CPAN
//! modules. These tests verify that real-world Perl idioms parse cleanly
//! without producing Error or Missing nodes.
//!
//! Covers patterns from: Moose/Moo, Try::Tiny, List::Util, DBI,
//! File::Find, and common OO/functional Perl idioms.

use perl_parser_core::{NodeKind, Parser};
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse the given source and return the top-level AST node.
/// Panics (via `must`) if the parser returns Err.
fn parse(source: &str) -> perl_parser_core::Node {
    let mut parser = Parser::new(source);
    must(parser.parse())
}

/// Assert that a parsed AST has no Error / Missing* nodes anywhere in the
/// S-expression representation. This is a conservative "clean parse" check.
fn assert_clean_parse(source: &str) {
    let ast = parse(source);
    let sexp = ast.to_sexp();

    // Check for error sentinels in the sexp output.
    // We look for the node-kind markers, not arbitrary substrings.
    let error_markers = [
        "(error ",
        "(Error ",
        "(missing_expression",
        "(missing_statement",
        "(missing_identifier",
        "(missing_block",
        "MissingExpression",
        "MissingStatement",
        "MissingIdentifier",
        "MissingBlock",
    ];

    for marker in &error_markers {
        assert!(
            !sexp.contains(marker),
            "Clean-parse assertion failed: found '{}' in sexp for source:\n{}\n\nsexp:\n{}",
            marker,
            source,
            sexp,
        );
    }
}

/// Extract top-level statement kinds from a Program node.
fn top_level_kinds(ast: &perl_parser_core::Node) -> Vec<&str> {
    match &ast.kind {
        NodeKind::Program { statements } => statements.iter().map(|s| s.kind.kind_name()).collect(),
        _ => vec![ast.kind.kind_name()],
    }
}

// ===========================================================================
// Moose / Moo patterns
// ===========================================================================

mod moose_moo {
    use super::*;

    #[test]
    fn has_attribute_with_quoted_name() {
        let code = "has 'name' => (is => 'ro', isa => 'Str', default => sub { 'unknown' });";
        assert_clean_parse(code);
    }

    #[test]
    fn has_attribute_with_bare_name() {
        let code = "has name => (is => 'ro', required => 1);";
        assert_clean_parse(code);
    }

    #[test]
    fn has_attribute_rw_with_builder() {
        let code = "has 'cache' => (is => 'rw', lazy => 1, builder => '_build_cache');";
        assert_clean_parse(code);
    }

    #[test]
    fn has_attribute_with_type_coercion() {
        let code = "has 'count' => (is => 'ro', isa => 'Int', coerce => 1, default => 0);";
        assert_clean_parse(code);
    }

    #[test]
    fn has_arrayref_attribute() {
        let code = "has 'items' => (is => 'ro', isa => 'ArrayRef[Str]', default => sub { [] });";
        assert_clean_parse(code);
    }

    #[test]
    fn with_single_role() {
        let code = "with 'Some::Role';";
        assert_clean_parse(code);
    }

    #[test]
    fn with_multiple_roles() {
        let code = "with 'Role::One', 'Role::Two', 'Role::Three';";
        assert_clean_parse(code);
    }

    #[test]
    fn extends_single_class() {
        let code = "extends 'Base::Class';";
        assert_clean_parse(code);
    }

    #[test]
    fn extends_multiple_classes() {
        let code = "extends 'Base::One', 'Base::Two';";
        assert_clean_parse(code);
    }

    #[test]
    fn around_modifier() {
        let code =
            r#"around 'method' => sub { my $orig = shift; my $self = shift; $self->$orig(@_) };"#;
        assert_clean_parse(code);
    }

    #[test]
    fn before_modifier() {
        let code = "before 'save' => sub { my $self = shift; $self->validate };";
        assert_clean_parse(code);
    }

    #[test]
    fn after_modifier() {
        let code = "after 'load' => sub { my $self = shift; $self->_post_load };";
        assert_clean_parse(code);
    }

    #[test]
    fn override_modifier() {
        let code = "override 'render' => sub { my $self = shift; return super() . ' extra' };";
        assert_clean_parse(code);
    }

    #[test]
    fn augment_modifier() {
        let code = "augment 'render' => sub { return ' more stuff' };";
        assert_clean_parse(code);
    }

    #[test]
    fn full_moose_class() {
        let code = r#"
package Animal;
use Moose;

has 'name' => (is => 'ro', isa => 'Str', required => 1);
has 'age'  => (is => 'rw', isa => 'Int', default => 0);

sub speak {
    my $self = shift;
    return "My name is " . $self->name;
}

around 'speak' => sub {
    my $orig = shift;
    my $self = shift;
    return uc($self->$orig(@_));
};

__PACKAGE__->meta->make_immutable;
1;
"#;
        assert_clean_parse(code);
        let ast = parse(code);
        let kinds = top_level_kinds(&ast);
        // Should contain Package, Use, Subroutine, and ExpressionStatements
        // (has/around/make_immutable/1; are all expression statements)
        assert!(kinds.contains(&"Package"), "expected Package node, got: {:?}", kinds);
        assert!(kinds.contains(&"Use"), "expected Use node, got: {:?}", kinds);
        assert!(kinds.contains(&"Subroutine"), "expected Subroutine node, got: {:?}", kinds);
        assert!(
            kinds.contains(&"ExpressionStatement"),
            "expected ExpressionStatement nodes for has/around/1;, got: {:?}",
            kinds
        );
    }

    #[test]
    fn moo_class_with_types() {
        let code = r#"
package Dog;
use Moo;
use Types::Standard qw(Str Int);

has name => (is => 'ro', isa => Str, required => 1);
has age  => (is => 'rw', isa => Int, default => sub { 0 });

sub bark {
    my $self = shift;
    return "Woof! I'm " . $self->name;
}

1;
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn make_immutable_chain() {
        let code = "__PACKAGE__->meta->make_immutable;";
        assert_clean_parse(code);
        let ast = parse(code);
        let kinds = top_level_kinds(&ast);
        assert!(
            kinds.contains(&"ExpressionStatement"),
            "expected ExpressionStatement for chained method call, got: {:?}",
            kinds
        );
    }
}

// ===========================================================================
// Try::Tiny patterns
// ===========================================================================

mod try_tiny {
    use super::*;

    #[test]
    fn try_catch() {
        let code = r#"try { dangerous_op() } catch { warn "caught: $_" };"#;
        assert_clean_parse(code);
    }

    #[test]
    fn try_catch_finally() {
        let code = r#"try { dangerous_op() } catch { warn "caught: $_" } finally { cleanup() };"#;
        assert_clean_parse(code);
    }

    #[test]
    fn try_finally_no_catch() {
        let code = "try { work() } finally { cleanup() };";
        assert_clean_parse(code);
    }

    #[test]
    fn try_catch_assigned_to_variable() {
        let code = r#"
my $result = try {
    might_fail();
} catch {
    warn "caught: $_";
    undef;
};
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn try_catch_finally_multiline() {
        let code = r#"
use Try::Tiny;
my $result = try {
    might_fail();
} catch {
    warn "caught: $_";
    undef;
} finally {
    cleanup();
};
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn nested_try_catch() {
        let code = r#"
try {
    try {
        inner_op();
    } catch {
        warn "inner: $_";
    };
} catch {
    warn "outer: $_";
};
"#;
        assert_clean_parse(code);
    }
}

// ===========================================================================
// List::Util / List::MoreUtils patterns
// ===========================================================================

mod list_util {
    use super::*;

    #[test]
    fn reduce_sum() {
        let code = "my $sum = reduce { $a + $b } 0, @numbers;";
        assert_clean_parse(code);
    }

    #[test]
    fn first_match() {
        let code = "my $first = first { $_->is_valid } @objects;";
        assert_clean_parse(code);
    }

    #[test]
    fn any_check() {
        let code = "my $found = any { $_ eq 'target' } @items;";
        assert_clean_parse(code);
    }

    #[test]
    fn all_check() {
        let code = "my $ok = all { defined $_ } @values;";
        assert_clean_parse(code);
    }

    #[test]
    fn none_check() {
        let code = "my $clean = none { /error/i } @log_lines;";
        assert_clean_parse(code);
    }

    #[test]
    fn max_by() {
        let code = "my $longest = max_by { length $_ } @strings;";
        assert_clean_parse(code);
    }

    #[test]
    fn uniq_values() {
        let code = "my @unique = uniq @items;";
        assert_clean_parse(code);
    }

    #[test]
    fn zip_lists() {
        let code = "my @pairs = zip @keys, @values;";
        assert_clean_parse(code);
    }

    #[test]
    fn use_list_util_qw() {
        let code = "use List::Util qw(reduce first any all none max min sum);";
        assert_clean_parse(code);
        let ast = parse(code);
        if let NodeKind::Program { statements } = &ast.kind {
            assert_eq!(statements.len(), 1);
            if let NodeKind::Use { module, args, .. } = &statements[0].kind {
                assert_eq!(module, "List::Util");
                // qw() imports are stored as a single string arg
                let qw_str = args.join(" ");
                assert!(
                    qw_str.contains("reduce") && qw_str.contains("first"),
                    "expected qw args containing reduce and first, got: {:?}",
                    args
                );
            }
        }
    }
}

// ===========================================================================
// DBI patterns
// ===========================================================================

mod dbi {
    use super::*;

    #[test]
    fn connect() {
        let code =
            r#"my $dbh = DBI->connect("dbi:Pg:dbname=test", "user", "pass", { RaiseError => 1 });"#;
        assert_clean_parse(code);
    }

    #[test]
    fn prepare_execute() {
        let code = r#"
my $sth = $dbh->prepare("SELECT * FROM foo WHERE id = ?");
$sth->execute($id);
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn fetchrow_hashref_loop() {
        let code = r#"while (my $row = $sth->fetchrow_hashref) { process($row) }"#;
        assert_clean_parse(code);
        let ast = parse(code);
        let kinds = top_level_kinds(&ast);
        assert!(kinds.contains(&"While"), "expected While loop, got: {:?}", kinds);
    }

    #[test]
    fn fetchrow_array_loop() {
        let code = "while (my @row = $sth->fetchrow_array) { print $row[0] }";
        assert_clean_parse(code);
    }

    #[test]
    fn selectall_arrayref() {
        let code =
            r#"my $rows = $dbh->selectall_arrayref("SELECT * FROM users", { Slice => {} });"#;
        assert_clean_parse(code);
    }

    #[test]
    fn do_statement() {
        let code = r#"$dbh->do("DELETE FROM sessions WHERE expired < ?", undef, time());"#;
        assert_clean_parse(code);
    }

    #[test]
    fn transaction_pattern() {
        let code = r#"
eval {
    $dbh->begin_work;
    $dbh->do("INSERT INTO log (msg) VALUES (?)", undef, $message);
    $dbh->do("UPDATE counters SET count = count + 1 WHERE name = ?", undef, 'inserts');
    $dbh->commit;
};
if ($@) {
    $dbh->rollback;
    die "Transaction failed: $@";
}
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn full_dbi_workflow() {
        let code = r#"
my $dbh = DBI->connect("dbi:SQLite:dbname=test.db", "", "", { RaiseError => 1 });
my $sth = $dbh->prepare("SELECT id, name FROM users WHERE active = ?");
$sth->execute(1);
while (my $row = $sth->fetchrow_hashref) {
    printf "%d: %s\n", $row->{id}, $row->{name};
}
$sth->finish;
$dbh->disconnect;
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn placeholder_bind_values() {
        let code = r#"
my $sth = $dbh->prepare("INSERT INTO users (name, email, age) VALUES (?, ?, ?)");
$sth->execute($name, $email, $age);
"#;
        assert_clean_parse(code);
    }
}

// ===========================================================================
// File::Find patterns
// ===========================================================================

mod file_find {
    use super::*;

    #[test]
    fn find_with_anonymous_sub() {
        let code = "find(sub { push @files, $File::Find::name if -f }, @dirs);";
        assert_clean_parse(code);
    }

    #[test]
    fn find_with_options_hash() {
        let code = "find({ wanted => sub { 1 }, follow => 1 }, $dir);";
        assert_clean_parse(code);
    }

    #[test]
    fn find_filtering_by_extension() {
        let code = r#"
use File::Find;
my @pm_files;
find(
    sub {
        return unless -f;
        return unless /\.pm$/;
        push @pm_files, $File::Find::name;
    },
    @INC
);
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn find_with_no_chdir() {
        let code = r#"
find({
    wanted   => sub { push @found, $_ if -f && /\.t$/ },
    no_chdir => 1,
}, 't/');
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn find_with_preprocess() {
        let code = r#"
find({
    wanted     => sub { process($_) },
    preprocess => sub { sort @_ },
}, $start_dir);
"#;
        assert_clean_parse(code);
    }
}

// ===========================================================================
// Object-oriented patterns (bless, constructors, AUTOLOAD, DESTROY)
// ===========================================================================

mod oo_patterns {
    use super::*;

    #[test]
    fn basic_constructor() {
        let code = r#"
sub new {
    my ($class, %args) = @_;
    my $self = bless {}, $class;
    $self->{name} = $args{name};
    return $self;
}
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn constructor_with_defaults() {
        let code = r#"
sub new {
    my ($class, %args) = @_;
    my $self = bless {
        name    => $args{name} || 'unknown',
        verbose => $args{verbose} || 0,
        _cache  => {},
    }, $class;
    return $self;
}
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn method_call_chain() {
        let code = "$obj->method1->method2->method3;";
        assert_clean_parse(code);
        let ast = parse(code);
        // Should be a chain of MethodCall nodes
        if let NodeKind::Program { statements } = &ast.kind {
            assert_eq!(statements.len(), 1);
            assert_eq!(statements[0].kind.kind_name(), "ExpressionStatement");
        }
    }

    #[test]
    fn class_method_call() {
        let code = "my $obj = Foo::Bar->new(name => 'test', id => 42);";
        assert_clean_parse(code);
    }

    #[test]
    fn isa_check() {
        let code = "if (ref($obj) && $obj->isa('Foo::Bar')) { $obj->do_thing() }";
        assert_clean_parse(code);
    }

    #[test]
    fn can_check() {
        let code = "if ($obj->can('process')) { $obj->process(@args) }";
        assert_clean_parse(code);
    }

    #[test]
    fn autoload_pattern() {
        let code = r#"
sub AUTOLOAD {
    my $self = shift;
    our $AUTOLOAD;
    my $method = $AUTOLOAD;
    $method =~ s/.*:://;
    return if $method eq 'DESTROY';
}
"#;
        assert_clean_parse(code);
        let ast = parse(code);
        let kinds = top_level_kinds(&ast);
        assert!(kinds.contains(&"Subroutine"), "expected Subroutine for AUTOLOAD");
    }

    #[test]
    fn destroy_method() {
        let code = r#"
sub DESTROY {
    my $self = shift;
    close $self->{fh} if $self->{fh};
}
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn multiple_inheritance() {
        let code = r#"
package Child;
use parent qw(Mother Father);
1;
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn package_block_form() {
        let code = r#"
package My::Module {
    use strict;
    use warnings;
    sub new { bless {}, shift }
    1;
}
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn builder_pattern() {
        let code = r#"
my $query = SQL::Builder->new
    ->select('id', 'name')
    ->from('users')
    ->where('active = ?', 1)
    ->order_by('name')
    ->limit(10);
"#;
        assert_clean_parse(code);
    }
}

// ===========================================================================
// Data structure patterns (HoH, AoH, HoA, AoA)
// ===========================================================================

mod data_structures {
    use super::*;

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
}

// ===========================================================================
// Functional patterns (map, grep, sort, etc.)
// ===========================================================================

mod functional {
    use super::*;

    #[test]
    fn map_method_call() {
        let code = "my @names = map { $_->name } @objects;";
        assert_clean_parse(code);
    }

    #[test]
    fn grep_method_call() {
        let code = "my @active = grep { $_->is_active } @users;";
        assert_clean_parse(code);
    }

    #[test]
    fn map_grep_chain() {
        let code = "my @results = map { $_->name } grep { $_->is_active } @users;";
        assert_clean_parse(code);
    }

    #[test]
    fn sort_with_custom_comparator() {
        let code = r#"my @sorted = sort { $a->{score} <=> $b->{score} || $a->{name} cmp $b->{name} } @players;"#;
        assert_clean_parse(code);
    }

    #[test]
    fn sort_by_key() {
        let code = "my @sorted = sort { lc($a) cmp lc($b) } @words;";
        assert_clean_parse(code);
    }

    #[test]
    fn grep_complex_condition() {
        let code = "my @valid = grep { defined $_ && length($_) > 0 && $_ !~ /^#/ } @lines;";
        assert_clean_parse(code);
    }

    #[test]
    fn map_transform() {
        let code = "my @upper = map { uc $_ } @strings;";
        assert_clean_parse(code);
    }

    #[test]
    fn map_expression_form() {
        let code = "my @doubled = map { $_ * 2 } 1 .. 10;";
        assert_clean_parse(code);
    }

    #[test]
    fn chained_string_ops() {
        let code = r#"my $clean = lc(join('-', split(/\s+/, $input)));"#;
        assert_clean_parse(code);
    }

    #[test]
    fn for_postfix() {
        let code = "print $_ for @items;";
        assert_clean_parse(code);
    }

    #[test]
    fn foreach_with_variable() {
        let code = "foreach my $item (@list) { process($item) }";
        assert_clean_parse(code);
    }
}

// ===========================================================================
// Error handling patterns
// ===========================================================================

mod error_handling {
    use super::*;

    #[test]
    fn eval_die_pattern() {
        let code = "eval { die 'oops' }; warn $@ if $@;";
        assert_clean_parse(code);
    }

    #[test]
    fn eval_with_error_check() {
        let code = r#"
eval {
    require Some::Module;
    Some::Module->import;
};
if ($@) {
    warn "Module not available: $@";
}
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn die_with_reference() {
        let code = r#"die { code => 404, message => "Not found" };"#;
        assert_clean_parse(code);
    }

    #[test]
    fn croak_confess() {
        let code = r#"
use Carp qw(croak confess);
croak "Invalid argument" unless defined $arg;
confess "Deep error: $msg";
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn local_sig_warn() {
        let code = "local $SIG{__WARN__} = sub { };";
        assert_clean_parse(code);
    }

    #[test]
    fn local_sig_die() {
        let code = "local $SIG{__DIE__} = sub { log_error($_[0]) };";
        assert_clean_parse(code);
    }

    #[test]
    fn conditional_require() {
        let code = "eval { require JSON::XS }; my $json = $@ ? JSON::PP->new : JSON::XS->new;";
        assert_clean_parse(code);
    }

    #[test]
    fn string_eval() {
        let code = r#"eval "use $module";"#;
        assert_clean_parse(code);
    }
}

// ===========================================================================
// Regex patterns commonly seen in CPAN code
// ===========================================================================

mod regex_patterns {
    use super::*;

    #[test]
    fn simple_match() {
        let code = r#"if ($line =~ /^#/) { next }"#;
        assert_clean_parse(code);
    }

    #[test]
    fn match_with_captures() {
        let code = r#"
if ($line =~ /^(\d{4})-(\d{2})-(\d{2})\s+(\d{2}:\d{2}:\d{2})\s+(.*)$/) {
    my ($year, $month, $day, $time, $msg) = ($1, $2, $3, $4, $5);
}
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn named_captures() {
        let code = r#"my @matches = ($text =~ m/(?<name>\w+):\s*(?<value>\d+)/g);"#;
        assert_clean_parse(code);
    }

    #[test]
    fn substitution() {
        let code = r#"
$text =~ s/^\s+//;
$text =~ s/\s+$//;
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn global_substitution_with_eval() {
        let code = r#"$text =~ s/\$(\w+)/$vars{$1}/ge;"#;
        assert_clean_parse(code);
    }

    #[test]
    fn transliteration() {
        let code = "($count = $str) =~ tr/aeiou//;";
        assert_clean_parse(code);
    }

    #[test]
    fn regex_in_grep() {
        let code = "my @matches = grep { /pattern/i } @lines;";
        assert_clean_parse(code);
    }

    #[test]
    fn negative_match() {
        let code = "next unless $line !~ /^\\s*$/;";
        assert_clean_parse(code);
    }

    #[test]
    fn split_with_regex() {
        let code = r#"my @fields = split /\s*,\s*/, $line;"#;
        assert_clean_parse(code);
    }
}

// ===========================================================================
// Module / import patterns
// ===========================================================================

mod module_patterns {
    use super::*;

    #[test]
    fn use_strict_warnings() {
        let code = "use strict;\nuse warnings;";
        assert_clean_parse(code);
    }

    #[test]
    fn use_with_qw_import() {
        let code = "use List::Util qw(reduce first uniq);";
        assert_clean_parse(code);
    }

    #[test]
    fn use_with_version() {
        let code = "use v5.36;";
        assert_clean_parse(code);
    }

    #[test]
    fn use_parent() {
        let code = "use parent qw(Base::Class);";
        assert_clean_parse(code);
    }

    #[test]
    fn use_base() {
        let code = "use base 'Exporter';";
        assert_clean_parse(code);
    }

    #[test]
    fn require_module() {
        let code = "require Foo::Bar;";
        assert_clean_parse(code);
    }

    #[test]
    fn do_file() {
        let code = "do 'config.pl';";
        assert_clean_parse(code);
    }

    #[test]
    fn exporter_our() {
        let code = r#"
use Exporter 'import';
our @EXPORT_OK = qw(foo bar baz);
our %EXPORT_TAGS = (all => [qw(foo bar baz)]);
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn begin_block() {
        let code = r#"
BEGIN {
    push @INC, 'lib';
}
"#;
        assert_clean_parse(code);
        let ast = parse(code);
        let kinds = top_level_kinds(&ast);
        assert!(kinds.contains(&"PhaseBlock"), "expected PhaseBlock for BEGIN");
    }

    #[test]
    fn end_block() {
        let code = r#"
END {
    cleanup();
}
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn package_with_version() {
        let code = "package My::Module 1.23;";
        assert_clean_parse(code);
    }
}

// ===========================================================================
// use if pragma patterns
// ===========================================================================

mod use_if_pragma {
    use super::*;

    /// Basic `use if` with string equality condition (Win32 compatibility).
    #[test]
    fn use_if_os_check() {
        let code = r#"use if $^O eq "MSWin32", "Win32";"#;
        assert_clean_parse(code);
    }

    /// `use if` with version comparison and fat arrow.
    #[test]
    fn use_if_version_fat_arrow() {
        let code = r"use if $] < 5.008 => 'IO::Scalar';";
        assert_clean_parse(code);
    }

    /// `use if` with a constant condition.
    #[test]
    fn use_if_constant_condition() {
        let code = "use if DEBUG, 'Data::Dumper';";
        assert_clean_parse(code);
    }

    /// Multiple `use if` statements in the same file.
    #[test]
    fn use_if_multiple_in_file() {
        let code = r#"
package Test;
use strict;
use warnings;
use if $^O eq "MSWin32", "Win32";
use if $^O eq "MSWin32", "Win32::Console";
use Carp;
1;
"#;
        assert_clean_parse(code);
    }

    /// `use if` doesn't interfere with regular `if` statements.
    #[test]
    fn use_if_doesnt_break_if_statements() {
        let code = r#"
use if $^O eq "MSWin32", "Win32";
sub foo {
    if ($x > 0) {
        return 1;
    }
    return 0;
}
"#;
        assert_clean_parse(code);
    }

    /// Regular `use parent` still works (keyword 'parent' is not affected).
    #[test]
    fn use_parent_regression() {
        let code = "use parent qw(Base::Class Other::Base);";
        assert_clean_parse(code);
    }
}

// ===========================================================================
// Statement modifier patterns
// ===========================================================================

mod statement_modifiers {
    use super::*;

    #[test]
    fn if_modifier() {
        let code = "print $x if defined $x;";
        assert_clean_parse(code);
    }

    #[test]
    fn unless_modifier() {
        let code = "die 'not found' unless $file;";
        assert_clean_parse(code);
    }

    #[test]
    fn while_modifier() {
        let code = "print while <STDIN>;";
        assert_clean_parse(code);
    }

    #[test]
    fn until_modifier() {
        let code = "$count++ until $count > 10;";
        assert_clean_parse(code);
    }

    #[test]
    fn for_modifier() {
        let code = "print $_ for @items;";
        assert_clean_parse(code);
    }

    #[test]
    fn foreach_modifier() {
        let code = "push @results, process($_) foreach @inputs;";
        assert_clean_parse(code);
    }

    #[test]
    fn chained_modifier_with_next() {
        let code = "next unless defined $row;";
        assert_clean_parse(code);
    }

    #[test]
    fn return_modifier() {
        let code = "return $cached if exists $cache{$key};";
        assert_clean_parse(code);
    }
}

// ===========================================================================
// Variable declaration edge cases
// ===========================================================================

mod variable_declarations {
    use super::*;

    #[test]
    fn my_scalar() {
        let code = "my $x = 42;";
        assert_clean_parse(code);
    }

    #[test]
    fn my_array() {
        let code = "my @items = (1, 2, 3);";
        assert_clean_parse(code);
    }

    #[test]
    fn my_hash() {
        let code = "my %opts = (verbose => 1, debug => 0, output => 'file.txt');";
        assert_clean_parse(code);
    }

    #[test]
    fn our_variable() {
        let code = "our $VERSION = '1.23';";
        assert_clean_parse(code);
    }

    #[test]
    fn local_variable() {
        let code = "local $/ = undef;";
        assert_clean_parse(code);
    }

    #[test]
    fn state_variable() {
        let code = "state $count = 0;";
        assert_clean_parse(code);
    }

    #[test]
    fn list_assignment() {
        let code = "my ($first, @rest) = @ARGV;";
        assert_clean_parse(code);
    }

    #[test]
    fn multiple_my_in_list() {
        let code = "my ($x, $y, $z) = (1, 2, 3);";
        assert_clean_parse(code);
    }

    #[test]
    fn anonymous_sub_assignment() {
        let code = "my $cb = sub { return $_[0] + 1 };";
        assert_clean_parse(code);
    }

    #[test]
    fn ternary_initializer() {
        let code = "my $val = defined($x) ? $x : 'default';";
        assert_clean_parse(code);
    }

    #[test]
    fn wantarray_pattern() {
        let code = "my @result = wantarray() ? @list : ($list[0]);";
        assert_clean_parse(code);
    }

    #[test]
    fn chomp_with_readline() {
        let code = "chomp(my $line = <STDIN>);";
        assert_clean_parse(code);
    }
}

// ===========================================================================
// Complex real-world multi-statement programs
// ===========================================================================

mod real_world_programs {
    use super::*;

    #[test]
    fn config_file_reader() {
        let code = r#"
sub read_config {
    my ($file) = @_;
    open my $fh, '<', $file or die "Cannot open $file: $!";
    my %config;
    while (my $line = <$fh>) {
        chomp $line;
        next if $line =~ /^\s*#/;
        next if $line =~ /^\s*$/;
        if ($line =~ /^(\w+)\s*=\s*(.*)$/) {
            $config{$1} = $2;
        }
    }
    close $fh;
    return %config;
}
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn csv_processor() {
        let code = r#"
use Text::CSV;
my $csv = Text::CSV->new({ binary => 1, auto_diag => 1 });
open my $fh, '<:encoding(utf8)', 'data.csv' or die "Cannot open: $!";
my @rows;
while (my $row = $csv->getline($fh)) {
    push @rows, { name => $row->[0], value => $row->[1] };
}
close $fh;
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn logger_class() {
        let code = r#"
package My::Logger;
use strict;
use warnings;

my %LEVELS = (debug => 0, info => 1, warn => 2, error => 3);

sub new {
    my ($class, %args) = @_;
    return bless {
        level  => $args{level} || 'info',
        output => $args{output} || \*STDERR,
    }, $class;
}

sub log {
    my ($self, $level, $message) = @_;
    return if $LEVELS{$level} < $LEVELS{$self->{level}};
    my $fh = $self->{output};
    printf $fh "[%s] %s: %s\n", scalar localtime, uc($level), $message;
}

1;
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn cgi_handler() {
        let code = r#"
use CGI;
my $q = CGI->new;

print $q->header('text/html');
print $q->start_html('My Page');

if ($q->param('action') eq 'search') {
    my $term = $q->param('q');
    my @results = search($term);
    print $q->ul($q->li(\@results));
} else {
    print $q->p('Welcome!');
}

print $q->end_html;
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn file_slurp_and_process() {
        let code = r#"
sub slurp {
    my ($filename) = @_;
    local $/;
    open my $fh, '<', $filename or die "Cannot read $filename: $!";
    my $content = <$fh>;
    close $fh;
    return $content;
}

my $text = slurp('input.txt');
my @words = split /\s+/, $text;
my %freq;
$freq{$_}++ for @words;

my @top = (sort { $freq{$b} <=> $freq{$a} } keys %freq)[0..9];
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn test_script_pattern() {
        let code = r#"
use strict;
use warnings;
use Test::More tests => 3;

my $obj = My::Module->new(name => 'test');
ok(defined $obj, 'constructor works');
is($obj->name, 'test', 'name accessor works');
can_ok($obj, 'process');
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn complex_data_munging() {
        let code = r#"
my @raw_data = map { chomp; $_ } <DATA>;
my @records = map {
    my @fields = split /\t/, $_;
    { id => $fields[0], name => $fields[1], score => $fields[2] }
} @raw_data;

my @passing = grep { $_->{score} >= 70 } @records;
my @sorted = sort { $b->{score} <=> $a->{score} } @passing;
my @names = map { $_->{name} } @sorted;
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn socket_server_excerpt() {
        let code = r#"
use IO::Socket::INET;
my $server = IO::Socket::INET->new(
    LocalPort => 8080,
    Proto     => 'tcp',
    Listen    => 5,
    Reuse     => 1,
) or die "Cannot create socket: $!";

while (my $client = $server->accept()) {
    my $request = <$client>;
    print $client "HTTP/1.0 200 OK\r\n\r\nHello\n";
    close $client;
}
"#;
        assert_clean_parse(code);
    }
}

// ===========================================================================
// Miscellaneous CPAN idioms
// ===========================================================================

mod misc_idioms {
    use super::*;

    #[test]
    fn bless_hashref() {
        let code = "my $self = bless {}, $class;";
        assert_clean_parse(code);
    }

    #[test]
    fn scalar_context_force() {
        let code = "my $count = scalar @array;";
        assert_clean_parse(code);
    }

    #[test]
    fn ref_check() {
        let code = "my $type = ref($thing) || 'not a reference';";
        assert_clean_parse(code);
    }

    #[test]
    fn defined_or_operator() {
        let code = "my $val = $input // 'default';";
        assert_clean_parse(code);
    }

    #[test]
    fn chained_defined_or() {
        let code = "my $val = $first // $second // $third // 'fallback';";
        assert_clean_parse(code);
    }

    #[test]
    fn string_repetition() {
        let code = "my $line = '-' x 80;";
        assert_clean_parse(code);
    }

    #[test]
    fn heredoc_in_function_call() {
        let code = r#"print <<END;
Hello, $name!
Welcome to $place.
END
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn qw_list() {
        let code = "my @days = qw(Mon Tue Wed Thu Fri Sat Sun);";
        assert_clean_parse(code);
    }

    #[test]
    fn complex_deref_chain() {
        let code = "$config->{database}{hosts}[0]{port}";
        assert_clean_parse(code);
    }

    #[test]
    fn exists_delete() {
        let code = r#"
if (exists $cache{$key}) {
    my $val = delete $cache{$key};
}
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn delete_arrow_hash_deref() {
        assert_clean_parse("delete $self->{key};");
    }

    #[test]
    fn delete_arrow_array_deref() {
        assert_clean_parse("delete $ref->[0];");
    }

    #[test]
    fn delete_chained_subscripts() {
        assert_clean_parse("delete $self->{a}{b};");
    }

    #[test]
    fn delete_chained_arrow_deref() {
        assert_clean_parse("delete $self->{a}->{b};");
    }

    #[test]
    fn exists_arrow_hash_deref() {
        assert_clean_parse("exists $self->{key};");
    }

    #[test]
    fn exists_arrow_array_deref() {
        assert_clean_parse("exists $ref->[0];");
    }

    #[test]
    fn delete_with_statement_modifier() {
        assert_clean_parse("delete $self->{missing} if $self->{present};");
    }

    #[test]
    fn exists_in_if_condition() {
        assert_clean_parse("my $val = delete $cache->{$key} if exists $cache->{$key};");
    }

    #[test]
    fn local_input_record_separator() {
        let code = "local $/ = undef;";
        assert_clean_parse(code);
    }

    #[test]
    fn data_section() {
        let code = r#"
while (<DATA>) {
    chomp;
    print "Line: $_\n";
}
__DATA__
line one
line two
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn multiline_string_concat() {
        let code = r#"
my $sql = "SELECT u.id, u.name, u.email "
        . "FROM users u "
        . "JOIN orders o ON o.user_id = u.id "
        . "WHERE o.total > ? "
        . "ORDER BY u.name";
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn open_three_arg() {
        let code =
            r#"open my $fh, '<:encoding(UTF-8)', $filename or die "Cannot open $filename: $!";"#;
        assert_clean_parse(code);
    }

    #[test]
    fn printf_format() {
        let code = r#"printf "%04d-%02d-%02d %02d:%02d:%02d", $y, $m, $d, $h, $min, $sec;"#;
        assert_clean_parse(code);
    }

    #[test]
    fn complex_sprintf() {
        let code = r#"my $msg = sprintf("Found %d items in %.2f seconds", $count, $elapsed);"#;
        assert_clean_parse(code);
    }

    #[test]
    fn array_slice() {
        let code = "my @first_three = @array[0..2];";
        assert_clean_parse(code);
    }

    #[test]
    fn negative_array_index() {
        let code = "my $last = $array[-1];";
        assert_clean_parse(code);
    }

    #[test]
    fn push_pop_shift_unshift() {
        let code = r#"
push @stack, $item;
my $top = pop @stack;
my $first = shift @queue;
unshift @queue, $new_item;
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn splice_usage() {
        let code = "my @removed = splice(@array, 2, 3, @replacement);";
        assert_clean_parse(code);
    }
}

// ---------------------------------------------------------------------------
// print/say/printf with block-form filehandle: print { $fh } ...
// ---------------------------------------------------------------------------

#[cfg(test)]
mod print_block_filehandle {
    use super::*;

    #[test]
    fn print_block_scalar_fh() {
        let code = r#"print { $fh } "data\n";"#;
        assert_clean_parse(code);
    }

    #[test]
    fn print_block_scalar_fh_is_indirect_call() {
        let code = r#"print { $fh } "data\n";"#;
        let ast = parse(code);
        let sexp = ast.to_sexp();
        assert!(sexp.contains("indirect_call"), "Expected indirect_call, got: {sexp}");
    }

    #[test]
    fn say_block_scalar_fh() {
        let code = r#"say { $fh } "data";"#;
        assert_clean_parse(code);
    }

    #[test]
    fn printf_block_scalar_fh() {
        let code = r#"printf { $fh } "%s\n", $line;"#;
        assert_clean_parse(code);
    }

    #[test]
    fn print_block_typeglob_stderr() {
        let code = r#"print { *STDERR } "error\n";"#;
        assert_clean_parse(code);
    }

    #[test]
    fn print_block_typeglob_stdout() {
        let code = r#"print { *STDOUT } "ok\n";"#;
        assert_clean_parse(code);
    }

    #[test]
    fn print_block_hash_accessor() {
        let code = r#"print { $self->{fh} } "msg\n";"#;
        assert_clean_parse(code);
    }

    #[test]
    fn print_block_method_call() {
        let code = r#"print { $self->fh() } "msg\n";"#;
        assert_clean_parse(code);
    }

    #[test]
    fn print_without_block_still_works() {
        // Regression: plain print without block filehandle must still work
        let code = r#"
print "hello\n";
print STDOUT "message\n";
print STDERR "error\n";
print $fh "data\n";
"#;
        assert_clean_parse(code);
    }

    #[test]
    fn print_block_with_multiple_args() {
        // print { $fh } with comma-separated arguments
        let code = r#"print { $fh } "key=", $value, "\n";"#;
        assert_clean_parse(code);
    }
}

mod compound_stmt_modifier {
    use super::*;

    /// Two consecutive if-blocks must NOT be misread as if-block + postfix modifier.
    #[test]
    fn two_consecutive_if_blocks() {
        let code = r#"
if ($a) { foo(); }
if ($b) { bar(); }
"#;
        assert_clean_parse(code);
    }

    /// while-block followed by a bare if-block.
    #[test]
    fn while_block_then_if_block() {
        let code = r#"
while (1) { last; }
if ($done) { return; }
"#;
        assert_clean_parse(code);
    }

    /// for-block followed by another for-block.
    #[test]
    fn for_block_then_for_block() {
        let code = r#"
for my $i (1..10) { print $i; }
for my $j (1..5) { print $j; }
"#;
        assert_clean_parse(code);
    }

    /// foreach-block followed by an if-block.
    #[test]
    fn foreach_block_then_if_block() {
        let code = r#"
foreach my $item (@list) { process($item); }
if (@list) { done(); }
"#;
        assert_clean_parse(code);
    }

    /// sub definition followed by an if-block.
    #[test]
    fn sub_then_if_block() {
        let code = r#"
sub foo { return 1; }
if ($x) { foo(); }
"#;
        assert_clean_parse(code);
    }

    /// Postfix modifier on a plain expression statement still works.
    #[test]
    fn postfix_if_on_expression() {
        let code = "print $x if $debug;";
        assert_clean_parse(code);
    }

    /// Postfix unless on a plain expression statement still works.
    #[test]
    fn postfix_unless_on_expression() {
        let code = "return if $done;";
        assert_clean_parse(code);
    }

    /// Postfix while on a plain expression statement still works.
    #[test]
    fn postfix_while_on_expression() {
        let code = "do_something() while $running;";
        assert_clean_parse(code);
    }

    /// Common OO pattern: multiple method definitions followed by logic.
    #[test]
    fn multiple_subs_then_if() {
        let code = r#"
sub init { return 1; }
sub run  { return 2; }
if ($start) { init(); run(); }
"#;
        assert_clean_parse(code);
    }
}

mod undef_in_list_assignment {
    use super::*;

    #[test]
    fn undef_middle_of_my_list() {
        let code = "my ($a, undef, $b) = @_;";
        assert_clean_parse(code);
    }

    #[test]
    fn undef_first_in_my_list() {
        let code = "my (undef, $x, $y) = @_;";
        assert_clean_parse(code);
    }

    #[test]
    fn undef_last_in_my_list() {
        let code = "my ($a, $b, undef) = @_;";
        assert_clean_parse(code);
    }

    #[test]
    fn multiple_undef_in_my_list() {
        let code = "my ($a, undef, undef, $b) = @_;";
        assert_clean_parse(code);
    }

    #[test]
    fn undef_in_method_signature() {
        let code = r#"
sub cat_decode {
    my ( $obj, undef, $src, $pos ) = @_;
    return $src;
}
"#;
        assert_clean_parse(code);
    }

    /// Full OO pattern from Encode::CN::HZ — the original failing file.
    #[test]
    fn encode_module_pattern() {
        let code = r#"
sub cat_decode {
    my ( $obj, undef, $src, $pos, $trm, $chk ) = @_;
    my ( $rdst, $rsrc, $rpos ) = \@_[ 1 .. 3 ];
    return $rdst;
}
"#;
        assert_clean_parse(code);
    }
}
