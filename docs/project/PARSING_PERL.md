# Parsing Perl: A Deep Dive Into the Language That Fights Back

*How perl-lsp tackles the famously unparseable language, one ambiguity at a time.*

---

## "Only Perl Can Parse Perl" -- The Famous Claim

In 2008, Jeffrey Kegler published a proof that Perl's grammar is undecidable in
the general case.  The argument boils down to this: Perl allows arbitrary code
execution at compile time via `BEGIN` blocks, source filters, and overloaded
operators.  Because the *meaning* of subsequent tokens can depend on code that
runs during compilation, fully correct parsing requires actually *executing*
Perl -- which makes it Turing-complete at parse time.

Here is the simplest demonstration:

```perl
# What does the slash mean?
whatever / 25 ; # + 5765 ; {}
```

If `whatever` is a function that takes zero arguments, `/` is a division
operator and `25` is the divisor.  If `whatever` is a function expecting a
regex argument, then `/25 ; # + 5765 ; {}/` is the pattern passed to it.
You cannot know which without consulting the symbol table -- and that
symbol table can be modified at compile time by code you have not parsed yet.

This is not a theoretical curiosity.  It shows up in real CPAN code every day.

So what do you do when you need to build an IDE for a language that is
mathematically impossible to parse perfectly?  You get creative.

---

## The Rogues' Gallery of Parsing Challenges

### 1. Slash Ambiguity: Division or Regex?

The `/` character is Perl's most notorious syntactic prank.  It plays three
distinct roles:

```perl
my $quotient = $a / $b;          # Division operator
my $matched  = $str =~ /pattern/; # Regex delimiter
my $fallback = $x // $default;   # Defined-or operator (Perl 5.10+)
```

The trouble is that these three interpretations share the same character and
cannot be distinguished by local lookahead.  Consider:

```perl
$x / $y / $z            # ($x / $y) / $z   -- chained division
$x =~ /$y/ / $z         # regex match, then divide result by $z
if (/pattern/) { ... }  # regex in boolean context, no preceding term
```

What makes this truly devilish is that a *function call without parentheses*
can flip the interpretation:

```perl
print / 25 ;             # Is this  print(/25 ;/)  or  print / 25  ?
```

If `print` expects a filehandle or list, the slash is part of a regex.
If it is being used in void context with numeric division... well, it
is still a regex, because `print` is a named unary operator.  But a
user-defined function `whatever` with prototype `()` would make it
division.

### 2. Heredocs From Hell

Heredocs look simple at first glance:

```perl
my $text = <<EOF;
Hello, world!
EOF
```

Then you discover that Perl allows *multiple heredocs on a single line*,
with their bodies stacked in order below:

```perl
print <<'A', <<'B', <<"C";
First heredoc body
A
Second heredoc body
B
Third heredoc with $interpolation
C
```

The bodies appear in FIFO order, each terminated by its own label.  The lexer
must queue up all three declarations from the first line, then consume each
body sequentially.  And it gets worse:

- **Indented heredocs** (`<<~EOF`) strip leading whitespace, using the
  terminator line's indentation as a baseline (Perl 5.26+).
- **Heredocs in expressions**: `$obj->method(<<EOF)->trim` is legal.
  The heredoc body appears after the *entire statement line*, not after
  the method call.
- **Heredocs with deceptive content**: The word `EOF` appearing *inside*
  the body does not terminate it -- only `EOF` alone on a line does:

```perl
my $text = <<'END';
This line mentions END but is not the terminator.
ENDINGS are tricky too.
END
```

The depth limit is real, too.  perl-lsp's test corpus includes a file with
109 nested heredocs to verify the parser survives:

```perl
my $h1 = <<EOF1;
my $h2 = <<EOF2;
...
my $h109 = <<EOF109;
EOF109
EOF108
...
EOF1
```

### 3. Context Sensitivity and Sigils

Perl's sigils (`$`, `@`, `%`, `*`, `&`) change meaning based on context.
The `%` character is modulo after a term and a hash sigil at the start of
an expression:

```perl
my $remainder = $x % $y;    # Modulo operator
my %config = (key => "val"); # Hash declaration
```

Similarly, `*` is multiplication, exponentiation (`**`), or a glob/typeglob
sigil depending on context.  Even `<<` pulls triple duty: left shift,
heredoc declaration, and (in older code) a glob pattern.

The deeper issue is that Perl determines list-vs-scalar context at *runtime*,
and many builtins change their behavior accordingly:

```perl
my $count = caller;        # Scalar context: returns package name
my @info  = caller(0);     # List context: returns (package, file, line)
```

A static parser cannot know the runtime context, so it must produce an AST
that represents the *syntactic* form and defer context resolution to the
semantic analyzer.

### 4. Quote-Like Operators: q, qq, qw, qr, s, tr, y

Perl's quote-like operators accept almost any character as a delimiter:

```perl
my $str   = q/single-quoted/;
my $str2  = q{with {nested} braces};
my $str3  = q!bang delimiters!;
my $str4  = q#hash is not a comment here#;
my $regex = qr<angle brackets>;
my $cmd   = qx|pipe delimiters|;
```

When paired delimiters (`{}`, `[]`, `()`, `<>`) are used, nesting is
tracked:  `q{a{b}c}` contains the string `a{b}c`, not `a{b`.

Substitution operators add another dimension -- the pattern and replacement
can use *different* paired delimiters:

```perl
s{pattern}{replacement}    # Standard paired
s[pattern]{replacement}    # Mixed paired delimiters -- legal Perl
s/pattern/replacement/gi   # Classic non-paired
```

And then there are the modifiers.  Substitution alone accepts 15 valid
modifier characters: `g`, `i`, `m`, `s`, `x`, `o`, `e`, `r`, `a`, `d`,
`l`, `u`, `n`, `p`, `c`.  The `e` flag is particularly nasty because it
means the replacement is *evaluated as code*:

```perl
$text =~ s/(\w+)/uc($1)/eg;  # Replacement is Perl code
```

### 5. Indirect Object Notation

Perl's indirect object syntax is a parsing minefield:

```perl
my $obj = new MyClass "arg1", "arg2";  # Indirect: MyClass->new(...)
my $fh  = new FileHandle "test.txt";   # Same pattern with built-in
print STDOUT "hello\n";                # Indirect filehandle
```

The parser sees `new` followed by a bareword and must decide:  is this an
indirect method call (`MyClass->new(...)`), a function call (`new(MyClass, ...)`),
or something else entirely?  The ambiguity is severe:

```perl
method $object;      # Indirect method call?
method($object);     # Function call named 'method'?

# These are parsed differently:
new Class "arg";     # Indirect constructor
Class->new("arg");   # Arrow method call
```

Even Perl itself sometimes guesses wrong with indirect object syntax, which
is why `no indirect;` has become a best practice in modern Perl.

### 6. Format Statements

The `format` statement is one of Perl's most unusual constructs.  It
defines a report template using a completely different micro-language
embedded within Perl source:

```perl
format STDOUT =
@<<<<<< @>>>>  @####.##
$name, $age, $salary
.
```

The body between `format NAME =` and the terminating `.` on a line by itself
is *not Perl code*.  The `@` characters are field specifiers, not array
sigils.  The `<`, `>`, `|`, and `#` characters define alignment and numeric
formatting, not operators.  A Perl parser must recognize the `format` keyword,
switch into an entirely different lexing mode, consume raw text until a
lone `.` appears on a line, and then switch back.

### 7. Source Filters: The Truly Impossible Case

Source filters are the ultimate parsing spoiler.  They rewrite your source
code *before the parser sees it*:

```perl
use Filter::Simple;
FILTER {
    s/BANG!/return "excited"/g;
    s/MAGIC/42/g;
};

sub get_mood {
    BANG!;  # Becomes: return "excited"
}

my $answer = MAGIC;  # Becomes: 42
```

The `FILTER` block receives the *entire source text* as `$_` and can modify
it arbitrarily -- including inserting valid Perl that changes the meaning
of everything that follows.  There is literally no way to parse source-filtered
code without executing the filter.  This is the hard wall behind the "only
Perl can parse Perl" claim.

Real CPAN modules that use source filters include `Devel::Declare`,
`TryCatch`, `Method::Signatures`, `Filter::cpp` (C preprocessor directives
in Perl!), and the legendary `Acme::Bleach` (which encodes your entire
program as whitespace).

### 8. Prototypes and Attributes

Perl subroutine prototypes change how the parser treats call sites:

```perl
sub mygrep (&@) {         # Prototype: expects code block, then list
    my $code = shift;
    grep { $code->() } @_;
}

mygrep { $_->is_valid } @items;  # First arg parsed as block, not hashref
```

Without seeing the prototype, the parser cannot know whether `{ ... }` after
a function name is a hash reference or a code block.  Attributes add further
complexity:

```perl
sub cached : method : lvalue {
    return $self->{cache};
}

my $x : shared = 42;  # Variable attribute
```

### 9. The Hash-vs-Block Ambiguity

Curly braces in Perl are overloaded.  `{ }` can be:

- A code block: `if ($x) { print "yes"; }`
- A hash reference: `my $h = { key => "val" };`
- A hash slice: `@hash{@keys}`
- A dereference: `${$ref}`

The classic ambiguity:

```perl
sub handle { return $_[0] }

handle { key => 1 };    # Hash reference argument? Or code block?
handle({ key => 1 });   # Clearly a hash reference (parenthesized)
+{ key => 1 };          # Unary plus forces hash reference interpretation
```

Perl itself uses heuristics: if the first thing inside braces looks like a
string followed by `=>`, it is probably a hash reference.  Otherwise, it is
a block.  The `+` prefix is the conventional disambiguation.

---

## How perl-lsp Tackles Each One

### Mode-Based Lexer: 5 Lexer States

The heart of perl-lsp's disambiguation strategy is a **mode-tracking lexer**
with five states:

| Mode | Meaning | `/` becomes | `%` becomes |
|------|---------|-------------|-------------|
| `ExpectTerm` | Next token should be a value | Regex delimiter | Hash sigil |
| `ExpectOperator` | Next token should be an operator | Division (or `//`) | Modulo |
| `ExpectDelimiter` | Next token is a quote operator delimiter | N/A | N/A |
| `InFormatBody` | Inside `format` declaration | Raw text | Raw text |
| `InDataSection` | After `__DATA__` or `__END__` | Raw text | Raw text |

Mode transitions are driven by the previous token.  After emitting a number,
identifier, or closing delimiter, the lexer enters `ExpectOperator`.  After
emitting an operator, keyword, or opening delimiter, it enters `ExpectTerm`.
This is the single-pass, O(1) decision that resolves the slash ambiguity
without backtracking:

```
Previous Token           -> Next Mode       -> Slash Interpretation
-----------------------------------------------------------------
$x (variable)            -> ExpectOperator  -> division
42 (number)              -> ExpectOperator  -> division
) (closing paren)        -> ExpectOperator  -> division
if (keyword)             -> ExpectTerm      -> regex
=~ (match operator)      -> ExpectTerm      -> regex
( (opening paren)        -> ExpectTerm      -> regex
, (comma)                -> ExpectTerm      -> regex
```

The mode-based approach is a well-known technique (used by `perl` itself
internally), but getting the transition table right for Perl's ~80 token
types requires careful calibration against real-world code.

### FIFO Heredoc Collection

perl-lsp handles heredocs with a **pending queue** design:

1. When the lexer encounters `<<LABEL`, it pushes a `HeredocSpec` onto a
   `Vec<HeredocSpec>` queue with the label, indentation flag (`<<~`), and a
   placeholder for the body start offset.

2. When the lexer reaches the end of the statement line (after all heredoc
   declarations), it sets `body_start` for the first pending heredoc.

3. The lexer then scans line by line looking for the terminator.  When found,
   it pops the spec from the queue, optionally emits a `HeredocBody` token,
   and sets `body_start` for the next pending heredoc if any remain.

4. For indented heredocs (`<<~`), the terminator line's leading whitespace
   becomes the baseline, and that prefix is stripped from all content lines
   using a byte-level common prefix algorithm.

Safety limits prevent pathological input from causing hangs:

- `MAX_HEREDOC_BYTES`: 256 KB per heredoc body
- `MAX_HEREDOC_DEPTH`: 100 nested heredocs
- `HEREDOC_TIMEOUT_MS`: 5-second wall-clock timeout

When any limit is exceeded, the lexer emits an `UnknownRest` token and
continues, preserving all previously parsed tokens for IDE features.

### Quote Operator Parsing with Delimiter Tracking

The `perl-quote` crate provides uniform parsing for all quote-like operators.
It handles:

- **Paired delimiters** with depth tracking: `q{a{b}c}` correctly identifies
  the nesting and returns the full content.
- **Non-paired delimiters**: `s/pat/repl/` uses the same character for all
  three boundaries.
- **Mixed delimiters**: `s[pat]{repl}` uses different paired delimiters for
  pattern and replacement.
- **Escape handling**: `\\` inside delimited content is processed correctly.
- **Modifier validation**: Each operator type has a defined set of valid
  modifiers (e.g., `s///` accepts `g`, `i`, `m`, `s`, `x`, `o`, `e`, `r`,
  `a`, `d`, `l`, `u`, `n`, `p`, `c`).

The lexer enters `ExpectDelimiter` mode after recognizing a quote operator
keyword, which prevents the `#` in `qr#pattern#` from being interpreted as
a comment.

### Format Body as a Dedicated Lexer Mode

When the lexer encounters `format NAME =`, it transitions to `InFormatBody`
mode.  In this mode, all normal tokenization is suspended.  The lexer
consumes raw text line by line until it finds a line containing only `.`
(optionally preceded by whitespace).  The entire body is emitted as a single
`FormatBody` token, then the lexer returns to `ExpectTerm` mode.

This clean mode separation means the `@` and `#` characters inside format
bodies never trigger sigil or comment parsing.

### Recursive Descent with Error Recovery for IDE Usage

The parser (in `perl-parser-core`) uses an **IDE-friendly error recovery
model**:

- **Returns `Ok(ast)` with ERROR nodes** for most parse failures (recovered
  errors).
- **Returns `Err`** only for catastrophic failures (recursion limits,
  timeouts).

This means the LSP server always gets a partial AST, even for incomplete
or malformed code.  A developer typing `sub foo { if (` gets code completion,
hover information, and go-to-definition on the valid portions.

The parser protects itself with multiple layers:

| Guard | Limit | Purpose |
|-------|-------|---------|
| Recursion depth | 128 levels | Prevents stack overflow on deeply nested code |
| Parse budget | Configurable | Caps error recovery iterations |
| AST node count | 100,000 nodes | Memory protection |
| Wall-clock timeout | 5 seconds | Prevents hangs on pathological input |

### Checkpointing for Backtracking

The lexer supports **checkpointing** via the `Checkpointable` trait.
A checkpoint captures the complete lexer state -- position, mode, delimiter
stack, prototype tracking, and context.  The parser can save a checkpoint,
attempt a parse path, and restore if the path fails.  This is used sparingly
(mostly for disambiguating constructs like hash-vs-block) to avoid the
performance cost of speculative parsing.

---

## What's Still Hard: The Remaining Gaps

perl-lsp tracks its known limitations rigorously.  The
[corpus gap index](../issues/corpus/gaps/README.md) documents 13 timeout
and hang risks across three priority tiers:

### P0: Must-Fix

- **Catastrophic regex backtracking**: Patterns like `/(a+)+b/` applied to
  long strings can cause exponential backtracking in the regex engine.
  perl-lsp mitigates this at the *parser* level with a 64 KB regex body
  budget, but cannot prevent the user's Perl runtime from encountering
  these patterns.

- **Deep nesting stack overflow**: Extremely deep block nesting can exceed
  the parser's 128-level recursion limit.

### P1: High Impact

- **Hash-vs-block disambiguation**: The heuristic (look for `=>`) covers
  most cases but can misfire on constructs like `{ $computed_key }`.

- **Indirect object syntax**: `new Class "arg"` vs. `new("Class", "arg")`
  requires knowledge of the symbol table that a static parser does not have.

- **Multiple heredocs on a single line**: Fully supported but complex
  interactions with expressions and method calls remain a source of
  edge cases.

### P2: Long-Tail

- **Source filters**: Fundamentally impossible without executing the filter
  code.  perl-lsp parses the `use Filter::Simple` declaration and the
  `FILTER { }` block as normal Perl, but cannot apply the source
  transformation.

- **Regex code execution blocks**: `(?{ code })` and `(??{ code })` embed
  arbitrary Perl inside regex patterns.  The parser treats these as opaque
  text within the regex token.

- **Unicode property lookups**: `\p{Script=Devanagari}` requires access to
  Unicode property tables that the lexer does not consult.

---

## Can You Really Parse Perl Without Running It?

The honest answer is: **not with 100% accuracy, but with high enough accuracy
to be useful.**

perl-lsp's approach is pragmatic:

1. **Handle the common cases correctly.**  The mode-based lexer resolves
   slash ambiguity for the vast majority of real-world code.  The FIFO
   heredoc queue handles multiple heredocs on one line.  Quote operators
   with arbitrary delimiters Just Work.

2. **Degrade gracefully on the impossible cases.**  Source filters, runtime
   prototypes, and compile-time code execution produce syntactically valid
   but semantically approximate parse trees.  The LSP server still provides
   navigation, completion, and diagnostics -- they might just be slightly
   wrong in pathological edge cases.

3. **Never hang, never crash.**  Budget limits, timeouts, and recursion
   guards ensure the parser always terminates.  When limits are exceeded,
   an `UnknownRest` token preserves everything parsed so far.

4. **Let the user help.**  When the parser genuinely cannot disambiguate
   a construct, it picks the most common interpretation and moves on.
   Users who write `+{ key => val }` instead of `{ key => val }` are
   giving the parser (and future readers) a gift.

The gap between "mathematically perfect" and "practically useful" turns
out to be narrow.  Most Perl code does not exercise the Turing-complete
corners of the grammar.  Most developers do not put source filters in
their LSP-edited files.  Most heredocs are not 109 levels deep.

---

## Lessons for Language Tooling Developers

Building a parser for Perl surfaces challenges that apply to any
context-sensitive language.  Here are the transferable lessons:

### 1. Mode-Based Lexing Is Underrated

Many language tooling projects reach for parser generators (PEG, Earley,
GLR) when a hand-written lexer with explicit modes would be simpler, faster,
and more debuggable.  perl-lsp's five-mode lexer resolves the most critical
ambiguity (slash) with a single flag check -- no backtracking, no
ambiguity tables, no runtime costs.

### 2. Budget Everything

Real-world input is adversarial.  Fuzz testing will find the pathological
heredoc, the 200-level nested regex, the 10 MB single-line string.  Set
explicit byte budgets, recursion limits, and wall-clock timeouts from
day one.  Emit a degraded token and move on.  A slow parser that eventually
produces output is worse than a fast parser that says "I gave up here" and
keeps going.

### 3. IDE Parsers Are Not Compiler Parsers

Compiler parsers return an AST or an error.  IDE parsers must *always*
return an AST -- even for incomplete, malformed, or actively-being-typed
code.  This requires a fundamentally different error recovery philosophy:
insert ERROR nodes, skip to synchronization points, and keep going.

### 4. Test the Impossible Cases

perl-lsp's test corpus includes files specifically designed to break
parsers: 109 nested heredocs, regex with embedded code blocks, source
filter declarations, indirect object chains, and ambiguous slash sequences.
These tests do not assert *correct* parsing (which may be impossible) --
they assert *bounded* parsing: the parser terminates, does not panic,
and produces some output.

### 5. Separate Lexer and Parser Concerns

The heredoc problem illustrates why clean separation matters.  The lexer
handles the line-by-line body scanning and terminator matching.  The parser
deals with the declaration syntax and AST construction.  The `perl-heredoc`
crate handles indentation stripping.  Each piece is testable in isolation.

### 6. Accept Imperfection

The most important lesson from Perl parsing is knowing when to stop.
Source filters make perfect parsing impossible.  Indirect object syntax
makes correct disambiguation impossible without a symbol table.  The
pragmatic choice is to handle 98% of real code correctly, degrade
gracefully on the rest, and document the gaps honestly.

A parser that handles common Perl correctly and admits its limitations
is infinitely more useful than one that claims perfection and crashes on
the first source filter.

---

*This article is based on the perl-lsp codebase.  Key source files:*

- *Lexer modes: `crates/perl-lexer/src/mode.rs`*
- *Lexer core: `crates/perl-lexer/src/lib.rs`*
- *Heredoc collection: `crates/perl-heredoc/src/lib.rs`*
- *Quote parsing: `crates/perl-quote/src/lib.rs`*
- *Token definitions: `crates/perl-token/src/lib.rs`*
- *Parser core: `crates/perl-parser-core/src/engine/parser/mod.rs`*
- *Error recovery: `crates/perl-parser-core/src/engine/parser/enhanced_recovery.rs`*
- *Corpus gaps: `docs/issues/corpus/gaps/README.md`*
- *Test corpus: `test_corpus/`*
