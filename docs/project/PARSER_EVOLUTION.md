# Parser Evolution: From Tree-sitter to Native Recursive Descent

> A technical deep dive into perl-lsp's parser journey -- three generations
> of Perl parsing, the trade-offs at each stage, and the architecture
> that emerged.

---

## The Problem: Parsing Perl

Perl is often called "the only language where the parser cannot be separated
from the compiler." This reputation is well-earned. Unlike most languages,
Perl's grammar is **context-sensitive** at the lexer level:

- **Slash ambiguity**: `/` can be division (`$x / 2`) or the start of a regex
  (`/pattern/`). The correct interpretation depends on what came *before* it.
- **Sigil overloading**: `%` is both the modulo operator and the hash sigil.
  `*` is both multiplication and a typeglob.
- **Heredocs**: A `<<EOF` marker on one line begins a string literal whose
  body appears on *subsequent* lines, while the rest of the original line
  continues to be parsed as normal code. Multiple heredocs can stack.
- **Quote-like operators**: `q{}`, `qq{}`, `qw()`, `qr//`, `s///`, `tr///`,
  and `y///` each use arbitrary delimiters (paired or non-paired) with nested
  delimiter support.
- **Indirect object syntax**: `new Class` vs `Class->new()` -- the parser
  must decide whether a bareword after a method-like word is an indirect
  object call.
- **Format declarations**: `format STDOUT =` introduces a completely
  different sub-language terminated by a lone `.` on a line.
- **Context-dependent keywords**: Words like `print`, `say`, `die`, and
  `warn` accept arguments without parentheses, consuming tokens until a
  statement boundary -- but the definition of "statement boundary" depends
  on operator precedence.

These properties mean that a naive context-free grammar (BNF, EBNF, PEG)
cannot correctly parse Perl without significant workarounds. Every parser
generation in this project wrestled with this fundamental tension.

---

## Phase 1: Tree-sitter (July 2022 -- Mid 2025)

### Origins

The project began in July 2022 as `tree-sitter-perl` -- a tree-sitter
grammar for Perl written in JavaScript (`grammar.js`, ~1,300 lines). The
initial commits tell the story:

```
a05f4820  start tapping out grammar; statement + declaration
36040d43  get a super simple basic comment parser up
998412dc  implement simple numeric parsing
7ae93702  Follow perly.y; also implement a lot more things
```

Tree-sitter was a natural first choice. It provides:

- Incremental parsing out of the box
- Editor integration via the tree-sitter ecosystem (Neovim, Emacs, Helix)
- A well-understood GLR parsing algorithm
- Precedence-based conflict resolution

### Architecture

The tree-sitter grammar lived in `tree-sitter-perl/grammar.js` and followed
Perl's own `perly.y` precedence table:

```javascript
const TERMPREC = {
  LOW: 0, LOOPEX: 1, OROP: 2, ANDOP: 3, LSTOP: 4,
  COMMA: 5, ASSIGNOP: 6, QUESTION_MARK: 7, DOTDOT: 8,
  OROR: 9, ANDAND: 10, /* ... */ ARROW: 24, PAREN: 25,
};
```

A companion C scanner (`scanner.c`) handled context-sensitive constructs
that tree-sitter's GLR parser could not express declaratively -- particularly
string literals, heredocs, and regex delimiters.

### What Worked

- Rapid prototyping of Perl syntax coverage
- Built-in incremental parsing for editor integration
- Community familiarity with tree-sitter tooling
- S-expression output format (which the project still uses for test assertions)

### What Didn't

Tree-sitter grammars are powerful but ultimately constrained:

1. **Scanner complexity explosion**: Perl's context-sensitive lexing pushed
   more and more logic into `scanner.c`, which became a maintenance burden.
   The scanner had to track state across tokens in ways that tree-sitter's
   model was not designed for.

2. **C dependency**: The scanner required a C compiler (`libclang`), which
   complicated cross-platform builds and CI. The `tree-sitter-perl-c/`
   directory is still excluded from the default workspace for this reason.

3. **Limited error recovery control**: Tree-sitter's built-in error recovery
   was not flexible enough for IDE-quality diagnostics. The parser needed to
   produce *partial ASTs with specific error nodes*, not just mark subtrees
   as ERROR.

4. **Abstraction mismatch**: Tree-sitter produces a *concrete* syntax tree
   (CST). Building LSP features requires an *abstract* syntax tree (AST)
   with semantic annotations. The CST-to-AST transformation layer added
   complexity and performance overhead.

---

## Phase 2: Pest Grammar (July 2025)

### The PEG Approach

On July 16, 2025, a second parser was introduced using
[Pest](https://pest.rs/), a PEG (Parsing Expression Grammar) parser
generator for Rust. The Pest grammar (`crates/perl-parser-pest/src/grammar.pest`,
~1,000 lines) represented a significant step: **pure Rust, no C dependency**.

The Pest parser used a three-stage pipeline:

1. **Pest Parsing** -- The PEG grammar produces a parse tree
2. **AST Building** -- `build_ast()` / `build_node()` construct a typed
   `AstNode` tree, using a Pratt parser for operator expressions
3. **S-Expression Output** -- `SexpFormatter` generates tree-sitter-compatible
   strings for test comparison

### Grammar Coverage

The Pest grammar was remarkably comprehensive. It handled:

- All statement types (if/unless/while/until/for/foreach/given/when)
- Subroutine declarations with signatures, prototypes, and attributes
- Modern Perl features (try/catch/finally, defer, class/method/field/role)
- All quote-like operators (q, qq, qw, qr, qx) with nested delimiter support
- Regular expressions with all delimiter types and flags
- Substitution (`s///`) and transliteration (`tr///`, `y///`)
- Heredocs (declaration only -- body handled by preprocessor)
- String interpolation (scalar, array, complex `${}` and `@{[]}` forms)
- Format declarations (parsed atomically)
- POD sections, `__DATA__`, `__END__`
- Phase blocks (BEGIN, END, CHECK, INIT, UNITCHECK)
- Operator precedence covering all of Perl's ~50 operators
- Builtin list operators (print, say, push, pop, etc.) with argument parsing

### Performance Characteristics

From the project's own benchmarks (`PURE_RUST_PERFORMANCE_ANALYSIS.md`):

| Metric | Value |
|--------|-------|
| Parse throughput | ~180-200 us/KB |
| Small files (<5KB) | ~1ms (startup-dominated) |
| Time complexity | O(n) linear |
| Thread safety | Full (no shared mutable state) |
| Memory safety | Guaranteed by Rust |

### Limitations That Drove the Next Evolution

Despite its comprehensive coverage, the Pest parser hit fundamental limits:

1. **Slash disambiguation was a preprocessor hack**: Because PEG grammars
   are context-free, the Pest parser could not natively distinguish `/` as
   division vs regex. It relied on preprocessor markers (`_SUB_`, `_TRANS_`,
   `_QR_`, `_DIV_`) injected before parsing -- a fragile workaround that
   broke on edge cases.

2. **Heredoc handling was external**: The `heredoc_placeholder` rule
   (`__HEREDOC_N__`) shows that heredoc bodies were collected by a separate
   scanner pass, not by the grammar itself. This two-pass architecture
   added complexity and made error recovery harder.

3. **PEG backtracking overhead**: Pest uses ordered choice (`|`) which
   means earlier alternatives are tried first and backtracking occurs on
   failure. For Perl's deeply ambiguous syntax, this caused performance
   cliffs on pathological inputs. The `stacker` crate was needed for stack
   overflow protection during deep recursion.

4. **Error recovery was poor**: PEG parsers fail fast on the first
   unrecognized input. There is no built-in mechanism for producing partial
   parse trees with error nodes -- a hard requirement for IDE integration.

5. **Operator precedence was a separate layer**: The `PrattParser` module
   (~14,500 lines in `pratt_parser.rs`) implemented operator precedence
   outside the grammar, adding another layer of complexity.

6. **Performance gap vs hand-written parsers**: At ~200us/KB, the Pest
   parser was estimated to be 5-10x slower than hand-written C or Rust
   parsers. For large workspace indexing (50GB+ codebases), this gap
   mattered.

The Pest parser remains in the codebase as `perl-parser-pest` (Tier 7,
not in the default CI gate), serving as a learning tool, compatibility
reference, and benchmark baseline.

---

## Phase 3: Native Recursive Descent (July 2025 -- Present)

### The Breakthrough

On July 21, 2025 -- just five days after the Pest parser was introduced --
the commit `b675ad31` landed: "Implement a modern two-crate architecture
for Perl parsing." This was the beginning of the v3 parser, a hand-written
recursive descent parser in pure Rust.

The native parser was not a rewrite of the Pest grammar. It was a ground-up
redesign that solved the fundamental problems:

- **Context-sensitive lexing** is handled by the lexer itself, not by
  preprocessor hacks
- **Heredocs** are collected inline during parsing via a FIFO queue
- **Error recovery** produces partial ASTs with typed error nodes
- **Operator precedence** is built into the recursive descent structure
- **Performance** targets sub-millisecond for typical files

### Architecture

The v3 parser is split across a family of focused microcrates:

```
perl-lexer          Context-aware tokenizer (Tier 1, ~3,100 lines)
perl-token          Token type definitions (Tier 1)
perl-ast            AST node definitions (Tier 1, ~2,600 lines)
perl-parser-core    Recursive descent parser engine (Tier 2, ~10,500 lines)
perl-quote          Quote-like operator parsing (Tier 1, ~700 lines)
perl-regex          Regex literal parsing (Tier 1, ~300 lines)
perl-heredoc        Heredoc collection with indent stripping (Tier 1, ~200 lines)
perl-error          Error types and recovery strategies (Tier 1)
perl-parser         Composition crate re-exporting all of the above (Tier 6)
```

This architecture follows the **Single Responsibility Principle** (SRP)
rigorously. Each crate has one job and can be tested, benchmarked, and
evolved independently.

#### The Lexer: Context-Aware Tokenization

The lexer (`perl-lexer`) is the key innovation. It uses a **mode-based
state machine** to resolve Perl's context-sensitive ambiguities at tokenization
time:

```rust
pub enum LexerMode {
    ExpectTerm,       // slash starts regex, % starts hash
    ExpectOperator,   // slash is division, % is modulo
    ExpectDelimiter,  // # is not a comment (inside s///)
    InFormatBody,     // consume until lone dot
    InDataSection,    // consume everything to EOF
}
```

Mode transitions follow simple heuristics based on the previous token:

| Previous Token | Next Mode | Rationale |
|---------------|-----------|-----------|
| Identifier, number, `)`, `]` | ExpectOperator | A term just ended |
| Keyword, operator, `(`, `[` | ExpectTerm | A new term is expected |
| `s`, `tr`, `y` | ExpectDelimiter | Quote operator follows |

This two-mode approach resolves the slash ambiguity that plagued both
tree-sitter (requiring a C scanner) and Pest (requiring preprocessor
markers). The lexer also enforces **budget limits** to prevent
denial-of-service on pathological input:

- `MAX_REGEX_BYTES`: 64 KB per regex literal
- `MAX_HEREDOC_BYTES`: 256 KB per heredoc
- `MAX_DELIM_NEST`: 128 levels of delimiter nesting
- `HEREDOC_TIMEOUT_MS`: 5-second timeout

The lexer supports **checkpointing** for incremental parsing, allowing the
parser to save and restore lexer state for efficient re-parsing.

#### The Parser: Recursive Descent with Include Composition

The parser (`perl-parser-core`) uses Rust's `include!` macro to compose
parsing logic from focused files while keeping everything in a single
`impl Parser<'a>` block:

```rust
include!("helpers.rs");          // Token consumption and position tracking
include!("heredoc.rs");          // Heredoc collection with FIFO queue
include!("statements.rs");       // Statement-level parsing
include!("variables.rs");        // Variable and sigil parsing
include!("control_flow.rs");     // if/while/for/foreach/given/when
include!("declarations.rs");     // sub/my/our/package/class
include!("expressions/mod.rs");
include!("expressions/precedence.rs");  // Operator precedence climbing
include!("expressions/unary.rs");
include!("expressions/postfix.rs");     // Method calls, array/hash access
include!("expressions/primary.rs");     // Literals, variables, parens
include!("expressions/calls.rs");       // Function and method calls
include!("expressions/hashes.rs");      // Hash vs block disambiguation
include!("expressions/quotes.rs");      // Quote-like operators
```

The recursion depth is capped at 128 levels, providing stack overflow
protection. Real Perl code rarely exceeds 20-30 nesting levels.

#### The AST: 67 Node Kinds

The AST (`perl-ast`) defines 67 `NodeKind` variants covering the full
Perl syntax:

**Declarations**: `VariableDeclaration`, `VariableListDeclaration`,
`Subroutine`, `Method`, `Package`, `Class`, `Format`

**Control flow**: `If`, `While`, `For`, `Foreach`, `Given`, `When`,
`Default`, `StatementModifier`, `LabeledStatement`, `LoopControl`

**Expressions**: `Binary`, `Unary`, `Ternary`, `Assignment`, `FunctionCall`,
`MethodCall`, `IndirectCall`

**Literals**: `Number`, `String`, `Heredoc`, `ArrayLiteral`, `HashLiteral`,
`Regex`, `Match`, `Substitution`, `Transliteration`

**Variables**: `Variable`, `VariableWithAttributes`, `Typeglob`

**Modules**: `Use`, `No`, `PhaseBlock`, `DataSection`

**Error recovery**: `Error`, `MissingExpression`, `MissingStatement`,
`MissingIdentifier`, `MissingBlock`, `UnknownRest`

Every node carries a `SourceLocation { start, end }` for byte-precise
position tracking. The AST produces tree-sitter-compatible S-expressions
via `to_sexp()`, maintaining test compatibility with earlier parser
generations.

#### Heredoc Handling

Heredocs are one of Perl's most challenging parsing constructs. The v3
parser handles them with a **FIFO queue** approach:

1. When the parser encounters `<<EOF`, it records a `PendingHeredoc` in a
   `VecDeque` with the delimiter, quoting style, and indentation mode.
2. After the current line is fully parsed, the parser switches to heredoc
   collection mode, consuming lines until the terminator is found.
3. Multiple heredocs on the same line (e.g., `foo(<<A, <<B)`) are queued
   and collected in order.
4. Indented heredocs (`<<~EOF`) have leading whitespace stripped according
   to Perl 5.26+ rules.

This inline approach eliminates the two-pass architecture that both
tree-sitter and Pest required.

#### Error Recovery: IDE-Friendly by Design

The parser uses an **IDE-friendly error recovery model**:

- `parse()` returns `Ok(ast)` with ERROR nodes for most failures
- `Err` is reserved for catastrophic conditions (recursion limit exceeded)
- `parse_with_recovery()` always returns a `ParseOutput` with both the
  AST and diagnostics

Synchronization points for error recovery:

- **Semicolons** (`;`) -- statement boundaries
- **Closing braces** (`}`) -- block boundaries
- **Keywords** (`my`, `if`, `sub`, etc.) -- statement starts
- **End of file**

Recovery strategies:

- **Skip and recover**: Skip tokens until a synchronization point
- **Insert missing**: Create `MissingExpression`, `MissingBlock`, etc. nodes
- **Partial parsing**: Continue parsing even with unclosed delimiters

This design enables code completion, go-to-definition, and hover
information to work *even while the user is typing incomplete code*.

---

## Incremental Parsing

The `perl-incremental-parsing` crate (~6,000 lines) provides efficient
re-parsing for real-time editing:

```rust
let mut state = IncrementalState::new("my $x = 1;");
let ast = state.parse()?;

let edit = Edit { start_byte: 3, old_end_byte: 5, new_end_byte: 5, text: "$y" };
apply_edits(&mut state, vec![edit]);

let new_ast = state.parse()?;  // reuses unchanged nodes
```

Key components:

- **`incremental_v2`** (~2,300 lines): Second-generation incremental parsing
  with improved node reuse
- **`incremental_advanced_reuse`** (~940 lines): Strategies for maximizing
  AST node reuse across edits
- **`incremental_checkpoint`** (~375 lines): Checkpoint-based parsing with
  rollback support
- **`incremental_document`** (~1,050 lines): Document-level state management

Performance from the project's current status: **931ns incremental updates**.

---

## Quality Assurance

### Test Corpus

The parser is validated against a comprehensive test corpus:

- **`tree-sitter-perl/test/corpus/`**: 32 test files with ~23,000 lines
  covering expressions, statements, functions, heredocs, interpolation,
  regex, operators, variables, subroutines, object-oriented features,
  modern Perl (5.36+), format declarations, signal handling, tie
  interface, typeglobs, and edge cases.

- **Per-construct test suites**: The parser engine includes dedicated test
  modules for challenging constructs: `slash_ambiguity_tests`,
  `hash_vs_block_tests`, `heredoc_security_tests`, `indirect_object_tests`,
  `format_comprehensive_tests`, `regex_delimiter_tests`, `glob_tests`,
  `tie_tests`, and `loop_control_tests`.

### Fuzz Testing

Eight fuzz targets exercise the most fragile parsing paths:

- `parser_integration` -- parser, trivia, and symbol-extraction integration
- `quote_operators` -- regex, substitution, and transliteration delimiter handling
- `heredoc_parsing` -- heredoc edge cases
- `substitution_parsing` -- s/// variants
- `builtin_functions` -- list operator argument parsing
- `unicode_positions` -- UTF-8/UTF-16 position mapping
- `lsp_navigation` -- LSP feature integration
- `lsp_cancellation_registry` -- concurrent request handling

Bounded fuzzing runs for 60 seconds per target as part of the nightly CI.

### Mutation Testing

The project tracks mutation testing scores, currently at **87%**. The
mutation testing subset runs as part of the nightly CI tier (`just ci-full`).

### Property-Based Testing

`proptest` is used for property-based testing of the parser, ensuring that
randomly generated Perl-like inputs do not cause crashes or infinite loops.

### CI Gate Tiers

| Tier | Command | Time | Scope |
|------|---------|------|-------|
| A (PR-fast) | `just pr-fast` | ~1-2 min | Format, clippy, fast tests |
| B (Merge gate) | `just ci-gate` | ~3-5 min | All lib tests, policy checks |
| C (Nightly) | `just ci-full` | ~15-30 min | Mutation testing, fuzzing, benchmarks |

---

## Performance Results

### Current v3 Parser

From `docs/project/CURRENT_STATUS.md`:

| Metric | Value |
|--------|-------|
| Parse time (typical files) | 1-150 us |
| Incremental update | 931 ns |
| Workspace index (small) | ~369 us |
| Workspace index (medium) | ~721 us |
| Incremental index update | ~213 us |

### Comparative Performance

| Parser | Throughput | Context-Sensitive | Error Recovery | Dependencies |
|--------|-----------|-------------------|----------------|-------------|
| v1 (tree-sitter/C) | ~20-50 us/KB | Via C scanner | Basic | C compiler |
| v2 (Pest) | ~180-200 us/KB | Via preprocessor | None | Pure Rust |
| v3 (native) | ~1-150 us/file | Native mode-based | Full AST recovery | Pure Rust |

The v3 parser's performance is not directly comparable by throughput
because it includes context-sensitive lexing and error recovery in
its pipeline -- work that the other parsers either skip or handle
externally.

---

## Lessons Learned

### 1. Context sensitivity must live in the lexer

Both tree-sitter and Pest tried to handle Perl's context sensitivity
*outside* the grammar: tree-sitter via a C scanner, Pest via preprocessor
markers. Both approaches were fragile. The v3 parser's mode-based lexer
solves the problem at the right level of abstraction.

### 2. PEG grammars are excellent prototyping tools

The Pest grammar was written in a matter of days and achieved comprehensive
Perl syntax coverage. It validated the approach of pure-Rust parsing and
provided a reference implementation that the native parser could be tested
against. The Pest parser remains valuable as a "second opinion" for
ambiguous cases.

### 3. Error recovery is not an add-on

Tree-sitter's error recovery was insufficient. Pest had none. The v3
parser was designed from the ground up with IDE-friendly error recovery,
and this design decision shapes every aspect of the architecture: the
`MissingExpression`/`MissingBlock` node kinds, the synchronization point
strategy, the distinction between recoverable and catastrophic errors.

### 4. Microcrate architecture enables velocity

Splitting the parser into 12+ focused crates (lexer, token, AST, quote,
regex, heredoc, error, parser-core, etc.) enables:
- Independent testing and benchmarking
- Clear API boundaries
- Parallel compilation
- Focused code review

### 5. Heredocs require special architecture

Every parser generation had to develop a special strategy for heredocs.
The v3 FIFO queue approach -- collecting heredoc bodies inline after
parsing each line -- is the cleanest solution, but it required designing
the parser around this constraint from the start.

### 6. Keep the old parsers around

The Pest parser (v2) and tree-sitter grammar (v1) remain in the
repository. They serve as:
- Benchmark baselines for performance regression testing
- Reference implementations for ambiguous syntax decisions
- Learning tools for contributors new to parser development
- Compatibility layers for editor integrations that expect tree-sitter

### 7. Perl coverage is achievable

Despite Perl's reputation as "unparseable," the v3 parser achieves
~100% coverage of Perl 5.8 through 5.40 syntax. The key insight is
that Perl's context sensitivity is *bounded* -- there are a finite
number of disambiguation rules, and they can be encoded in a mode-based
lexer with well-defined transitions.

---

## Current State

As of v0.10.0, the parser stack is:

- **Production**: v3 native recursive descent (`perl-parser-core`)
- **Legacy reference**: v2 Pest grammar (`perl-parser-pest`)
- **Archived**: v1 tree-sitter (`tree-sitter-perl/`, `tree-sitter-perl-c/`)

The v3 parser powers all LSP features (100% coverage of 53 advertised
capabilities), semantic analysis (100% NodeKind handler coverage),
workspace indexing, code actions, and the Debug Adapter Protocol bridge.

The parser handles 1,543 lib tests with 87% mutation score, sub-millisecond
parsing, and sub-microsecond incremental updates -- making it suitable for
real-time IDE integration with large Perl codebases.
