#!/usr/bin/env bash
set -euo pipefail

exec cargo xtask ci-hygiene check-p0-locks "$@"
