# Jenkins Pipeline

`perl-lsp` ships a Jenkins template at [templates/ci/jenkins/Jenkinsfile](../../templates/ci/jenkins/Jenkinsfile).
Use it as a starting point for a multibranch pipeline or copy it to the repository root as `Jenkinsfile`.

## Recommended Setup

1. Create a Jenkins multibranch pipeline job for the repository.
2. Prefer a Docker-backed agent so the Rust toolchain stays reproducible.
3. Point the job at the repository root `Jenkinsfile`, or copy the template into your repo if you want to customize it.
4. Open the build in Blue Ocean to view the parallel `Format`, `Clippy`, and `Tests` stages clearly.

## What The Template Does

- Runs inside the official `rust:1.92-bookworm` container.
- Executes the repository fast path in parallel: formatting, clippy, and library tests.
- Publishes per-stage JUnit records and log artifacts under `target/jenkins-*`.
- Keeps the implementation declarative so Jenkins can visualize stage flow cleanly.

## Customization Notes

- If your Jenkins environment needs extra system packages, add them to the Docker image or bootstrap them in the `Checkout` stage.
- If you prefer a bare agent instead of Docker, swap the `agent { docker { ... } }` block for your own Rust bootstrap logic.
- If you want full-gate validation, extend the `Fast Gates` parallel block with the repo's heavier checks.
