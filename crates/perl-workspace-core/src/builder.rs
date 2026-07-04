//! Building a [`ProjectModel`] from a workspace directory.
//!
//! The builder walks a root, reads and parses each Perl source file via the
//! allowed leaf crates (`perl-parser-core` for parsing,
//! `perl-symbol::surface::extract_symbol_decls` for declaration projection),
//! and assembles typed facts with real ranges, deterministic IDs, and
//! provenance. It honors the requested [`FactClasses`] — a request that omits
//! symbols never pays to parse — and never panics on a bad file: read/parse
//! failures become [`ModelLimitation`]s, not errors.
//!
//! This is PR 3 of the PLSP-ADR-0006 rollout: files + packages + subs/methods.
//! Import/export facts and dynamic-boundary detection land in PR 4.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use perl_parser_core::Parser;
use perl_symbol::SymbolKind;
use perl_symbol::surface::extract_symbol_decls;

use crate::effects::CompileEffectFacts;
use crate::error::{ModelLimitation, WorkspaceCoreError};
use crate::fact_classes::FactClasses;
use crate::file::{FileRecord, FileRole, ParseStatus};
use crate::id::{Digest, FileId, PackageId, SymbolId};
use crate::import_walk;
use crate::model::ProjectModel;
use crate::package::PackageRecord;
use crate::provenance::Confidence;
use crate::range::Utf8LineIndex;
use crate::symbol::{SymbolFactKind, SymbolRecord, Visibility};

/// A request to build a project model.
#[derive(Debug, Clone, Copy)]
pub struct ProjectModelRequest<'a> {
    /// The workspace root to scan (a real filesystem path).
    pub root: &'a str,
    /// Which fact classes to compute.
    pub fact_classes: FactClasses,
}

/// Directory names never descended into during the walk.
const SKIP_DIRS: &[&str] =
    &[".git", "target", "node_modules", "blib", ".build", "_build", ".svn", "vendor"];

/// Build a [`ProjectModel`] for a workspace root.
///
/// Returns [`WorkspaceCoreError`] only for request-level failures (the root is
/// not a directory, or no fact classes were requested). Per-file failures are
/// recorded as limitations on the returned model.
pub fn build_project_model(
    request: &ProjectModelRequest<'_>,
) -> Result<ProjectModel, WorkspaceCoreError> {
    if request.fact_classes.is_empty() {
        return Err(WorkspaceCoreError::NoFactClasses);
    }
    let root = Path::new(request.root);
    if !root.is_dir() {
        return Err(WorkspaceCoreError::InvalidRoot {
            path: request.root.to_string(),
            reason: "not a directory".to_string(),
        });
    }

    let mut model = ProjectModel::empty(request.root, request.fact_classes);

    // Parse work is only done when the request needs syntax-derived facts.
    // (POD is read from raw source, not the AST, so it does not gate parsing.)
    let wants_parse = request.fact_classes.intersects(
        FactClasses::SYMBOLS
            | FactClasses::SYNTAX
            | FactClasses::IMPORTS
            | FactClasses::EXPORTS
            | FactClasses::COMPILE_EFFECTS
            | FactClasses::TESTS
            | FactClasses::RELATIONS
            | FactClasses::DYNAMIC_BOUNDARIES,
    );

    for relative_path in collect_perl_files(root) {
        let absolute = root.join(&relative_path);
        let content = match std::fs::read_to_string(&absolute) {
            Ok(content) => content,
            Err(error) => {
                // Never silently drop: a digest needs the content, so emit no
                // file fact — just a limitation recording why.
                model.limitations.push(ModelLimitation {
                    id: format!("read-failed:{relative_path}"),
                    kind: "read_failure".to_string(),
                    message: format!("could not read `{relative_path}`: {error}"),
                });
                continue;
            }
        };

        let digest = Digest::of(&content);
        let file_id = FileId::new(&relative_path, &digest);
        let role = FileRole::from_path(&relative_path);

        let parse_status = if wants_parse && is_parseable(role) {
            extract_facts(
                &mut model,
                &file_id,
                &relative_path,
                &content,
                role,
                request.fact_classes,
            )
        } else {
            ParseStatus::NotParsed
        };

        // Distribution-metadata facts: metadata files are not "parsed" as Perl,
        // but when DIST is requested their content is read for name/version/
        // license/prereqs.
        if role == FileRole::DistMetadata && request.fact_classes.contains(FactClasses::DIST) {
            if let Some(facts) = extract_dist_metadata(&file_id, &relative_path, &content) {
                model.dist_metadata.push(facts);
            }
        }

        // POD facts are read from raw source (independent of code parsing), so a
        // file whose Perl fails to parse can still yield POD.
        if request.fact_classes.contains(FactClasses::POD) && is_parseable(role) {
            let line_index = Utf8LineIndex::new(&content);
            if let Some(facts) = crate::pod::extract_pod_facts(&file_id, &content, &line_index) {
                model.pod.push(facts);
            }
        }

        model.files.push(FileRecord { file_id, relative_path, role, digest, parse_status });
    }

    model.sort_for_determinism();
    Ok(model)
}

/// Extract distribution-metadata facts from a metadata file, dispatched by
/// filename. Only `META.json` and `cpanfile` are read today (PR 7); other
/// metadata formats (`Makefile.PL`, `Build.PL`, `dist.ini`, `META.yml`) are
/// indexed as files but not yet content-parsed.
fn extract_dist_metadata(
    file_id: &FileId,
    relative_path: &str,
    content: &str,
) -> Option<crate::dist::DistMetadataFacts> {
    let name = relative_path.rsplit('/').next().unwrap_or(relative_path);
    match name {
        "META.json" => crate::dist::parse_meta_json(file_id.clone(), content),
        "cpanfile" => Some(crate::dist::parse_cpanfile(file_id.clone(), content)),
        _ => None,
    }
}

/// Only source-bearing roles are parsed; metadata/unknown files are not.
fn is_parseable(role: FileRole) -> bool {
    matches!(role, FileRole::Lib | FileRole::Test | FileRole::Script | FileRole::Pod)
}

/// Parse one file and push its package/symbol facts onto the model.
///
/// Returns the [`ParseStatus`] for the file record. On parse failure the file
/// is not dropped — a limitation is recorded and the status is
/// [`ParseStatus::Failed`].
fn extract_facts(
    model: &mut ProjectModel,
    file_id: &FileId,
    relative_path: &str,
    content: &str,
    role: FileRole,
    fact_classes: FactClasses,
) -> ParseStatus {
    let parsed = {
        let mut parser = Parser::new(content);
        parser.parse()
    };
    let ast = match parsed {
        Ok(ast) => ast,
        Err(_error) => {
            model.limitations.push(ModelLimitation {
                id: format!("parse-failed:{relative_path}"),
                kind: "parse_failure".to_string(),
                message: format!(
                    "could not parse `{relative_path}` as Perl; emitted the file fact with no symbols"
                ),
            });
            return ParseStatus::Failed;
        }
    };

    let line_index = Utf8LineIndex::new(content);

    // The import walk (imports + dynamic boundaries + inheritance) and the
    // compile-effect pass (perl-pragma) both read the same parsed AST. Run the
    // walk once if any of them is requested — effects want its `perl_version`.
    let wants_imports = fact_classes.intersects(
        FactClasses::IMPORTS
            | FactClasses::EXPORTS
            | FactClasses::RELATIONS
            | FactClasses::DYNAMIC_BOUNDARIES,
    );
    let wants_effects = fact_classes.contains(FactClasses::COMPILE_EFFECTS);

    let walk = if wants_imports || wants_effects {
        Some(import_walk::walk_imports(&ast, file_id, &line_index))
    } else {
        None
    };

    if wants_effects {
        // Reuse perl-pragma for strict/warnings/feature/version semantics rather
        // than hand-rolling a version→feature table (external-truth-gate).
        let pragma_map = perl_pragma::PragmaTracker::build(&ast);
        let state = perl_pragma::PragmaTracker::final_state(&pragma_map);
        let perl_version = walk.as_ref().and_then(|w| w.perl_version.clone());
        model.compile_effects.push(CompileEffectFacts::from_pragma_state(
            file_id.clone(),
            &state,
            perl_version,
        ));
    }

    let parents_by_package = if let Some(walk) = walk {
        // Relations are synthesized from imports + inheritance before those are
        // (conditionally) consumed into the model.
        if fact_classes.contains(FactClasses::RELATIONS) {
            model.relations.extend(crate::relation::synthesize_relations(
                file_id,
                relative_path,
                role,
                &walk.imports,
                &walk.parents_by_package,
            ));
        }
        if fact_classes.contains(FactClasses::IMPORTS) {
            model.imports.extend(walk.imports);
        }
        if fact_classes.contains(FactClasses::EXPORTS) {
            model.exports.extend(walk.exports);
        }
        if fact_classes.contains(FactClasses::DYNAMIC_BOUNDARIES) {
            model.dynamic_boundaries.extend(walk.boundaries);
        }
        walk.parents_by_package
    } else {
        BTreeMap::new()
    };

    // Test facts: for test-role files, detect the framework + assertion counts.
    if fact_classes.contains(FactClasses::TESTS) && role == FileRole::Test {
        if let Some(facts) = crate::test::extract_test_facts(&ast, file_id, &line_index) {
            model.tests.push(facts);
        }
    }

    // Symbol/package facts are only assembled when requested; a SYNTAX-only
    // request still parses (to set the status) but emits no declarations.
    if fact_classes.contains(FactClasses::SYMBOLS) {
        for decl in extract_symbol_decls(&ast, Some("main")) {
            let (start, end) = decl.full_span;
            let start_byte = u32::try_from(start).unwrap_or(u32::MAX);
            let end_byte = u32::try_from(end).unwrap_or(u32::MAX);
            let range = line_index.source_range(start_byte, end_byte);
            let kind = SymbolFactKind::from_perl_symbol(decl.kind);

            // `decl` is owned, so IDs borrow the name first and the fields move
            // it afterwards — no per-declaration string clones.
            if matches!(decl.kind, SymbolKind::Package | SymbolKind::Class | SymbolKind::Role) {
                let package_id = PackageId::new(file_id, &decl.qualified_name, start_byte);
                let parents =
                    parents_by_package.get(&decl.qualified_name).cloned().unwrap_or_default();
                model.packages.push(PackageRecord {
                    package_id,
                    name: decl.qualified_name,
                    file_id: file_id.clone(),
                    declaration_range: range,
                    version: None,
                    parents,
                    roles: Vec::new(),
                    confidence: Confidence::High,
                });
            } else {
                let symbol_id =
                    SymbolId::new(file_id, kind.tag(), &decl.qualified_name, start_byte, end_byte);
                let visibility = visibility_of(decl.declarator.as_deref());
                model.symbols.push(SymbolRecord {
                    symbol_id,
                    file_id: file_id.clone(),
                    kind,
                    package: decl.container,
                    name: decl.name,
                    qualified_name: decl.qualified_name,
                    declaration_range: range,
                    visibility,
                    confidence: Confidence::High,
                });
            }
        }
    }

    ParseStatus::Clean
}

/// Map a scope declarator to a visibility.
fn visibility_of(declarator: Option<&str>) -> Visibility {
    match declarator {
        Some("my") | Some("state") => Visibility::Private,
        Some("our") | Some("local") => Visibility::Public,
        // Subs, packages, methods, constants have no declarator and are
        // package-visible.
        None => Visibility::Public,
        Some(_) => Visibility::Unknown,
    }
}

/// Recursively collect repo-relative (forward-slash) paths of Perl source files
/// under `root`. Dependency-free depth-first walk; skips VCS/build dirs.
fn collect_perl_files(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(file_type) = entry.file_type() else { continue };
            // Skip symlinks (their own type, not the target): a circular dir
            // symlink would otherwise recurse forever, and a file symlink could
            // escape `root`. `file_type()` reports the link, not its target.
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if SKIP_DIRS.contains(&name.as_ref()) || name.starts_with('.') {
                    continue;
                }
                stack.push(path);
            } else if file_type.is_file() && is_indexable(&path) {
                if let Ok(relative) = path.strip_prefix(root) {
                    out.push(relative.to_string_lossy().replace('\\', "/"));
                }
            }
        }
    }
    out
}

/// True for files the substrate indexes: Perl source **or** distribution
/// metadata **or** an extensionless script with a Perl shebang. Metadata
/// files are indexed (as [`FileRole::DistMetadata`]) but not parsed — their
/// contents are read by the dist-metadata fact pass (PR 7).
fn is_indexable(path: &Path) -> bool {
    is_perl_source(path) || is_dist_metadata(path) || is_shebang_perl_script(path)
}

/// True for file extensions the substrate treats as Perl source.
fn is_perl_source(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("pm") | Some("pl") | Some("t") | Some("pod") | Some("psgi")
    )
}

/// True for a regular file with **no** extension whose first line is a Perl
/// shebang (`#!/usr/bin/perl`, `#!/usr/bin/env perl`, …).
///
/// Distributions commonly ship executables under `bin/`/`script/` with no
/// `.pl` suffix; without this check such a script would be silently invisible
/// to the substrate even though [`FileRole::from_path`] already classifies a
/// `bin/`/`script/` path as [`FileRole::Script`]. Kept tight: only files with
/// no extension are even considered, and only a real Perl shebang qualifies —
/// an extensionless non-Perl file (`README`, a shell script, …) stays out.
fn is_shebang_perl_script(path: &Path) -> bool {
    path.extension().is_none() && has_perl_shebang(path)
}

/// Read a small bounded prefix of `path` and check whether its first line is
/// a Perl shebang. Never panics or reads the whole file: any I/O or encoding
/// failure — or a first line beyond the bounded prefix — reads as "no".
fn has_perl_shebang(path: &Path) -> bool {
    use std::io::Read;
    let Ok(mut file) = std::fs::File::open(path) else { return false };
    let mut buf = [0u8; 256];
    let Ok(n) = file.read(&mut buf) else { return false };
    let Ok(text) = std::str::from_utf8(&buf[..n]) else { return false };
    let first_line = text.lines().next().unwrap_or("");
    first_line.starts_with("#!") && first_line.contains("perl")
}

/// Known distribution-metadata filenames.
fn is_dist_metadata(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|n| n.to_str()),
        Some(
            "META.json"
                | "META.yml"
                | "Makefile.PL"
                | "Build.PL"
                | "dist.ini"
                | "cpanfile"
                | "MANIFEST"
                | "MANIFEST.SKIP"
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Materialize a fixture tree under a unique temp dir, run the builder, and
    /// clean up. Returns the model.
    fn model_for(dir: &str, files: &[(&str, &str)], classes: FactClasses) -> ProjectModel {
        let root = std::env::temp_dir().join(format!("pwc-builder-{dir}"));
        let _ = std::fs::remove_dir_all(&root);
        for (rel, content) in files {
            let path = root.join(rel);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&path, content).unwrap();
        }
        let model = build_project_model(&ProjectModelRequest {
            root: root.to_str().unwrap(),
            fact_classes: classes,
        })
        .unwrap();
        let _ = std::fs::remove_dir_all(&root);
        model
    }

    #[test]
    fn rejects_missing_root() {
        let err = build_project_model(&ProjectModelRequest {
            root: "/definitely/not/a/dir/xyzzy",
            fact_classes: FactClasses::FILES,
        })
        .unwrap_err();
        assert!(matches!(err, WorkspaceCoreError::InvalidRoot { .. }));
    }

    #[test]
    fn rejects_empty_fact_classes() {
        let err = build_project_model(&ProjectModelRequest {
            root: ".",
            fact_classes: FactClasses::NONE,
        })
        .unwrap_err();
        assert_eq!(err, WorkspaceCoreError::NoFactClasses);
    }

    #[test]
    fn emits_file_fact_with_role_and_digest() {
        let model = model_for(
            "file-fact",
            &[("lib/Widget.pm", "package Widget;\nsub build { 1 }\n1;\n")],
            FactClasses::FILES | FactClasses::SYMBOLS,
        );
        let file = model.file_by_path("lib/Widget.pm").unwrap();
        assert_eq!(file.role, FileRole::Lib);
        assert!(file.digest.as_str().starts_with("fnv64:"));
        assert_eq!(file.parse_status, ParseStatus::Clean);
    }

    #[test]
    fn extracts_package_with_real_range() {
        let model = model_for(
            "pkg-range",
            &[("lib/App.pm", "package App;\nsub run { 1 }\n1;\n")],
            FactClasses::FILES | FactClasses::SYMBOLS,
        );
        let pkg = model.packages.iter().find(|p| p.name == "App").unwrap();
        assert_eq!(pkg.declaration_range.start_line, 0);
        assert_eq!(pkg.declaration_range.start_column_utf8, 0);
        assert_eq!(pkg.confidence, Confidence::High);
    }

    #[test]
    fn extracts_sub_with_real_range_and_package() {
        let model = model_for(
            "sub-range",
            &[("lib/App.pm", "package App;\nsub discount { return 42; }\n1;\n")],
            FactClasses::FILES | FactClasses::SYMBOLS,
        );
        let sub = model
            .symbols
            .iter()
            .find(|s| s.kind == SymbolFactKind::Sub && s.name == "discount")
            .unwrap();
        assert_eq!(sub.declaration_range.start_line, 1, "sub is on line 1 (0-based)");
        assert_eq!(sub.package.as_deref(), Some("App"));
        assert_eq!(sub.visibility, Visibility::Public);
    }

    #[test]
    fn files_only_request_does_not_parse() {
        let model = model_for(
            "files-only",
            &[("lib/App.pm", "package App;\nsub run { 1 }\n1;\n")],
            FactClasses::FILES,
        );
        let file = model.file_by_path("lib/App.pm").unwrap();
        assert_eq!(
            file.parse_status,
            ParseStatus::NotParsed,
            "no parse when symbols not requested"
        );
        assert!(model.packages.is_empty(), "no package facts without SYMBOLS");
        assert!(model.symbols.is_empty(), "no symbol facts without SYMBOLS");
    }

    #[test]
    fn indexes_dist_metadata_files_without_parsing_them() {
        let model = model_for(
            "dist-meta",
            &[("cpanfile", "requires 'strict';\n"), ("lib/App.pm", "package App;\n1;\n")],
            FactClasses::FILES | FactClasses::SYMBOLS,
        );
        let cpanfile = model.file_by_path("cpanfile").unwrap();
        assert_eq!(cpanfile.role, FileRole::DistMetadata);
        assert_eq!(cpanfile.parse_status, ParseStatus::NotParsed, "metadata is not parsed");
    }

    #[cfg(unix)]
    #[test]
    fn circular_directory_symlink_does_not_recurse_forever() {
        // A directory symlink pointing back at its parent would loop forever if
        // the walk followed links. The build must terminate and still index the
        // real file.
        let root = std::env::temp_dir().join("pwc-symlink-loop");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("lib")).unwrap();
        std::fs::write(root.join("lib/App.pm"), "package App;\n1;\n").unwrap();
        // lib/loop -> .. (points back above lib): a cycle.
        std::os::unix::fs::symlink("..", root.join("lib/loop")).unwrap();

        let model = build_project_model(&ProjectModelRequest {
            root: root.to_str().unwrap(),
            fact_classes: FactClasses::FILES,
        })
        .unwrap();
        let _ = std::fs::remove_dir_all(&root);

        assert!(model.file_by_path("lib/App.pm").is_some(), "real file still indexed");
        // The symlink is not followed, so no path traverses through `loop/`.
        assert!(
            model.files.iter().all(|f| !f.relative_path.contains("loop")),
            "symlink entries are not indexed"
        );
    }

    #[test]
    fn every_fact_class_has_a_producer() {
        // All 11 fact classes are implemented — requesting any one never records
        // an "unimplemented-fact-class" limitation.
        let model = model_for(
            "full-coverage",
            &[("lib/App.pm", "package App;\nuse strict;\nsub run { 1 }\n1;\n")],
            FactClasses::all(),
        );
        assert!(
            !model.limitations.iter().any(|l| l.kind == "unimplemented_fact_class"),
            "no fact class is unimplemented; limitations={:?}",
            model.limitations
        );
    }

    #[test]
    fn classifies_test_files() {
        let model = model_for(
            "test-role",
            &[("t/basic.t", "use Test::More;\nok(1);\ndone_testing;\n")],
            FactClasses::FILES,
        );
        let file = model.file_by_path("t/basic.t").unwrap();
        assert_eq!(file.role, FileRole::Test);
    }

    #[test]
    fn is_deterministic_across_builds() {
        let files: &[(&str, &str)] = &[
            ("lib/Zebra.pm", "package Zebra;\nsub z { 1 }\n1;\n"),
            ("lib/Apple.pm", "package Apple;\nsub a { 1 }\n1;\n"),
        ];
        let a = model_for("det-a", files, FactClasses::all());
        let b = model_for("det-b", files, FactClasses::all());
        // File ordering is stable and sorted by path.
        let paths_a: Vec<&str> = a.files.iter().map(|f| f.relative_path.as_str()).collect();
        let paths_b: Vec<&str> = b.files.iter().map(|f| f.relative_path.as_str()).collect();
        assert_eq!(paths_a, paths_b);
        assert!(paths_a.windows(2).all(|w| w[0] <= w[1]), "files sorted: {paths_a:?}");
        // Symbol IDs match across builds (deterministic identity).
        let ids_a: Vec<&str> = a.symbols.iter().map(|s| s.symbol_id.as_str()).collect();
        let ids_b: Vec<&str> = b.symbols.iter().map(|s| s.symbol_id.as_str()).collect();
        assert_eq!(ids_a, ids_b);
    }

    #[test]
    fn parse_failure_records_limitation_not_drop() {
        // Deeply unbalanced braces trip the parser's recursion guard.
        let bad = "{".repeat(5000);
        let model = model_for(
            "parse-fail",
            &[("lib/Bad.pm", &bad)],
            FactClasses::FILES | FactClasses::SYMBOLS,
        );
        // The file is still present.
        assert!(
            model.file_by_path("lib/Bad.pm").is_some(),
            "file fact emitted despite parse issue"
        );
        // Fail-soft: either a parse-failed limitation, or it recovered with no
        // symbols — never a silent drop.
        let had_limitation =
            model.limitations.iter().any(|l| l.id.starts_with("parse-failed:lib/Bad.pm"));
        let file = model.file_by_path("lib/Bad.pm").unwrap();
        assert!(
            had_limitation || file.parse_status == ParseStatus::Clean,
            "parse failure must surface a limitation"
        );
    }

    #[test]
    fn extensionless_shebang_script_is_indexed_as_script() {
        // Regression: `bin/app` with no `.pl` suffix but a real Perl shebang
        // must be indexed and classified `Script`, not silently invisible.
        let model = model_for(
            "shebang-perl",
            &[("bin/app", "#!/usr/bin/perl\nuse strict;\nprint \"hi\\n\";\n")],
            FactClasses::FILES,
        );
        let file = model.file_by_path("bin/app").unwrap();
        assert_eq!(file.role, FileRole::Script, "shebang'd extensionless file is a Script");
    }

    #[test]
    fn extensionless_env_perl_shebang_is_indexed() {
        // The `#!/usr/bin/env perl` form is at least as common as a direct path.
        let model = model_for(
            "shebang-env-perl",
            &[("script/tool", "#!/usr/bin/env perl\nuse strict;\n1;\n")],
            FactClasses::FILES,
        );
        assert!(model.file_by_path("script/tool").is_some(), "env-perl shebang is recognized");
    }

    #[test]
    fn extensionless_non_perl_shebang_is_not_indexed() {
        // A non-Perl extensionless script (shell, in this case) must stay out —
        // the shebang check is deliberately tight.
        let model = model_for(
            "shebang-sh",
            &[("bin/sh-thing", "#!/bin/sh\necho hi\n")],
            FactClasses::FILES,
        );
        assert!(
            model.file_by_path("bin/sh-thing").is_none(),
            "a non-Perl shebang script must not be indexed"
        );
    }

    #[test]
    fn visibility_of_maps_declarators() {
        assert_eq!(visibility_of(Some("my")), Visibility::Private);
        assert_eq!(visibility_of(Some("state")), Visibility::Private);
        assert_eq!(visibility_of(Some("our")), Visibility::Public);
        assert_eq!(visibility_of(Some("local")), Visibility::Public);
        assert_eq!(
            visibility_of(None),
            Visibility::Public,
            "no declarator (sub/package) is public"
        );
        assert_eq!(visibility_of(Some("weird")), Visibility::Unknown);
    }

    #[test]
    fn unreadable_file_records_limitation_not_silent_drop() {
        // Invalid UTF-8 content makes `read_to_string` fail regardless of
        // process privilege (unlike permission bits, which root ignores in CI
        // sandboxes) — a portable trigger for the read-failure path.
        let root = std::env::temp_dir().join("pwc-builder-unreadable");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("lib")).unwrap();
        std::fs::write(root.join("lib/Bad.pm"), [0x50, 0x61, 0x63, 0xFF, 0xFE]).unwrap();

        let model = build_project_model(&ProjectModelRequest {
            root: root.to_str().unwrap(),
            fact_classes: FactClasses::FILES,
        })
        .unwrap();
        let _ = std::fs::remove_dir_all(&root);

        assert!(model.file_by_path("lib/Bad.pm").is_none(), "unreadable file emits no FileRecord");
        assert!(
            model
                .limitations
                .iter()
                .any(|l| l.kind == "read_failure" && l.id.contains("lib/Bad.pm")),
            "read failure surfaces a limitation, not a silent drop; limitations={:?}",
            model.limitations
        );
    }
}
