START=$(date +%s); \
just ci-workflow-audit && \
just ci-check-no-nested-lock && \
just ci-format && \
just ci-docs-check && \
just ci-clippy-lib && \
just clippy-prod-no-unwrap && \
just ci-test-lib && \
just ci-policy && \
just ci-lsp-def && \
just ci-parser-features-check && \
just ci-features-invariants; \
RC=$?; \
END=$(date +%s); \
echo ""; \
if [ $RC -eq 0 ]; then \
    echo "Merge gate passed! (total: $((END - START))s)"; \
else \
    echo "Merge gate FAILED (total: $((END - START))s)"; \
    exit $RC; \
fi
