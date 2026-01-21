//! Static edge case fixtures and complex data structure samples.

/// A single edge case fixture with metadata.
#[derive(Debug, Clone, Copy)]
pub struct EdgeCase {
    /// Stable identifier for the fixture.
    pub id: &'static str,
    /// Short human-readable description.
    pub description: &'static str,
    /// Tags for filtering and grouping.
    pub tags: &'static [&'static str],
    /// Perl source for the edge case.
    pub source: &'static str,
}

/// A complex data structure fixture for DAP/LSP variable inspection.
#[derive(Debug, Clone, Copy)]
pub struct ComplexDataStructureCase {
    /// Stable identifier for the fixture.
    pub id: &'static str,
    /// Short human-readable description.
    pub description: &'static str,
    /// Perl source for the fixture.
    pub source: &'static str,
}

static EDGE_CASES: &[EdgeCase] = &[
    EdgeCase {
        id: "heredoc.basic",
        description: "Basic quoted heredoc with multiple lines.",
        tags: &["heredoc", "edge-case"],
        source: r#"my $text = <<'EOF';
line one
line two
EOF
"#,
    },
    EdgeCase {
        id: "heredoc.indented",
        description: "Indented heredoc using <<~ syntax.",
        tags: &["heredoc", "edge-case"],
        source: r#"my $text = <<~EOF;
  indented line
EOF
"#,
    },
    EdgeCase {
        id: "quote.like",
        description: "Quote-like operator with interpolation.",
        tags: &["quote-like", "interpolation"],
        source: r#"my $name = "Ada";
my $text = qq{Hello $name};
"#,
    },
    EdgeCase {
        id: "regex.code",
        description: "Regex with embedded code block.",
        tags: &["regex", "regex-code", "edge-case"],
        source: r#"my $count = 0;
"x" =~ /(?{ $count++ })x/;
"#,
    },
    EdgeCase {
        id: "regex.named.capture",
        description: "Regex with named capture and hash access.",
        tags: &["regex", "edge-case"],
        source: r#"my $text = "abc";
if ($text =~ /(?<word>abc)/) {
    print $+{word};
}
"#,
    },
    EdgeCase {
        id: "substitution.balanced",
        description: "Substitution with balanced delimiters and modifiers.",
        tags: &["substitution", "regex", "edge-case"],
        source: r#"my $text = "foo bar";
$text =~ s{foo}{bar}g;
"#,
    },
    EdgeCase {
        id: "transliteration.basic",
        description: "Transliteration with character ranges.",
        tags: &["transliteration", "tr", "edge-case"],
        source: r#"my $text = "abc";
$text =~ tr/a-z/A-Z/;
"#,
    },
    EdgeCase {
        id: "map.grep",
        description: "Map/grep with block syntax.",
        tags: &["map", "grep", "list-context"],
        source: r#"my @nums = (1, 2, 3);
my @doubled = map { $_ * 2 } @nums;
my @even = grep { $_ % 2 == 0 } @nums;
"#,
    },
    EdgeCase {
        id: "map.empty.block",
        description: "Map with an empty block.",
        tags: &["map", "list-context", "edge-case"],
        source: r#"my @nums = (1, 2, 3);
my @mapped = map { } @nums;
"#,
    },
    EdgeCase {
        id: "grep.empty.block",
        description: "Grep with an empty block.",
        tags: &["grep", "list-context", "edge-case"],
        source: r#"my @nums = (1, 2, 3);
my @kept = grep { } @nums;
"#,
    },
    EdgeCase {
        id: "format.statement",
        description: "Format statement with picture lines.",
        tags: &["format", "legacy", "edge-case"],
        source: r#"my ($name, $age) = ("Ada", 37);
format STDOUT =
@<<<<<< @>>>>>
$name, $age
.
write;
"#,
    },
    EdgeCase {
        id: "glob.angle",
        description: "Glob expression using angle brackets.",
        tags: &["glob", "file", "edge-case"],
        source: r#"my @files = <*.pl>;
my @more = glob "*.pm";
"#,
    },
    EdgeCase {
        id: "tie.hash",
        description: "Tie and untie a hash.",
        tags: &["tie", "hash", "edge-case"],
        source: r#"tie my %cache, "Tie::StdHash";
$cache{a} = 1;
untie %cache;
"#,
    },
    EdgeCase {
        id: "redo.loop",
        description: "Redo inside a loop.",
        tags: &["redo", "loop", "edge-case"],
        source: r#"my $count = 0;
while ($count < 3) {
    $count++;
    redo if $count == 2;
}
"#,
    },
    EdgeCase {
        id: "continue.block",
        description: "Continue block after a for loop.",
        tags: &["continue", "loop", "edge-case"],
        source: r#"for my $i (1..3) {
    next if $i == 2;
} continue {
    my $j = $i * 2;
}
"#,
    },
    EdgeCase {
        id: "defined.or",
        description: "Defined-or operator with undef fallback.",
        tags: &["defined-or", "operator", "edge-case"],
        source: r#"my $value = undef // 42;
"#,
    },
    EdgeCase {
        id: "given.when",
        description: "Given/when flow with default branch.",
        tags: &["given", "when", "flow", "edge-case"],
        source: r#"use v5.10;
my $value = 2;
given ($value) {
    when (1) { print "one"; }
    when (2) { print "two"; }
    default { print "other"; }
}
"#,
    },
    EdgeCase {
        id: "eval.block",
        description: "Eval block with error handling.",
        tags: &["eval", "error", "edge-case"],
        source: r#"eval { die "boom" };
warn $@ if $@;
"#,
    },
    EdgeCase {
        id: "do.block",
        description: "Do block returning a computed value.",
        tags: &["do", "block", "edge-case"],
        source: r#"my $result = do {
    my $x = 1;
    $x + 1;
};
"#,
    },
    EdgeCase {
        id: "package.qualified",
        description: "Package-qualified subroutine call.",
        tags: &["package", "subroutine", "edge-case"],
        source: r#"My::Pkg::helper();
"#,
    },
    EdgeCase {
        id: "signature.defaults",
        description: "Subroutine signatures with defaults and slurpy params.",
        tags: &["signature", "subroutine", "edge-case"],
        source: r#"sub add($x, $y = 0, @rest) {
    return $x + $y + @rest;
}
"#,
    },
    EdgeCase {
        id: "package.block",
        description: "Package block with nested subroutine.",
        tags: &["package", "subroutine", "edge-case"],
        source: r#"package Foo::Bar {
    sub helper { return 1; }
}
"#,
    },
    EdgeCase {
        id: "method.chain",
        description: "Chained method calls with arrows.",
        tags: &["method", "arrow", "edge-case"],
        source: r#"my $value = $obj->foo->bar(1, 2);
"#,
    },
    EdgeCase {
        id: "try.catch.finally",
        description: "Try/catch/finally control flow.",
        tags: &["try", "catch", "finally", "edge-case"],
        source: r#"try {
    die "boom";
}
catch ($e) {
    warn $e;
}
finally {
    print "done";
}
"#,
    },
    EdgeCase {
        id: "defer.block",
        description: "Defer blocks running on scope exit.",
        tags: &["defer", "block", "edge-case"],
        source: r#"use v5.36;
use feature 'defer';
no warnings 'experimental::defer';

sub cleanup {
    defer { print "cleanup\n"; }
    return 1;
}
"#,
    },
    EdgeCase {
        id: "postfix.deref.slice",
        description: "Postfix dereference with slice.",
        tags: &["postfix", "dereference", "edge-case"],
        source: r#"my $aref = [1, 2, 3];
my @slice = $aref->@[0, 2];
"#,
    },
    EdgeCase {
        id: "postfix.deref.hash",
        description: "Postfix dereference with hash expansion.",
        tags: &["postfix", "dereference", "edge-case"],
        source: r#"my $href = { a => 1, b => 2 };
my %copy = $href->%*;
my @keys = $href->@{qw(a b)};
"#,
    },
    EdgeCase {
        id: "class.field.method",
        description: "Class with fields and method.",
        tags: &["class", "field", "method", "edge-case"],
        source: r#"class Point {
    field $x :param = 0;
    method get_x { return $x; }
}
"#,
    },
    EdgeCase {
        id: "state.counter",
        description: "State variable with initialization.",
        tags: &["state", "edge-case"],
        source: r#"sub counter($step = 1) {
    state $count = 0;
    return $count += $step;
}
"#,
    },
    EdgeCase {
        id: "smartmatch.array",
        description: "Smartmatch with array of roles.",
        tags: &["smartmatch", "operator", "edge-case"],
        source: r#"my @roles = qw(admin user);
if ("admin" ~~ @roles) {
    print "has role";
}
"#,
    },
    EdgeCase {
        id: "pack.unpack",
        description: "Pack and unpack byte arrays.",
        tags: &["pack", "unpack", "edge-case"],
        source: r#"my $packed = pack("C*", 65, 66, 67);
my @bytes = unpack("C*", $packed);
"#,
    },
    EdgeCase {
        id: "filetest.stack",
        description: "Stacked filetest operators.",
        tags: &["filetest", "edge-case"],
        source: r#"if (-r -w -x $path) {
    print "read write exec";
}
"#,
    },
    EdgeCase {
        id: "filetest.handle",
        description: "Filetest operator on a filehandle.",
        tags: &["filetest", "file", "edge-case"],
        source: r#"open my $fh, "<", "file.txt";
if (-t $fh) {
    print "tty";
}
"#,
    },
    EdgeCase {
        id: "ambiguous.slash",
        description: "Division vs regex slash ambiguity.",
        tags: &["regex", "operator", "ambiguous", "edge-case"],
        source: r#"my $ratio = $a / $b;
my $match = $a =~ /$b/;
my $complex = $x / $y / $z;
my $regex = /$x\/$y/;
"#,
    },
    EdgeCase {
        id: "indirect.object",
        description: "Indirect object syntax for constructors.",
        tags: &["method", "ambiguous", "parser-sensitive", "edge-case"],
        source: r#"my $logger = new Logger "app.log";
my $time = new DateTime (year => 2024, month => 1, day => 1);
"#,
    },
    EdgeCase {
        id: "special.vars",
        description: "Special variables and sigil-heavy globals.",
        tags: &["special-var", "variable", "edge-case"],
        source: r#"my $program = $0;
my $error = $!;
my $status = $?;
my $count = @ARGV;
my $env_home = $ENV{HOME};
"#,
    },
    EdgeCase {
        id: "typeglob.alias",
        description: "Typeglob aliasing and symbol table entries.",
        tags: &["typeglob", "glob", "edge-case"],
        source: r#"local *STDOUT = *DATA;
*Alias::printer = \&Other::printer;
"#,
    },
    EdgeCase {
        id: "sort.block",
        description: "Sort with comparison block.",
        tags: &["sort", "list-context", "edge-case"],
        source: r#"my @sorted = sort { $a <=> $b } @values;
"#,
    },
    EdgeCase {
        id: "eval.string",
        description: "String eval with error handling.",
        tags: &["eval", "error", "edge-case"],
        source: r#"my $code = "sub generated { return 42; }";
eval $code;
warn $@ if $@;
"#,
    },
    EdgeCase {
        id: "sub.attribute",
        description: "Subroutines with attributes.",
        tags: &["subroutine", "method", "edge-case"],
        source: r#"my $value = 1;
sub getter :lvalue { return $value; }
sub setter :method { $value = shift; }
"#,
    },
    EdgeCase {
        id: "lexical.sub",
        description: "Lexical subroutine declaration.",
        tags: &["subroutine", "declaration", "feature", "edge-case"],
        source: r#"use feature 'lexical_subs';
my sub helper ($x) { return $x + 1; }
my $value = helper(1);
"#,
    },
    EdgeCase {
        id: "pod.basic",
        description: "POD section with a simple header and cut.",
        tags: &["pod", "edge-case"],
        source: r#"=pod

=head1 NAME

Sample::Module

=cut

my $value = 1;
"#,
    },
    EdgeCase {
        id: "vstring.literal",
        description: "V-string version literals.",
        tags: &["vstring", "version", "edge-case"],
        source: r#"my $ver = v5.36.0;
my $min = v5.10;
"#,
    },
    EdgeCase {
        id: "prototype.sub",
        description: "Subroutine with prototype.",
        tags: &["prototype", "subroutine", "edge-case"],
        source: r#"sub sum ($$) { return $_[0] + $_[1]; }
"#,
    },
    EdgeCase {
        id: "postfix.control",
        description: "Postfix control flow with if/unless.",
        tags: &["postfix", "if", "unless", "flow", "edge-case"],
        source: r#"my $ready = 1;
print "go\n" if $ready;
warn "no\n" unless $ready;
"#,
    },
    EdgeCase {
        id: "goto.label",
        description: "Goto with label and conditional loop.",
        tags: &["goto", "labels", "flow", "edge-case"],
        source: r#"my $count = 0;
START:
$count++;
goto START if $count < 2;
"#,
    },
    EdgeCase {
        id: "use.feature.signatures",
        description: "Feature and warning pragmas for signatures.",
        tags: &["use", "feature", "signatures", "warnings", "edge-case"],
        source: r#"use feature 'signatures';
no warnings 'experimental::signatures';
sub add ($x, $y) { return $x + $y; }
"#,
    },
    EdgeCase {
        id: "builtin.truth",
        description: "Builtin boolean helpers and predicates.",
        tags: &["builtin", "feature", "edge-case"],
        source: r#"use v5.36;
use builtin qw(true false is_bool);

my $flag = true;
my $ok = is_bool($flag);
"#,
    },
    EdgeCase {
        id: "state.local.our",
        description: "State, local, and our variable declarations.",
        tags: &["state", "local", "our", "declaration", "edge-case"],
        source: r#"our $global = 1;
local $global = 2;
sub tick {
    state $count = 0;
    return ++$count;
}
"#,
    },
    EdgeCase {
        id: "phaser.blocks",
        description: "Compile-time phase blocks with setup and teardown.",
        tags: &["block", "statement", "edge-case"],
        source: r#"BEGIN { $| = 1; }
UNITCHECK { my $setup = 1; }
CHECK { my $ok = 1; }
INIT { srand 42; }
END { print "done\n"; }
"#,
    },
    EdgeCase {
        id: "regex.branch.reset",
        description: "Regex branch reset groups with shared capture numbering.",
        tags: &["regex", "branch-reset", "edge-case"],
        source: r#"my $text = "ab";
if ($text =~ /(?|(a)(b)|(ab))/) {
    print $1;
}
"#,
    },
    EdgeCase {
        id: "regex.lookaround",
        description: "Regex lookahead and lookbehind assertions.",
        tags: &["regex", "assertion", "edge-case"],
        source: r#"my $text = "foobar";
if ($text =~ /foo(?=bar)/) {
    print "ahead";
}
if ($text =~ /(?<=foo)bar/) {
    print "behind";
}
"#,
    },
    EdgeCase {
        id: "regex.verbs",
        description: "Regex with control verbs.",
        tags: &["regex", "edge-case"],
        source: r#"my $text = "abc";
if ($text =~ /a(*SKIP)(*FAIL)|abc/) {
    print "match";
}
"#,
    },
    EdgeCase {
        id: "regex.recursive",
        description: "Regex with recursion.",
        tags: &["regex", "edge-case"],
        source: r#"my $text = "abc";
if ($text =~ /(a(?R)?c)/) {
    print $1;
}
"#,
    },
    EdgeCase {
        id: "hash.block.ambiguity",
        description: "Hash vs block ambiguity in function calls.",
        tags: &["hash", "block", "ambiguous", "parser-sensitive", "edge-case"],
        source: r#"sub handle { return 1; }
handle { key => 1 };
handle({ key => 1 });
"#,
    },
    EdgeCase {
        id: "flipflop.operator",
        description: "Flip-flop range operator in scalar context.",
        tags: &["range", "flipflop", "operator", "flow", "edge-case"],
        source: r#"my $hit = 0;
for my $i (1..6) {
    $hit = 1 if $i == 2 .. $i == 4;
}
"#,
    },
    EdgeCase {
        id: "autoload.destroy",
        description: "AUTOLOAD and DESTROY subroutines for method fallback and cleanup.",
        tags: &["autoload", "destructor", "method", "edge-case"],
        source: r#"package Auto::Demo;
sub AUTOLOAD {
    our $AUTOLOAD;
    return $AUTOLOAD;
}
sub DESTROY { }
1;
"#,
    },
    EdgeCase {
        id: "overload.stringify",
        description: "Operator overloading for string and numeric contexts.",
        tags: &["overload", "operator", "method", "edge-case"],
        source: r#"package Counter;
use overload '""' => sub { $_[0]->{count} }, '0+' => sub { $_[0]->{count} }, fallback => 1;
sub new { bless { count => 1 }, shift }
1;
"#,
    },
    EdgeCase {
        id: "symbolic.reference",
        description: "Symbolic reference with disabled strict refs.",
        tags: &["reference", "parser-sensitive", "edge-case"],
        source: r#"no strict 'refs';
my $name = "value";
${$name} = 42;
my $value = ${"value"};
"#,
    },
    EdgeCase {
        id: "data.section",
        description: "DATA section with filehandle reads.",
        tags: &["file", "io", "edge-case"],
        source: r#"while (my $line = <DATA>) {
    print $line;
}
__DATA__
alpha
beta
"#,
    },
    EdgeCase {
        id: "utf8.escape",
        description: "UTF-8 pragma with escaped Unicode code points.",
        tags: &["utf8", "unicode", "edge-case"],
        source: r#"use utf8;
my $text = "na\x{EF}ve";
my $smile = "\x{1F600}";
"#,
    },
    EdgeCase {
        id: "end.section",
        description: "END section with trailing content ignored by the parser.",
        tags: &["end-section", "file", "edge-case"],
        source: r#"print "before\n";
__END__
this should be ignored
"#,
    },
    EdgeCase {
        id: "source.filter.simple",
        description: "Source filter using Filter::Simple.",
        tags: &["source-filter", "use", "edge-case", "parser-sensitive"],
        source: r#"use Filter::Simple;
FILTER {
    s/foo/bar/g;
}
my $text = "foo";
print $text;
"#,
    },
    EdgeCase {
        id: "inline.c",
        description: "Inline::C heredoc embedding C source.",
        tags: &["inline", "xs", "ffi", "edge-case"],
        source: r#"use Inline C => <<'END_C';
int add(int x, int y) {
    return x + y;
}
END_C

my $sum = add(1, 2);
"#,
    },
    EdgeCase {
        id: "bareword.filehandle",
        description: "Legacy bareword filehandle open/print/close.",
        tags: &["file", "io", "legacy", "edge-case"],
        source: r#"open FH, "<", "file.txt" or die $!;
print FH "ok\n";
close FH;
"#,
    },
    EdgeCase {
        id: "lvalue.substr",
        description: "Lvalue substring assignment.",
        tags: &["lvalue", "string", "edge-case"],
        source: r#"my $text = "foobar";
substr($text, 1, 3) = "OOO";
"#,
    },
    EdgeCase {
        id: "mro.c3",
        description: "Method resolution order pragma.",
        tags: &["mro", "inheritance", "edge-case", "use"],
        source: r#"use mro "c3";
our @ISA = ("Base");
sub method { return 1; }
"#,
    },
    EdgeCase {
        id: "super.method",
        description: "SUPER:: method dispatch from a subclass.",
        tags: &["method", "inheritance", "edge-case", "package"],
        source: r#"package Child;
use parent "Base";
sub new {
    my $class = shift;
    return $class->SUPER::new(@_);
}
1;
"#,
    },
    EdgeCase {
        id: "goto.sub",
        description: "Goto to a subroutine for tail-call style dispatch.",
        tags: &["goto", "subroutine", "edge-case"],
        source: r#"sub helper { return 42; }
sub wrapper { goto &helper; }
my $value = wrapper();
"#,
    },
    EdgeCase {
        id: "transliteration.y",
        description: "Transliteration using the y/// alias.",
        tags: &["transliteration", "tr", "edge-case"],
        source: r#"my $text = "abc";
$text =~ y/a-z/A-Z/;
"#,
    },
    EdgeCase {
        id: "variable.attribute.shared",
        description: "Variable attribute using threads::shared.",
        tags: &["attribute", "variable", "edge-case", "declaration"],
        source: r#"use threads::shared;
my $counter :shared = 0;
"#,
    },
];

static COMPLEX_DATA_STRUCTURE_CASES: &[ComplexDataStructureCase] = &[
    ComplexDataStructureCase {
        id: "nested.hash.array",
        description: "Nested hash/array structure.",
        source: r#"my $data = {
    users => [
        { id => 1, name => "Ada" },
        { id => 2, name => "Bob" },
    ],
    flags => { active => 1, admin => 0 },
};
"#,
    },
    ComplexDataStructureCase {
        id: "circular.reference",
        description: "Self-referential hash.",
        source: r#"my $node = {};
$node->{self} = $node;
"#,
    },
    ComplexDataStructureCase {
        id: "blessed.object",
        description: "Blessed hash reference.",
        source: r#"my $obj = bless { name => "Widget", count => 3 }, "My::Class";
"#,
    },
    ComplexDataStructureCase {
        id: "mapped.records",
        description: "Array of hash records created via map.",
        source: r#"my @values = map { { id => $_, name => "item_$_" } } (1..5);
"#,
    },
    ComplexDataStructureCase {
        id: "typeglob.alias",
        description: "Typeglob aliasing and filehandle.",
        source: r#"open my $fh, "<", "file.txt";
*ALIAS = *STDOUT;
"#,
    },
    ComplexDataStructureCase {
        id: "graph.refs",
        description: "Graph-like structure with nested edges.",
        source: r#"my $graph = {
    nodes => [
        { id => 1, edges => [2, 3] },
        { id => 2, edges => [1] },
    ],
    meta => { directed => 0 },
};
"#,
    },
    ComplexDataStructureCase {
        id: "handlers.hash",
        description: "Hash of handlers with coderefs.",
        source: r#"my $handlers = {
    on_ready => sub { return 1; },
    on_error => sub { return 0; },
};
"#,
    },
    ComplexDataStructureCase {
        id: "deep.nested.refs",
        description: "Deeply nested references with arrays and hashes.",
        source: r#"my $data = {
    items => [
        { id => 1, children => [ { id => 2 }, { id => 3 } ] },
        { id => 4, children => [] },
    ],
    meta => { count => 2 },
};
"#,
    },
    ComplexDataStructureCase {
        id: "hash.special.keys",
        description: "Hash with empty and spaced keys.",
        source: r#"my $data = {
    "" => 0,
    " spaced key " => 1,
    "0" => "zero",
};
"#,
    },
    ComplexDataStructureCase {
        id: "array.of.blessed",
        description: "Array of blessed hash references.",
        source: r#"my $objs = [
    bless({ id => 1, label => "a" }, "Obj"),
    bless({ id => 2, label => "b" }, "Obj"),
];
"#,
    },
    ComplexDataStructureCase {
        id: "mixed.types",
        description: "Array with mixed scalar and reference types.",
        source: r#"my $data = [
    1,
    "two",
    [3, 4],
    { five => 5 },
    sub { return 6; },
];
"#,
    },
    ComplexDataStructureCase {
        id: "array.self.ref",
        description: "Array that contains a reference to itself.",
        source: r#"my $list = [];
push @$list, $list;
"#,
    },
    ComplexDataStructureCase {
        id: "blessed.array",
        description: "Blessed array reference object.",
        source: r#"my $obj = bless [1, 2, 3], "ArrayObj";
"#,
    },
    ComplexDataStructureCase {
        id: "refs.in.hash",
        description: "Hash with scalar references and nested collections.",
        source: r#"my $value = 3;
my $data = {
    value => \$value,
    list => [1, 2, 3],
    lookup => { a => 1 },
};
"#,
    },
    ComplexDataStructureCase {
        id: "hash.with.undef",
        description: "Hash with undef and falsey values.",
        source: r#"my $data = {
    ok => 1,
    nope => 0,
    maybe => undef,
    note => "value",
};
"#,
    },
    ComplexDataStructureCase {
        id: "regex.and.refs",
        description: "Hash with compiled regex and scalar reference.",
        source: r#"my $value = 10;
my $data = {
    matcher => qr/^item_\d+$/i,
    value_ref => \$value,
    flags => [undef, 0, 1],
};
"#,
    },
    ComplexDataStructureCase {
        id: "scalar.ref.chain",
        description: "Nested scalar references.",
        source: r#"my $value = 42;
my $ref1 = \$value;
my $ref2 = \$ref1;
"#,
    },
    ComplexDataStructureCase {
        id: "tied.array",
        description: "Tied array with queued values.",
        source: r#"tie my @queue, "Tie::Array";
push @queue, "first";
my $item = $queue[0];
"#,
    },
    ComplexDataStructureCase {
        id: "tied.hash",
        description: "Tied hash with stored values.",
        source: r#"tie my %cache, "Tie::StdHash";
$cache{foo} = 1;
my $value = $cache{foo};
"#,
    },
    ComplexDataStructureCase {
        id: "closure.capture",
        description: "Closure capturing a lexical variable.",
        source: r#"my $count = 0;
my $next = sub { return ++$count; };
"#,
    },
    ComplexDataStructureCase {
        id: "stash.entries",
        description: "Symbol table stash reference with package entries.",
        source: r#"package Stash::Demo;
our $VERSION = "0.01";
sub helper { return 1; }
my $stash = \%Stash::Demo::;
"#,
    },
];

/// Return the static edge case fixtures.
pub fn edge_cases() -> &'static [EdgeCase] {
    EDGE_CASES
}

/// Return the static complex data structure fixtures.
pub fn complex_data_structure_cases() -> &'static [ComplexDataStructureCase] {
    COMPLEX_DATA_STRUCTURE_CASES
}

/// Backwards-compatible accessor for complex data structure fixtures.
pub fn get_complex_data_structure_tests() -> &'static [ComplexDataStructureCase] {
    complex_data_structure_cases()
}

/// Convenience helper for working with static edge cases.
pub struct EdgeCaseGenerator;

impl EdgeCaseGenerator {
    /// Return all available edge cases.
    pub fn all_cases() -> &'static [EdgeCase] {
        edge_cases()
    }

    /// Return edge cases with a matching tag.
    pub fn by_tag(tag: &str) -> Vec<&'static EdgeCase> {
        edge_cases().iter().filter(|case| case.tags.iter().any(|t| *t == tag)).collect()
    }

    /// Return edge cases that match any of the provided tags.
    pub fn by_tags_any(tags: &[&str]) -> Vec<&'static EdgeCase> {
        if tags.is_empty() {
            return edge_cases().iter().collect();
        }

        edge_cases().iter().filter(|case| case.tags.iter().any(|tag| tags.contains(tag))).collect()
    }

    /// Return edge cases that match all of the provided tags.
    pub fn by_tags_all(tags: &[&str]) -> Vec<&'static EdgeCase> {
        if tags.is_empty() {
            return edge_cases().iter().collect();
        }

        edge_cases()
            .iter()
            .filter(|case| tags.iter().all(|tag| case.tags.iter().any(|t| t == tag)))
            .collect()
    }

    /// Find a single edge case by ID.
    pub fn find(id: &str) -> Option<&'static EdgeCase> {
        edge_cases().iter().find(|case| case.id == id)
    }

    /// Return sorted unique edge case tags.
    pub fn tags() -> Vec<&'static str> {
        let mut tags: Vec<&'static str> =
            edge_cases().iter().flat_map(|case| case.tags.iter().copied()).collect();
        tags.sort();
        tags.dedup();
        tags
    }
}

/// Find a complex data structure fixture by ID.
pub fn find_complex_case(id: &str) -> Option<&'static ComplexDataStructureCase> {
    complex_data_structure_cases().iter().find(|case| case.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn edge_cases_have_ids() {
        assert!(edge_cases().iter().all(|case| !case.id.is_empty()));
    }

    #[test]
    fn edge_cases_can_filter_by_tag() {
        let heredocs = EdgeCaseGenerator::by_tag("heredoc");
        assert!(!heredocs.is_empty());
    }

    #[test]
    fn edge_cases_can_filter_by_any_tag() {
        let matches = EdgeCaseGenerator::by_tags_any(&["regex", "heredoc"]);
        assert!(!matches.is_empty());
    }

    #[test]
    fn edge_cases_can_filter_by_all_tags() {
        let matches = EdgeCaseGenerator::by_tags_all(&["regex", "regex-code"]);
        assert!(matches.iter().any(|case| case.id == "regex.code"));
    }

    #[test]
    fn edge_case_tags_are_unique() {
        let tags = EdgeCaseGenerator::tags();
        let mut deduped = tags.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(tags, deduped);
    }

    #[test]
    fn edge_case_ids_are_unique() {
        let mut seen = HashSet::new();
        for case in edge_cases() {
            assert!(seen.insert(case.id), "Duplicate edge case id: {}", case.id);
        }
    }

    #[test]
    fn complex_case_lookup_by_id() {
        let case = find_complex_case("nested.hash.array");
        assert!(case.is_some());
    }

    #[test]
    fn complex_case_ids_are_unique() {
        let mut seen = HashSet::new();
        for case in complex_data_structure_cases() {
            assert!(seen.insert(case.id), "Duplicate complex case id: {}", case.id);
        }
    }
}
