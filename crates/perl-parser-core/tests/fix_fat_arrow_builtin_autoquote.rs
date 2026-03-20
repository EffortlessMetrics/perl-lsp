mod cpan_test_helpers;
use cpan_test_helpers::*;
<<<<<<< HEAD
=======

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn has_log_fat_arrow_sub() {
    assert_clean_parse("has log => sub { 1 };");
}
<<<<<<< HEAD
=======

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn has_log_fat_arrow_string() {
    assert_clean_parse("has log => 'default';");
}
<<<<<<< HEAD
=======

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn has_log_fat_arrow_complex_sub() {
    assert_clean_parse("has log => sub {\n    Mojo::Log->new;\n};");
}
<<<<<<< HEAD
=======

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn has_abs_fat_arrow() {
    assert_clean_parse("has abs => sub { 1 };");
}
<<<<<<< HEAD
=======

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn has_die_fat_arrow() {
    assert_clean_parse("has die => sub { 1 };");
}
<<<<<<< HEAD
=======

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn has_keys_fat_arrow() {
    assert_clean_parse("has keys => sub { 1 };");
}
<<<<<<< HEAD
=======

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn has_join_fat_arrow() {
    assert_clean_parse("has join => sub { ',' };");
}
<<<<<<< HEAD
=======

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn has_print_fat_arrow() {
    assert_clean_parse("has print => sub { 1 };");
}
<<<<<<< HEAD
=======

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn has_open_fat_arrow() {
    assert_clean_parse("has open => sub { 0 };");
}
<<<<<<< HEAD
=======

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn has_close_fat_arrow() {
    assert_clean_parse("has close => sub { 0 };");
}
<<<<<<< HEAD
=======

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn has_read_fat_arrow() {
    assert_clean_parse("has read => sub { 0 };");
}
<<<<<<< HEAD
=======

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn has_length_fat_arrow() {
    assert_clean_parse("has length => 0;");
}
<<<<<<< HEAD
=======

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn has_index_fat_arrow() {
    assert_clean_parse("has index => 0;");
}
<<<<<<< HEAD
=======

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn has_push_fat_arrow() {
    assert_clean_parse("has push => sub { 1 };");
}
<<<<<<< HEAD
=======

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn has_pop_fat_arrow() {
    assert_clean_parse("has pop => sub { 1 };");
}
<<<<<<< HEAD
=======

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn has_shift_fat_arrow() {
    assert_clean_parse("has shift => sub { 1 };");
}
<<<<<<< HEAD
=======

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn has_split_fat_arrow() {
    assert_clean_parse("has split => sub { 1 };");
}
<<<<<<< HEAD
=======

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn has_sort_fat_arrow() {
    assert_clean_parse("has sort => sub { 1 };");
}
<<<<<<< HEAD
#[test]
fn has_log_multiline_sub() {
    assert_clean_parse(r#"has log => sub { my $self = shift; return Mojo::Log->new; };"#);
}
=======

#[test]
fn has_log_multiline_sub() {
    let source = r#"has log => sub {
    my $self = shift;
    return Mojo::Log->new->path($self->mode eq 'development' ? undef : $self->home->child('log'));
};"#;
    assert_clean_parse(source);
}

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn has_acceptors_sub() {
    assert_clean_parse("has acceptors => sub { [] };");
}
<<<<<<< HEAD
=======

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn has_inactivity_timeout() {
    assert_clean_parse(r#"has inactivity_timeout => sub { $ENV{MOJO_INACTIVITY_TIMEOUT} // 30 };"#);
}
<<<<<<< HEAD
=======

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn has_max_requests_value() {
    assert_clean_parse("has max_requests => 100;");
}
<<<<<<< HEAD
=======

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn has_controller_class_string() {
    assert_clean_parse("has controller_class => 'Mojolicious::Controller';");
}
<<<<<<< HEAD
#[test]
fn multiple_has_declarations() {
    assert_clean_parse(
        "has controller_class => 'Mojolicious::Controller';\nhas exception_format => 'html';\nhas max_requests => 100;",
    );
}
=======

#[test]
fn multiple_has_declarations() {
    let source = r#"has commands => sub { Mojolicious::Commands->new(app => shift) };
has controller_class => 'Mojolicious::Controller';
has exception_format => 'html';
has home => sub { Mojo::Home->new->detect(ref shift) };
has log => sub {
    my $self = shift;
    my $log = Mojo::Log->new;
    return $log;
};
has max_requests => 100;"#;
    assert_clean_parse(source);
}

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn before_delete_fat_arrow() {
    assert_clean_parse("before delete => sub { check_value($_[1]); };");
}
<<<<<<< HEAD
#[test]
fn around_delete_all_fat_arrow() {
    assert_clean_parse("around delete_all => sub { my ($orig, $self) = @_; $self->$orig(); };");
}
=======

#[test]
fn around_delete_all_fat_arrow() {
    assert_clean_parse(
        r#"around delete_all => sub {
    my ($orig, $self) = @_;
    $self->$orig();
};"#,
    );
}

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn hash_with_builtin_keys() {
    assert_clean_parse(r#"my %h = (log => 1, abs => 2, die => 3);"#);
}
<<<<<<< HEAD
=======

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn hashref_with_builtin_keys() {
    assert_clean_parse(r#"my $h = { log => 'file.log', die => sub { exit 1 } };"#);
}
<<<<<<< HEAD
=======

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn log_as_function() {
    assert_clean_parse("my $x = log(42);");
}
<<<<<<< HEAD
=======

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn abs_as_function() {
    assert_clean_parse("my $x = abs(-5);");
}
<<<<<<< HEAD
=======

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn join_as_function() {
    assert_clean_parse(r#"my $s = join(",", @list);"#);
}
<<<<<<< HEAD
=======

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn die_as_statement() {
    assert_clean_parse(r#"die "error";"#);
}
<<<<<<< HEAD
=======

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn print_as_statement() {
    assert_clean_parse(r#"print "hello\n";"#);
}
<<<<<<< HEAD
=======

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn builtin_fat_arrow_in_method_call_args() {
    assert_clean_parse("$obj->method(log => 'file.log');");
}
<<<<<<< HEAD
=======

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn builtin_fat_arrow_in_constructor() {
    assert_clean_parse("my $obj = Class->new(log => Mojo::Log->new);");
}
<<<<<<< HEAD
=======

>>>>>>> 79f70270d (fix(parser): resolve unexpected_fat_arrow_expr errors in CPAN corpus)
#[test]
fn chained_builtin_fat_arrow_pairs() {
    assert_clean_parse("my %opts = (log => 'info', die => 0, warn => 1);");
}
