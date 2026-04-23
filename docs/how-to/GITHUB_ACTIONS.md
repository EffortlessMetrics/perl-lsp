# GitHub Actions Integration

Use this guide when you want to install `perllsp` in a consumer repository's CI
workflow.

The reusable composite action is:

```yaml
- uses: EffortlessMetrics/perl-lsp/.github/actions/setup-perl-lsp@master
  with:
    version: '0.12.3'
    cache: true
```

## What the action does

- Resolves a release tag or uses the version you pin.
- Downloads the matching `perllsp` release archive for Linux, macOS, or
  Windows.
- Verifies the download against the published `SHA256SUMS` file.
- Falls back to a local source build when you set `build-from-source: true`.
- Adds the installed directory to `PATH` so later steps can call `perllsp`
  directly.

## Example consumer workflow

Copy the example workflow at
[docs/examples/github-actions/setup-perl-lsp-consumer.yml](../examples/github-actions/setup-perl-lsp-consumer.yml)
into your downstream repository and replace the placeholder test command with
your project's own `Makefile.PL`, `Build.PL`, or `dist.ini` flow.

Common downstream commands are:

- `perl Makefile.PL && make test`
- `perl Build.PL && ./Build test`
- `dzil test`

## Version pinning

Use `version: '0.12.3'` when you want a stable release.
Use `version: latest` when you want the newest published binary at workflow
runtime.

## Source builds

Set `build-from-source: true` when you want to build `perllsp` from the
checked-out source instead of downloading a release archive. That keeps the same
consumer interface while letting you test unreleased changes or unsupported
platforms.
