# perl-lsp-color-provider

Perl color detection and presentation provider. It finds color values in source
text and turns them into LSP color information and presentation variants.

## Use this crate when

Use `perl-lsp-color-provider` if you need document-color behavior for Perl
strings, hex values, ANSI colors, or named CSS colors. It is the feature layer
that sits beneath the editor-facing protocol implementation.

## Key exports

- `ColorInformation` / `Color` - detected color ranges and parsed color values
- `detect_colors` - scan source text for supported color forms
- `color_to_presentations` - build alternative color presentations for the UI

## Example

```rust,ignore
use perl_lsp_color_provider::detect_colors;

let colors = detect_colors("my $red = '#ff0000';");
```

## Stack role

`perl-lsp` uses this crate for `textDocument/documentColor` and related editor
features. It is intentionally small and focused on color extraction only.
