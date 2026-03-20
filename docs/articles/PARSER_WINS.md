# Perl Parsing Hall of Fame

The hardest Perl constructs to parse correctly -- and how perl-lsp handles them.

Perl is famously difficult to parse. Larry Wall once said that "only perl can parse Perl," and there is a well-known theoretical result that parsing Perl is undecidable in the general case. perl-lsp's v3 recursive descent parser takes on this challenge head-on, handling the gnarliest corners of the language with clean, error-free ASTs.

This document showcases the weirdest, most complex, and most surprising valid Perl that our parser handles correctly. Every example below is backed by a passing test in the `perl-parser-core` test suite.

---

## 1. Heredoc Inception

**Difficulty: 3/5**

Heredocs are a mini-language embedded in Perl's syntax. The body of a heredoc appears on the *next line* after the declaration, which means the parser must track pending heredoc bodies as a FIFO queue while continuing to lex the rest of the current line.

```perl
# Heredoc with interpolation in a function call
print <<END;
Hello, $name!
Welcome to $place.
END

# Word operator AFTER a heredoc on the same line
print <<END or die;
Hello
END
```

**Why it's tricky:** The `or die` belongs to the `print` statement, but the heredoc body physically separates them. The parser must remember to resume the expression after collecting the heredoc content. Most naive parsers lose track of the statement boundary here.

**Component:** `perl-heredoc` crate with FIFO `PendingHeredoc` queue, plus `perl-parser-core` expression continuation after heredoc collection.

**Test files:** `cpan_misc_idioms.rs`, `word_operator_tests.rs`

---

## 2. The Slash Ambiguity

**Difficulty: 4/5**

Is `/` a division operator or the start of a regex? The answer depends entirely on what came before it -- and Perl has dozens of different contexts to consider.

```perl
# Slash as regex after builtins
my @parts = split /,/, $string;
my @matches = grep /pattern/, @list;
my @flags = map /pattern/, @list;
print /pattern/;

# Regex with the /e modifier -- evaluates replacement as code
$text =~ s/\$(\w+)/$vars{$1}/ge;
```

**Why it's tricky:** After `split`, `/` starts a regex. After a variable like `$x`, `/` is division. After `print`, `/` is a regex. The parser must track operator-expectation context through every single expression. The `split /,/, $string` case is particularly nasty because it contains two slashes used as regex delimiters and a third as a list separator, all in one expression.

**Component:** `perl-parser-core` expression parser with context-aware regex disambiguation in `fix_builtin_regex_disambiguation`.

**Test files:** `fix_builtin_regex_disambiguation.rs`, `cpan_regex_patterns.rs`

---

## 3. Block vs Hash

**Difficulty: 5/5**

Perl uses `{ }` for code blocks, anonymous hash references, and hash constructors. The parser must decide which one it is seeing based on surrounding context and contents.

```perl
# Hash reference (key => value inside)
my $href = { a => 1, b => 2 };

# Code block (statement inside)
my $r = { print 'hello' };

# Unary plus forces hash interpretation
+{ key => 'val' };

# eval block -- it's a code block, not a hash
eval { die 'test' };

# sort block -- code block with special $a/$b variables
sort { $a <=> $b } @list;

# do block
do { my $x = 1; $x + 2 };

# die with a hash reference (not a block!)
die { code => 404, message => "Not found" };

# map/grep block -- no semicolon before closing brace
if (1) { map { $_ * 2 } @arr }
```

**Why it's tricky:** `{ foo => 1 }` is a hash ref. `{ foo() }` is a code block. `{ foo }` could be either. The parser uses heuristics including: does it start with a string/bareword followed by `=>`? Does it contain semicolons? Is it in a context that expects a value? The `map`/`grep` case adds another wrinkle: the block before the list argument looks like it could be a hash ref, and when the block is the last expression in an outer block, there's no trailing semicolon to disambiguate.

**Component:** `perl-parser-core` hash/block disambiguation engine.

**Test files:** `hash_block_disambiguation_tests.rs`, `map_grep_sort_no_semicolon_tests.rs`

---

## 4. Fat Arrow Gymnastics

**Difficulty: 3/5**

The `=>` operator (fat arrow/fat comma) auto-quotes the bareword on its left side. But what happens when the "bareword" is actually a Perl keyword?

```perl
# Keywords become strings before =>
my %h = (if => 1, for => 2, while => 3);
my %h = (return => 1, my => "value", use => "something");
my %h = (BEGIN => 1, END => 2);
my %h = (eval => 1, do => 2);
my %h = (package => 1, class => 2, method => 3);

# At statement level -- is `if => 1` an if-statement or a pair?
if => 1;   # It's a fat-arrow pair, not control flow!

# DBI connect-style hashref argument
DBI->connect($dsn, $user, $pass, { RaiseError => 1 });

# Nested fat arrows in Moose-style declarations
has 'name' => (is => 'ro', isa => 'Str', default => sub { 'unknown' });
```

**Why it's tricky:** When the parser sees `if`, it normally begins parsing an if-statement. But if the *next* token is `=>`, it must backtrack and treat `if` as a simple string. This look-ahead must work for every keyword in the language. The Moose `has` example combines fat arrows with nested parentheses, sub references, and string arguments -- all in one expression.

**Component:** `perl-parser-core` keyword autoquoting with `=>` look-ahead.

**Test files:** `keyword_autoquoting_tests.rs`, `fat_arrow_args_tests.rs`

---

## 5. Moose/Moo DSL

**Difficulty: 4/5**

Moose and Moo turn Perl into a declarative OOP language using ordinary function calls that *look* like new syntax. The parser must handle these without special-casing every CPAN module.

```perl
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
```

**Why it's tricky:** `has`, `around`, `before`, `after`, `with`, `extends` -- these all look like keywords but are really just function calls. `has 'name' => (...)` is syntactically a function call with a string argument followed by a fat arrow and a parenthesized list. `around 'speak' => sub { ... }` passes an anonymous sub as a hash value. `$self->$orig(@_)` is a dynamic method dispatch through a variable. The parser must handle all of this as standard Perl without any Moose-specific grammar rules.

**Component:** `perl-parser-core` expression parser treating DSL keywords as regular function calls.

**Test files:** `cpan_moose_moo.rs`

---

## 6. Typeglob Chains

**Difficulty: 5/5**

Typeglobs (`*`) provide access to Perl's symbol table entries. When combined with dereferencing and method calls, they create some of the most complex expression chains in the language.

```perl
# Typeglob with scalar dereference and hash access
*$self->{field} = 'auto';

# Typeglob dereference into method call chain
*$self->{Compress}->crc32();

# Dynamic typeglob with string concatenation
*{$pkg . '::' . $name} = $code;

# Glob slot access
return ref \$_ eq 'GLOB' && *$_{HASH} && exists $$_{$sub};
return ref \$_ eq 'GLOB' ? *$_{CODE} : undef;

# Nested scalar deref of glob slot
return ${*$_{SCALAR}};

# From IO::Compress -- pack with glob dereference chain
return pack("V V", *$self->{Compress}->crc32(),
                   *$self->{UnCompSize}->get32bit());
```

**Why it's tricky:** `*$self->{field}` is a typeglob dereference of `$self` followed by hash access -- NOT `*` applied to the expression `$self->{field}`. The parser must correctly bind the precedence: the `*` applies to `$self`, then `->{field}` chains off the result. `*$_{HASH}` accesses the HASH slot of a glob, not the hash element "HASH" of `*$_`. This is one of the constructs where Perl's precedence rules are most counterintuitive, and CPAN modules like IO::Compress rely on it heavily.

**Component:** `perl-parser-core` typeglob expression parsing with correct precedence binding.

**Test files:** `fix_typeglob_arrow_tests.rs`, `glob_deref_arrow_tests.rs`

---

## 7. The eval/use Constant Dance

**Difficulty: 4/5**

Perl's `use constant` combined with `eval` blocks creates parsing challenges where block boundaries become ambiguous.

```perl
# Feature detection at compile time
use constant HAS_FOO => eval { require Foo::Bar; Foo::Bar->import; 1 } || 0;
use constant ROLES => !!(eval { require Role::Tiny; 1 });
use constant JSON_XS => $ENV{X} ? 0 : !!eval { require Foo; 1 };

# eval block assigned to typeglob with ternary
*_fs_encode = eval { require Encode; 1 }
    ? sub { Encode::encode("iso-8859-1", $_[0]) }
    : sub { $_[0] };

# Nested eval in capture
my ($stdout, $stderr, $ok) = capture {
    eval {
        local @ARGV = @$argv;
        $run_rv = $app->run;
        1;
    };
};

# use constant with ternary and nested hash refs
use constant MAP => 1 ? { foo => 'a' } : { bar => 'b' };
```

**Why it's tricky:** In `use constant FOO => eval { ... } || 0`, the `}` that ends the eval block must not be confused with a hash constructor or a statement block. The `|| 0` continues the expression *after* the block. The typeglob example combines eval blocks, ternary operators, and anonymous sub definitions. The parser must track that the `}` of `eval { ... }` is a block terminator that yields a value, not the end of a statement.

**Component:** `perl-parser-core` eval block parsing with expression continuation, `use constant` value expression handling.

**Test files:** `fix_use_eval_block_tests.rs`, `eval_brace_tests.rs`, `use_constant_ternary_tests.rs`

---

## 8. Special Variables

**Difficulty: 4/5**

Perl has dozens of special variables with punctuation names that collide with operators. The parser must distinguish between them without ambiguity.

```perl
# Format/report variables (single punctuation)
$~ = 'REPORT';        # Current format name (not bitwise complement!)
$^ = 'REPORT_TOP';    # Current top-of-page format
$= = 60;              # Page length (not comparison!)
$% = 0;               # Page number (not modulo!)
$, = ", ";             # Output field separator
$" = ":";             # List separator (inside a string!)
$; = "\034";           # Subscript separator

# Caret variables
$^W = 1;               # Warnings flag (not $^ XOR W!)
my $os = $^O;           # Operating system name
my $perl = $^X;         # Perl executable path

# $$ ambiguity: PID vs scalar dereference
my $pid = $$;           # Process ID
my $x = $$sv;           # Scalar dereference of $sv

# Both in one expression!
my $s = sprintf("%s #%d %s", class($sv), $$sv, $specialsv_name[$$sv]);

# local $/ for slurp mode
local $/ = undef;       # Not division!

# Signal handlers
local $SIG{__WARN__} = sub { };
local $SIG{__DIE__} = sub { log_error($_[0]) };
```

**Why it's tricky:** `$=` looks like a variable followed by an assignment operator, but it's actually the variable `$=` (page length) being assigned to. `$^W` looks like `$^` (format top name) followed by `W`, but it's the single variable `$^W` (warnings). `$$sv` looks like `$$` (PID) followed by `sv`, but it's actually `$` (scalar deref) applied to `$sv`. The parser must use maximal munch rules and context to correctly identify each special variable.

**Component:** `perl-lexer` special variable recognition with maximal munch, `perl-parser-core` variable parsing.

**Test files:** `special_punct_variables_tests.rs`, `cpan_misc_idioms.rs`

---

## 9. Subroutine One-Liners

**Difficulty: 4/5**

CPAN is full of terse one-liner subroutines that combine multiple dereference operations and implicit returns in a single expression.

```perl
# Array deref of shift result with hash access
sub count { scalar @{shift->{_nodes}} }

# Scalar deref of deep method chain
sub ancestor { ${shift->widget->toplevel->WindowId} }

# Array deref of shift with hash access
sub heredoc { @{shift->{_heredoc}} }

# Symbolic deref with delete (Tk::Image)
delete ${"$class\::"}{'::ISA::CACHE::'};

# Block-form delete/exists as last expression
sub close { delete $_[0]{cb} }
sub has_foo { exists $_[0]{foo} }
foreach $sym (@names) { delete $imports{$sym} }
```

**Why it's tricky:** In `sub count { scalar @{shift->{_nodes}} }`, the `}` that closes `@{...}` is *not* the `}` that closes the subroutine -- but they're adjacent. `shift` is called without parentheses, its result is dereferenced as a hash with `->{_nodes}`, and that result is block-dereferenced as an array with `@{...}`. The symbolic deref `${"$class\::"}` constructs a package name at runtime and accesses its symbol table. `delete` and `exists` as the last expression in a block means the parser must recognize them as block-terminable expressions, not statements requiring a semicolon.

**Component:** `perl-parser-core` block termination with expression-last-in-block detection, `fix_rbrace_terminator`.

**Test files:** `test_oneliners.rs`, `fix_rbrace_terminator_tests.rs`

---

## 10. Modern Perl (v5.38+ class/field/method)

**Difficulty: 4/5**

Perl 5.38 introduced native class syntax with `class`, `field`, and `method` keywords. The parser must handle these as new declaration forms while still allowing them as barewords in pre-5.38 code.

```perl
# Full v5.38 class with field declarations and attributes
use v5.38;
class Point {
    field $x :param;
    field $y :param;
    field $z = 0;
}

# Namespaced class with method
class My::App::Service { method run { 1; } }

# But "field" is also a common bareword in older code!
my %config = (field => 'name', type => 'text');
$form->field('username');
sub field { return $_[0]->{field} }
field('name', type => 'text');

# And "class" as a hash key
my %h = (class => 2, method => 3);
```

**Why it's tricky:** `field $x` is a field declaration inside a class body, but `field($x)` is a function call. `field => 'name'` is a fat-arrow pair. `field;` is a bareword expression statement. The parser must use context (are we inside a `class` body? is the next token a sigil?) to decide which interpretation applies. This is a microcosm of Perl's fundamental parsing challenge: the same token sequence has different meanings in different contexts.

**Component:** `perl-parser-core` class/field declaration parsing with context-sensitive keyword resolution.

**Test files:** `field_declaration_tests.rs`, `fix_field_keyword_regression.rs`, `parser_tests.rs`

---

## 11. Postfix Dereference

**Difficulty: 3/5**

Perl 5.20+ introduced postfix dereference syntax that reverses the traditional deref order.

```perl
# Postfix array deref with push (5.20+)
push $aref->@*, $x;
push $h->{key}->@*, $val;

# Postfix last-index
my $n = $aref->$#*;
my $len = $aref->$#* + 1;
for (my $i = 0; $i <= $aref->$#*; $i++) { }

# Deep chain postfix deref (Biber-style)
unless (first { $_ eq $name } $ref->{names}->{key}->@*) { 1; }
```

**Why it's tricky:** `->@*` looks like an arrow operator followed by `@*`, which isn't a valid variable. `->$#*` combines arrow, the `$#` (last index) sigil, and `*` in a way that no other language construct does. The parser must recognize these as postfix dereference operations, not as arrow-method-calls or binary operators.

**Component:** `perl-parser-core` postfix dereference expression parsing.

**Test files:** `postfix_deref_regression_tests.rs`, `list_util_block_funcs_tests.rs`

---

## 12. The print Filehandle Ambiguity

**Difficulty: 3/5**

`print` can take an optional filehandle before its argument list, and that filehandle can be specified in several different ways.

```perl
# Bare filehandle
print STDOUT "message\n";
print STDERR "error\n";

# Scalar filehandle
print $fh "data\n";

# Block-form filehandle (required for complex expressions)
print { $fh } "data\n";
print { *STDERR } "error\n";
print { $self->{fh} } "msg\n";
print { $self->fh() } "msg\n";

# Multiple args with block filehandle
print { $fh } "key=", $value, "\n";
```

**Why it's tricky:** In `print { $fh } "data"`, the `{ $fh }` is a block expression producing a filehandle -- NOT a hash reference. But `{ $fh }` by itself could be either a block or a hash ref. The parser must know that after `print`, a leading `{` introduces a filehandle block, not a hash argument. This is context-dependent parsing at its finest: the same `{ expr }` construct has three different meanings depending on what precedes it (block, hash ref, or filehandle expression).

**Component:** `perl-parser-core` indirect call handling for print/say/printf.

**Test files:** `cpan_misc_idioms.rs`

---

## 13. The Try::Tiny Protocol

**Difficulty: 3/5**

Try::Tiny implements try/catch/finally using anonymous subs and function calls -- no source filtering or syntax hacks.

```perl
# Basic try/catch
try { dangerous_op() } catch { warn "caught: $_" };

# Full try/catch/finally
try { dangerous_op() }
catch { warn "caught: $_" }
finally { cleanup() };

# Nested try/catch
try {
    try { inner_op() }
    catch { warn "inner: $_" };
} catch {
    warn "outer: $_";
};

# Result capture
my $result = try { might_fail() }
catch { warn "caught: $_"; undef };
```

**Why it's tricky:** `try { ... } catch { ... }` looks like two adjacent blocks, but it's actually `try(sub { ... }, catch(sub { ... }))` -- a function call passing anonymous subs. The `catch` and `finally` keywords are not Perl builtins; they're imported functions. The parser handles this naturally because its expression grammar correctly parses `identifier BLOCK identifier BLOCK` as chained function calls with block arguments.

**Component:** `perl-parser-core` block-argument function calls.

**Test files:** `cpan_try_tiny.rs`

---

## 14. The DBI Transaction Pattern

**Difficulty: 3/5**

DBI database transactions combine eval blocks, method chains, error checking, and multiple statement forms.

```perl
my $dbh = DBI->connect("dbi:SQLite:dbname=test.db", "", "",
                        { RaiseError => 1 });
eval {
    $dbh->begin_work;
    $dbh->do("INSERT INTO log (msg) VALUES (?)", undef, $message);
    $dbh->do("UPDATE counters SET count = count + 1 WHERE name = ?",
             undef, 'inserts');
    $dbh->commit;
};
if ($@) {
    $dbh->rollback;
    die "Transaction failed: $@";
}
```

**Why it's tricky:** This pattern combines a class method call (`DBI->connect`) with a hash ref argument, an eval block with multiple method calls inside, and error handling with `$@`. The `undef` as a placeholder argument, the embedded SQL strings, and the conditional `die` after rollback -- all must parse correctly. The semicolon after the eval block's `}` is required (it's an expression statement), while semicolons inside the eval block separate statements within the block.

**Component:** `perl-parser-core` eval block handling, method call parsing.

**Test files:** `cpan_dbi.rs`

---

## 15. The Carp `_fetch_sub` Gauntlet

**Difficulty: 5/5**

This real-world function from `Carp.pm` (shipped with Perl core) combines almost every tricky construct in one function:

```perl
sub _fetch_sub {
    my($pack, $sub) = @_;
    $pack .= '::';
    return unless exists($::{$pack});
    for ($::{$pack}) {
        return unless ref \$_ eq 'GLOB'
            && *$_{HASH}
            && exists $$_{$sub};
        for ($$_{$sub}) {
            return ref \$_ eq 'GLOB' ? *$_{CODE} : undef
        }
    }
}
```

**Why it's tricky:** This function is a parsing stress test. It features:
- `$::` (the main symbol table hash)
- `$::{$pack}` (hash access on the stash)
- `ref \$_` (reference to a scalar dereference)
- `*$_{HASH}` (glob slot access -- NOT a hash subscript on `*$_`)
- `$$_{$sub}` (scalar deref of `$_` used as hash, with variable key)
- `*$_{CODE}` (glob CODE slot access)
- Nested `for` loops aliasing `$_` to different values
- Ternary operator with `undef` as the false branch
- `return` without value as flow control

Every one of these constructs individually trips up naive parsers. Having them all in a single function body is the ultimate integration test.

**Component:** All of `perl-parser-core` -- typeglob handling, special variable recognition, dereference chains, flow control, ternary expressions.

**Test files:** `glob_deref_arrow_tests.rs`

---

## By the Numbers

| Metric | Value |
|--------|-------|
| Test files in `perl-parser-core` | 90+ |
| Individual parse assertions | 500+ |
| CPAN pattern categories tested | 15+ |
| Special variables handled | 30+ |
| Disambiguation heuristics | block/hash, regex/division, keyword/bareword |

---

## Think you can stump our parser?

Open an issue with your weirdest valid Perl! If `perl -c` accepts it and perl-lsp doesn't, we want to know. Tag it with `parser-bug` and we'll add it to our test suite.

https://github.com/EffortlessMetrics/perl-lsp/issues
