# ============================================================================
# Security Scanning Commands (Issue #282)
# ============================================================================
# These commands integrate Trivy, cargo-audit, and cargo-deny for comprehensive
# security vulnerability scanning.
#
# Quick start:
#   just security-scan      # Run all security checks
#   just security-quick     # Fast pre-commit check
#   just security-report    # Generate detailed report
#
# Documentation: docs/SECURITY_SCANNING.md
# ============================================================================

# Comprehensive security scan (all tools)
security-scan:
    @echo "=============================================="
    @echo "  COMPREHENSIVE SECURITY SCAN"
    @echo "=============================================="
    @START=$$(date +%s); \
    just _timed "cargo-audit" "just security-audit-strict" && \
    just _timed "cargo-deny" "just security-deny" && \
    just _timed "trivy-scan" "just security-trivy"; \
    RC=$$?; \
    END=$$(date +%s); \
    echo ""; \
    echo "=============================================="
    @if [ $$RC -eq 0 ]; then \
        echo "  Security scan PASSED (total: $$((END - START))s)"; \
    else \
        echo "  Security scan FAILED (total: $$((END - START))s)"; \
    fi
    @echo "=============================================="
    @exit $$RC

# Cargo audit (strict mode - fail on warnings)
security-audit-strict:
    @echo "Running cargo-audit (strict mode)..."
    @if command -v cargo-audit >/dev/null 2>&1; then \
        cargo audit --deny warnings; \
    else \
        echo "Installing cargo-audit..."; \
        cargo install cargo-audit --locked && cargo audit --deny warnings; \
    fi

# Cargo deny (policy enforcement)
security-deny:
    @echo "Running cargo-deny policy checks..."
    @if command -v cargo-deny >/dev/null 2>&1; then \
        cargo deny --locked check advisories licenses bans sources; \
    else \
        echo "Installing cargo-deny..."; \
        cargo install cargo-deny --locked && \
        cargo deny --locked check advisories licenses bans sources; \
    fi

# Trivy vulnerability scanner
security-trivy:
    @echo "Running Trivy vulnerability scanner..."
    @if command -v trivy >/dev/null 2>&1; then \
        trivy fs --severity CRITICAL,HIGH,MEDIUM \
            --scanners vuln,config,secret \
            --skip-dirs target,archive,.runs,.git \
            --ignore-unfixed \
            .; \
    else \
        echo "SKIP: trivy not installed"; \
        echo "Install: https://aquasecurity.github.io/trivy/latest/getting-started/installation/"; \
    fi

# Trivy with SARIF output
security-trivy-sarif:
    @echo "Running Trivy with SARIF output..."
    @if command -v trivy >/dev/null 2>&1; then \
        trivy fs --severity CRITICAL,HIGH,MEDIUM \
            --scanners vuln,config,secret \
            --skip-dirs target,archive,.runs,.git \
            --ignore-unfixed \
            --format sarif \
            --output trivy-results.sarif \
            .; \
        echo "SARIF report saved to: trivy-results.sarif"; \
    else \
        echo "SKIP: trivy not installed"; \
    fi

# Trivy Docker image scan
security-trivy-docker image='perl-lsp:dev':
    @echo "Running Trivy Docker image scan..."
    @if command -v trivy >/dev/null 2>&1; then \
        trivy image --severity CRITICAL,HIGH,MEDIUM \
            --ignore-unfixed \
            {{image}}; \
    else \
        echo "SKIP: trivy not installed"; \
    fi

# Generate comprehensive security report
security-report:
    @echo "Generating comprehensive security report..."
    @mkdir -p target/security-reports
    @echo "# Security Scan Report" > target/security-reports/report.md
    @echo "" >> target/security-reports/report.md
    @echo "**Date:** $$(date -u +%Y-%m-%d\ %H:%M:%S\ UTC)" >> target/security-reports/report.md
    @echo "**Commit:** $$(git rev-parse --short HEAD 2>/dev/null || echo 'unknown')" >> target/security-reports/report.md
    @echo "" >> target/security-reports/report.md
    @echo "## Cargo Audit" >> target/security-reports/report.md
    @if command -v cargo-audit >/dev/null 2>&1; then \
        cargo audit --json > target/security-reports/audit.json 2>&1 || true; \
        echo "See: target/security-reports/audit.json" >> target/security-reports/report.md; \
    else \
        echo "⚠️ cargo-audit not installed" >> target/security-reports/report.md; \
    fi
    @echo "" >> target/security-reports/report.md
    @echo "## Trivy Scan" >> target/security-reports/report.md
    @if command -v trivy >/dev/null 2>&1; then \
        trivy fs --format json --output target/security-reports/trivy.json \
            --severity CRITICAL,HIGH,MEDIUM \
            --skip-dirs target,archive,.runs,.git \
            --ignore-unfixed \
            . 2>&1 || true; \
        echo "See: target/security-reports/trivy.json" >> target/security-reports/report.md; \
    else \
        echo "⚠️ trivy not installed" >> target/security-reports/report.md; \
    fi
    @echo "" >> target/security-reports/report.md
    @echo "Report generated at: target/security-reports/report.md"
    @cat target/security-reports/report.md

# Quick security check (fast, for pre-commit)
security-quick:
    @echo "Running quick security check..."
    @if command -v cargo-audit >/dev/null 2>&1; then \
        cargo audit --deny warnings; \
    else \
        echo "SKIP: cargo-audit not installed"; \
    fi
