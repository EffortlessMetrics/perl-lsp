# perl-lsp-color-provider

Document-color detection and presentation helpers for `perl-lsp`.

## Problem it solves

Editors that support `textDocument/documentColor` need a provider that can find
color literals in Perl source and turn them into LSP color payloads. This crate
detects common Perl-facing color forms and returns editor-friendly ranges and
presentations.

## Public API

- `detect_colors` scans source text for supported color literals.
- `ColorInformation` carries the detected range plus normalized RGBA values.
- `Color` stores the color itself.
- `color_to_presentations` produces replacement variants such as hex strings.

## Supported inputs

- Hex colors such as `#RGB`, `#RRGGBB`, and `#RRGGBBAA`
- ANSI escape sequences like `\e[31m`
- Named CSS colors inside quoted strings
- `Term::ANSIColor` calls such as `color("red")`

## Example

```rust,ignore
use perl_lsp_color_provider::detect_colors;

let colors = detect_colors(r#"print "#ff8800";"#);
```

## Workspace role

`perl-lsp` uses this crate for document color discovery and color presentation
responses without mixing color parsing logic into the server runtime.

## License

MIT OR Apache-2.0
