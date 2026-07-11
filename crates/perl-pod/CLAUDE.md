# CLAUDE.md (perl-pod)

## Role

POD (Plain Old Documentation) extractor for Perl source files. Turns POD
sections into structured data suitable for LSP hover display.

## Owns

- `PodDoc` -- extracted documentation: `name`, `synopsis`, `description`,
  `methods` (keyed by name from `=head2`), `arguments`, `return_values`,
  `examples`, `see_also`.
- `extract_pod(&str) -> PodDoc` and `extract_pod_from_file(&Path) ->
  io::Result<PodDoc>` -- the two entry points.

## Does not own

- Rendering POD to HTML/Markdown for display -- that's a consumer concern
  (e.g. hover formatting downstream).
- Perl code parsing -- this is regex/line-based POD scanning, not an AST
  walk.
- Any other crate as a dependency -- this is a zero-dependency leaf crate.

## Neighbors

- Upstream: none (leaf crate, no internal dependencies).
- Downstream: `perl-lsp-rs` (hover), `perl-workspace-core` (`pod.rs` /
  `PodFact` reuses this crate for structured POD facts rather than
  re-implementing extraction).

## Read first

- `src/lib.rs` -- the entire crate; single file, whole public API.

## Focused validation

`cargo test -p perl-pod` -- see `tests/pod_extraction_tests.rs` and
`tests/pod_coverage_tests.rs`.

## Review hotspots

`=head1`/`=head2` section-name matching is string-based, not a formal POD
grammar -- non-standard POD (unusual heading names, `=over`/`=item` nested
inside a method's docs) is the most likely source of extraction gaps.

## Claim boundary

Describes extraction scope as authored (the specific headings `PodDoc`
models). Does not assert full perlpod-spec coverage -- only the subset of
sections this crate's fields represent.
