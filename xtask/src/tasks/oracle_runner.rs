//! Differential oracle runner — vertical slice.
//!
//! Implements the general runner described in #2386:
//! - Fixture manifest loader (#2528)
//! - Hermetic Perl execution harness (#2532)
//! - PackageSubTable per-class extractor (one class, end-to-end — #2540)
//! - Receipt emission
//! - `check-oracle-compare` xtask subcommand
//!
//! The other comparison classes (ImportExport, IsaComposition, ConstantPrototype,
//! FrameworkGeneratedMember, CompileEffect) are follow-up work.

use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

const MANIFEST_PATH: &str = "crates/perl-corpus/fixtures/differential_oracle/manifest.json";

/// Perl execution timeout for oracle probes.
const PERL_TIMEOUT_SECS: u64 = 5;

// ── Manifest types (mirrors oracle_fixture_manifest.rs for loading) ─────────

#[derive(Debug, Deserialize)]
struct OracleFixtureManifest {
    fixtures: Vec<ManifestFixture>,
}

#[derive(Debug, Deserialize, Clone)]
struct ManifestFixture {
    id: String,
    source: String,
    comparison_classes: Vec<String>,
    module_roots: Vec<String>,
    environment_denials: Vec<String>,
    claim_boundary: String,
}

// ── Public types ─────────────────────────────────────────────────────────────

/// A loaded fixture ready for comparison.
#[derive(Debug, Clone)]
pub struct Fixture {
    pub id: String,
    pub source_path: PathBuf,
    pub source_text: String,
    pub comparison_classes: Vec<String>,
    pub module_roots: Vec<PathBuf>,
    pub environment_denials: Vec<String>,
    pub claim_boundary: String,
}

/// Output from the Perl execution harness.
#[derive(Debug)]
pub struct PerlOutput {
    /// Perl version string (e.g. "v5.38.0"), available for receipt emission.
    #[allow(dead_code)]
    pub perl_version: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// A normalized comparison fact: (family, name).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct NormalizedFact {
    pub family: String,
    pub name: String,
}

/// A disagreement between Rust extractor and Perl oracle.
#[derive(Debug, Clone, Serialize)]
pub struct FactDisagreement {
    pub family: String,
    pub name: String,
    /// `"rust_only"` or `"perl_only"`
    pub side: String,
}

/// A receipt emitted after running one comparison class against one fixture.
#[derive(Debug, Serialize)]
pub struct OracleCompareReceipt {
    pub schema_version: &'static str,
    pub fixture_id: String,
    pub comparison_class: String,
    pub perl_version: String,
    pub matched_facts: Vec<NormalizedFact>,
    pub disagreements: Vec<FactDisagreement>,
    pub provider_behavior_changed: bool,
    pub editor_runtime_dependency: bool,
    pub claim_boundary: String,
}

impl OracleCompareReceipt {
    fn new(
        fixture_id: String,
        comparison_class: String,
        perl_version: String,
        rust_facts: BTreeSet<NormalizedFact>,
        perl_facts: BTreeSet<NormalizedFact>,
        claim_boundary: String,
    ) -> Self {
        let matched_facts = rust_facts.intersection(&perl_facts).cloned().collect::<Vec<_>>();
        let rust_only = rust_facts.difference(&perl_facts).map(|f| FactDisagreement {
            family: f.family.clone(),
            name: f.name.clone(),
            side: "rust_only".to_string(),
        });
        let perl_only = perl_facts.difference(&rust_facts).map(|f| FactDisagreement {
            family: f.family.clone(),
            name: f.name.clone(),
            side: "perl_only".to_string(),
        });
        let disagreements = rust_only.chain(perl_only).collect::<Vec<_>>();

        Self {
            schema_version: "oracle_receipt.v1",
            fixture_id,
            comparison_class,
            perl_version,
            matched_facts,
            disagreements,
            provider_behavior_changed: false,
            editor_runtime_dependency: false,
            claim_boundary,
        }
    }
}

// ── Fixture loader ────────────────────────────────────────────────────────────

/// Loads oracle fixtures from the manifest.
pub struct FixtureLoader {
    root: PathBuf,
}

impl FixtureLoader {
    /// Create a loader rooted at the given project root.
    pub fn from_root(root: PathBuf) -> Self {
        Self { root }
    }

    /// Load all fixtures declared in the manifest.
    pub fn load_all(&self) -> Result<Vec<Fixture>> {
        let manifest_path = self.root.join(MANIFEST_PATH);
        let text = fs::read_to_string(&manifest_path)
            .with_context(|| format!("failed to read {}", manifest_path.display()))?;
        let manifest: OracleFixtureManifest = serde_json::from_str(&text)
            .with_context(|| format!("failed to parse {}", manifest_path.display()))?;

        manifest.fixtures.iter().map(|mf| self.load_fixture(mf)).collect()
    }

    /// Load a single fixture by id.
    #[allow(dead_code)]
    pub fn load_by_id(&self, id: &str) -> Result<Fixture> {
        let all = self.load_all()?;
        all.into_iter()
            .find(|f| f.id == id)
            .ok_or_else(|| color_eyre::eyre::eyre!("no fixture with id {:?} in manifest", id))
    }

    fn load_fixture(&self, mf: &ManifestFixture) -> Result<Fixture> {
        let source_path = self.root.join(&mf.source);
        if !source_path.exists() {
            bail!("fixture {:?}: source path does not exist: {}", mf.id, source_path.display());
        }
        let source_text = fs::read_to_string(&source_path)
            .with_context(|| format!("failed to read fixture source {}", source_path.display()))?;

        let module_roots = mf
            .module_roots
            .iter()
            .map(|rel| {
                let path = self.root.join(rel);
                if !path.exists() {
                    bail!("fixture {:?}: module_root does not exist: {}", mf.id, path.display());
                }
                Ok(path)
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Fixture {
            id: mf.id.clone(),
            source_path,
            source_text,
            comparison_classes: mf.comparison_classes.clone(),
            module_roots,
            environment_denials: mf.environment_denials.clone(),
            claim_boundary: mf.claim_boundary.clone(),
        })
    }
}

// ── Perl execution harness ────────────────────────────────────────────────────

/// Hermetic Perl execution harness.
///
/// Provides the shared `query_perl_version` helper used by extractors.
/// Each extractor implements its own `run_probe` with piped stdio and timeout.
pub struct PerlExecutionHarness;

impl PerlExecutionHarness {
    fn query_perl_version() -> Result<String> {
        let output = Command::new("perl")
            .arg("-e")
            .arg("print $^V")
            .env_remove("PERL5LIB")
            .env_remove("PERL5OPT")
            .env("LC_ALL", "C")
            .output()
            .context("query Perl version for oracle harness")?;

        if !output.status.success() {
            bail!("Perl version probe failed: {}", String::from_utf8_lossy(&output.stderr));
        }
        String::from_utf8(output.stdout).context("decode Perl version output")
    }
}

// ── PackageSubTable extractor ─────────────────────────────────────────────────

/// Perl probe script for PackageSubTable extraction.
///
/// Snapshots `%main::` before and after loading the fixture, then reports
/// only newly-introduced packages and subs. This avoids counting ambient
/// packages brought in by `use strict`, `use warnings`, and Perl builtins.
///
/// Emits tab-separated lines: `package\t<name>` and `sub\t<qualified_name>`.
const PACKAGE_SUB_TABLE_PROBE: &str = r#"
my $file = shift @ARGV;
die "usage: oracle_probe.pl <file>\n" unless defined $file;

no strict 'refs';

# Recursively snapshot all package names reachable from main:: before loading
# the fixture so we can subtract ambient namespaces (strict, warnings, builtins).
my %before_pkgs;
sub snapshot_pkgs {
    my ($stash_name) = @_;
    return if $before_pkgs{$stash_name}++;
    for my $sym (keys %{"${stash_name}::"}) {
        next unless $sym =~ /::$/;
        my $child = $sym;
        $child =~ s/::$//;
        next unless length $child;
        my $full = $stash_name eq 'main' ? $child : "${stash_name}::${child}";
        snapshot_pkgs($full);
    }
}
snapshot_pkgs('main');

my $loaded = do $file;
if (!$loaded) {
    die $@ || $! || "do $file returned false\n";
}

# Recursively enumerate all packages and subs introduced after loading.
# Only report a package if it was not present before, or now has new subs.
my %visited;
sub enumerate_pkgs {
    my ($stash_name) = @_;
    return if $visited{$stash_name}++;
    my $is_new = !exists $before_pkgs{$stash_name};

    # A package is "substantive" if it has any direct symbols other than
    # sub-namespace entries (i.e., subs, scalars, arrays, or hashes).
    # Intermediate namespaces created implicitly by nested package declarations
    # (e.g., Oracle:: when Oracle::E2E is declared) have only ::-suffixed entries
    # and are not emitted.
    my $has_direct_syms = 0;
    for my $sym (sort keys %{"${stash_name}::"}) {
        if ($sym =~ /::$/) {
            my $child = $sym;
            $child =~ s/::$//;
            next unless length $child;
            my $full = $stash_name eq 'main' ? $child : "${stash_name}::${child}";
            enumerate_pkgs($full);
        } elsif ($is_new) {
            if (defined &{"${stash_name}::${sym}"}) {
                print "sub\t${stash_name}::${sym}\n";
                $has_direct_syms = 1;
            } elsif (defined ${"${stash_name}::${sym}"}) {
                $has_direct_syms = 1;
            }
        }
    }

    if ($is_new && $stash_name ne 'main' && $has_direct_syms) {
        print "package\t${stash_name}\n";
    }
}
enumerate_pkgs('main');
"#;

/// Extractor for the `PackageSubTable` comparison class.
///
/// Rust side: uses compile effects from the HIR (DeclarePackage, DeclareSub).
/// Perl side: introspects the symbol table after loading the fixture.
pub struct PackageSubTableExtractor;

impl PackageSubTableExtractor {
    /// Extract facts from the Rust HIR side.
    pub fn extract_rust_facts(source: &str) -> Result<BTreeSet<NormalizedFact>> {
        use perl_parser_core::Parser;
        use perl_parser_core::hir::{CompileEffectKind, lower_ast};

        let mut parser = Parser::new(source);
        let output = parser.parse_with_recovery();
        let hir = lower_ast(&output.ast);

        let facts =
            hir.compile_effects()
                .iter()
                .filter_map(|effect| match effect.kind {
                    CompileEffectKind::DeclarePackage => effect.fact_name.as_ref().map(|name| {
                        NormalizedFact { family: "package".into(), name: name.clone() }
                    }),
                    CompileEffectKind::DeclareSub => {
                        effect.fact_name.as_ref().map(|name| NormalizedFact {
                            family: "sub".into(),
                            name: qualify_name(effect.package_context.as_deref(), name),
                        })
                    }
                    _ => None,
                })
                .collect();
        Ok(facts)
    }

    /// Extract facts from the Perl oracle side.
    pub fn extract_perl_facts(fixture: &Fixture) -> Result<BTreeSet<NormalizedFact>> {
        let output = Self::run_probe(fixture)?;
        if output.exit_code != 0 {
            bail!(
                "PackageSubTable Perl probe failed (exit {}): {}",
                output.exit_code,
                output.stderr
            );
        }
        parse_tab_separated_facts(&output.stdout)
    }

    fn run_probe(fixture: &Fixture) -> Result<PerlOutput> {
        use std::process::Stdio;
        use std::thread;

        let tempdir = tempfile::tempdir().context("create PackageSubTable harness tempdir")?;

        let fixture_filename = fixture
            .source_path
            .file_name()
            .ok_or_else(|| color_eyre::eyre::eyre!("fixture source has no file name"))?;
        let fixture_in_temp = tempdir.path().join(fixture_filename);
        fs::write(&fixture_in_temp, &fixture.source_text)
            .context("write fixture to PackageSubTable harness tempdir")?;

        let perl_version = PerlExecutionHarness::query_perl_version()?;

        let mut cmd = Command::new("perl");
        for root in &fixture.module_roots {
            cmd.arg("-I").arg(root);
        }
        cmd.arg("-I").arg(tempdir.path());
        cmd.arg("-e").arg(PACKAGE_SUB_TABLE_PROBE);
        cmd.arg(&fixture_in_temp);

        for denial in &fixture.environment_denials {
            if denial == "local::lib" {
                cmd.env_remove("PERL_LOCAL_LIB_ROOT");
                cmd.env_remove("PERL_MB_OPT");
            } else {
                cmd.env_remove(denial.as_str());
            }
        }
        cmd.env_remove("PERL5LIB");
        cmd.env_remove("PERL5OPT");
        cmd.env("LC_ALL", "C");
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let timeout = Duration::from_secs(PERL_TIMEOUT_SECS);
        let mut child = cmd.spawn().context("spawn PackageSubTable Perl probe")?;

        let start = std::time::Instant::now();
        loop {
            match child.try_wait().context("polling Perl oracle probe process")? {
                Some(_) => break,
                None => {
                    if start.elapsed() >= timeout {
                        let _ = child.kill();
                        bail!("PackageSubTable Perl probe timed out after {}s", timeout.as_secs());
                    }
                    thread::sleep(Duration::from_millis(50));
                }
            }
        }
        let raw = child.wait_with_output().context("collect PackageSubTable probe output")?;
        let stdout = String::from_utf8(raw.stdout).context("decode probe stdout")?;
        let stderr = String::from_utf8(raw.stderr).context("decode probe stderr")?;
        Ok(PerlOutput { perl_version, stdout, stderr, exit_code: raw.status.code().unwrap_or(-1) })
    }
}

fn parse_tab_separated_facts(stdout: &str) -> Result<BTreeSet<NormalizedFact>> {
    let mut facts = BTreeSet::new();
    for line in stdout.lines().filter(|l| !l.trim().is_empty()) {
        let mut parts = line.splitn(2, '\t');
        let family = parts
            .next()
            .filter(|v| !v.is_empty())
            .ok_or_else(|| color_eyre::eyre::eyre!("oracle probe line missing family: {line:?}"))?;
        let name = parts
            .next()
            .filter(|v| !v.is_empty())
            .ok_or_else(|| color_eyre::eyre::eyre!("oracle probe line missing name: {line:?}"))?;
        let family = match family {
            "package" | "sub" => family.to_string(),
            other => bail!("unknown fact family from PackageSubTable probe: {other:?}"),
        };
        facts.insert(NormalizedFact { family, name: name.to_string() });
    }
    Ok(facts)
}

fn qualify_name(package: Option<&str>, name: &str) -> String {
    if name.contains("::") {
        return name.to_string();
    }
    match package {
        Some(p) if !p.is_empty() => format!("{p}::{name}"),
        _ => name.to_string(),
    }
}

// ── xtask entry point ─────────────────────────────────────────────────────────

/// Run the oracle comparison for all fixtures that include `PackageSubTable`.
pub fn run() -> Result<()> {
    let root = project_root()?;
    let loader = FixtureLoader::from_root(root.clone());
    let fixtures = loader.load_all()?;

    let package_sub_fixtures: Vec<_> = fixtures
        .iter()
        .filter(|f| f.comparison_classes.iter().any(|c| c == "PackageSubTable"))
        .collect();

    if package_sub_fixtures.is_empty() {
        println!("oracle-compare: no PackageSubTable fixtures declared");
        return Ok(());
    }

    let mut all_passed = true;
    for fixture in &package_sub_fixtures {
        println!("oracle-compare: running PackageSubTable on fixture {:?}", fixture.id);

        let rust_facts = PackageSubTableExtractor::extract_rust_facts(&fixture.source_text)
            .with_context(|| {
                format!("PackageSubTable Rust extraction failed for {:?}", fixture.id)
            })?;

        let perl_facts = PackageSubTableExtractor::extract_perl_facts(fixture)
            .with_context(|| format!("PackageSubTable Perl probe failed for {:?}", fixture.id))?;

        let receipt = OracleCompareReceipt::new(
            fixture.id.clone(),
            "PackageSubTable".to_string(),
            // Perl version is embedded in the receipt via perl_facts extraction,
            // but we query separately for a clean value.
            PerlExecutionHarness::query_perl_version().unwrap_or_default(),
            rust_facts,
            perl_facts,
            fixture.claim_boundary.clone(),
        );

        let rendered =
            serde_json::to_string_pretty(&receipt).context("serialize oracle compare receipt")?;

        if receipt.disagreements.is_empty() {
            println!("  PASS: {} matched facts, 0 disagreements", receipt.matched_facts.len());
        } else {
            eprintln!("  FAIL: {} disagreements:", receipt.disagreements.len());
            eprintln!("{rendered}");
            all_passed = false;
        }

        // Emit receipt to target/receipts/oracle/ directory.
        let receipt_dir = root.join("target/receipts/oracle");
        fs::create_dir_all(&receipt_dir).context("create oracle receipt dir")?;
        let receipt_path = receipt_dir.join(format!("{}-PackageSubTable.json", fixture.id));
        fs::write(&receipt_path, &rendered)
            .with_context(|| format!("write oracle receipt {}", receipt_path.display()))?;
        println!("  receipt: {}", receipt_path.display());
    }

    if all_passed {
        println!("oracle-compare: all PackageSubTable comparisons passed");
        Ok(())
    } else {
        bail!("oracle-compare: PackageSubTable disagreements detected")
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    type TestResult<T = ()> = Result<T>;

    // ── FixtureLoader tests ───────────────────────────────────────────────────

    #[test]
    fn fixture_loader_loads_manifest_fixtures() -> TestResult {
        let ws = make_workspace()?;
        let loader = FixtureLoader::from_root(ws.path().to_path_buf());

        let fixtures = loader.load_all()?;

        assert!(!fixtures.is_empty(), "expected at least one fixture");
        let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
        assert!(ids.contains(&"pkg_test"), "expected 'pkg_test' fixture, got: {ids:?}");
        Ok(())
    }

    #[test]
    fn fixture_loader_validates_source_paths_exist() -> TestResult {
        let ws = make_workspace()?;
        let loader = FixtureLoader::from_root(ws.path().to_path_buf());

        let fixtures = loader.load_all()?;

        for fixture in &fixtures {
            assert!(
                fixture.source_path.exists(),
                "fixture {:?} source_path does not exist: {}",
                fixture.id,
                fixture.source_path.display()
            );
        }
        Ok(())
    }

    #[test]
    fn fixture_loader_rejects_missing_source() -> TestResult {
        let ws = make_workspace()?;
        let manifest_path = ws.path().join(MANIFEST_PATH);
        let text = fs::read_to_string(&manifest_path)?;
        fs::write(
            &manifest_path,
            text.replace(
                r#""source": "fixtures/pkg_test.pl""#,
                r#""source": "fixtures/does_not_exist.pl""#,
            ),
        )?;
        let loader = FixtureLoader::from_root(ws.path().to_path_buf());

        let err = loader.load_all().expect_err("missing source should fail");

        assert!(err.to_string().contains("does not exist"), "unexpected error: {err:?}");
        Ok(())
    }

    #[test]
    fn fixture_loader_populates_module_roots() -> TestResult {
        let ws = make_workspace()?;
        let loader = FixtureLoader::from_root(ws.path().to_path_buf());

        let fixtures = loader.load_all()?;

        let fixture =
            fixtures.iter().find(|f| f.id == "pkg_test").expect("expected pkg_test fixture");
        assert_eq!(fixture.module_roots.len(), 1, "expected exactly one module root");
        assert!(
            fixture.module_roots[0].exists(),
            "module root does not exist: {}",
            fixture.module_roots[0].display()
        );
        Ok(())
    }

    // ── parse_tab_separated_facts tests ──────────────────────────────────────

    #[test]
    fn parse_facts_handles_package_and_sub_lines() -> TestResult {
        let stdout = "package\tFoo\nsub\tFoo::bar\npackage\tFoo::Nested\n";

        let facts = parse_tab_separated_facts(stdout)?;

        assert!(facts.contains(&NormalizedFact { family: "package".into(), name: "Foo".into() }));
        assert!(facts.contains(&NormalizedFact { family: "sub".into(), name: "Foo::bar".into() }));
        assert!(
            facts
                .contains(&NormalizedFact { family: "package".into(), name: "Foo::Nested".into() })
        );
        Ok(())
    }

    #[test]
    fn parse_facts_rejects_unknown_family() -> TestResult {
        let stdout = "unknown_family\tFoo::bar\n";

        let err = parse_tab_separated_facts(stdout).expect_err("unknown family should fail");

        assert!(err.to_string().contains("unknown fact family"), "unexpected error: {err:?}");
        Ok(())
    }

    #[test]
    fn parse_facts_skips_blank_lines() -> TestResult {
        let stdout = "\npackage\tFoo\n\nsub\tFoo::bar\n\n";

        let facts = parse_tab_separated_facts(stdout)?;

        assert_eq!(facts.len(), 2);
        Ok(())
    }

    // ── qualify_name tests ────────────────────────────────────────────────────

    #[test]
    fn qualify_name_already_qualified_is_unchanged() {
        assert_eq!(qualify_name(Some("Foo"), "Foo::bar"), "Foo::bar");
    }

    #[test]
    fn qualify_name_unqualified_with_package() {
        assert_eq!(qualify_name(Some("Foo"), "bar"), "Foo::bar");
    }

    #[test]
    fn qualify_name_no_package_returns_bare() {
        assert_eq!(qualify_name(None, "bar"), "bar");
    }

    // ── OracleCompareReceipt tests ────────────────────────────────────────────

    #[test]
    fn receipt_records_matched_and_disagreements() {
        let rust_facts: BTreeSet<_> = [
            NormalizedFact { family: "package".into(), name: "Foo".into() },
            NormalizedFact { family: "sub".into(), name: "Foo::bar".into() },
            NormalizedFact { family: "sub".into(), name: "Foo::rust_only".into() },
        ]
        .into();
        let perl_facts: BTreeSet<_> = [
            NormalizedFact { family: "package".into(), name: "Foo".into() },
            NormalizedFact { family: "sub".into(), name: "Foo::bar".into() },
            NormalizedFact { family: "sub".into(), name: "Foo::perl_only".into() },
        ]
        .into();

        let receipt = OracleCompareReceipt::new(
            "test_fixture".into(),
            "PackageSubTable".into(),
            "v5.38.0".into(),
            rust_facts,
            perl_facts,
            "test claim boundary".into(),
        );

        assert_eq!(receipt.matched_facts.len(), 2, "expected 2 matched facts");
        assert_eq!(receipt.disagreements.len(), 2, "expected 2 disagreements");

        let rust_only: Vec<_> =
            receipt.disagreements.iter().filter(|d| d.side == "rust_only").collect();
        let perl_only: Vec<_> =
            receipt.disagreements.iter().filter(|d| d.side == "perl_only").collect();
        assert_eq!(rust_only.len(), 1);
        assert_eq!(perl_only.len(), 1);
        assert_eq!(rust_only[0].name, "Foo::rust_only");
        assert_eq!(perl_only[0].name, "Foo::perl_only");
    }

    #[test]
    fn receipt_has_no_disagreements_when_facts_agree() {
        let facts: BTreeSet<_> = [
            NormalizedFact { family: "package".into(), name: "Foo".into() },
            NormalizedFact { family: "sub".into(), name: "Foo::bar".into() },
        ]
        .into();

        let receipt = OracleCompareReceipt::new(
            "test_fixture".into(),
            "PackageSubTable".into(),
            "v5.38.0".into(),
            facts.clone(),
            facts,
            "test claim boundary".into(),
        );

        assert!(receipt.disagreements.is_empty());
        assert_eq!(receipt.matched_facts.len(), 2);
    }

    #[test]
    fn receipt_editor_runtime_dependency_is_always_false() {
        let receipt = OracleCompareReceipt::new(
            "f".into(),
            "PackageSubTable".into(),
            "v5.38.0".into(),
            BTreeSet::new(),
            BTreeSet::new(),
            "claim".into(),
        );
        assert!(!receipt.editor_runtime_dependency);
    }

    // ── PackageSubTableExtractor Rust side (no Perl required) ─────────────────

    #[test]
    fn rust_extractor_finds_packages_and_subs() -> TestResult {
        let source = "package Foo;\nsub bar { 1 }\nsub baz { 2 }\n1;\n";

        let facts = PackageSubTableExtractor::extract_rust_facts(source)?;

        assert!(
            facts.contains(&NormalizedFact { family: "package".into(), name: "Foo".into() }),
            "expected Foo package; got: {facts:?}"
        );
        assert!(
            facts.contains(&NormalizedFact { family: "sub".into(), name: "Foo::bar".into() }),
            "expected Foo::bar sub; got: {facts:?}"
        );
        assert!(
            facts.contains(&NormalizedFact { family: "sub".into(), name: "Foo::baz".into() }),
            "expected Foo::baz sub; got: {facts:?}"
        );
        Ok(())
    }

    #[test]
    fn rust_extractor_qualifies_names_correctly() -> TestResult {
        let source = "package Acme::Base;\nsub inherited { 1 }\npackage Acme::Derived;\nour @ISA = qw(Acme::Base);\nsub own { 1 }\n1;\n";

        let facts = PackageSubTableExtractor::extract_rust_facts(source)?;

        assert!(
            facts.contains(&NormalizedFact {
                family: "sub".into(),
                name: "Acme::Base::inherited".into()
            }),
            "expected qualified sub; got: {facts:?}"
        );
        assert!(
            facts.contains(&NormalizedFact {
                family: "sub".into(),
                name: "Acme::Derived::own".into()
            }),
            "expected qualified sub; got: {facts:?}"
        );
        Ok(())
    }

    // ── End-to-end with real Perl (skipped when perl unavailable) ────────────

    #[test]
    fn end_to_end_package_sub_table_agrees_on_fixture() -> TestResult {
        if !perl_available() {
            eprintln!("SKIP: perl not available in this environment");
            return Ok(());
        }

        let source = r#"
package Oracle::E2E;
sub alpha { 1 }
sub beta { 2 }
1;
"#;
        // Build a temporary fixture.
        let tempdir = tempfile::tempdir()?;
        let fixture_dir = tempdir.path().join("fixtures");
        fs::create_dir_all(&fixture_dir)?;
        let src_path = fixture_dir.join("oracle_e2e.pl");
        fs::write(&src_path, source)?;

        let fixture = Fixture {
            id: "oracle_e2e".into(),
            source_path: src_path,
            source_text: source.to_string(),
            comparison_classes: vec!["PackageSubTable".into()],
            module_roots: vec![fixture_dir.clone()],
            environment_denials: vec!["PERL5LIB".into(), "PERL5OPT".into(), "local::lib".into()],
            claim_boundary: "test".into(),
        };

        let rust_facts = PackageSubTableExtractor::extract_rust_facts(source)?;
        let perl_facts = PackageSubTableExtractor::extract_perl_facts(&fixture)?;

        let receipt = OracleCompareReceipt::new(
            fixture.id.clone(),
            "PackageSubTable".into(),
            "v5.x".into(),
            rust_facts,
            perl_facts,
            fixture.claim_boundary.clone(),
        );

        let rendered = serde_json::to_string_pretty(&receipt)?;
        println!("{rendered}");

        assert!(
            receipt.disagreements.is_empty(),
            "PackageSubTable oracle disagreements: {rendered}"
        );
        assert!(
            receipt.matched_facts.iter().any(|f| f.family == "package" && f.name == "Oracle::E2E"),
            "expected Oracle::E2E package in matched facts; got: {rendered}"
        );
        Ok(())
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn perl_available() -> bool {
        Command::new("perl")
            .arg("-e")
            .arg("1")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn make_workspace() -> TestResult<tempfile::TempDir> {
        let tempdir = tempfile::tempdir()?;
        let root = tempdir.path();

        // Minimal directory structure for manifest loading.
        fs::create_dir_all(root.join("crates/perl-corpus/fixtures/differential_oracle"))?;
        fs::create_dir_all(root.join("fixtures"))?;
        fs::write(root.join("fixtures/pkg_test.pl"), "package PkgTest;\nsub foo { 1 }\n1;\n")?;
        fs::write(root.join(MANIFEST_PATH), minimal_manifest_json())?;

        Ok(tempdir)
    }

    fn minimal_manifest_json() -> String {
        r#"{
  "schema_version": "oracle_fixture_manifest.v1",
  "manifest": "differential-real-perl-oracle-fixtures",
  "owner": "test",
  "status": "declaration-only",
  "updated": "2026-06-21",
  "spec": "docs/specs/PLSP-SPEC-0027-differential-real-perl-oracle.md",
  "runner": "none",
  "editor_runtime_dependency": false,
  "comparison_classes": ["PackageSubTable"],
  "result_classes": ["oracle_agrees"],
  "required_environment_denials": ["PERL5LIB", "PERL5OPT", "local::lib"],
  "default_claim_boundary": "test claim boundary; no oracle runner, Perl execution, provider behavior, support-tier promotion, or parser/corpus bucket movement.",
  "fixtures": [
    {
      "id": "pkg_test",
      "source": "fixtures/pkg_test.pl",
      "path_class": "public_test_fixture",
      "perl_version_constraint": "any-supported-real-perl",
      "include_path_authority": "declared_fixture_root",
      "module_roots": ["fixtures"],
      "environment_denials": ["PERL5LIB", "PERL5OPT", "local::lib"],
      "comparison_classes": ["PackageSubTable"],
      "dynamic_boundaries": [],
      "unsupported_effects": [],
      "framework_adapters": [],
      "claim_boundary": "test fixture claim; no oracle runner, Perl execution, provider behavior, support-tier promotion, or parser/corpus bucket movement."
    }
  ]
}"#
        .to_string()
    }
}
