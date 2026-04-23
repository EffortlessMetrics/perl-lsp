# Parsing Perl: Why It's Hard, How We Do It Anyway

Larry Wall once said "only perl can parse Perl." He was not exaggerating. Perl's grammar is context-sensitive, ambiguous, and extensible at parse time. The same sequence of characters can mean completely different things depending on what came before, what module has been `use`d, and even what subroutine prototypes are in scope.

This document explains why Perl is one of the hardest mainstream languages to parse statically, and how the perl-lsp parser tackles each challenge.

---

## 1. "Only Perl Can Parse Perl"

Most programming languages have context-free grammars. You can write a BNF, generate a parser, and the output is deterministic. Perl is different in fundamental ways:

- **The lexer needs parser state.** The token `/` is division or a regex depending on what the parser just finished parsing. You cannot tokenize Perl without simultaneously parsing it.
- **The parser needs runtime state.** `use constant FOO => 1;` changes how `FOO` is parsed for the rest of the file. `use feature 'signatures'` changes whether `sub foo($x)` is a prototype or a parameter list.
- **Modules can rewrite source code.** Source filters (`use Filter::Simple`) transform text before the parser sees it. `Devel::Declare` and `Keyword::Simple` inject new syntax.

The consequence: a static parser (one that does not execute Perl code) can never be 100% correct. The goal is to be correct enough for IDE features -- completions, go-to-definition, error highlighting -- on the real-world Perl code people actually write.

---

## 2. The 10 Ambiguities

### 2.1 `/` -- Division or Regex?

```perl
my $avg = $sum / $count;    # division
my @matches = /pattern/;    # regex
if (/error/) { die; }       # regex after keyword
$x /= 2;                    # division-assign
```

**Why it's hard:** The character `/` has two completely different meanings. Unlike most languages where operators are syntactically unambiguous, Perl reuses `/` for both arithmetic and pattern matching.

**How we solve it:** The lexer tracks a mode state machine (`LexerMode` in `crates/perl-lexer/src/mode.rs`). After a term (variable, number, closing paren/bracket), the mode is `ExpectOperator` and `/` means division. At expression start or after a keyword, the mode is `ExpectTerm` and `/` starts a regex. The same mechanism disambiguates `//` (defined-or vs empty regex) and `%` (modulo vs hash sigil).

| Previous Token | Mode | `/` Means |
|---|---|---|
| `$variable` | ExpectOperator | division |
| `)` | ExpectOperator | division |
| `if` | ExpectTerm | regex |
| `=~` | ExpectTerm | regex |
| `(` | ExpectTerm | regex |

**Status:** Fully solved. The test suite in `crates/perl-parser-core/src/engine/parser/slash_ambiguity_tests.rs` covers edge cases including division after hash dereference, regex after binding operators, and the `//` defined-or operator.

### 2.2 `{}` -- Hash Reference, Block, or Bare Block?

```perl
my $ref  = { key => 'value' };    # hash reference
my $code = sub { print "hi" };    # block (sub body)
{ print "bare block"; }           # bare block statement
map { $_ * 2 } @list;             # block (builtin argument)
+{ key => 'value' }               # hash ref (disambiguated by +)
```

**Why it's hard:** Braces introduce three completely different constructs. In `map { ... } @list`, the braces are always a block. In `$ref = { key => value }`, they are a hash constructor. In statement position, `{ ... }` is a bare block. Perl programmers use unary `+` to disambiguate (`+{ }` forces hash ref), but a parser must handle the case when they do not.

**How we solve it:** The parser uses a multi-strategy approach in `parse_hash_or_block_inner()` (`crates/perl-parser-core/src/engine/parser/expressions/hashes.rs`):

1. **Builtin context:** `map`, `grep`, and `sort` get dedicated handling via `parse_builtin_block()` -- braces always mean "block" for these builtins.
2. **Empty braces `{}`** default to hash literal (the more common intent in expression context).
3. **Lookahead on content:** The parser tentatively parses the first expression. If a fat arrow (`=>`) follows, it is a hash. If the content looks like statements (keywords, semicolons), it is a block.
4. **Statement position vs expression position:** In statement context, bare `{ ... }` is always a block.

**Status:** Mostly solved. The test suite in `crates/perl-parser-core/src/engine/parser/hash_vs_block_tests.rs` covers hash refs, blocks, `map`/`grep`/`sort` blocks, and nested ambiguity. Some edge cases in deeply nested constructs remain as known gaps.

### 2.3 Heredocs -- Body Starts on the Next Line

```perl
my $text = <<END;
This is the heredoc body.
It continues until the terminator.
END

# Multiple heredocs on one line
print <<A, <<B;
First
A
Second
B

# Indented heredocs (5.26+)
my $html = <<~HTML;
    <div>
        <p>Hello</p>
    </div>
    HTML
```

**Why it's hard:** Heredocs break the fundamental assumption that parsing proceeds left-to-right, top-to-bottom. When the lexer sees `<<END`, the heredoc body does not start until the *next line* -- but the rest of the current line is still valid code. Multiple heredocs can be declared on one line, and their bodies are interleaved in FIFO order. Indented heredocs (`<<~`) require computing a common whitespace prefix across all body lines.

**How we solve it:** The parser maintains a FIFO queue of `PendingHeredoc` declarations (`crates/perl-heredoc/src/lib.rs`). When `<<LABEL` is encountered during token scanning, a `PendingHeredoc` is pushed onto the queue. At the next newline, `collect_all()` processes the queue in order, scanning source bytes line-by-line for matching terminators. For `<<~` heredocs, the terminator line's leading whitespace becomes the baseline, and that prefix is stripped from every content line.

The heredoc collector operates on raw `&[u8]` bytes (not `&str`) for maximum flexibility, handles CRLF normalization for Windows, and enforces a 256KB budget per heredoc to prevent hangs on pathological input.

**Status:** Fully solved, including multiple stacked heredocs, indented `<<~` heredocs, all quoting styles (bare, single-quoted, double-quoted, backtick), and CRLF line endings. Tested in `crates/perl-parser-core/src/engine/parser/heredoc_security_tests.rs`.

### 2.4 Prototypes vs Signatures

```perl
# Prototype (traditional)
sub mysub ($) { ... }     # takes one scalar argument

# Signature (modern, with `use feature 'signatures'`)
sub mysub ($x) { ... }    # parameter named $x

# Prototype characters: $ @ % & * + ; \
sub sum (\@) { ... }      # takes array reference
```

**Why it's hard:** `sub foo ($)` is either a prototype declaration or a signature depending on whether `use feature 'signatures'` (or `use v5.36`+) is in effect. A static parser cannot execute `use` statements, so it cannot definitively know which interpretation is correct.

**How we solve it:** The parser recognizes both forms. Prototype characters (`$`, `@`, `%`, `&`, `*`, `+`, `;`, `\`) without variable names are treated as prototypes. When the content between parentheses contains variable declarations (e.g., `$x`, `@args`), it is parsed as a signature. This heuristic is correct for virtually all real-world code because prototype syntax and signature syntax have distinct lexical shapes.

Declaration attributes (`:lvalue`, `:prototype($)`) are parsed via `parse_declaration_attributes()` in `crates/perl-parser-core/src/engine/parser/declarations.rs`.

**Status:** Partially solved. The heuristic handles the common cases. Edge cases involving `use feature` toggling within the same file are not tracked.

### 2.5 Special Variables -- `$/` Looks Like Division

```perl
local $/ = undef;        # input record separator (not division!)
my $pid = $$;            # process ID (not dereference!)
print $!;                # errno
$_ = "hello";            # default variable
$^W = 1;                 # warnings flag
${^MATCH}                # named capture group
```

**Why it's hard:** Perl has dozens of special variables that look like operators or syntax errors to a naive parser. `$/` looks like a variable `$` followed by division. `$$` looks like a scalar dereference. `$^W` looks like a variable `$` followed by the XOR operator. The parser needs a complete catalog of special variable forms to avoid misinterpretation.

**How we solve it:** The lexer recognizes special variable patterns during tokenization. When `$` is followed by a punctuation character that forms a known special variable (`/`, `!`, `\\`, `^`, etc.), the lexer emits a single `Variable` token for the entire construct rather than splitting it into separate tokens. The variable parsing in `crates/perl-parser-core/src/engine/parser/variables.rs` handles sigiled variables (`$`, `@`, `%`), caret variables (`$^W`), and long-form special variables (`${^MATCH}`).

**Status:** Mostly solved. Common special variables are recognized. Some exotic forms (e.g., `$:`, `$;`, `$"`) may be partially handled.

### 2.6 Format Statements -- A Different Mini-Language

```perl
format STDOUT =
Name:    @<<<<<<<<<
$name
Address: @>>>>>>>>>>
$address
.
```

**Why it's hard:** A `format` declaration contains a completely different mini-language. The body between `=` and the terminating `.` uses format-specific syntax (`@<<<`, `@>>>`, `@|||` for left/right/center justification) that is not valid Perl. The parser must recognize `format NAME =` as a declaration and consume the body verbatim until a line containing only `.`.

**How we solve it:** The lexer has a dedicated `InFormatBody` mode (`crates/perl-lexer/src/mode.rs:54`). When the parser encounters `format IDENTIFIER =`, it switches the lexer to format mode. In this mode, the lexer consumes all text verbatim until it finds a line containing only `.`, then switches back to normal parsing. The parser stores the body as a raw string in a `Format` AST node.

**Status:** Fully solved. Named formats (`format STDOUT =`), anonymous formats (`format =`), and body content are correctly parsed. See `crates/perl-parser-core/src/engine/parser/format_tests.rs`.

### 2.7 Indirect Object Syntax

```perl
new Foo('arg');           # Foo->new('arg')
move $player 10, 20;     # $player->move(10, 20)
close $fh;               # $fh->close()
```

**Why it's hard:** `new Foo()` looks syntactically identical to calling a function `new` with argument `Foo()`. The parser must decide whether a bareword in statement position is a function call or a method name in indirect-object syntax. This is officially discouraged in modern Perl but remains extremely common in legacy code (especially `new ClassName`).

**How we solve it:** The parser detects indirect-object patterns at statement start via `is_indirect_call_pattern()` in `crates/perl-parser-core/src/engine/parser/expressions/calls.rs`. It uses a curated list of builtins known to accept indirect syntax (`print`, `say`, `close`, `open`, etc.) and the special case of `new`. The detection checks what follows the method name: if it is a variable or class name followed by arguments, it is indirect syntax. If the method name is followed by a string literal directly (like `print "hello"`), it is treated as a regular function call.

**Status:** Mostly solved. Common patterns including `new Class()` and `print $fh "msg"` are handled correctly. Arbitrary user-defined methods in indirect style may not be detected. See `crates/perl-parser-core/src/engine/parser/indirect_object_tests.rs`.

### 2.8 `print STDERR "msg"` -- Is STDERR a Function Argument?

```perl
print STDERR "error\n";     # STDERR is a filehandle, not an argument
print $fh "data\n";         # $fh is a filehandle
print { $self->{fh} } "x"; # block-form filehandle
say $fh $message;           # say with filehandle
```

**Why it's hard:** In `print STDERR "msg"`, the bareword `STDERR` is a filehandle -- the output destination. But syntactically, it looks like a function call with two arguments. The parser must distinguish `print FILEHANDLE LIST` from `print LIST`. This extends to all I/O builtins: `printf`, `say`, `write`.

**How we solve it:** The parser treats `print`, `say`, and `printf` as special cases with multi-phase lookahead:

1. If the next token after `print` is a known bareword filehandle (`STDERR`, `STDOUT`, `STDIN`) or an uppercase identifier, it is treated as a filehandle.
2. If the next token is `$variable` and a second argument follows *without* a comma, the variable is the filehandle (e.g., `print $fh "data"`).
3. If the next token is `{`, the parser checks for block-form filehandle syntax (`print { expr } list`).
4. Otherwise, everything after `print` is the argument list.

This logic lives in the indirect-call detection path (`is_indirect_call_pattern()`) and produces `IndirectCall` AST nodes where the `object` is the filehandle.

**Status:** Mostly solved. Standard filehandle patterns are handled. Edge cases involving dynamically computed filehandles may fall back to regular function-call parsing.

### 2.9 Source Filters -- Modules That Rewrite Code

```perl
use Filter::Simple sub {
    s/greet/print "Hello, World!"/g;
};

greet;  # becomes: print "Hello, World!"
```

**Why it's hard:** Source filters transform the source code *text* before Perl's parser sees it. They can change syntax arbitrarily -- inserting new keywords, removing constructs, translating between languages. A static parser sees the pre-filter text, which may not be valid Perl at all. This is the strongest argument for "only perl can parse Perl": the filter runs Perl code to rewrite the source.

**How we solve it:** We do not solve it. Source filters are fundamentally incompatible with static analysis. The parser processes the source text as-is and may emit errors for filter-transformed code. In practice, source filters are rare in modern Perl code. More common transformation mechanisms like `Devel::Declare` and `Keyword::Simple` are also not supported but affect fewer modules.

**Status:** Known gap. Source-filtered code will produce parse errors. This is an inherent limitation of static Perl analysis.

### 2.10 `sort { $a <=> $b }` -- Block or Hash Ref?

```perl
sort { $a <=> $b } @list;           # block (comparison function)
sort { lc($a) cmp lc($b) } @list;  # block with function calls
my $cmp = sub { $a <=> $b };       # block (sub body)
my $ref = { a => 1, b => 2 };      # hash reference
```

**Why it's hard:** In `sort { ... } @list`, the braces are a comparison block. In `$ref = { a => 1 }`, they are a hash constructor. The same `{ ... }` syntax means fundamentally different things. Perl's own parser knows `sort` takes a block, but a general-purpose parser must encode this knowledge about every builtin.

**How we solve it:** `sort`, `map`, and `grep` receive special handling in the parser. When these builtins are followed by `{`, the parser calls `parse_builtin_block()` (`crates/perl-parser-core/src/engine/parser/expressions/hashes.rs:7`) which *always* produces a Block node, never a hash. This is unconditional -- Perl guarantees that `sort { ... }` is always a block.

**Status:** Fully solved for `sort`, `map`, and `grep`. User-defined functions that accept blocks (e.g., from `List::Util`) may have their block arguments misparsed as hash refs.

---

## 3. Three Parsers, One Language

Building a Perl parser is hard enough that we tried three times. Each attempt taught us what would not work.

### v1: Tree-sitter (C-based grammar)

Tree-sitter uses a GLR (Generalized LR) parser generator. You write a grammar in JavaScript, and tree-sitter compiles it to C. This approach works brilliantly for languages like Python, JavaScript, and Rust.

It does not work for Perl. Tree-sitter grammars are context-free. Perl's grammar is context-sensitive. The `/` ambiguity alone defeats any context-free approach -- the grammar cannot express "this is a regex if the previous token was a keyword, but division if it was a variable." Tree-sitter's conflict resolution mechanisms (precedence, associativity) cannot encode parser-state-dependent lexing.

The v1 parser lives in `tree-sitter-perl/` and is kept only for benchmark comparison.

### v2: Pest (PEG grammar)

Pest is a Parsing Expression Grammar (PEG) library for Rust. PEGs are more powerful than context-free grammars in some ways -- they support ordered choice and unlimited lookahead. We hoped this would be enough for Perl.

It was not. PEGs parse top-down with backtracking, which handled some ambiguities, but the fundamental problem remained: the grammar is context-sensitive. PEG parsers cannot maintain state between alternatives. When parsing `{ ... }`, a PEG can try "parse as hash" then "parse as block," but it cannot carry contextual information (like "we just saw `sort`") into the choice. Performance was also a challenge -- PEG backtracking on deeply nested Perl constructs caused exponential behavior.

The v2 parser is no longer in the default build.

### v3: Recursive Descent (current)

The current parser (`crates/perl-parser-core/src/engine/parser/`) is a hand-written recursive descent parser in Rust. This approach works for Perl because:

- **Stateful lexer:** The `LexerMode` state machine tracks whether to expect a term or an operator, solving the `/` ambiguity at the token level.
- **Contextual parsing:** The parser can pass context through function arguments and struct fields. When it sees `sort`, it calls `parse_builtin_block()` instead of the generic `parse_hash_or_block()`.
- **Arbitrary lookahead:** The parser can peek ahead multiple tokens (`peek_second()`, `peek_third()`) to disambiguate constructs like indirect object syntax.
- **IDE-friendly error recovery:** On syntax errors, the parser emits ERROR nodes and synchronizes to the next statement boundary, producing partial ASTs that are still useful for IDE features.

The tradeoff is maintenance cost. A generated parser (tree-sitter, Pest) gets structure and correctness from its grammar definition. A hand-written parser must encode every rule manually. For Perl, this tradeoff is worth it: the language requires a level of context sensitivity that generators cannot express.

---

## 4. The CPAN Corpus Oracle

How do you know your parser works? Unit tests cover individual constructs, but real Perl code is messier, more creative, and more surprising than any test author imagines. We test against reality.

### The Corpus

The CPAN corpus is a curated set of **4,355 Perl modules** from CPAN (the Comprehensive Perl Archive Network) -- the central repository of Perl libraries. These are production modules written by hundreds of different authors, covering everything from web frameworks to bioinformatics to Unicode processing.

The corpus baseline is tracked in `.ci/cpan-corpus-baseline.json` and is checked in CI. Every PR must not regress the number of cleanly parsed files.

### Error Buckets

When a file fails to parse cleanly, the parser reports the *first* error it encountered. These errors are categorized into buckets:

| Bucket | Files | Description |
|--------|-------|-------------|
| `unexpected_token_in_expr` | 146 | Catch-all for unexpected tokens during expression parsing |
| `unclosed_paren_identifier` | 140 | Unclosed parenthesis followed by identifier |
| `unexpected_question_expr` | 109 | Ternary operator misparsed |
| `unclosed_paren` | 106 | Unclosed parenthesis |
| `unexpected_rbrace_expr` | 83 | Unexpected closing brace in expression |
| `unexpected_comma_expr` | 70 | Unexpected comma in expression context |
| `expected_left_brace` | 66 | Missing opening brace |
| `expected_variable` | 66 | Expected a variable, found something else |
| `unexpected_fat_arrow_expr` | 66 | Fat arrow (`=>`) in unexpected position |
| `expected_comma_or_close_paren` | 55 | Missing comma or closing paren |
| `unclosed_bracket` | 38 | Unclosed bracket `[` |
| `unclosed_brace_semicolon` | 32 | Unclosed brace followed by semicolon |
| `unclosed_brace` | 32 | Unclosed brace `{` |
| `expected_identifier` | 30 | Expected bareword identifier |
| `expected_colon` | 26 | Missing colon (usually ternary) |

Each bucket drives development priorities. The top buckets (`unexpected_token_in_expr`, `unclosed_paren_identifier`) represent the largest payoff: fixing the root cause of a single bucket can make dozens of files parse cleanly.

### Ratcheting

The corpus uses a ratcheting mechanism: the number of cleanly parsed files can only increase. If a PR causes a regression (fewer files parse cleanly), CI fails. New clean files are automatically added to the baseline via `just cpan-corpus-ratchet`.

---

## 5. Error Recovery

Traditional compilers stop at the first error or produce a cascade of confusing follow-on diagnostics. An IDE parser cannot afford this. When a developer is in the middle of typing `my $x = `, the parser must:

1. Recognize the incomplete declaration
2. Insert an ERROR node for the missing expression
3. Continue parsing the rest of the file correctly
4. Report the error without losing context

### The Recovery Model

The parser uses **IDE-friendly error recovery** (`crates/perl-parser-core/src/engine/parser/mod.rs`):

- `parse()` returns `Ok(ast)` with embedded ERROR nodes for recoverable failures
- `parse()` returns `Err` only for catastrophic failures (stack overflow, recursion limits)
- `parser.errors()` provides the list of recovered errors with locations

This means checking `result.is_err()` is *not* the way to detect parse errors. You must inspect the AST for ERROR nodes.

### Synchronization

When the parser encounters an error, it calls `synchronize()` (`crates/perl-parser-core/src/engine/parser/helpers.rs:420`) to skip tokens until it reaches a statement boundary:

- Semicolons (`;`) -- statement terminators
- Closing braces (`}`) -- block endings
- Keywords (`sub`, `if`, `while`, `for`) -- statement starters
- EOF

The synchronizer skips at most 100 tokens to prevent infinite recovery loops on pathological input.

### Recursion Protection

Deeply nested or malformed input can cause stack overflow through recursive descent. The parser enforces:

- **MAX_RECURSION_DEPTH = 128:** Every recursive parsing function increments a depth counter. At 128, the parser returns `Err(RecursionLimit)`.
- **Postfix chain depth:** Postfix expression parsing (array subscripts, method calls, arrow dereferences) uses a separate chain-depth counter to prevent pathological chains like `$x->[0]->[1]->[2]->...` from overflowing.
- **Lexer budgets:** Regex patterns are limited to 64KB, heredoc bodies to 256KB, delimiter nesting to 128 levels. Exceeding a budget emits an `UnknownRest` token instead of hanging.

---

## 6. The Numbers

As of the latest corpus sweep:

| Metric | Value |
|--------|-------|
| **Total files** | 4,355 |
| **Clean parses** | 3,139 (72.1%) |
| **Files with errors** | 1,212 (27.9%) |
| **Total error nodes** | 6,817 |
| **Error buckets** | 30 distinct categories |
| **Parse time (full corpus)** | ~1.2 seconds |
| **Average per file** | ~275 microseconds |
| **Typical single-file parse** | 150 microseconds to 1 millisecond |

The parser handles the entire 4,355-file CPAN corpus in about 1.2 seconds on a single core. Typical developer files parse in under a millisecond, well within the latency budget for interactive IDE features.

---

## 7. What We Cannot Parse (Yet)

An honest accounting of remaining gaps.

### Source Filters

Modules using `Filter::Simple`, `Filter::Util::Call`, or other source filter mechanisms transform code before Perl's own parser sees it. A static parser cannot replicate this without executing Perl. Fortunately, source filters are increasingly rare in modern Perl.

### BEGIN Blocks That Modify the Parser

```perl
BEGIN { $^H |= 0x00000200 }  # enable strict refs
BEGIN { require Some::Import::Magic }
```

`BEGIN` blocks execute at compile time and can modify the compilation environment. Some modules use `BEGIN` to inject syntax, modify the symbol table, or change parser behavior. A static parser sees the `BEGIN` block but does not execute it.

### Eval'd Code

```perl
my $code = 'print "Hello"';
eval $code;
```

Code constructed as strings and evaluated at runtime cannot be statically parsed. The parser sees the string literal, not the Perl code inside it.

### Custom DSLs via Import

```perl
use Moose;

has 'name' => (is => 'ro', isa => 'Str');
```

Moose, Moo, and other object systems import DSL keywords (`has`, `extends`, `with`, `before`, `after`, `around`). The parser handles common patterns from these frameworks, but novel DSL constructs from less common modules may produce errors.

### Overloaded Operators That Change Syntax

```perl
use overload '+' => \&add, '""' => \&stringify;
```

Operator overloading changes the semantics of operators but does not change syntax. This is not a parsing problem per se, but it means static analysis of operator behavior requires understanding overloads.

### The Remaining 28%

Of the 1,212 files that do not parse cleanly, most fail due to combinations of the above challenges -- complex expression patterns, unusual builtin usage, and constructs that require knowing the prototypes or import effects of called functions. The error bucket analysis (Section 4) guides ongoing development: each bucket represents a family of related parse failures with a common root cause.

---

## Further Reading

- [perlsyn](https://perldoc.perl.org/perlsyn) -- Perl syntax documentation
- [perlop](https://perldoc.perl.org/perlop) -- Perl operators and precedence
- [perlvar](https://perldoc.perl.org/perlvar) -- Perl special variables
- [perlfilter](https://perldoc.perl.org/perlfilter) -- Source filters
- [CPAN Corpus Baseline](.ci/cpan-corpus-baseline.json) -- Current parse success rates
