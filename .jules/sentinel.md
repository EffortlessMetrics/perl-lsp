# Sentinel's Journal

## 2025-02-18 - Code Injection in Perl Wrapper Construction
**Vulnerability:** `perl-lsp` constructed Perl scripts using `format!` with user-provided file paths and subroutine names, leading to Code Injection.
**Learning:** Constructing code strings from user input is dangerous even in "internal" glue code.
**Prevention:** Use environment variables or argument passing (ARGV) to transfer data to the interpreted script, avoiding string interpolation of data into code.
