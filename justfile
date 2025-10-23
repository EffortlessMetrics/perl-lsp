# Perl LSP Documentation Truth System
# Usage: just <command>

# List available commands
default:
    @just --list

# Generate canonical receipts from tests and docs
receipts:
    @echo "Generating receipts..."
    @./scripts/generate-receipts.sh

# Render documentation templates with receipt values
docs-render: receipts
    @echo "Rendering documentation..."
    @./scripts/render-docs.sh

# Check if docs are in sync with receipts (render + diff)
docs-check: docs-render
    @echo "Checking for documentation drift..."
    @if diff -ruN docs tmp/docs > /dev/null 2>&1; then \
        echo "✓ docs/ directory in sync"; \
    else \
        echo "✗ docs/ directory has drift"; \
        diff -ruN docs tmp/docs | head -50; \
    fi
    @for md_file in *.md; do \
        if [ -f "$$md_file" ] && [ -f "tmp/$$md_file" ]; then \
            if diff -ruN "$$md_file" "tmp/$$md_file" > /dev/null 2>&1; then \
                echo "✓ $$md_file in sync"; \
            else \
                echo "✗ $$md_file has drift"; \
                diff -ruN "$$md_file" "tmp/$$md_file" | head -20; \
            fi; \
        fi; \
    done

# Apply rendered docs to committed files (rsync tmp → docs and root .md files)
docs-apply: docs-render
    @echo "Applying rendered documentation..."
    @rsync -av tmp/docs/ docs/
    @for md_file in tmp/*.md; do \
        if [ -f "$$md_file" ]; then \
            cp -v "$$md_file" .; \
        fi; \
    done
    @echo "✓ Documentation applied successfully"
    @echo "Review changes with: git diff"

# Full validation (receipts + render + check)
docs-validate: docs-check
    @echo ""
    @echo "=== Documentation Truth Validation Complete ==="

# Clean temporary files
clean:
    @echo "Cleaning temporary files..."
    @rm -rf tmp/
    @rm -rf artifacts/
    @echo "✓ Clean complete"
