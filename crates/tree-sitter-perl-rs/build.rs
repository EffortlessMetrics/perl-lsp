//! Build script for tree-sitter-perl
//!
//! This build script handles compilation of the C parser and scanner,
//! and conditionally includes the Rust scanner based on feature flags.

use std::env;
use std::path::PathBuf;

fn main() {
    // Only build C components if requested
    #[cfg(feature = "c-parser")]
    {
        // Always build the C parser (required for tree-sitter C interop)
        build_c_parser();

        // Conditionally build scanner based on features
        if cfg!(feature = "c-scanner") {
            build_c_scanner();
        } else {
            // Default to rust-scanner stub if C scanner not requested
            build_rust_scanner_stub();
        }
    }

    // Generate bindings for the C parser only if requested
    #[cfg(feature = "bindings")]
    generate_bindings();

    // Tell cargo to rerun this script if any of these files change
    println!("cargo:rerun-if-changed=src/parser.c");
    println!("cargo:rerun-if-changed=src/scanner.c");
    println!("cargo:rerun-if-changed=src/tree_sitter/");
    println!("cargo:rerun-if-changed=grammar.js");

    // Set feature flags for conditional compilation
    if cfg!(feature = "rust-scanner") {
        println!("cargo:rustc-cfg=rust_scanner");
    }
    if cfg!(feature = "c-scanner") {
        println!("cargo:rustc-cfg=c_scanner");
    }
}

#[cfg(feature = "c-parser")]
fn build_c_parser() {
    let mut build = cc::Build::new();

    // Add parser source files
    build.file("src/parser.c");

    // Add tree-sitter runtime
    if let Some(tree_sitter_dir) = find_tree_sitter_runtime() {
        build.include(&tree_sitter_dir);
        build.file(tree_sitter_dir.join("src/lib.c"));
    }

    // Compile with appropriate flags
    build
        .flag_if_supported("-std=c99")
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-variable")
        .flag_if_supported("-Wno-unused-function");

    build.compile("tree-sitter-perl-parser");
}

#[cfg(feature = "c-parser")]
fn build_c_scanner() {
    let mut build = cc::Build::new();

    // Add scanner source files
    build.file("src/scanner.c");

    // Add tree-sitter runtime
    if let Some(tree_sitter_dir) = find_tree_sitter_runtime() {
        build.include(&tree_sitter_dir);
    }

    // Compile with appropriate flags
    build
        .flag_if_supported("-std=c99")
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-variable")
        .flag_if_supported("-Wno-unused-function");

    build.compile("tree-sitter-perl-scanner");
}

#[cfg(feature = "c-parser")]
fn build_rust_scanner_stub() {
    // Create a minimal stub that redirects to Rust scanner
    // This ensures the C scanner functions exist but delegate to Rust
    let mut build = cc::Build::new();

    // Create a simple stub implementation
    let stub_code = r#"
#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

// Forward declarations for tree-sitter types
typedef struct TSLexer TSLexer;

// Stub functions that will be overridden by Rust scanner
bool tree_sitter_perl_external_scanner_scan(void *payload, TSLexer *lexer, const bool *valid_symbols) {
    // This should be overridden by the Rust scanner
    return false;
}

unsigned tree_sitter_perl_external_scanner_serialize(void *payload, char *buffer) {
    return 0;
}

void tree_sitter_perl_external_scanner_deserialize(void *payload, const char *buffer, unsigned length) {
    // No-op for stub
}

void *tree_sitter_perl_external_scanner_create() {
    return NULL;
}

void tree_sitter_perl_external_scanner_destroy(void *payload) {
    // No-op for stub
}
"#;

    // Write stub to temporary file
    let out_dir = env::var("OUT_DIR").unwrap();
    let stub_path = PathBuf::from(&out_dir).join("scanner_stub.c");
    std::fs::write(&stub_path, stub_code).expect("Failed to write scanner stub");

    // Add tree-sitter runtime
    if let Some(tree_sitter_dir) = find_tree_sitter_runtime() {
        build.include(&tree_sitter_dir);
    }

    // Compile the stub
    build
        .file(&stub_path)
        .flag_if_supported("-std=c99")
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-variable")
        .flag_if_supported("-Wno-unused-function");

    build.compile("tree-sitter-perl-scanner-stub");
}

fn find_tree_sitter_runtime() -> Option<PathBuf> {
    // Try to find tree-sitter runtime in common locations
    let possible_paths = ["tree-sitter/lib", "../tree-sitter/lib", "../../tree-sitter/lib"];

    // Check static paths first
    for path in possible_paths.iter() {
        let runtime_dir = PathBuf::from(path);
        if runtime_dir.join("src/lib.c").exists() {
            return Some(runtime_dir);
        }
    }

    // Check environment variable
    if let Ok(runtime_dir) = env::var("TREE_SITTER_RUNTIME_DIR") {
        let runtime_dir = PathBuf::from(runtime_dir);
        if runtime_dir.join("src/lib.c").exists() {
            return Some(runtime_dir);
        }
    }

    None
}

#[cfg(feature = "bindings")]
fn generate_bindings() {
    // Generate bindings for the C parser
    let bindings = bindgen::Builder::default()
        .header("src/parser.c")
        .header("src/scanner.c")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        // For Rust 2024 compatibility: generate safe extern blocks
        .wrap_unsafe_ops(true)
        .generate_inline_functions(true)
        .size_t_is_usize(true)
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings.write_to_file(out_path.join("bindings.rs")).expect("Couldn't write bindings!");
}
