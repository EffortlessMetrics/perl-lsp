use assert_cmd::cargo::cargo_bin_cmd;

fn fixture(path: &str) -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    std::path::Path::new(manifest).join("tests").join("fixtures").join(path).display().to_string()
}

#[test]
fn queue_snapshot_from_fixture_derives_buckets() {
    let temp = tempfile::tempdir().expect("tempdir");
    let out = temp.path().join("snapshot.json");

    let mut cmd = cargo_bin_cmd!("xtask");
    cmd.args([
        "queue",
        "snapshot",
        "--fixture",
        fixture("queue-snapshot/snapshot-fixture.json").as_str(),
        "--out",
        out.display().to_string().as_str(),
    ])
    .assert()
    .success();

    let rendered = std::fs::read_to_string(out).expect("read snapshot");
    assert!(rendered.contains("\"merge_ready\""));
    assert!(rendered.contains("\"ci_green\""));
}
