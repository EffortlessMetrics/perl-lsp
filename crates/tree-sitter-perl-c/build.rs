//! Build script for `tree-sitter-perl-c`.
//!
//! Compiles the vendored C parser (`parser.c`) and external scanner
//! (`scanner.c`) via the `cc` crate and exposes the resulting static
//! library as `tree-sitter-perl-c`.
//!
//! Upstream-owned vendored files are isolated under `c-src/`; local wrapper
//! logic lives in this build script and Rust sources under `src/`.
//! Provenance and refresh instructions are documented in
//! `UPSTREAM_SNAPSHOT.md`.
//!
//! No bindgen is involved: the single symbol we need from the C library
//! (`tree_sitter_perl`) is declared by hand in `src/lib.rs`.

use std::path::PathBuf;

fn main() {
    let grammar_src = PathBuf::from("c-src");
    let parser_c = grammar_src.join("parser.c");
    let scanner_c = grammar_src.join("scanner.c");

    let mut build = cc::Build::new();
    build.file(&parser_c);
    build.file(&scanner_c);

    build
        .include(&grammar_src)
        .flag_if_supported("-std=c99")
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-variable")
        .flag_if_supported("-Wno-unused-function")
        .compile("tree-sitter-perl-c");

    // Link the static library we just built.
    println!("cargo:rustc-link-lib=static=tree-sitter-perl-c");

    // Rebuild when any vendored C source or header changes.
    println!("cargo:rerun-if-changed={}", parser_c.display());
    println!("cargo:rerun-if-changed={}", scanner_c.display());
    println!("cargo:rerun-if-changed={}", grammar_src.join("bsearch.h").display());
    println!("cargo:rerun-if-changed={}", grammar_src.join("tsp_unicode.h").display());
    println!("cargo:rerun-if-changed={}", grammar_src.join("tree_sitter/parser.h").display());
    println!("cargo:rerun-if-changed={}", grammar_src.join("tree_sitter/array.h").display());
    println!("cargo:rerun-if-changed={}", grammar_src.join("tree_sitter/alloc.h").display());
}
