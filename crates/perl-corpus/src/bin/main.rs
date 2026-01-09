#![allow(clippy::pedantic)] // Binary tool - focus on core clippy lints only

use anyhow::Result;
use clap::{Parser, Subcommand};
use perl_corpus::{index::write_indices, parse_dir};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "perl-corpus", version, about = "Perl test corpus management tool")]
struct Cli {
    /// Path to test/corpus directory
    #[arg(short, long, default_value = "test/corpus")]
    corpus: PathBuf,

    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate metadata and structure
    Lint {
        /// Maximum sections per file
        #[arg(long, default_value = "12")]
        max_sections: usize,

        /// Check for unknown tags
        #[arg(long, default_value = "true")]
        check_tags: bool,

        /// Check for unknown flags
        #[arg(long, default_value = "true")]
        check_flags: bool,
    },

    /// Build _index.json and _tags.json
    Index,

    /// Print corpus statistics
    Stats {
        /// Show detailed statistics
        #[arg(short, long)]
        detailed: bool,
    },

    /// Generate test cases
    Gen {
        /// Generator to use
        #[command(subcommand)]
        generator: Generator,

        /// Number of cases to generate
        #[arg(short, long, default_value = "10")]
        count: usize,

        /// Random seed
        #[arg(short, long)]
        seed: Option<u64>,
    },
}

#[derive(Subcommand)]
enum Generator {
    /// Generate qw expressions
    Qw,
    /// Generate quote-like operators
    Quote,
    /// Generate heredocs
    Heredoc,
    /// Generate whitespace-heavy code
    Whitespace,
}

fn main() -> Result<()> {
    let args = Cli::parse();

    match args.cmd {
        Command::Lint { max_sections, check_tags, check_flags } => {
            let sections = parse_dir(&args.corpus)?;

            let config = perl_corpus::lint::LintConfig {
                max_sections_per_file: max_sections,
                check_unknown_tags: check_tags,
                check_unknown_flags: check_flags,
                require_perl_version: false,
            };

            perl_corpus::lint::lint_with_config(&sections, &config)?;

            println!("✅ Corpus validation passed ({} sections)", sections.len());
        }

        Command::Index => {
            let sections = parse_dir(&args.corpus)?;
            write_indices(&args.corpus, &sections)?;

            println!("✅ Generated index files:");
            println!("   - {}", args.corpus.join("_index.json").display());
            println!("   - {}", args.corpus.join("_tags.json").display());
            println!("   - {}", args.corpus.join("COVERAGE_SUMMARY.md").display());
            println!("   Total sections: {}", sections.len());
        }

        Command::Stats { detailed } => {
            let sections = parse_dir(&args.corpus)?;

            // Basic stats
            let unique_files: std::collections::HashSet<_> =
                sections.iter().map(|s| &s.file).collect();
            let all_tags: std::collections::HashSet<_> =
                sections.iter().flat_map(|s| s.tags.iter()).collect();
            let all_flags: std::collections::HashSet<_> =
                sections.iter().flat_map(|s| s.flags.iter()).collect();

            println!("📊 Corpus Statistics");
            println!("====================");
            println!("Files:    {}", unique_files.len());
            println!("Sections: {}", sections.len());
            println!("Tags:     {}", all_tags.len());
            println!("Flags:    {}", all_flags.len());

            if detailed {
                println!("\n📁 Files:");
                let mut file_counts: std::collections::BTreeMap<&str, usize> =
                    std::collections::BTreeMap::new();
                for s in &sections {
                    *file_counts.entry(&s.file).or_default() += 1;
                }
                for (file, count) in file_counts {
                    println!("  {} ({})", file, count);
                }

                println!("\n🏷️  Top Tags:");
                let mut tag_counts: std::collections::BTreeMap<&str, usize> =
                    std::collections::BTreeMap::new();
                for s in &sections {
                    for tag in &s.tags {
                        *tag_counts.entry(tag).or_default() += 1;
                    }
                }
                let mut sorted_tags: Vec<_> = tag_counts.into_iter().collect();
                sorted_tags.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
                for (tag, count) in sorted_tags.iter().take(10) {
                    println!("  {} ({})", tag, count);
                }

                if !all_flags.is_empty() {
                    println!("\n🚩 Flags:");
                    for flag in all_flags {
                        let count = sections.iter().filter(|s| s.has_flag(flag)).count();
                        println!("  {} ({})", flag, count);
                    }
                }
            }
        }

        Command::Gen { generator, count, seed } => {
            use proptest::prelude::*;
            use proptest::strategy::ValueTree;
            use proptest::test_runner::{Config, TestRunner};

            let seed = seed.unwrap_or_else(|| {
                use std::time::{SystemTime, UNIX_EPOCH};
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                    .as_secs()
            });

            let config = Config { cases: count as u32, ..Config::default() };

            let mut runner = TestRunner::new_with_rng(
                config,
                proptest::test_runner::TestRng::from_seed(
                    proptest::test_runner::RngAlgorithm::ChaCha,
                    &seed.to_le_bytes(),
                ),
            );

            println!("# Generated with perl-corpus (seed: {})", seed);
            println!();

            match generator {
                Generator::Qw => {
                    use perl_corpus::r#gen::qw::qw_in_context;
                    for i in 0..count {
                        let value = qw_in_context()
                            .new_tree(&mut runner)
                            .map_err(|e| anyhow::anyhow!("{e:?}"))?
                            .current();
                        println!("# Test case {} (qw)", i + 1);
                        println!("{}", value);
                        println!();
                    }
                }
                Generator::Quote => {
                    use perl_corpus::r#gen::quote_like::quote_like_single;
                    for i in 0..count {
                        let value = quote_like_single()
                            .new_tree(&mut runner)
                            .map_err(|e| anyhow::anyhow!("{e:?}"))?
                            .current();
                        println!("# Test case {} (quote-like)", i + 1);
                        println!("{}", value);
                        println!();
                    }
                }
                Generator::Heredoc => {
                    use perl_corpus::r#gen::heredoc::heredoc_in_context;
                    for i in 0..count {
                        let value = heredoc_in_context()
                            .new_tree(&mut runner)
                            .map_err(|e| anyhow::anyhow!("{e:?}"))?
                            .current();
                        println!("# Test case {} (heredoc)", i + 1);
                        println!("{}", value);
                        println!();
                    }
                }
                Generator::Whitespace => {
                    use perl_corpus::r#gen::whitespace::whitespace_stress_test;
                    for i in 0..count {
                        let value = whitespace_stress_test()
                            .new_tree(&mut runner)
                            .map_err(|e| anyhow::anyhow!("{e:?}"))?
                            .current();
                        println!("# Test case {} (whitespace-heavy)", i + 1);
                        println!("{}", value);
                        println!();
                    }
                }
            }
        }
    }

    Ok(())
}
