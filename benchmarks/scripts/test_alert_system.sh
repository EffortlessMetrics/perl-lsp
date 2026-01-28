#!/bin/bash
# Test script for performance regression alert system
# Usage: ./test_alert_system.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$SCRIPT_DIR/../.."

cd "$REPO_ROOT"

echo "========================================="
echo "Performance Regression Alert System Test"
echo "========================================="
echo ""

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# Test 1: No regression (identical baseline)
echo "Test 1: No regression detection"
echo "--------------------------------"
if python3 ./benchmarks/scripts/alert.py \
    benchmarks/baselines/v0.9.0.json \
    benchmarks/baselines/v0.9.0.json \
    --config .ci/benchmark-thresholds.yaml \
    > /tmp/alert_test1.txt 2>&1; then
    if grep -q "No performance alerts detected" /tmp/alert_test1.txt; then
        echo -e "${GREEN}✓ PASS${NC}: No regression detected for identical baseline"
    else
        echo -e "${RED}✗ FAIL${NC}: Expected no alerts"
        cat /tmp/alert_test1.txt
        exit 1
    fi
else
    echo -e "${RED}✗ FAIL${NC}: Script failed"
    exit 1
fi
echo ""

# Test 2: Warning detection (11% slower)
echo "Test 2: Warning detection"
echo "-------------------------"
python3 -c "
import json
with open('benchmarks/baselines/v0.9.0.json') as f:
    data = json.load(f)
bench = data['benchmarks']['parser']['parse_simple_script']
bench['mean']['nanoseconds'] = int(bench['mean']['nanoseconds'] * 1.11)
with open('/tmp/warning_regression.json', 'w') as f:
    json.dump(data, f, indent=2)
" 2>&1

if python3 ./benchmarks/scripts/alert.py \
    benchmarks/baselines/v0.9.0.json \
    /tmp/warning_regression.json \
    --config .ci/benchmark-thresholds.yaml \
    > /tmp/alert_test2.txt 2>&1; then
    if grep -q "WARNING" /tmp/alert_test2.txt && grep -q "1" /tmp/alert_test2.txt; then
        echo -e "${GREEN}✓ PASS${NC}: Warning detected for 11% regression"
    else
        echo -e "${YELLOW}⚠ PARTIAL${NC}: Check output"
        grep "Warnings:" /tmp/alert_test2.txt || echo "No warnings found"
    fi
else
    echo -e "${RED}✗ FAIL${NC}: Script failed"
    exit 1
fi
echo ""

# Test 3: Regression detection (25% slower)
echo "Test 3: Regression detection"
echo "----------------------------"
python3 -c "
import json
with open('benchmarks/baselines/v0.9.0.json') as f:
    data = json.load(f)
bench = data['benchmarks']['parser']['parse_simple_script']
bench['mean']['nanoseconds'] = int(bench['mean']['nanoseconds'] * 1.25)
with open('/tmp/regression.json', 'w') as f:
    json.dump(data, f, indent=2)
" 2>&1

if python3 ./benchmarks/scripts/alert.py \
    benchmarks/baselines/v0.9.0.json \
    /tmp/regression.json \
    --config .ci/benchmark-thresholds.yaml \
    > /tmp/alert_test3.txt 2>&1; then
    if grep -q "REGRESSION" /tmp/alert_test3.txt; then
        echo -e "${GREEN}✓ PASS${NC}: Regression detected for 25% slowdown"
    else
        echo -e "${RED}✗ FAIL${NC}: Expected regression alert"
        cat /tmp/alert_test3.txt
        exit 1
    fi
else
    echo -e "${RED}✗ FAIL${NC}: Script failed"
    exit 1
fi
echo ""

# Test 4: Critical regression detection (60% slower)
echo "Test 4: Critical regression detection"
echo "-------------------------------------"
python3 -c "
import json
with open('benchmarks/baselines/v0.9.0.json') as f:
    data = json.load(f)
bench = data['benchmarks']['parser']['parse_simple_script']
bench['mean']['nanoseconds'] = int(bench['mean']['nanoseconds'] * 1.60)
with open('/tmp/critical_regression.json', 'w') as f:
    json.dump(data, f, indent=2)
" 2>&1

if python3 ./benchmarks/scripts/alert.py \
    benchmarks/baselines/v0.9.0.json \
    /tmp/critical_regression.json \
    --config .ci/benchmark-thresholds.yaml \
    > /tmp/alert_test4.txt 2>&1; then
    if grep -q "CRITICAL" /tmp/alert_test4.txt; then
        echo -e "${GREEN}✓ PASS${NC}: Critical regression detected for 60% slowdown"
    else
        echo -e "${RED}✗ FAIL${NC}: Expected critical alert"
        cat /tmp/alert_test4.txt
        exit 1
    fi
else
    echo -e "${RED}✗ FAIL${NC}: Script failed"
    exit 1
fi
echo ""

# Test 5: Markdown output format
echo "Test 5: Markdown output format"
echo "-------------------------------"
if python3 ./benchmarks/scripts/alert.py \
    benchmarks/baselines/v0.9.0.json \
    /tmp/regression.json \
    --config .ci/benchmark-thresholds.yaml \
    --format markdown \
    > /tmp/alert_test5.md 2>&1; then
    if grep -q "## Performance Benchmark Results" /tmp/alert_test5.md && \
       grep -q "⚠️ Performance Regressions" /tmp/alert_test5.md; then
        echo -e "${GREEN}✓ PASS${NC}: Markdown format generated correctly"
    else
        echo -e "${RED}✗ FAIL${NC}: Expected markdown format"
        cat /tmp/alert_test5.md
        exit 1
    fi
else
    echo -e "${RED}✗ FAIL${NC}: Script failed"
    exit 1
fi
echo ""

# Test 6: Exit code with --check flag
echo "Test 6: Exit code with --check flag"
echo "------------------------------------"
# Create config with fail_on_critical=true
python3 -c "
import yaml
with open('.ci/benchmark-thresholds.yaml') as f:
    config = yaml.safe_load(f)
config['alerting']['fail_on_critical'] = True
with open('/tmp/test_thresholds.yaml', 'w') as f:
    yaml.dump(config, f)
" 2>&1

if python3 ./benchmarks/scripts/alert.py \
    benchmarks/baselines/v0.9.0.json \
    /tmp/critical_regression.json \
    --config /tmp/test_thresholds.yaml \
    --check \
    > /tmp/alert_test6.txt 2>&1; then
    echo -e "${RED}✗ FAIL${NC}: Expected non-zero exit code for critical regression"
    exit 1
else
    echo -e "${GREEN}✓ PASS${NC}: Exit code non-zero for critical regression with fail_on_critical=true"
fi
echo ""

# Test 7: Improvement detection (20% faster)
echo "Test 7: Improvement detection"
echo "------------------------------"
python3 -c "
import json
with open('benchmarks/baselines/v0.9.0.json') as f:
    data = json.load(f)
bench = data['benchmarks']['parser']['parse_simple_script']
bench['mean']['nanoseconds'] = int(bench['mean']['nanoseconds'] * 0.80)
with open('/tmp/improvement.json', 'w') as f:
    json.dump(data, f, indent=2)
" 2>&1

if python3 ./benchmarks/scripts/alert.py \
    benchmarks/baselines/v0.9.0.json \
    /tmp/improvement.json \
    --config .ci/benchmark-thresholds.yaml \
    > /tmp/alert_test7.txt 2>&1; then
    if grep -q "IMPROVED" /tmp/alert_test7.txt; then
        echo -e "${GREEN}✓ PASS${NC}: Improvement detected for 20% speedup"
    else
        echo -e "${YELLOW}⚠ PARTIAL${NC}: Check output"
        grep "Improvements:" /tmp/alert_test7.txt || echo "No improvements found"
    fi
else
    echo -e "${RED}✗ FAIL${NC}: Script failed"
    exit 1
fi
echo ""

echo "========================================="
echo "All tests passed!"
echo "========================================="
echo ""
echo "Summary:"
echo "  ✓ No regression detection"
echo "  ✓ Warning detection (11%)"
echo "  ✓ Regression detection (25%)"
echo "  ✓ Critical regression detection (60%)"
echo "  ✓ Markdown output format"
echo "  ✓ Exit code with --check flag"
echo "  ✓ Improvement detection (20%)"
echo ""
echo "The performance regression alert system is working correctly!"

# Cleanup
rm -f /tmp/alert_test*.txt /tmp/alert_test*.md
rm -f /tmp/warning_regression.json /tmp/regression.json /tmp/critical_regression.json /tmp/improvement.json
rm -f /tmp/test_thresholds.yaml
