# Test2 Integration

`perl-lsp` **reads** [Test2](https://metacpan.org/pod/Test2::V0) source and Test2
runner output, and drives your project's real test runner. It does **not**
implement Test2, execute `subtest` blocks in isolation, or replace `yath` /
`prove` / `perl`. Test2 remains the test framework; `perl-lsp` is the
editor/compiler/debug surface that understands it.

> One-sentence rule: **`perl-lsp` reads Test2 source, imports, structure, and
> output; Test2 itself still runs the tests.**

## What it reads

### 1. Imports (`use Test2::V0;`)

`perl-lsp` knows which symbols a Test2 import brings into scope, so ordinary
assertions are not flagged as unknown subs and completions include them. It
understands the recommended bundles (`Test2::V0`, `Test2::V1`,
`Test2::Bundle::*`) and the common `Test2::Tools::*` modules, plus import
modifiers:

| Import | Effect |
| --- | --- |
| `use Test2::V0;` | Default export set (`ok`, `is`, `like`, `subtest`, `done_testing`, `dies`, `lives`, `warnings`, …) **and** turns on `strict` + `warnings`. |
| `use Test2::V0 '!ok';` | Default set minus `ok`. |
| `use Test2::V0 ok => {-as => 'my_ok'};` | Imports `ok` under the name `my_ok`. |
| `use Test2::V0 -no_strict => 1;` | Does not imply `strict` (warnings still on). |
| `use Test2::V0 -no_warnings;` | Does not imply `warnings` (strict still on). |
| `use Test2::V0 -no_pragmas;` | Implies neither. |

Because `use Test2::V0;` turns on `strict`/`warnings`, the native critic does
**not** raise `require_use_strict` / `require_use_warnings` on a normal Test2
file — unless one of the opt-out options above is present.

Fact table: [`perl_lsp_rs_core::providers::testing::test2`](../../crates/perl-lsp-rs-core/src/providers/testing/test2.rs).
The export lists are verified against the canonical Test2-Suite source, not
inferred.

### 2. Structure (subtests)

`subtest 'name' => sub { ... }` blocks are discovered as a tree — nested
subtests become children. This drives the document-symbol outline and the
"Run/Debug Subtest" code lenses. Dynamic names (a variable, or an interpolated
string) are reported as *dynamic* rather than guessed.

```perl
subtest 'user lookup' => sub {
    ok(my $user = find_user('a@example.com'), 'found user');
    subtest 'email' => sub {
        is($user->{email}, 'a@example.com', 'email matches');
    };
};
```

```text
t/user.t
└── user lookup            (subtest)
    └── email              (subtest)
```

Discovery: [`providers::testing::subtest`](../../crates/perl-lsp-rs-core/src/providers/testing/subtest.rs).

### 3. Runner output (TAP)

When you run tests, `perl-lsp` reads the TAP the runner emits and maps failures
back to source:

```text
not ok 3 - email matches
#   at t/user.t line 12.
#          got: 'wrong@example.com'
#     expected: 'a@example.com'
```

becomes a structured failure (`file`, `line`, `got`, `expected`) attached to the
run result. `TODO` and `SKIP` are **not** counted as hard failures. The raw
stdout/stderr, exit code, and runner name are always preserved — parsed data is
additive.

Reader: [`providers::testing::tap`](../../crates/perl-lsp-rs-core/src/providers/testing/tap.rs).

## Commands and code lenses

| Surface | Command | Behaviour |
| --- | --- | --- |
| **Run All Tests** (lens, top of `.t`) | `perl.runTestFile` | Runs the whole file via `yath` → `prove` → `perl`. |
| **Run Subtest** (lens, above a subtest) | `perl.runSubtest` | Runs the **whole file** and focuses the TAP output on the named subtest (`subtestMode: "whole-file-focused"`). The anonymous block is never executed in isolation. |
| **Debug Test File** (lens, top of `.t`) | `perl.debugTestFile` | Returns a `perl-dap` launch configuration (`type: "perl"`, `request: "launch"`); the editor starts the debug session. |

## What it deliberately does **not** do

- It does not emulate Test2 assertions or a Test2 runtime.
- It does not execute anonymous `subtest` blocks on their own — "run subtest"
  is a whole-file run with focused output, clearly labelled as such.
- It does not replace `yath`, `prove`, or a project's own test command.
- It does not claim exact plan counts for dynamically generated tests; plan
  mismatches are reported conservatively and separately from hard failures.

## Snippets

The VS Code extension ships Test2-specific snippets: `usetest2` (a `Test2::V0`
file skeleton), `dies`, and `lives` (Test2::Tools::Exception). The common
assertion/structure snippets (`ok`, `is`, `like`, `subtest`, `done_testing`)
are already provided by the shared test snippets
([`snippets/perl.json`](../../vscode-extension/snippets/perl.json)).

## Architecture

| Concern | Home |
| --- | --- |
| Test2 import/export facts, subtest discovery, TAP reading | `perl-lsp-rs-core::providers::testing::{test2, subtest, tap}` |
| Test-aware critic (strict/warnings) | `perl-lsp-rs-core::tooling::perl_critic` |
| Running / debugging tests | `perl-lsp-rs::execute_command` (+ native `perl-dap`) |

These live inside the existing core/runtime crates rather than a separate
`perl-test2` crate: the work is *reading/discovery*, not a Test2 runtime. The
namespace is framework-neutral (`testing`) so `Test::More` and other frameworks
can grow into it.
