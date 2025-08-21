# Tree-sitter Perl Test Corpus

This directory contains a comprehensive test corpus for the tree-sitter Perl parser, covering ~100% of Perl 5 syntax.

## Organization

Files are organized by feature area with consistent naming:
- `builtins-*.txt` - Built-in functions and operators
- `regex-*.txt` - Regular expression features
- `special-*.txt` - Special variables and constructs
- `feature-*.txt` - Modern Perl features
- `oo-*.txt` - Object-oriented programming
- `sysproc-*.txt` - System and process control
- `fuzz-*.txt` - Edge cases and parser stress tests

## Metadata System

Each test section includes metadata comments for searchability:

```perl
==================
Section Title
==================
# @id: unique.identifier.001      # Unique test identifier
# @tags: tag1 tag2 tag3           # Searchable tags
# @perl: 5.14+                    # Minimum Perl version
# @flags: lexer-sensitive         # Special handling flags
```

### Tag Taxonomy

Common tags used throughout the corpus:
- **Core features**: `scalar`, `array`, `hash`, `reference`, `typeglob`
- **Flow control**: `if`, `while`, `for`, `foreach`, `given`, `when`, `labels`
- **Regex**: `regex`, `substitution`, `transliteration`, `anchor`, `lookahead`
- **OO**: `package`, `bless`, `method`, `inheritance`, `overload`
- **IO**: `open`, `filehandle`, `socket`, `pipe`, `select`
- **Modern**: `signatures`, `try-catch`, `defer`, `class`, `method`
- **Special**: `magic-var`, `special-var`, `signal`, `tie`, `unicode`

### Flag Meanings

- `lexer-sensitive` - Tests lexer mode switching (regex vs division, etc.)
- `ambiguous` - Tests ambiguous constructs requiring context
- `error-node-expected` - Intentional syntax errors for error recovery testing
- `version-gated` - Requires specific Perl version
- `experimental` - Tests experimental features
- `deprecated` - Tests deprecated but still parsed constructs
- `precedence` - Tests operator precedence edge cases
- `slow` - May take longer to parse (pathological cases)

## Searching the Corpus

Find tests by tag:
```bash
# Find all pack/unpack tests
rg '# @tags:.*pack' test/corpus

# Find lexer-sensitive tests
rg '# @flags:.*lexer-sensitive' test/corpus

# Find tests requiring Perl 5.26+
rg '# @perl: 5\.(2[6-9]|3[0-8])' test/corpus

# Find specific test by ID
rg '# @id: regex.pos.001' test/corpus
```

## Coverage Statistics

As of latest update:
- **Total test files**: 40+
- **Total test cases**: 1,100+
- **Perl version coverage**: 5.8 through 5.38
- **Feature coverage**: ~100% of Perl 5 syntax

Major areas covered:
- ✅ All built-in functions and operators
- ✅ Complete regex features including code blocks
- ✅ All special variables and magic globals
- ✅ Modern Perl features (try/catch, signatures, etc.)
- ✅ Unicode support and encoding pragmas
- ✅ Object-oriented programming and overloading
- ✅ Pack/unpack binary templates
- ✅ System calls and process control
- ✅ XS and C integration constructs
- ✅ Format declarations and heredocs
- ✅ Complete operator precedence
- ✅ Edge cases and error recovery

## Adding New Tests

1. Choose appropriate file or create new one following naming convention
2. Add test section with metadata:
   ```perl
   ==================
   My New Test
   ==================
   # @id: category.subcategory.NNN
   # @tags: relevant tags here
   # @perl: 5.XX+ (if version specific)
   # @flags: any-special-flags (if needed)
   
   # Perl code to test
   my $example = "test";
   
   ---
   
   (expected_tree_structure)
   ```

3. Run parser to generate/verify tree structure
4. Update this README if adding new category

## Test Validation

Run all corpus tests:
```bash
cargo test corpus
tree-sitter test
```

Run specific test file:
```bash
tree-sitter test -f test/corpus/regex-features.txt
```

## Maintenance

- Keep test files under ~300 lines for readability
- Ensure unique @id values across corpus
- Update tags consistently with existing taxonomy
- Document any new flags in this README
- Verify tests pass before committing