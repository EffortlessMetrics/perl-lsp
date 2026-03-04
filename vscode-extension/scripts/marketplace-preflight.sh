#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PACKAGE_JSON="${ROOT_DIR}/package.json"

if [[ ! -f "${PACKAGE_JSON}" ]]; then
  echo "❌ package.json not found at ${PACKAGE_JSON}"
  exit 1
fi

required_files=(
  "README.md"
  "CHANGELOG.md"
  "LICENSE"
  "icon.png"
  ".vscodeignore"
)

missing=0
for f in "${required_files[@]}"; do
  if [[ ! -f "${ROOT_DIR}/${f}" ]]; then
    echo "❌ Missing required marketplace asset: ${f}"
    missing=1
  else
    echo "✅ Found ${f}"
  fi
done

required_fields=(
  "name"
  "displayName"
  "description"
  "version"
  "publisher"
  "icon"
  "license"
  "engines.vscode"
  "repository.url"
  "bugs.url"
  "homepage"
)

for field in "${required_fields[@]}"; do
  value="$(node -e "const p=require('${PACKAGE_JSON}'); const v='${field}'.split('.').reduce((a,k)=>a&&a[k], p); if(v===undefined||v===null||String(v).trim()===''){process.exit(1)}; process.stdout.write(String(v));" 2>/dev/null || true)"
  if [[ -z "${value}" ]]; then
    echo "❌ Missing required package.json field: ${field}"
    missing=1
  else
    echo "✅ package.json ${field}=${value}"
  fi
done

if [[ "${missing}" -ne 0 ]]; then
  echo ""
  echo "Marketplace preflight failed."
  exit 1
fi

echo ""
echo "✅ Marketplace preflight passed."
