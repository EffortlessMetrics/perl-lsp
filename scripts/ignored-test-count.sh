#!/usr/bin/env bash
set -euo pipefail

exec cargo xtask ci-hygiene ignored-test-count "$@"
