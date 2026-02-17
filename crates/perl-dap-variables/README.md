# perl-dap-variables

Perl debugger variable parsing and DAP rendering utilities.

## Scope

- Models debugger values (`PerlValue`) for scalars, arrays, hashes, refs, and objects.
- Parses debugger output into structured values.
- Renders values into DAP-compatible variable responses with expansion metadata.

## Public Surface

- `PerlValue` value model.
- `VariableParser`, `VariableParseError`.
- `PerlVariableRenderer`, `RenderedVariable`, `VariableRenderer`.

## Workspace Role

Internal support crate used by `perl-dap` variable and evaluate flows.

## License

MIT OR Apache-2.0.
