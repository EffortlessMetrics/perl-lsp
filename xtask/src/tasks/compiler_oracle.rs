//! Differential proof helpers for compiler-substrate facts.
//!
//! This module keeps real Perl as a conformance oracle for selected bounded
//! facts. It is not used by LSP providers.

#[cfg(test)]
mod tests {
    use anyhow::{Context, Result, bail};
    use perl_parser_core::Parser;
    use perl_parser_core::hir::{CompileEffect, CompileEffectKind, HirFile, lower_ast};
    use serde::Serialize;
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    const FIXTURE_ID: &str = "compile_effect_basic_stash_facts";
    const FIXTURE_SOURCE: &str = r#"
package Oracle::Base;
sub inherited { 1 }

package Oracle::Demo;
our @ISA = qw(Oracle::Base);
use constant ANSWER => 42;
sub proto ($) { 1 }
sub ordinary { 1 }
1;
"#;

    const ORACLE_PROBE: &str = r#"
use strict;
use warnings;

my $file = shift @ARGV;
my $loaded = do $file;
if (!$loaded) {
    die $@ || $!;
}

for my $package (qw(Oracle::Base Oracle::Demo)) {
    no strict 'refs';
    print "package\t$package\n" if scalar keys %{"${package}::"};
}

for my $sub (qw(Oracle::Base::inherited Oracle::Demo::proto Oracle::Demo::ordinary)) {
    no strict 'refs';
    print "sub\t$sub\n" if defined &{$sub};
}

for my $sub (qw(Oracle::Demo::proto)) {
    my $prototype = prototype($sub);
    print "prototype\t$sub\n" if defined $prototype;
}

{
    no strict 'refs';
    for my $parent (@{"Oracle::Demo::ISA"}) {
        print "isa\tOracle::Demo->$parent\n";
    }
}

{
    no strict 'refs';
    my $constant = "Oracle::Demo::ANSWER";
    print "constant\t$constant\n" if defined &{$constant} && defined prototype($constant);
}
"#;

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
    struct NormalizedFact {
        family: &'static str,
        name: String,
    }

    #[derive(Debug, Clone, Serialize)]
    struct FactDisagreement {
        family: &'static str,
        name: String,
        side: &'static str,
    }

    #[derive(Debug, Clone, Serialize)]
    struct DifferentialReceipt {
        fixture_id: &'static str,
        perl_version: String,
        compared_families: Vec<&'static str>,
        matched_facts: Vec<NormalizedFact>,
        disagreements: Vec<FactDisagreement>,
    }

    #[test]
    fn compiler_compile_effect_oracle_compares_selected_facts() -> Result<()> {
        let hir = lower_source(FIXTURE_SOURCE);
        let rust_facts = normalize_rust_compile_effects(&hir);
        let observed = run_perl_oracle(FIXTURE_SOURCE)?;

        let receipt = compare_facts(observed.perl_version, rust_facts, observed.facts);
        let rendered = serde_json::to_string_pretty(&receipt)
            .context("serialize compile-effect oracle receipt")?;
        println!("{rendered}");

        assert!(
            receipt.disagreements.is_empty(),
            "compile-effect oracle disagreements: {rendered}"
        );
        assert!(
            receipt.compared_families.iter().any(|family| *family == "package")
                && receipt.compared_families.iter().any(|family| *family == "sub")
                && receipt.compared_families.iter().any(|family| *family == "constant")
                && receipt.compared_families.iter().any(|family| *family == "prototype")
                && receipt.compared_families.iter().any(|family| *family == "isa"),
            "receipt should cover the selected bounded fact families"
        );

        Ok(())
    }

    struct PerlOracleOutput {
        perl_version: String,
        facts: BTreeSet<NormalizedFact>,
    }

    fn lower_source(source: &str) -> HirFile {
        let mut parser = Parser::new(source);
        let output = parser.parse_with_recovery();
        lower_ast(&output.ast)
    }

    fn normalize_rust_compile_effects(file: &HirFile) -> BTreeSet<NormalizedFact> {
        file.compile_effects().iter().filter_map(normalize_rust_compile_effect).collect()
    }

    fn normalize_rust_compile_effect(effect: &CompileEffect) -> Option<NormalizedFact> {
        match effect.kind {
            CompileEffectKind::DeclarePackage => {
                effect.fact_name.as_ref().map(|name| normalized("package", name.clone()))
            }
            CompileEffectKind::DeclareSub => effect.fact_name.as_ref().map(|name| {
                normalized("sub", qualify_name(effect.package_context.as_deref(), name))
            }),
            CompileEffectKind::AssignInheritance => {
                effect.fact_name.as_ref().map(|name| normalized("isa", name.clone()))
            }
            CompileEffectKind::DefineConstant => {
                effect.fact_name.as_ref().map(|name| normalized("constant", name.clone()))
            }
            CompileEffectKind::RegisterPrototype => effect.fact_name.as_ref().map(|name| {
                normalized("prototype", qualify_name(effect.package_context.as_deref(), name))
            }),
            _ => None,
        }
    }

    fn run_perl_oracle(source: &str) -> Result<PerlOracleOutput> {
        let tempdir = tempfile::tempdir().context("create compiler-oracle fixture tempdir")?;
        let fixture_path = tempdir.path().join("OracleDemo.pm");
        fs::write(&fixture_path, source).context("write compiler-oracle fixture")?;

        let perl_version = query_perl_version()?;
        let output = Command::new("perl")
            .arg("-I")
            .arg(tempdir.path())
            .arg("-e")
            .arg(ORACLE_PROBE)
            .arg(&fixture_path)
            .env_remove("PERL5OPT")
            .env_remove("PERL5LIB")
            .env("LC_ALL", "C")
            .output()
            .context("run Perl compile-effect oracle probe")?;

        if !output.status.success() {
            bail!("Perl compile-effect oracle failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        let stdout =
            String::from_utf8(output.stdout).context("decode Perl compile-effect oracle stdout")?;
        Ok(PerlOracleOutput { perl_version, facts: parse_oracle_facts(&stdout)? })
    }

    fn query_perl_version() -> Result<String> {
        let output = Command::new("perl")
            .arg("-e")
            .arg("print $^V")
            .env_remove("PERL5OPT")
            .env_remove("PERL5LIB")
            .env("LC_ALL", "C")
            .output()
            .context("query Perl version for compile-effect oracle")?;

        if !output.status.success() {
            bail!("Perl version probe failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        String::from_utf8(output.stdout).context("decode Perl version output")
    }

    fn parse_oracle_facts(stdout: &str) -> Result<BTreeSet<NormalizedFact>> {
        let mut facts = BTreeSet::new();
        for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
            let mut parts = line.splitn(2, '\t');
            let family = parts
                .next()
                .filter(|value| !value.is_empty())
                .context("Perl oracle line missing fact family")?;
            let name = parts
                .next()
                .filter(|value| !value.is_empty())
                .context("Perl oracle line missing fact name")?;
            let family = match family {
                "package" => "package",
                "sub" => "sub",
                "constant" => "constant",
                "prototype" => "prototype",
                "isa" => "isa",
                other => bail!("unknown Perl oracle fact family: {other}"),
            };
            facts.insert(normalized(family, name.to_string()));
        }
        Ok(facts)
    }

    fn compare_facts(
        perl_version: String,
        rust_facts: BTreeSet<NormalizedFact>,
        perl_facts: BTreeSet<NormalizedFact>,
    ) -> DifferentialReceipt {
        let compared_families = vec!["package", "sub", "constant", "prototype", "isa"];
        let matched_facts =
            rust_facts.intersection(&perl_facts).cloned().collect::<Vec<NormalizedFact>>();
        let missing_in_perl = rust_facts
            .difference(&perl_facts)
            .map(|fact| disagreement(fact.family, fact.name.clone(), "rust_only"));
        let missing_in_rust = perl_facts
            .difference(&rust_facts)
            .map(|fact| disagreement(fact.family, fact.name.clone(), "perl_only"));
        let disagreements = missing_in_perl.chain(missing_in_rust).collect::<Vec<_>>();

        DifferentialReceipt {
            fixture_id: FIXTURE_ID,
            perl_version,
            compared_families,
            matched_facts,
            disagreements,
        }
    }

    fn normalized(family: &'static str, name: String) -> NormalizedFact {
        NormalizedFact { family, name }
    }

    fn disagreement(family: &'static str, name: String, side: &'static str) -> FactDisagreement {
        FactDisagreement { family, name, side }
    }

    fn qualify_name(package: Option<&str>, name: &str) -> String {
        if name.contains("::") {
            return name.to_string();
        }
        match package {
            Some(package) if !package.is_empty() => format!("{package}::{name}"),
            _ => name.to_string(),
        }
    }

    #[test]
    fn compiler_oracle_fixture_path_is_repo_independent() -> Result<()> {
        let tempdir = tempfile::tempdir().context("create compiler-oracle path test tempdir")?;
        let path = tempdir.path().join("OracleDemo.pm");
        assert!(
            is_relative_to(&path, tempdir.path()),
            "fixture path should stay in the oracle tempdir"
        );
        Ok(())
    }

    fn is_relative_to(path: &Path, root: &Path) -> bool {
        path.starts_with(root)
    }
}
