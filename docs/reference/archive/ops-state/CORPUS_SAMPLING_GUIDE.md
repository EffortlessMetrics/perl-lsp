# Corpus Sampling Guide — Quick Reference for Scouts

## How to Find CPAN Files in Error Buckets

### Prerequisites

1. CPAN corpus must be installed:
   ```bash
   just cpan-corpus-install
   ```

2. Latest corpus baseline must exist:
   ```bash
   .ci/cpan-corpus-baseline.json
   ```

### Quick Commands

**List all files in the corpus:**
```bash
find target/cpan-corpus/lib/perl5 -name "*.pm" | sort
```

**Search for files by module name:**
```bash
find target/cpan-corpus/lib/perl5 -path "*Moose*" -name "*.pm"
```

**Run parser on a specific file with debug output:**
```bash
cargo run -p perl-parser -- <file.pm> 2>&1 | head -50
```

**Run corpus sweep and capture full results:**
```bash
just cpan-corpus-sweep --output target/sweep-report.json
```

### Finding Files in a Specific Error Bucket

The `cpan-corpus-baseline.json` only shows **first error per file** (which bucket). To find all files in a bucket:

**Method 1: Re-run sweep with verbose output**
```bash
just cpan-corpus-sweep --verbose --output target/verbose-report.json
```

This produces a JSON with `file_results[]` array showing every file's status and first error bucket.

**Method 2: Quick sampling (manual)**

1. Get a random sample of CPAN modules:
   ```bash
   find target/cpan-corpus/lib/perl5 -name "*.pm" | shuf | head -100
   ```

2. Run parser on each, filter by error message:
   ```bash
   for file in $(find target/cpan-corpus/lib/perl5 -name "*.pm" | shuf | head -200); do
       error=$(cargo run -p perl-parser -- "$file" 2>&1 | grep "error" | head -1)
       if echo "$error" | grep -q "unexpected_token_in_expr"; then
           echo "$file"
       fi
   done
   ```

### Corpus Structure

```
target/cpan-corpus/
└── lib/
    └── perl5/
        ├── Moose.pm
        ├── Moose/
        │   ├── Meta/
        │   │   ├── Attribute.pm
        │   │   └── Class.pm
        │   ├── Role.pm
        │   └── ...
        ├── DBIx/
        │   ├── Class.pm
        │   ├── Class/
        │   │   ├── ResultSet.pm
        │   │   └── ...
        │   └── ...
        └── ...
```

**Module path = filesystem path with / → ::**
- `Moose/Meta/Attribute.pm` = `Moose::Meta::Attribute`
- `DBIx/Class/ResultSet.pm` = `DBIx::Class::ResultSet`

### Common Sampling Patterns

**Find a specific module:**
```bash
find target/cpan-corpus/lib/perl5 -path "*Module/Setup*" -name "*.pm"
# Output: target/cpan-corpus/lib/perl5/Module/Setup.pm
#         target/cpan-corpus/lib/perl5/Module/Setup/Plugin.pm
#         ...
```

**Count files in a distribution:**
```bash
find target/cpan-corpus/lib/perl5 -path "*Try/Tiny*" -name "*.pm" | wc -l
```

**Test a file:**
```bash
cargo run -p perl-parser -- target/cpan-corpus/lib/perl5/Try/Tiny.pm
```

**See the error:**
```bash
cargo run -p perl-parser -- target/cpan-corpus/lib/perl5/Try/Tiny.pm 2>&1 | grep -A5 "error:"
```

## Phase A Bucket Examples

Scouts for Phase A should sample files from these buckets:

### Bucket #1: unexpected_token_in_expr

**Command:** Find first 10 files with this error
```bash
for file in $(find target/cpan-corpus/lib/perl5 -name "*.pm" | sort | head -500); do
    error=$(cargo run -p perl-parser -- "$file" 2>&1 | head -5)
    if echo "$error" | grep -q "unexpected_token_in_expr"; then
        echo "$file"
        count=$((count+1))
        [ $count -ge 10 ] && break
    fi
done
```

**Look for patterns:**
- Keywords in bareword position (format, local, state, given, when)
- Postfix operators followed by modifiers
- Complex nesting with blocks

### Bucket #2: unclosed_paren_identifier

**Command:** Similar to above, but search for `unclosed_paren_identifier`

**Look for patterns:**
- Implicit `$_` in grep/map/sort
- Bareword function calls with multiple arguments
- Interpolation in parentheses

### Bucket #3: unexpected_question_expr

**Command:** Search for ternary operator issues

**Look for patterns:**
- Nested ternary: `$a ? $b : $c ? $d : $e`
- Ternary in list context: `@arr = ($x ? A : B, $y ? C : D)`
- Ternary with logical operators

### Bucket #4: unclosed_paren

**Command:** Search for general unclosed paren errors

**Look for patterns:**
- Semicolon in parens
- Bracket mismatches
- XS module markers (bootstrap)

### Bucket #5: unexpected_rbrace_expr

**Command:** Search for rbrace in expression

**Look for patterns:**
- Hash literal in expression position
- Nested blocks
- Postfix `}` in complex nesting

## Creating a Scout Report

Once you've sampled files, create a GitHub issue with this structure:

```markdown
## Scout Report: [Bucket Name] (N files)

### Root Causes Identified

**Category 1: [Description]**
- Estimated files: X-Y
- Fixability: EASY | MEDIUM | HARD
- Example code:
  ```perl
  [sample code from CPAN]
  ```
- CPAN files triggering this:
  - Module::Name (file.pm:line_number)
  - Another::Module (file.pm:line_number)

**Category 2: [Description]**
- ... (repeat)

### Recommended Fixes

1. [Fix #1 description]
   - Implementation: [which files to modify]
   - Difficulty: EASY | MEDIUM | HARD
   - Expected file recovery: X-Y files

2. [Fix #2 description]
   - ... (repeat)

### Sample Files for Regression Testing

- `Module::Name` (file.pm:line)
- `Another::Module` (file.pm:line)
- ... (pick 5-10 representative examples)

### Summary

- Total bucket size: N files
- Fixable portion: X-Y% (estimated)
- Unfixable (source filters, etc): Z files
- Overall difficulty: MEDIUM
```

## Tips for Efficient Sampling

1. **Sample 15-20 files per bucket** — enough to identify patterns without exhaustion
2. **Diversify**: Don't just sample the first 20 files; use `shuf` or `sort | head -200 | shuf | head -20`
3. **Look at line numbers**: The error message usually shows the exact parse failure point
4. **Check the code context**: Read 2-3 lines before/after the failure point
5. **Test locally**: Run the problematic code snippet through the parser individually
6. **Document examples verbatim**: Copy exact code from CPAN, including context

## Troubleshooting

**"file not found"**
- Corpus not installed? Run `just cpan-corpus-install`
- Path has spaces? Quote it: `"path with spaces/file.pm"`

**"error: failed to read file"**
- File may be symlink? Use `realpath` to resolve
- File may be unreadable? Check permissions: `ls -l`

**"parser hangs on large files"**
- Kill with Ctrl+C and skip
- Large files (>100KB) may have pathological parsing
- Document as "timeout" in report

**"can't find example of pattern X in bucket Y"**
- Pattern may be rare within this distribution
- Sample more files (increase head count)
- Or wait for builder to implement fix and re-run sweep
