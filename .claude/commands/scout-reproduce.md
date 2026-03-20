---
description: Scout step 3 — reproduce the problem with a minimal example
user-invocable: false
---

# Scout Reproduce

Confirm the bug/gap with a concrete, minimal example.

## For parser issues

1. Find a corpus file that triggers the error:
   ```bash
   # Check the corpus sweep results or baseline
   cat .ci/cpan-corpus-baseline.json | python3 -c "import json,sys; ..."
   ```

2. Extract the minimal Perl snippet that fails:
   ```perl
   # The smallest code that triggers the error
   sub try (&;@) { goto &Foo::try }
   ```

3. Verify it actually fails:
   ```bash
   echo 'sub try (&;@) { 1 }' | cargo run -p perl-parser -- --stdin 2>&1
   ```

## For LSP issues

1. Identify the LSP request that's wrong/missing
2. Construct a test document and expected response
3. Note the gap between expected and actual

## For perf issues

1. Identify the slow operation
2. Estimate or measure the impact (ms, files affected)

## Output

Record in your task:
```
Reproduction:
  Input: <minimal code/request that triggers the issue>
  Error: <exact error message or missing behavior>
  Command: <how to reproduce>
```

Do NOT move to step 4 without a confirmed reproduction.
