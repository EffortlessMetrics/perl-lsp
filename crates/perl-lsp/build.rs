
fn main() {
    // Get git tag for embedding in version output
    let tag = std::process::Command::new("git")
        .args(["describe", "--tags", "--always"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "unknown".into());

    println!("cargo:rustc-env=GIT_TAG={}", tag.trim());

    // Configure check-cfg for test-only cfg attributes
    println!("cargo:rustc-check-cfg=cfg(ci)");

    // Also try to get the exact tag if we're on one
    let exact_tag = std::process::Command::new("git")
        .args(["describe", "--tags", "--exact-match"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok());

    if let Some(tag) = exact_tag {
        println!("cargo:rustc-env=GIT_EXACT_TAG={}", tag.trim());
    }

    // Re-run build script when HEAD or refs change, so GIT_TAG stays fresh
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/refs/");
    println!("cargo:rerun-if-changed=../../.git/packed-refs");
}