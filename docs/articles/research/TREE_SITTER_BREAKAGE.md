# Why Tree-Sitter Failed for Perl

*Seven concrete patterns that defeated tree-sitter's GLR parser generator, and why a stateless grammar formalism cannot express Perl's context-sensitive syntax.*

---

## Background

The perl-lsp project began as a tree-sitter grammar for Perl. Tree-sitter is the dominant parser generator for IDE use cases — it powers syntax highlighting, code folding, and structural navigation in Neovim, Helix, Zed, and GitHub's code view. It works brilliantly for Python, JavaScript, TypeScript, Rust, Go, C, and dozens of other languages.

It does not work for Perl.

The v1 tree-sitter parser lives in `tree-sitter-perl/` and is kept only for benchmark comparison. The grammar file (`grammar.js`) and the scanner (`scanner.c`) together tell the story of why Perl defeated the formalism.

This document catalogs the seven specific patterns that caused failure, explains why tree-sitter's architecture cannot handle them, and describes the key insight that made the v3 recursive descent parser succeed.

---

## 1. The `/` Ambiguity (Division vs. Regex Delimiter)

### The Pattern

```perl
my $avg = $sum / $count;    # / is division
my @m = /pattern/;          # / starts a regex
if (/error/) { die; }       # / starts a regex after keyword
$x /= 2;                    # /= is division-assign
```

### Why Tree-Sitter Fails

Tree-sitter grammars are context-free. The lexer (external scanner in `scanner.c`) runs independently of the parser — it does not know what the parser just finished parsing. But the meaning of `/` depends entirely on what came before:

- After a term (`$variable`, number, `)`, `]`, `}`): `/` is division
- After an operator, keyword, `(`, `[`, `{`, `=~`: `/` starts a regex

This is **lexer-parser coupling** — the lexer needs parser state to produce correct tokens. Tree-sitter's architecture explicitly prohibits this. The external scanner can maintain internal state, but it cannot query the parse stack.

### Attempted Workarounds in scanner.c

The tree-sitter grammar attempted to solve this with a state machine in the scanner that tracked the "last significant token type." The scanner maintained a variable that was updated on every token to remember whether the previous token was a term or an operator.

This worked for simple cases but failed on:
- Closing delimiters: `)` at the end of `if ($x)` should make `/` a regex (division makes no sense), but `)` at the end of `foo()` should make `/` division
- Newlines: In Perl, newlines are not significant syntax — `$x\n/ 2` is still division
- Comments: `$x # comment\n/ 2` — the comment must be transparent to the state machine
- Nested constructs: `map { $_ / 2 } @list` — the `/` inside the block is division, but the block's `}` ends a term that makes the next `/` ambiguous again

### Lines of scanner.c Spent

The `/` disambiguation alone consumed approximately 150 lines of `scanner.c`, and it was still not correct for all cases.

---

## 2. Heredoc Body Location

### The Pattern

```perl
my $text = <<END;
This is the heredoc body.
END

# Body starts on the NEXT line, but the current line continues:
print <<A, <<B;
First heredoc body
A
Second heredoc body
B
```

### Why Tree-Sitter Fails

Tree-sitter parsers are incremental — they process input left-to-right, token-by-token. When the parser sees `<<END`, it expects the heredoc body to follow. But the body starts on the *next* line. The rest of the current line (after `<<END;`) is still valid code.

This means the parser must:
1. Record that a heredoc is pending
2. Continue parsing the rest of the current line normally
3. At the next newline, switch to consuming the heredoc body
4. After finding the terminator, resume normal parsing

This requires **deferred token production** — the scanner must queue tokens for later delivery. Tree-sitter's scanner API is synchronous: `scan()` returns one token at the call site. It cannot say "I'll produce this token later."

### Attempted Workarounds

The scanner maintained a FIFO queue of pending heredoc declarations (`PendingHeredoc` structs). At each newline, it checked the queue and consumed heredoc bodies before returning the next token. This worked for single heredocs but broke on:

- Multiple heredocs on one line (`<<A, <<B` — bodies interleaved in FIFO order)
- Indented heredocs (`<<~END` — requires computing minimum whitespace prefix across all body lines)
- Heredocs inside expressions (`foo(<<END, 42)` — the parser sees the heredoc declaration mid-expression)

### Lines of scanner.c Spent

Heredoc handling consumed approximately 200 lines of `scanner.c`.

### How v3 Solves It

The v3 parser's heredoc handling lives in a dedicated crate (`crates/perl-heredoc/`). The lexer pushes `PendingHeredoc` entries onto a queue. At newline boundaries, `collect_all()` processes the queue in FIFO order, scanning raw `&[u8]` bytes for matching terminators. The collector handles CRLF normalization, `<<~` indentation stripping, all quoting styles, and enforces a 256KB budget per heredoc.

The key difference: the v3 lexer and parser share state. The heredoc collector is called *by* the parser at the right moment, not by an independent scanner that must guess when to intervene.

---

## 3. The `{}` Ambiguity (Hash vs. Block vs. Bare Block)

### The Pattern

```perl
my $ref  = { key => 'value' };    # hash reference
sub foo  { print "hi" }           # block (sub body)
{ print "bare block"; }           # bare block statement
map { $_ * 2 } @list;             # block (builtin argument)
+{ key => 'value' }               # hash ref (disambiguated by +)
```

### Why Tree-Sitter Fails

Tree-sitter uses GLR (Generalized LR) parsing, which handles ambiguity by maintaining multiple parse stacks in parallel. When the parser sees `{`, it forks: one stack treats it as a hash constructor, another as a block start. When one stack encounters an error, it is discarded.

For Perl's `{}` ambiguity, GLR works in simple cases. But it fails on:

- `{ $x => $y }` — is this a hash with key `$x` and value `$y`, or a block containing the expression `$x => $y`? Both parses are valid Perl. The fork never resolves until later context makes one parse impossible.
- `map { ... } @list` — the grammar must encode knowledge that `map` takes a block, not a hash. This is **semantic information** (function signatures), not syntax.
- `eval { ... }` — the grammar must know that `eval` takes a block, but also that `eval "string"` takes a string expression.

GLR's parallel stacks multiply memory usage and parsing time. Each ambiguous `{` doubles the number of active stacks. Nested ambiguities (`{ { { ... } } }`) create exponential growth.

### Impact on Tree-Sitter Performance

In the tree-sitter grammar, `{}` disambiguation caused:
- Conflict count in the grammar: 15+ (tree-sitter reports these during generation)
- Parsing time: 10-100x slower on deeply nested Perl compared to equivalent Python/JavaScript
- Memory usage: proportional to nesting depth due to GLR stack multiplication

---

## 4. Quote-Like Delimiters

### The Pattern

```perl
q{hello}            # single-quoted string with { } delimiters
qq(hello $world)    # double-quoted string with ( ) delimiters
qw[foo bar baz]     # word list with [ ] delimiters
qr/pattern/flags    # regex with / / delimiters
s|old|new|g         # substitution with | | delimiters
tr{a-z}{A-Z}        # transliteration with paired { } delimiters
```

### Why Tree-Sitter Fails

Perl's quote-like operators (`q`, `qq`, `qw`, `qr`, `s`, `tr`, `y`, `m`) accept **arbitrary delimiter pairs**. Any non-whitespace character can be a delimiter, and paired delimiters (`()`, `[]`, `{}`, `<>`) nest correctly.

This means the lexer must:
1. Recognize the quote operator (`s`, `tr`, etc.)
2. Read the next character as the opening delimiter
3. Find the matching closing delimiter (which depends on what the opening delimiter was)
4. For `s///` and `tr///`, repeat for the second pair
5. Handle nested delimiters: `s{foo{bar}}{baz}` is valid (nested `{` inside the pattern)

Tree-sitter's lexer (external scanner) cannot express this. The scanner would need to:
- Track which quote operator was seen (to know how many delimiter pairs to expect)
- Dynamically select the closing delimiter based on the opening character
- Count nesting for paired delimiters
- Handle escape sequences within the delimited content

The scanner can do some of this with manual code, but each combination of operator + delimiter + nesting level must be handled explicitly.

### Lines of scanner.c Spent

Quote-like operator handling consumed approximately 250 lines of `scanner.c`, covering common cases. Exotic delimiters (e.g., `s XoldXnewXg` using `X` as delimiter) were not handled.

### How v3 Solves It

The v3 lexer has a `QuoteContext` type that dynamically tracks the opening/closing delimiter pair and nesting depth. A single function handles all quote-like operators by parameterizing on the delimiter character. The implementation lives in `crates/perl-lexer/` with the `ExpectDelimiter` lexer mode.

---

## 5. Special Variables

### The Pattern

```perl
local $/ = undef;     # input record separator ($/ looks like $ + division)
my $pid = $$;          # process ID ($$ looks like scalar deref)
print $!;              # errno ($! looks like $ + not)
$_ = "hello";          # default variable
$^W = 1;               # warnings flag ($^ + W)
${^MATCH}              # named capture group
```

### Why Tree-Sitter Fails

Perl has dozens of special variables that look like syntax errors to a naive lexer:
- `$/` looks like a variable `$` followed by the division operator
- `$$` looks like a scalar dereference
- `$^W` looks like `$` followed by the XOR operator
- `${^MATCH}` looks like a complex dereference expression

The tree-sitter scanner must have a complete catalog of special variable forms to avoid splitting them into separate tokens. This catalog is large (~50 forms) and interacts with every other lexing rule:
- The `/` disambiguation must not trigger on `$/`
- The `{` disambiguation must not trigger on `${^...}`
- The `$` sigil recognition must check for multi-character special variables before emitting a plain `$`

### Impact

Missing even one special variable form causes cascading parse errors. `local $/ = undef;` misparsed as `local $ / = undef;` means the lexer produces a division operator token, the parser sees division in local-declaration context, and the rest of the statement fails.

### Lines of scanner.c Spent

Special variable recognition consumed approximately 100 lines of `scanner.c`, and the coverage was still incomplete — exotic forms like `$:`, `$;`, `$"` were partially handled at best.

---

## 6. Indirect Object Syntax

### The Pattern

```perl
new Foo('arg');         # Foo->new('arg')
print STDERR "error";  # STDERR is filehandle, not argument
close $fh;             # $fh->close() in some contexts
```

### Why Tree-Sitter Fails

`new Foo()` is syntactically identical to calling a function `new` with argument `Foo()`. The tree-sitter parser cannot distinguish between:
- `new Foo()` — indirect object syntax (method call)
- `new_function($arg)` — regular function call
- `print STDERR "msg"` — `STDERR` is a filehandle, not an argument

The distinction requires semantic knowledge:
- `new` is special — it implies indirect object syntax
- `STDERR` is a known filehandle
- `print`, `say`, `printf`, `close`, `open` have special argument handling

Tree-sitter grammars cannot encode function-specific parsing rules. The grammar sees `BAREWORD BAREWORD ARGS` and cannot decide which parse is correct without knowing what the first bareword means.

### Attempted Workaround

The grammar used precedence annotations to prefer function-call interpretation over indirect-object interpretation. This was wrong for `new Foo()` (should be indirect) but correct for most other cases. No single precedence ordering works for all cases.

---

## 7. Format Statements

### The Pattern

```perl
format STDOUT =
Name:    @<<<<<<<<<
$name
Address: @>>>>>>>>>>
$address
.
```

### Why Tree-Sitter Fails

A `format` declaration contains a completely different mini-language. The body between `=` and the terminating `.` uses format-specific syntax (`@<<<`, `@>>>`, `@|||`) that is not valid Perl. The parser must:

1. Recognize `format IDENTIFIER =` as switching to format mode
2. Consume all text verbatim until a line containing only `.`
3. Switch back to normal parsing

Tree-sitter's lexer can handle this with the external scanner, but it requires yet another modal state in `scanner.c`. The format body cannot be expressed in the grammar because its syntax is not Perl syntax — it's a DSL embedded within Perl.

### Lines of scanner.c Spent

Format handling consumed approximately 75 lines, and was one of the simpler cases because the format body is consumed verbatim.

---

## The scanner.c Problem

### Growth

The tree-sitter Perl grammar's `scanner.c` grew to approximately **975 lines** of hand-written C code. For comparison:
- tree-sitter-javascript's scanner: ~200 lines
- tree-sitter-python's scanner: ~300 lines
- tree-sitter-rust's scanner: ~150 lines

The Perl scanner is 3-5x larger than scanners for languages that tree-sitter handles well. This reflects Perl's inherent context sensitivity: every disambiguation rule that cannot be expressed in the grammar must be hand-coded in the scanner.

### Brittleness

Each rule in the scanner interacts with every other rule:
- The `/` disambiguation checks conflict with special variable recognition
- The heredoc queue interacts with the `{}` disambiguation
- Quote-like delimiter handling interacts with the `/` disambiguation (is `s/.../.../` a substitution or two divisions?)
- Format mode interacts with statement boundary detection

Adding a new rule requires understanding all existing rules and their interactions. The scanner became a **stateful spaghetti** that was correct for the tested cases but fragile to changes.

### State Management

The scanner maintained multiple pieces of state:
- `last_token_type`: for `/` disambiguation
- `pending_heredocs[]`: queue of heredoc declarations
- `heredoc_count`: number of pending heredocs
- `in_format`: boolean for format body mode
- `quote_delimiter`: current quote operator's delimiter
- `nesting_depth`: for paired delimiter tracking

This state had to be serialized and deserialized for tree-sitter's incremental parsing (the `serialize()` and `deserialize()` functions). Bugs in serialization caused incorrect incremental re-parses, producing different results depending on whether the file was parsed from scratch or incrementally.

---

## The Key Insight: Mode-Based Lexer

The fundamental problem with tree-sitter for Perl is the **separation of lexer and parser**. Tree-sitter's architecture requires the lexer (external scanner) to produce tokens independently of the parser's state. Perl requires the lexer to know what the parser is doing.

The v3 parser's key insight is the `LexerMode` state machine:

```rust
pub enum LexerMode {
    ExpectTerm,       // "/" starts a regex pattern
    ExpectOperator,   // "/" is division
    ExpectDelimiter,  // For quote-like operators
    InFormatBody,     // Format declarations
    InDataSection,    // __DATA__ / __END__
}
```

The parser tells the lexer what mode to be in. When the parser finishes parsing a term, it sets the mode to `ExpectOperator`. When it finishes parsing an operator, it sets the mode to `ExpectTerm`. The lexer uses this mode to disambiguate `/`, `{}`, and other context-sensitive tokens.

This is impossible in tree-sitter's architecture. The scanner cannot query the parse stack, and the grammar cannot pass information to the scanner. The mode-based lexer is the fundamental reason the recursive descent parser succeeds where tree-sitter fails.

---

## Summary

| Pattern | Tree-Sitter Limitation | v3 Solution |
|---------|----------------------|-------------|
| `/` ambiguity | Scanner can't query parse stack | `LexerMode` state machine |
| Heredoc body location | Scanner can't defer token production | `PendingHeredoc` FIFO queue |
| `{}` ambiguity | GLR exponential branching | Context-aware `parse_hash_or_block_inner()` |
| Quote-like delimiters | Dynamic delimiter pairs need runtime state | `QuoteContext` with `ExpectDelimiter` mode |
| Special variables | Incomplete catalog causes cascading errors | Exhaustive variable pattern recognition in lexer |
| Indirect objects | Grammar can't encode function-specific rules | Curated builtin list + `is_indirect_call_pattern()` |
| Format statements | Another scanner modal state | `InFormatBody` lexer mode |

The lesson: **tree-sitter works when the lexer can be context-free. Perl's lexer is context-sensitive by design.** A hand-written recursive descent parser with a mode-based lexer is the only architecture that correctly handles Perl's grammar.
