# perl-dap-value

Shared value model for Perl Debug Adapter Protocol crates.

This crate defines [`PerlValue`], a serializable enum used to represent Perl
runtime values (`undef`, scalar, array, hash, references, objects, etc.) across
DAP parsing and rendering components.
