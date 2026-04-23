# Buildkite CI

This page documents the Buildkite-specific template for `perl-lsp`.
It is intentionally narrow: the shared CI overview stays in
[docs/project/CI.md](../project/CI.md).

## Template

Use [`templates/ci/buildkite/pipeline.yml`](../../templates/ci/buildkite/pipeline.yml)
as the starting point for a Buildkite pipeline.

The template keeps the same high-level tiers used by the GitHub Actions path:

- PR smoke
- merge gate

Each step runs on a Rust-capable agent queue and is wrapped in the Docker
plugin so the toolchain and repo dependencies stay reproducible across agents.

## Docker Plugin Usage

The template expects `BUILDKITE_DOCKER_IMAGE` to point at a Rust-capable image
that already has the repo's runtime dependencies.

The important pieces are:

- `agents.queue: rust` targets the right self-hosted pool.
- `docker#v5.13.0` isolates the command inside the selected image.
- `mount-buildkite-agent: true` keeps the agent available for artifact upload
  and other Buildkite-native features.

If your Buildkite cluster uses a different queue name, keep the template and
change only the queue value.

## Artifact Handling

The merge-gate step uploads the `artifacts/` directory with
`artifact_paths`.

Example flow:

```yaml
commands:
  - mkdir -p artifacts
  - nix develop -c just ci-gate
artifact_paths:
  - artifacts/**
```

That keeps receipts and gate output in one predictable location. When a later
job needs those files, use Buildkite's artifact download flow against the same
path prefix.

## Private Artifact Storage

If your Buildkite deployment stores artifacts in a private bucket or an
internal artifact backend, keep the same `artifacts/` path convention and let
Buildkite handle the upload/download boundary.

Practical rules:

- keep receipts in `artifacts/`
- do not embed secrets in the receipt files
- restrict artifact access to the Buildkite organization or the private bucket
  policy your cluster already uses
- use `buildkite-agent artifact download` only from authenticated CI jobs or
  trusted operators

That preserves the private storage boundary without changing the pipeline
template itself.
