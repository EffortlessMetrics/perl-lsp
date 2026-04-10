#!/bin/bash
sed -i 's/pub fn resolve_module_path(/pub fn resolve_module_path(\n    root: \&Path,\n    module_name: \&str,\n    include_paths: \&[String],\n) -> Option<PathBuf> {/g' crates/perl-module-resolution-path/src/lib.rs
