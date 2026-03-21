---
description: Accuracy-scout step 3 — run minimal examples to reproduce issue claims (if applicable)
user-invocable: false
---

# Accuracy: Verify Claims

For reproduction claims and corpus example claims from /accuracy-read-issue,
run minimal checks to confirm or refute what the issue asserts.

## Steps

1. **Check corpus example claims:**

   For each corpus file the issue references:
   ```bash
   ls <corpus_path> 2>&1 || echo "MISSING: <corpus_path>"
   ```

   If the corpus file exists and the issue claims a specific parse failure,
   check CPAN manifest:
   ```bash
   grep "<module_name>" .ci/cpan-corpus-manifest.txt 2>/dev/null | head -5
   ```

2. **Check reproduction claims (lightweight only):**

   Only run reproduction checks if:
   - The issue provides a short, self-contained Perl snippet
   - The claim is specific ("parser returns error on `my $x = {}`")
   - Running the check takes under 30 seconds

   Do NOT attempt to reproduce if:
   - Reproduction requires large setup (database, network, multi-file project)
   - The claimed snippet is longer than 20 lines
   - The issue says "this happens intermittently"

   For lightweight checks, search for an existing test that covers the claim:
   ```bash
   grep -rn "<key_snippet_or_error_text>" crates/ --include="*.rs" | head -10
   ```

   If a test already exists and passes, the claim may already be fixed.

3. **Check already-fixed claims:**

   For each "introduced in #NNN" or "fixed in #NNN" reference:
   ```bash
   gh pr view <NNN> --json state,mergedAt,title --jq '{state, mergedAt, title}'
   ```

   Also check recent merge log for related keywords:
   ```bash
   git log --oneline -30 | grep -i "<keyword>" | head -5
   ```

   If the issue says "X is broken" but there's a recent merged PR fixing X,
   the issue may be stale.

4. **Check duplicate claims:**

   For each issue that sounds similar:
   ```bash
   gh issue list --state open --search "<key_terms>" --limit 10 --json number,title,labels \
     --jq '.[] | "#\(.number) \(.title) [\(.labels | map(.name) | join(","))]"'

   gh issue list --state closed --search "<key_terms>" --limit 5 --json number,title \
     --jq '.[] | "#\(.number) \(.title)"'
   ```

## Output

```
Claim verification for issue #NNN:

  C1: target/cpan-corpus/lib/perl5/YAML/XS.pm — EXISTS in corpus manifest
  X1: "introduced in #2528" — PR #2528 MERGED 2026-03-15. Issue may be stale.
  R1: "parse_foo panics" — found test test_parse_foo_no_panic at expressions/tests.rs:88, passes
  D1: Potential duplicate of #2501 (open, same keywords)

Already-fixed risk: HIGH (PR #2528 merged, covers this issue area)
Duplicate risk: MEDIUM (#2501 is similar but covers different angle)
```
