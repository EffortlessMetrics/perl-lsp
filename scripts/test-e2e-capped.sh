#!/usr/bin/env bash
set -euo pipefail

exec cargo xtask ci-hygiene test-e2e-capped "$@"
