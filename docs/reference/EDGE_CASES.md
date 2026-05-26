# LSP Edge Case Handling

This document describes how perl-lsp handles edge cases that often confuse users
into thinking the server is broken when it is working correctly.

## Table of Contents

1. [Empty and Trivially-Empty Files](#empty-and-trivially-empty-files)
2. [Comment-Only and POD-Only Files](#comment-only-and-pod-only-files)
3. [Files with `__END__` or `__DATA__`](#files-with-__end__-or-__data__)
4. [Large Files](#large-files)
5. [Binary Files](#binary-files)
6. [Single-Line Files Without a Trailing Newline](#single-line-files-without-a-trailing-newline)
7. [Windows CRLF Line Endings](#windows-crlf-line-endings)
8. [Unicode Content](#unicode-content)
9. [Parser Edge Cases (Heredocs)](#parser-edge-cases-heredocs)
10. [Diagnostic Code Reference](#diagnostic-code-reference)

---

## Empty and Trivially-Empty Files

### What happens

Opening an empty `.pl` or `.pm` file produces:

- Zero parse errors (the file is valid Perl)
- Zero diagnostics (no strict/warnings suggestions)
- Empty completion list (correct — nothing is defined)
- Empty go-to-definition results (correct — no symbols exist)

This is intentional. An empty file is valid Perl. The `use strict` and
`use warnings` suggestions (codes `PL100` and `PL101`) are suppressed for
files with no executable content because the suggestions would fire
immediately on every new file a developer creates, before they have typed
a single line. The guard was added in PR #2792 and lives in
`crates/perl-lsp-diagnostics/src/lints/strict_warnings.rs`.

### Files considered "no executable content"

The parser classifies a file as having no executable content when its
abstract syntax tree produces `Program { statements: [] }`. This covers:

| Input | Parser result | Strict/warnings? |
|-------|--------------|-----------------|
| Empty string (`""`) | `Program { statements: [] }` | Suppressed |
| Whitespace only (`"   \n\t\n"`) | `Program { statements: [] }` | Suppressed |
| Single comment (`"# comment\n"`) | `Program { statements: [] }` | Suppressed |
| Shebang only (`"#!/usr/bin/perl\n"`) | `Program { statements: [] }` | Suppressed |
| Shebang + comments | `Program { statements: [] }` | Suppressed |
| CRLF whitespace (`"\r\n\r\n"`) | `Program { statements: [] }` | Suppressed |
| Actual code (`"my $x = 1;\n"`) | `Program { statements: [...] }` | Fires normally |

### Why the user might think LSP is broken

- No error squiggles appear on an empty file. This is correct.
- Completion returns nothing. This is correct.
- Go-to-definition returns nothing. This is correct.
- The only sign the server is running is that the status bar shows the
  server is active.

---

## Comment-Only and POD-Only Files

### Comment-only files

A file containing only `#` comments parses cleanly with an empty statements
list. No diagnostics are produced.

```perl
# This is a comment-only file.
# LSP is working correctly — there is simply nothing to analyse.
```

### POD-only files

POD (Plain Old Documentation) blocks are consumed as trivia by the lexer, the
same way `#` comments are. A file containing only a POD block therefore
produces `Program { statements: [] }` — the same as a comment-only file — and
no diagnostics are produced.

```perl
=head1 NAME

My::Module - description

=cut
```

If you add any Perl code to the file (even a single statement), the empty-file
guard is lifted and the `use strict`/`use warnings` suggestions will fire
normally.

---

## Files with `__END__` or `__DATA__`

The LSP parses only the code section of a file — the portion before the first
`__END__` or `__DATA__` marker on a line by itself. Everything after the marker
is ignored by the parser.

```perl
use strict;
use warnings;

my $data = do { local $/; <DATA> };

__DATA__
This is raw data — not parsed as Perl.
Any content here is invisible to the LSP.
```

This means:

- Symbols defined before `__END__`/`__DATA__` are fully indexed.
- Symbols defined after are not indexed and will not appear in completions.
- No diagnostics are generated for the data section.

The `__END__` or `__DATA__` marker alone (with no code before it) produces
`Program { statements: [DataSection {...}] }` — a non-empty statements list —
so strict/warnings diagnostics still fire for such files. This is correct
because the file has executable content (the data section marker is a
statement).

---

## Large Files

Files exceeding the size limit (default: **1 MB**, configurable via
`.perl-lsp.toml`) are skipped by the parser. The LSP:

1. Stores the file text so text-sync operations work.
2. Publishes an empty diagnostics list (no squiggles).
3. Returns empty results for all LSP features (completion, go-to-definition,
   hover, etc.).

The server logs a warning:

```
Skipping parse for <uri> (<N> bytes exceeds <limit> byte limit)
```

### Configuring the limit

The limit is set via the `perl.limits.maxFileSizeBytes` key in the LSP
`workspace/didChangeConfiguration` notification. The setting accepts an
integer number of bytes.

The limit protects the server from hanging on generated or minified files.
For typical Perl source files (which rarely exceed 100KB), the default 1MB
limit is never triggered.

---

## Binary Files

The LSP detects binary content by checking for null bytes (`\0`) in the file
text received via the LSP `textDocument/didOpen` or `textDocument/didChange`
notification. When null bytes are detected:

1. Parsing is skipped.
2. A single `Information`-level diagnostic is published:
   > "File appears to contain binary content (null bytes detected). Perl
   > diagnostics are disabled."
3. All LSP features return empty results.

### Why this matters

Binary files occasionally have a `.pl` extension (for example, compiled XS
shared objects in some distributions). Without this guard, the parser would
attempt to parse binary data and could produce confusing noise.

---

## Single-Line Files Without a Trailing Newline

The parser handles single-line files without a trailing newline correctly.
A file containing only `my $x = 1;` (no newline) parses identically to one
containing `my $x = 1;\n`. No special handling is required.

---

## Windows CRLF Line Endings

CRLF (`\r\n`) line endings are treated as whitespace by the lexer. A file
containing only `\r\n` sequences is indistinguishable from a file containing
only `\n` sequences — both produce `Program { statements: [] }` and no
diagnostics.

Files with CRLF line endings in actual code also parse correctly.

---

## Unicode Content

The parser handles UTF-8 source correctly. Files with Unicode identifiers,
string literals, or comments parse without error. The LSP server advertises
`utf-16` position encoding and converts internal byte offsets to UTF-16 code
units when reporting positions to the editor, so ranges are correct for files
containing multibyte characters.

For Perl source that uses non-UTF-8 encodings, add `use encoding '...';`
or `use utf8;` as appropriate. The LSP does not re-encode files; it trusts
that the editor delivers UTF-8 text.

---

## Parser Edge Cases (Heredocs)

Heredocs with dynamic delimiters, phase-dependent evaluation (`BEGIN`/`END`
blocks), or source filters cannot be fully parsed statically. The parser
degrades gracefully for these patterns rather than failing. See
[KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md) for the exhaustive list.

---

## Diagnostic Code Reference

Codes relevant to the edge cases above:

| Code | Severity | Condition | Suppressed for empty files? |
|------|----------|-----------|-----------------------------|
| `PL100` | Information | `use strict` not found | Yes — when `statements` is empty |
| `PL101` | Information | `use warnings` not found | Yes — when `statements` is empty |
| `PL111` | Warning | Misspelled pragma | No — only fires when a `use` statement exists |

For the full code registry see
[`crates/perl-diagnostics/src/lib.rs`](../../crates/perl-diagnostics/src/lib.rs).
