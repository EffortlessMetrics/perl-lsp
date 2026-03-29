# perl-pod

POD extraction for Perl module files.

## Problem it solves

Perl modules often keep user-facing documentation inline as POD. Tools that
want hover text, summaries, or quick module overviews need a lightweight way to
extract the useful parts without running Perl. This crate parses POD sections
from `.pm` files and returns structured documentation.

## Public API

- `extract_pod` parses POD from a source string.
- `extract_pod_from_file` reads a file and extracts POD.
- `PodDoc` stores module name, synopsis, description, and method docs.

## Example

```rust,ignore
use perl_pod::extract_pod;

let doc = extract_pod(source);
if let Some(summary) = doc.description.as_deref() {
    println!("{summary}");
}
```

## Workspace role

`perl-lsp` and related tooling can use this crate to surface POD-backed hover
content without shelling out to Perl or depending on external documentation
tools.

## License

MIT OR Apache-2.0
