# perl-quote

Quote-like operator parsers for Perl token text.

## Scope

- Extracts pattern/body/modifier parts from regex-like operators.
- Parses substitution (`s///`) and transliteration operators across delimiter styles.
- Offers strict modifier validation paths for safer parsing.

## Public Surface

- `extract_regex_parts`.
- `extract_substitution_parts`, `extract_substitution_parts_strict`.
- `extract_transliteration_parts`.
- `validate_substitution_modifiers`.
- `SubstitutionError`.

## Workspace Role

Internal parsing helper crate used by lexer and parser components.

## License

MIT OR Apache-2.0.
