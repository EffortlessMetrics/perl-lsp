# perl-path-normalize

Secure normalization for workspace-relative paths.

This crate has one responsibility: normalize user-supplied paths against a
canonical workspace root while preventing `..` traversal above that root.
