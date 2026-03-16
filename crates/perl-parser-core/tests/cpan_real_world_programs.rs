//! CPAN Pattern Tests: Real-World Programs

mod cpan_test_helpers;
use cpan_test_helpers::*;

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
