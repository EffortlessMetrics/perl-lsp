use anyhow::Result;
use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_benchmark_saves_output() -> Result<()> {
    // Create a temporary cargo project with a simple benchmark
    let temp_dir = TempDir::new()?;
    let cargo_toml = temp_dir.path().join("Cargo.toml");
    fs::write(
        &cargo_toml,
        "[package]\nname = \"bench_test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[[bench]]\nname = \"dummy\"\nharness = false\n",
    )?;
    let benches_dir = temp_dir.path().join("benches");
    fs::create_dir(&benches_dir)?;
    fs::write(benches_dir.join("dummy.rs"), "fn main() { println!(\"dummy bench ran\"); }")?;

    // Path where benchmark results should be written
    let output_path = temp_dir.path().join("bench_output.txt");

    use perl_tdd_support::must_some;
    let output_str = must_some(output_path.to_str());

    // Run the xtask bench command and verify it succeeds
    let mut cmd = cargo_bin_cmd!("xtask");
    cmd.current_dir(temp_dir.path())
        .args(["bench", "--name", "dummy", "--save", "--output", output_str])
        .assert()
        .success();

    // Ensure the output file was created and contains bench output
    let contents = fs::read_to_string(&output_path)?;
    assert!(contents.contains("dummy bench ran"));

    Ok(())
}
