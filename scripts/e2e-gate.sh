#!/usr/bin/env bash
set -euo pipefail

exec cargo xtask ci-hygiene e2e-gate "$@"
