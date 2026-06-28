use perl_lsp_rs_core::config::{
    DeclaredDependency, DeclaredDependencySource, WorkspaceConfig, detect_declared_dependencies,
    extract_build_pl_requirements, extract_cpanfile_requirements, extract_dist_ini_requirements,
    extract_makefile_pl_requirements, extract_meta_json_requirements,
    extract_meta_yml_requirements,
};
use std::fs;
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn extracts_cpanfile_requirements_with_source_and_kind() {
    let cpanfile = r#"
        requires 'Moo', '2.0';
        requires "JSON::PP";
        recommends 'Data::Dumper';
        test_requires 'Test::More', '1.00';
        # requires 'Ignored::Comment';
    "#;

    let deps = extract_cpanfile_requirements(cpanfile);

    assert!(deps.contains(&DeclaredDependency::new(
        "Moo",
        Some("2.0"),
        "requires",
        DeclaredDependencySource::Cpanfile,
    )));
    assert!(deps.contains(&DeclaredDependency::new(
        "JSON::PP",
        None,
        "requires",
        DeclaredDependencySource::Cpanfile,
    )));
    assert!(deps.contains(&DeclaredDependency::new(
        "Data::Dumper",
        None,
        "recommends",
        DeclaredDependencySource::Cpanfile,
    )));
    assert!(deps.contains(&DeclaredDependency::new(
        "Test::More",
        Some("1.00"),
        "test_requires",
        DeclaredDependencySource::Cpanfile,
    )));
    assert_eq!(deps.len(), 4);
}

#[test]
fn declared_dependency_sources_have_user_facing_display_names() {
    let names = [
        (DeclaredDependencySource::Cpanfile, "cpanfile"),
        (DeclaredDependencySource::MakefilePl, "Makefile.PL"),
        (DeclaredDependencySource::BuildPl, "Build.PL"),
        (DeclaredDependencySource::DistIni, "dist.ini"),
        (DeclaredDependencySource::MetaJson, "META.json"),
        (DeclaredDependencySource::MetaYml, "META.yml"),
    ];

    for (source, expected) in names {
        assert_eq!(source.display_name(), expected);
    }
}

#[test]
fn cpanfile_parser_ignores_dynamic_or_malformed_requires() {
    let cpanfile = r#"
        requires $dynamic_module;
        requires;
        requires 'Valid::Module';
        requires 'Valid::Module';
    "#;

    let deps = extract_cpanfile_requirements(cpanfile);

    assert_eq!(
        deps,
        vec![DeclaredDependency::new(
            "Valid::Module",
            None,
            "requires",
            DeclaredDependencySource::Cpanfile,
        )],
    );
}

#[test]
fn perl_metadata_parsers_ignore_nonliteral_and_malformed_entries() {
    let makefile_pl = r#"
        WriteMakefile(
            PREREQ_PM => 'Not::A::Hash',
            TEST_REQUIRES => {
                'Valid::Test' => '1.00',
            },
        );
    "#;
    let dist_ini = r#"
        author = Someone

        [Prereqs::FromCPANfile]
        Ignored::FromCpanfile = 1

        [Prereqs]
        missing separator
        Bad Module = 1
        Valid::Module = 2.0
        Valid::Module = 3.0
    "#;
    let meta_yml = r#"
name: My-Dist
BareLine
requires:
  MissingSeparator
  Bad Module: 1
  Valid::Yaml: 0.42
"#;

    assert_eq!(
        extract_makefile_pl_requirements(makefile_pl),
        vec![DeclaredDependency::new(
            "Valid::Test",
            Some("1.00"),
            "TEST_REQUIRES",
            DeclaredDependencySource::MakefilePl,
        )],
    );
    assert_eq!(
        extract_dist_ini_requirements(dist_ini),
        vec![DeclaredDependency::new(
            "Valid::Module",
            Some("2.0"),
            "Prereqs",
            DeclaredDependencySource::DistIni,
        )],
    );
    assert_eq!(
        extract_meta_yml_requirements(meta_yml),
        vec![DeclaredDependency::new(
            "Valid::Yaml",
            Some("0.42"),
            "requires",
            DeclaredDependencySource::MetaYml,
        )],
    );
    assert!(extract_meta_json_requirements("{").is_empty());
}

#[test]
fn perl_metadata_parsers_keep_scanning_after_edge_shapes() {
    let makefile_pl = r#"
        my $ignored = 'PREREQ_PM\\not_a_key';
        WriteMakefile(
            PREREQ_PM => { 'Broken::Hash' => '1.0',
            TEST_REQUIRES => {
                'Bad Module' => '1.00',
                'No::Version',
                'Valid::Test' => '1.00',
            },
        );
    "#;
    let nested_meta_json = r#"
        {
          "prereqs": {
            "runtime": {
              "requires": {
                "Bad Module": "1.0",
                "Nested::Number": 2,
                "Nested::Unspecified": null
              }
            }
          }
        }
    "#;
    let top_level_meta_json = r#"
        {
          "requires": {
            "Bad Module": "1.0",
            "Top::Number": 3,
            "Top::Unspecified": null
          }
        }
    "#;

    assert_eq!(
        extract_makefile_pl_requirements(makefile_pl),
        vec![
            DeclaredDependency::new(
                "No::Version",
                None,
                "TEST_REQUIRES",
                DeclaredDependencySource::MakefilePl,
            ),
            DeclaredDependency::new(
                "Valid::Test",
                Some("1.00"),
                "TEST_REQUIRES",
                DeclaredDependencySource::MakefilePl,
            ),
        ],
    );

    assert_eq!(
        extract_meta_json_requirements(nested_meta_json),
        vec![
            DeclaredDependency::new(
                "Nested::Number",
                Some("2"),
                "runtime.requires",
                DeclaredDependencySource::MetaJson,
            ),
            DeclaredDependency::new(
                "Nested::Unspecified",
                None,
                "runtime.requires",
                DeclaredDependencySource::MetaJson,
            ),
        ],
    );
    assert_eq!(
        extract_meta_json_requirements(top_level_meta_json),
        vec![
            DeclaredDependency::new(
                "Top::Number",
                Some("3"),
                "requires",
                DeclaredDependencySource::MetaJson,
            ),
            DeclaredDependency::new(
                "Top::Unspecified",
                None,
                "requires",
                DeclaredDependencySource::MetaJson,
            ),
        ],
    );
}

#[test]
fn extracts_makefile_and_build_pl_requirements() {
    let makefile_pl = r#"
        WriteMakefile(
            NAME => 'My::App',
            PREREQ_PM => {
                'JSON::PP' => '4.0',
                "Moo" => 2.005,
            },
            TEST_REQUIRES => {
                'Test::More' => '1.00',
            },
        );
    "#;
    let build_pl = r#"
        Module::Build->new(
            requires => {
                'Try::Tiny' => '0.31',
            },
            build_requires => {
                'Module::Build' => 0,
            },
        );
    "#;

    let makefile_deps = extract_makefile_pl_requirements(makefile_pl);
    let build_deps = extract_build_pl_requirements(build_pl);

    assert!(makefile_deps.contains(&DeclaredDependency::new(
        "JSON::PP",
        Some("4.0"),
        "PREREQ_PM",
        DeclaredDependencySource::MakefilePl,
    )));
    assert!(makefile_deps.contains(&DeclaredDependency::new(
        "Test::More",
        Some("1.00"),
        "TEST_REQUIRES",
        DeclaredDependencySource::MakefilePl,
    )));
    assert!(build_deps.contains(&DeclaredDependency::new(
        "Try::Tiny",
        Some("0.31"),
        "requires",
        DeclaredDependencySource::BuildPl,
    )));
    assert!(build_deps.contains(&DeclaredDependency::new(
        "Module::Build",
        Some("0"),
        "build_requires",
        DeclaredDependencySource::BuildPl,
    )));
}

#[test]
fn extracts_meta_json_and_dist_ini_requirements() {
    let meta_json = r#"
        {
          "prereqs": {
            "runtime": { "requires": { "JSON::PP": "4.0" } },
            "test": { "requires": { "Test::More": "1.00" } }
          }
        }
    "#;
    let dist_ini = r#"
        [Prereqs]
        Moo = 2.0
        JSON::PP = 4.0

        [Prereqs / TestRequires]
        Test::More = 1.00
    "#;

    let meta_deps = extract_meta_json_requirements(meta_json);
    let dist_ini_deps = extract_dist_ini_requirements(dist_ini);

    assert!(meta_deps.contains(&DeclaredDependency::new(
        "JSON::PP",
        Some("4.0"),
        "runtime.requires",
        DeclaredDependencySource::MetaJson,
    )));
    assert!(meta_deps.contains(&DeclaredDependency::new(
        "Test::More",
        Some("1.00"),
        "test.requires",
        DeclaredDependencySource::MetaJson,
    )));
    assert!(dist_ini_deps.contains(&DeclaredDependency::new(
        "Moo",
        Some("2.0"),
        "Prereqs",
        DeclaredDependencySource::DistIni,
    )));
    assert!(dist_ini_deps.contains(&DeclaredDependency::new(
        "Test::More",
        Some("1.00"),
        "Prereqs / TestRequires",
        DeclaredDependencySource::DistIni,
    )));
}

#[test]
fn extracts_meta_yml_requirements() {
    let meta_yml = r#"
prereqs:
  runtime:
    requires:
      JSON::PP: 4.0
  test:
    requires:
      Test::More: 1.00
"#;

    let deps = extract_meta_yml_requirements(meta_yml);

    assert!(deps.contains(&DeclaredDependency::new(
        "JSON::PP",
        Some("4.0"),
        "requires",
        DeclaredDependencySource::MetaYml,
    )));
    assert!(deps.contains(&DeclaredDependency::new(
        "Test::More",
        Some("1.00"),
        "requires",
        DeclaredDependencySource::MetaYml,
    )));
}

#[test]
fn detects_declared_dependencies_from_workspace_metadata_files() -> TestResult {
    let temp = TempDir::new()?;
    fs::write(
        temp.path().join("cpanfile"),
        "requires 'JSON::PP', '4.0';\ntest_requires 'Test::More';\n",
    )?;
    fs::write(
        temp.path().join("Makefile.PL"),
        "WriteMakefile(PREREQ_PM => { 'Moo' => '2.0' });\n",
    )?;
    fs::write(
        temp.path().join("Build.PL"),
        "Module::Build->new(requires => { 'Try::Tiny' => '0.31' });\n",
    )?;
    fs::write(temp.path().join("dist.ini"), "[Prereqs]\nPath::Tiny = 0.146\n")?;
    fs::write(temp.path().join("META.json"), r#"{"requires":{"CPAN::Meta":"2.150010"}}"#)?;
    fs::write(temp.path().join("META.yml"), "requires:\n  YAML::PP: 0.38\n")?;

    let deps = detect_declared_dependencies(temp.path());

    assert!(deps.contains(&DeclaredDependency::new(
        "JSON::PP",
        Some("4.0"),
        "requires",
        DeclaredDependencySource::Cpanfile,
    )));
    assert!(deps.contains(&DeclaredDependency::new(
        "Moo",
        Some("2.0"),
        "PREREQ_PM",
        DeclaredDependencySource::MakefilePl,
    )));
    assert!(deps.contains(&DeclaredDependency::new(
        "Try::Tiny",
        Some("0.31"),
        "requires",
        DeclaredDependencySource::BuildPl,
    )));
    assert!(deps.contains(&DeclaredDependency::new(
        "Path::Tiny",
        Some("0.146"),
        "Prereqs",
        DeclaredDependencySource::DistIni,
    )));
    assert!(deps.contains(&DeclaredDependency::new(
        "CPAN::Meta",
        Some("2.150010"),
        "requires",
        DeclaredDependencySource::MetaJson,
    )));
    assert!(deps.contains(&DeclaredDependency::new(
        "YAML::PP",
        Some("0.38"),
        "requires",
        DeclaredDependencySource::MetaYml,
    )));
    assert_eq!(deps.len(), 7);
    Ok(())
}

#[test]
fn workspace_config_refreshes_declared_dependency_cache() -> TestResult {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("cpanfile"), "requires 'JSON::PP', '4.0';\n")?;

    let mut config = WorkspaceConfig::default();
    config.refresh_declared_dependencies(temp.path());

    assert_eq!(
        config.declared_dependencies,
        vec![DeclaredDependency::new(
            "JSON::PP",
            Some("4.0"),
            "requires",
            DeclaredDependencySource::Cpanfile,
        )],
    );
    Ok(())
}
