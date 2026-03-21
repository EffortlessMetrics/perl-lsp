---
description: Research verifier step 2 — verify Perl language claims via web search of official docs
user-invocable: false
---

# Research: Verify Perl

Verify all PERL claims extracted in step 1 against official Perl documentation.
Use web search to check perlsyn, perlfunc, perlop, perlre, perlmod, and
perldoc.perl.org version tables.

## Steps

For each PERL claim from step 1:

1. **Search perldoc.perl.org** for the relevant manual page:
   - Syntax claims → perlsyn
   - Built-in functions → perlfunc
   - Operators → perlop
   - Regular expressions → perlre
   - Modules and packages → perlmod
   - Version availability → search the manual for "since 5.XX" or check the
     feature version table at https://perldoc.perl.org/feature

2. **Search pattern:** Use WebSearch with queries like:
   - `site:perldoc.perl.org <topic> syntax`
   - `perl <claim> perldoc`
   - `perldoc perlsyn "<keyword>"`

3. **For version claims**, check:
   - https://perldoc.perl.org/feature for named features
   - The specific manual page's HISTORY section
   - https://metacpan.org/dist/perl/changes for changelogs

## Common errors to catch

- `if EXPR { }` without parens → FALSE. perlsyn requires parentheses around
  the condition for postfix-if. Block form requires parens too.
- Named captures (`(?<name>...)`) → available since Perl 5.10, NOT 5.32.
- `say` built-in → feature 'say' available since 5.10, default in 5.36.
- Indirect object syntax (`new Foo`) → valid but discouraged; not removed.
- Non-ASCII identifiers → allowed since 5.8 with `use utf8`.
- `//=` defined-or-assign → available since 5.10.

## Output

For each PERL claim:
```
P1: "<claim>"
  STATUS: VERIFIED | FALSE | UNVERIFIED
  FINDING: <1-2 sentences — what the docs actually say>
  SOURCE: <URL> — <what page/section confirmed this>
  VERSION: <if version-specific: "available since 5.XX">
```

If web search is unavailable or returns no authoritative results:
```
P1: "<claim>"
  STATUS: UNVERIFIED
  FINDING: Could not find authoritative source. Recommend plan-reviewer check manually.
  SOURCE: NONE
```
