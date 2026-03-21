---
description: Accuracy-scout step 1 — parse issue body and extract all file:line and function name claims
user-invocable: false
---

# Accuracy: Read Issue

Parse the issue body and extract every mechanical claim that can be verified
against the current codebase on `master`: file paths, line numbers, function
names, symbol names, corpus examples.

## Steps

1. **Read the issue:**

   ```bash
   gh issue view <number> --json title,body,labels,comments --jq '{title: .title, body: .body, labels: [.labels[].name], comments: [.comments[].body]}'
   ```

2. **Extract claims by category:**

   **File path claims** — any path like `crates/perl-parser/src/foo.rs` or
   `tests/corpus/bar.pl`. Note line numbers if given (`src/foo.rs:42`).

   **Function/symbol claims** — any `fn parse_foo()`, `struct Baz`, trait
   name, method call, or variable that the issue says exists or doesn't exist.

   **Corpus example claims** — references to specific CPAN modules, corpus
   files, or test fixtures (`YAML::XS`, `test_corpus/foo.pl`).

   **Already-fixed claims** — any "this regression appeared in..." or
   "introduced by commit..." or "fixed in #NNN" language.

   **Reproduction claims** — any assertion like "running X produces Y" or
   "the parser panics on input Z".

3. **Skip claims that are:**
   - Design opinions ("we should use X instead of Y")
   - Already fully cited with a verified link
   - About external Perl semantics (those go to research-verifier)

## Output

```
Claims extracted from issue #NNN:

FILE PATHS:
  F1: crates/perl-parser/src/expressions.rs:142 — function parse_method_call
  F2: ...

FUNCTIONS/SYMBOLS:
  S1: fn parse_hash_or_block() in perl-parser
  S2: ...

CORPUS EXAMPLES:
  C1: target/cpan-corpus/lib/perl5/YAML/XS.pm
  C2: ...

ALREADY-FIXED CHECKS:
  X1: "introduced in #2528" — check if #2528 is merged
  X2: ...

REPRODUCTION:
  R1: "parse_foo panics on 'my $x = ...' input"
  R2: ...

SKIP:
  - <reason>
```

Note the issue number prominently — subsequent steps need it to post comments.
