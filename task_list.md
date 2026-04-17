# Task List — work-5c1ec819

## Implementation Tasks

- [ ] 1. Create `scripts/check_release_history.sh` shell script
  - [ ] Enumerate v* tags (excluding v*-rc* prerelease patterns)
  - [ ] Check each tag has matching `docs/releases/v<version>.md` file
  - [ ] Check each tag has matching row in `RELEASE_HISTORY.md`
  - [ ] Check newest tag is in `CHANGELOG.md` as `## [X.Y.Z]`
  - [ ] Use (CL) convention to exempt CHANGELOG-only entries
  - [ ] Exit 0 on success, exit 1 with descriptive error on drift

- [ ] 2. Add script to `policy_checks` gate in `.ci/gate-policy.yaml`
  - [ ] Add `bash scripts/check_release_history.sh &&` to the command chain

- [ ] 3. Verify gate passes on current master
  - [ ] Run `cargo xtask gates` locally
  - [ ] Confirm script exits 0 with no errors

- [ ] 4. Commit and push implementation
  - [ ] Stage and commit the script and gate changes
  - [ ] Push to the feature branch
  - [ ] Verify CI passes on the PR