# perl-qualified-name

Small, focused helpers for Perl-qualified names.

- Split names like `Foo::Bar` into package and bare segments.
- Validate qualified names with Unicode-aware identifier rules.

This crate intentionally has one responsibility and zero side effects: it exists so
other parts of the workspace can share consistent qualified-name handling.
