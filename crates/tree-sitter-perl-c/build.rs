use std::env;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    
    // Build the C scanner
    let mut build = cc::Build::new();
    build
        .file("../../tree-sitter-perl/src/scanner.c")
        .file("../../tree-sitter-perl/src/parser.c")
        .include("../../tree-sitter-perl/src")
        .flag_if_supported("-std=c99")
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-variable")
        .flag_if_supported("-Wno-unused-function")
        .compile("tree-sitter-perl-c");

    // Generate bindings
    let bindings = bindgen::Builder::default()
        .header("../../tree-sitter-perl/src/tree_sitter/parser.h")
        .clang_arg("-I../../tree-sitter-perl/src")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    // Tell cargo to look for shared libraries in the specified directory
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    
    // Tell cargo to tell rustc to link our C library
    println!("cargo:rustc-link-lib=static=tree-sitter-perl-c");
    
    // Only rerun this script if the C source files change
    println!("cargo:rerun-if-changed=../../tree-sitter-perl/src/scanner.c");
    println!("cargo:rerun-if-changed=../../tree-sitter-perl/src/parser.c");
    println!("cargo:rerun-if-changed=../../tree-sitter-perl/src/tree_sitter/parser.h");
} 