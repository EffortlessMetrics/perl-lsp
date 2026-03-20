# CPAN Corpus Roadmap: Path to 100% Coverage

*How perl-lsp plans to parse every CPAN module that can be parsed statically.*

---

## Current State (March 19, 2026)

| Metric | Value |
|--------|-------|
| Total corpus files | 4,355 |
| Cleanly parsed (ratcheted) | 3,484 |
| Clean rate | 80.0% |
| Estimated actual (post-merge fixes) | ~85%+ |
| Error nodes total | 6,817 |
| Error buckets tracked | 15+ |

The corpus consists of 4,355 Perl modules from CPAN's top-1000 distributions. Each file is parsed by the v3 recursive descent parser, and the first error (if any) is categorized into a semantic bucket. The baseline is tracked in `.ci/cpan-corpus-baseline.json` and is ratcheted in CI — the clean count can only increase.

---

## Error Bucket Breakdown

The 871 files that currently fail to parse cleanly (after ratchet) are distributed across error buckets:

| Rank | Bucket | Files | Root Cause Summary |
|------|--------|-------|--------------------|
| 1 | `unexpected_token_in_expr` | 146 | 10 distinct subcategories (see below) |
| 2 | `unclosed_paren_identifier` | 140 | Qualified class names, complex call args |
| 3 | `unexpected_question_expr` | 109 | Ternary operator edge cases |
| 4 | `unclosed_paren` | 106 | Missing close paren in complex nesting |
| 5 | `unexpected_rbrace_expr` | 83-114 | Under investigation (#2189) |
| 6 | `unexpected_comma_expr` | 70 | Trailing commas, complex list contexts |
| 7 | `expected_left_brace` | 66 | Block-list function calls |
| 8 | `expected_variable` | 66 | REGRESSION from field decl merge |
| 9 | `unexpected_fat_arrow_expr` | 66 | Fat arrow in unexpected contexts |
| 10 | `expected_comma_or_close_paren` | 55 | Complex argument lists |
| 11 | `unclosed_bracket` | 38 | Array ref nesting |
| 12 | `unclosed_brace_semicolon` | 32 | Hash/block ambiguity |
| 13 | `unclosed_brace` | 32 | Brace nesting |
| 14 | `expected_identifier` | 30 | Bareword handling |
| 15 | `expected_colon` | 26 | Ternary/label disambiguation |

### unexpected_token_in_expr Subcategories

The largest bucket (146 files) decomposes into 10 distinct parsing gaps:

| # | Subcategory | Est. Files | Difficulty | Status |
|---|-------------|-----------|------------|--------|
| 1 | Keywords as bareword identifiers | 25-40 | EASY | Issue pending |
| 2 | Postfix operators + statement modifiers | 20-30 | MEDIUM | Issue pending |
| 3 | Complex paren/brace nesting | 15-25 | MEDIUM-HIGH | Issue pending |
| 4 | Built-in function edge cases | 15-20 | MEDIUM | Issue pending |
| 5 | Method call chains | 10-15 | MEDIUM | Issue pending |
| 6 | Dereference chains | 10-15 | MEDIUM | Partially fixed |
| 7 | Special variables in expressions | 8-12 | EASY-MEDIUM | Partially fixed |
| 8 | Regex binding with complex LHS | 5-10 | MEDIUM | Issue pending |
| 9 | Perl 5.20+ signatures | 5-10 | LOW priority | Known gap |
| 10 | Invalid Perl / source filters | 3-8 | UNFIXABLE | Accepted |

---

## Phase A: 80% to 90% (Target: 5 builders, 4-5 weeks)

### Strategy

Fix the top 5 error buckets, each with a dedicated builder agent. Each builder receives a scout-produced spec with exact function names, line numbers, and CPAN file samples.

### Work Items

| Item | Bucket | Files Affected | Builder Spec |
|------|--------|---------------|-------------|
| A.1 | Keywords as barewords | 25-40 | Allow `format`, `local`, `state`, `given`, `when` as barewords in hash key position |
| A.2 | `unclosed_paren_identifier` | 140 | Qualified class name parsing (`Foo::Bar::Baz->new()`) |
| A.3 | `unexpected_question_expr` | 109 | Ternary operator in nested contexts |
| A.4 | `expected_variable` regression | 66 | Narrow `field` keyword parsing to class context only |
| A.5 | `unexpected_fat_arrow_expr` | 66 | Fat arrow after complex expressions |

### Expected Outcome

- Files fixed: 350-450 (conservative estimate with overlap)
- New clean rate: ~88-92%
- Ratcheted baseline: 3,834-3,934 / 4,355

### Dependencies

- Buckets #2 and #3 may already be partially fixed on master (not yet ratcheted)
- Post-merge ratchet could reveal an even higher starting point
- Issues #2140, #2147, #2148, #2149, #2184-#2189 contain builder-ready specs

### Effort

- 5 builder agents, each working ~3-5 days
- 1 scout pass to verify bucket state before builders launch
- 1 corpus ratchet after each merge wave
- Total calendar time: 4-5 weeks (limited by CI merge throughput)

---

## Phase B: 90% to 95% (Target: 12-15 builders, 7-9 weeks)

### Strategy

Attack the remaining buckets and the harder subcategories within `unexpected_token_in_expr`. This phase requires more parser infrastructure changes (better error recovery, improved block/hash disambiguation, enhanced postfix operator handling).

### Work Items

| Item | Bucket | Files Affected | Complexity |
|------|--------|---------------|-----------|
| B.1 | Postfix operators + modifiers | 20-30 | Precedence rework |
| B.2 | Complex paren/brace nesting | 15-25 | Parser state management |
| B.3 | Built-in function edge cases | 15-20 | Special-case parsing |
| B.4 | `unclosed_paren` remaining | 40-60 | Enhanced paren recovery |
| B.5 | `unexpected_rbrace_expr` | 50-80 | Investigation required |
| B.6 | `unexpected_comma_expr` | 40-50 | List context handling |
| B.7 | Method call chains | 10-15 | Postfix operator stacking |
| B.8 | Dereference chains | 10-15 | Mixed sigil handling |
| B.9 | `expected_left_brace` remaining | 30-40 | Block-list functions |
| B.10 | `expected_comma_or_close_paren` | 30-40 | Complex arg list recovery |

### Expected Outcome

- Files fixed: 200-350 (harder problems, more overlap between buckets)
- New clean rate: ~93-96%
- Ratcheted baseline: 4,034-4,184 / 4,355

### Infrastructure Required

1. **Enhanced error recovery**: Current recovery synchronizes to next statement boundary. Need finer-grained recovery within expressions.
2. **Block/hash disambiguation improvements**: The `parse_hash_or_block_inner()` function needs more lookahead strategies.
3. **Postfix operator chain handling**: Method calls, subscripts, and dereferences need uniform stacking.

### Effort

- 12-15 builder agents across 3-4 waves
- 3-4 scout passes (one per wave)
- Multiple corpus ratchets
- Total calendar time: 7-9 weeks

---

## Phase C: 95% to 98% (Target: 20+ builders, 11-15 weeks)

### Strategy

Diminishing returns territory. Each percentage point requires fixing increasingly exotic constructs. The work shifts from "parser bugs" to "parser limitations."

### Work Items

| Item | Category | Files | Complexity |
|------|----------|-------|-----------|
| C.1 | Special variables (exotic) | 8-12 | Lexer catalog expansion |
| C.2 | Perl 5.20+ signatures | 5-10 | Feature detection heuristic |
| C.3 | Regex binding edge cases | 5-10 | LHS parsing rework |
| C.4 | `expected_identifier` remaining | 15-20 | Bareword context expansion |
| C.5 | `expected_colon` remaining | 15-20 | Ternary/label context |
| C.6 | Remaining bracket/brace nesting | 30-40 | Deep nesting handling |
| C.7 | Long-tail edge cases | 20-30 | Case-by-case |

### Expected Outcome

- Files fixed: 100-140
- New clean rate: ~97-98%
- Ratcheted baseline: 4,225-4,270 / 4,355

### Effort

- 20+ builder agents across many waves
- Significant scout investment (each fix requires root cause analysis)
- Total calendar time: 11-15 weeks
- ROI decreases significantly — each fixed file costs more in dev time

---

## The Unfixable Floor: 2-3%

Approximately 85-130 files (2-3% of corpus) will likely never parse cleanly with a static parser:

### Source Filters (~1-1.5%)

Files that use `Filter::Simple`, `Devel::Declare`, `Keyword::Simple`, or similar modules that transform source code before Perl's parser sees it. The text that perl-lsp parses is not valid Perl — it is pre-filter text that only becomes valid Perl after the filter executes.

**Examples from corpus**:
- Modules using `Method::Signatures` (rewrites `method` keyword)
- Modules using `Moose::Exporter` with custom syntax
- Modules using `Devel::Declare` to add new keywords

**Why unfixable**: Source filters can perform arbitrary text transformation, including translating between languages. A static parser sees the input text, not the output.

### BEGIN Block Side Effects (~0.5-1%)

Files where `BEGIN { }` blocks change parsing behavior for the rest of the file:
- `use constant` defining identifiers used as barewords
- `use feature 'signatures'` toggling prototype vs. signature parsing
- `use overload` changing operator interpretation

**Why unfixable**: Static analysis cannot execute `BEGIN` blocks. The parser would need to be an interpreter.

### Generated/Malformed Code (~0.3-0.5%)

Files that are:
- Auto-generated with unusual syntax
- XS modules with embedded C code fragments
- Intentionally non-standard for testing purposes

### Pragmatic Acceptance

The 2-3% floor is acceptable for an LSP. These files will produce parse errors, but:
- Error recovery ensures the rest of the file still gets IDE features
- Diagnostic messages clearly indicate what failed
- Users can suppress diagnostics for known-unfixable files via configuration

---

## Corpus Expansion Priorities

Beyond improving the clean rate on existing corpus files, the corpus itself should grow to cover more of the Perl ecosystem:

### Priority 1: Moose/Moo DSL Modules (High Impact)

Moose and Moo are the dominant OOP frameworks in modern Perl. Their DSL (`has`, `extends`, `with`, `before`, `after`, `around`, `augment`, `override`) is used by thousands of CPAN modules. Adding Moose/Moo-heavy modules to the corpus tests the parser's handling of:
- Bareword attribute names after `has`
- Block arguments to method modifiers
- `role` declarations (Moose::Role)
- Type constraint expressions

### Priority 2: v5.38 Class Syntax

Perl 5.38 introduced native `class` syntax:
```perl
class Foo {
    field $x :param;
    field $y = 0;
    method greet () { say "Hello from $x" }
}
```

Adding v5.38+ modules to the corpus is critical for forward compatibility.

### Priority 3: DBIx::Class and Catalyst

DBIx::Class (ORM) and Catalyst (web framework) represent some of the most complex real-world Perl code. They use:
- Deeply nested hash structures
- Complex method chains
- `__PACKAGE__->` class method calls
- Schema definitions with unusual syntax

### Priority 4: Dist::Zilla and Module::Build

Build system modules exercise parser edge cases:
- Dynamic module generation
- Complex `BEGIN` blocks
- Eval'd code strings
- Configuration DSLs

---

## Ratcheting Protocol

The corpus uses a strict ratcheting mechanism:

1. **Baseline**: `.ci/cpan-corpus-baseline.json` records which files parse cleanly
2. **CI check**: `just cpan-corpus-check` fails if any file regresses from clean to error
3. **Ratchet**: `just cpan-corpus-ratchet` adds newly-clean files to the baseline
4. **Timing**: Ratchet runs after each merge wave of parser fixes

### Ratchet Gap

There is often a gap between the actual clean rate and the ratcheted baseline. Parser fixes merge to master but the baseline isn't updated until someone runs the ratchet command. In cycle 5, this gap was estimated at ~5% — multiple error buckets had already been fixed but not ratcheted.

The fix: automate ratcheting as a post-merge-wave step, or add a hook that triggers ratchet after parser fix PRs merge.

---

## Timeline Summary

| Phase | Target | Files to Fix | Builders | Calendar Time |
|-------|--------|-------------|----------|--------------|
| A | 90% | 350-450 | 5 | 4-5 weeks |
| B | 95% | 200-350 | 12-15 | 7-9 weeks |
| C | 98% | 100-140 | 20+ | 11-15 weeks |
| **Total** | **98%** | **650-940** | **37+** | **22-29 weeks** |
| Floor | ~97-98% | N/A | N/A | Unfixable |

The 90% target for 0.12.0 public alpha is achievable in Phase A: 5 builders working for 4-5 weeks on the highest-impact error buckets.
