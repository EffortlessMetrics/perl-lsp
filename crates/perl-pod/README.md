# perl-pod

POD extractor for Perl `.pm` files.

Use this crate when you need structured documentation text for hover cards,
module docs, or other editor surfaces without pulling in the full parser or LSP
stack.

## Where it fits

`perl-pod` sits at the documentation edge of the workspace. It reads POD blocks
from source text or files and returns a small data model that downstream crates
can render or attach to diagnostics.

## Key entry points

- `PodDoc` - extracted `NAME`, `SYNOPSIS`, `DESCRIPTION`, and method docs
- `extract_pod(source)` - parse POD from a Perl source string
- `extract_pod_from_file(path)` - read a file and extract its POD

## Example

```rust
use perl_pod::{PodDoc, extract_pod};

let doc: PodDoc = extract_pod(
    r#"
=head1 NAME
My::Module - example module

=head1 SYNOPSIS
use My::Module;
=cut
"#,
);

assert_eq!(doc.name.as_deref(), Some("My::Module - example module"));
```

## Typical use

Use `perl-pod` when a caller already has Perl source and only needs the
documentation sections. If you need syntax analysis or symbol resolution first,
parse with `perl-parser` and then layer POD extraction on top.
