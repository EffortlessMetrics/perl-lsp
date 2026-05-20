# Perl Setup Troubleshooting

Use this guide when `perl-lsp` starts but the editor does not seem to see the
same Perl environment as your shell, tests, or debugger.

Most setup issues fall into one of these buckets:

```text
wrong Perl binary
missing include path
unexpected PERL5LIB policy
perldoc unavailable
DAP using different Perl or module paths
workspace opened at the wrong root
```

Start with **Perl LSP: Show Workspace Trust Report** in VS Code. The report is
read-only: it summarizes current server and client state without scanning the
workspace, running `perldoc`, starting DAP, probing Perl, or changing support
tiers.

For the support boundary behind the report, see
[Support tiers](../project/status/SUPPORT_TIERS.md). For the trust vocabulary
used in explanations and receipts, see [Editor Trust](EDITOR_TRUST.md).

## Perl Binary Looks Wrong

Symptoms:

- the server starts, but module resolution does not match your shell
- setup hints mention an unresolved Perl binary
- DAP uses a different Perl than the language server

Check:

```bash
perllsp --version
perllsp --health
perllsp --info
perl --version
```

Then run **Perl LSP: Show Workspace Trust Report** and compare:

- language-server Perl path or resolution state
- DAP Perl path, when reported by the VS Code extension
- workspace root
- setup hints

If you manage the server binary yourself, set the VS Code extension setting
`perl-lsp.serverPath` to the `perllsp` binary. If module probing must use a
specific Perl interpreter, configure the server-side `perl.workspace.perlPath`
setting through your editor or `.perl-lsp.toml`.

Do not assume the Perl used by your terminal, the LSP server, and the debugger
are the same until the trust report shows the same path or an intentional
difference.

## Modules Are Missing From `@INC`

Symptoms:

- PL701 says a module cannot be found
- completion does not suggest a project module
- goto-definition does not jump to a module file
- hover says module lookup failed

Use **Perl LSP: Explain Missing Module Lookup** for the specific module. It
shows the requested module, expected relative path, effective include paths,
configured include paths, `PERL5LIB` policy, and whether a candidate was found.

For project libraries, prefer explicit include paths:

```json
{
  "perl": {
    "workspace": {
      "includePaths": ["lib", "local/lib/perl5"]
    }
  }
}
```

You can also share project defaults in `.perl-lsp.toml`:

```toml
[perl]
include_paths = ["lib", "local/lib/perl5"]
```

If a module only exists through a shell-specific environment, check the
`PERL5LIB` section below. If a path exists only in DAP `launch.json`, do not
assume it applies to LSP diagnostics, completion, hover, or navigation.

## `PERL5LIB` Does Not Match Expectations

Symptoms:

- the module works in a terminal but not in the editor
- the editor finds a globally installed module when you expected isolation
- changing shell environment variables does not immediately change editor
  behavior

Check the workspace trust report and configuration reference for:

- `perl.workspace.usePerl5lib`
- `perl.workspace.perl5libPrecedence`
- configured `includePaths`
- effective include path order

`PERL5LIB` is environment state. It can differ depending on how the editor was
launched. If you want repeatable team behavior, prefer checked-in
`.perl-lsp.toml` include paths or editor settings over relying on each
developer's shell startup files.

After changing environment variables, restart the editor or language server so
the process sees the new environment. After changing LSP workspace settings,
run the workspace trust report again and confirm the effective paths changed.

## Configured Include Path Does Not Exist

Symptoms:

- setup hints mention a missing include path
- a path is listed in configuration but not in the effective lookup result
- PL701 persists after adding an include path

Check the path relative to the workspace root shown in the trust report. A
common mistake is opening a parent directory or a nested subdirectory instead of
the project root that owns `lib/`, `cpanfile`, `Makefile.PL`, or
`.perl-lsp.toml`.

If the path is editor-specific, make sure it is configured in the setting used
by your editor. If the path should be shared by the project, prefer
`.perl-lsp.toml`.

## `perldoc` Is Unavailable

Symptoms:

- hover documentation for built-ins or modules is missing
- setup hints say `perldoc` is unavailable
- hover falls back instead of showing external docs

Check:

```bash
perldoc -V
perldoc strict
```

Then run **Perl LSP: Show Workspace Trust Report**. The report exposes the
configured perldoc/oracle contract and VS Code client surface state, but it
does not run `perldoc` while generating the report.

If `perldoc` is intentionally unavailable, hover should still fall back instead
of presenting missing external docs as exact facts.

## DAP Uses Different Module Paths

Symptoms:

- the editor resolves a module, but the debugger cannot load it
- the debugger loads a different module version than goto-definition shows
- `launch.json` differs from LSP workspace settings

The language server and debug adapter are separate surfaces. Check both:

- LSP include paths in `.perl-lsp.toml`, editor settings, or initialization
  options
- DAP `perlPath`
- DAP `env.PERL5LIB`
- DAP launch configuration include path counts/classes in the workspace trust
  report

If a debug launch cannot find modules, make the debug runtime path explicit in
`launch.json`:

```json
{
  "type": "perl",
  "request": "launch",
  "name": "Debug app",
  "program": "${workspaceFolder}/script/app.pl",
  "perlPath": "perl",
  "cwd": "${workspaceFolder}",
  "env": {
    "PERL5LIB": "${workspaceFolder}/lib"
  }
}
```

Current trust reporting treats DAP launch state as setup evidence. Do not treat
that report as proof that DAP module-path behavior has been promoted beyond the
support tier documented in [Support tiers](../project/status/SUPPORT_TIERS.md).

## Workspace Root Is Wrong

Symptoms:

- relative include paths resolve to unexpected directories
- workspace symbols are sparse or noisy
- module lookup ignores the project `lib/`
- the trust report shows an unexpected root

Open the project root rather than a parent folder containing many unrelated
projects. Good roots usually contain one or more of:

```text
.perl-lsp.toml
cpanfile
Makefile.PL
Build.PL
dist.ini
.git
lib/
t/
```

After reopening the workspace, rerun **Perl LSP: Show Workspace Trust Report**
and confirm the root and include paths match the project you intended to edit.

## What To Paste Into An Issue

For setup-related reports, include:

- output from **Perl LSP: Show Workspace Trust Report**
- the `copyable_payload` object from the workspace trust report when you need
  structured setup evidence without raw local paths
- output from **Perl LSP: Explain Missing Module Lookup**, when PL701 or module
  lookup is involved
- the relevant `includePaths`, `.perl-lsp.toml`, or `launch.json` snippet with
  secrets removed
- whether the same module loads from your terminal with `perl -MModule::Name -e1`
- editor name and version
- operating system

Do not paste secrets from environment variables, private absolute paths, API
keys, or production credentials. The workspace trust report and provider
receipts are designed to summarize path classes and trust state without needing
raw sensitive values.
