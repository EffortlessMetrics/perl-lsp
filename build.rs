//! Build script for tree-sitter-perl
//!
//! This build script handles compilation of the C parser and scanner,
//! and conditionally includes the Rust scanner based on feature flags.

use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Always build the C parser (required for tree-sitter)
    build_c_parser();

    // Always build C scanner (required for external scanner functions)
    build_c_scanner();

    // Generate bindings for the C parser
    generate_bindings()?;

    // Tell cargo to rerun this script if any of these files change
    println!("cargo:rerun-if-changed=src/parser.c");
    println!("cargo:rerun-if-changed=src/scanner.c");
    println!("cargo:rerun-if-changed=src/tree_sitter/");
    println!("cargo:rerun-if-changed=grammar.js");

    Ok(())
}

fn build_c_parser() {
    let mut build = cc::Build::new();

    // Add parser source files
    build.file("src/parser.c");

    // Add tree-sitter runtime
    if let Some(tree_sitter_dir) = find_tree_sitter_runtime() {
        build.include(&tree_sitter_dir);
        build.file(tree_sitter_dir.join("lib/src/lib.c"));
    }

    // Compile with appropriate flags
    build
        .flag_if_supported("-std=c99")
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-variable")
        .flag_if_supported("-Wno-unused-function")
        .flag_if_supported("-Wno-empty-body");

    build.compile("tree-sitter-perl-parser");
}

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
        .flag_if_supported("-Wno-unused-function")
        .flag_if_supported("-Wno-empty-body");

    build.compile("tree-sitter-perl-scanner");
}

fn find_tree_sitter_runtime() -> Option<PathBuf> {
    // Try to find tree-sitter runtime in common locations
    let possible_paths = [
        "tree-sitter/lib",
        "../tree-sitter/lib",
        "../../tree-sitter/lib",
    ];

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

fn generate_bindings() -> Result<(), Box<dyn std::error::Error>> {
    // Generate bindings for the C parser
    let bindings = bindgen::Builder::default()
        .header("src/parser.c")
        .header("src/scanner.c")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .map_err(|e| format!("Unable to generate bindings: {}", e))?;

    let out_path = PathBuf::from(env::var("OUT_DIR")?);
    bindings
        .write_to_file(out_path.join("bindings.rs"))?;
    Ok(())
}
