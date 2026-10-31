#!/usr/bin/env python3
"""Check that PR build/test workflows subscribe only to explicit label events."""
from pathlib import Path
import unittest

import yaml

ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = (
    'agent-capability-gate.yml', 'changelog-advisory.yml', 'ci-gate-self-tests.yml',
    'ci-nightly.yml', 'ci-security.yml', 'ci.yml', 'em-ci-routed-rust.yml',
    'lsp-318-claim-guard.yml', 'methodology-gate.yml', 'perl-version-matrix.yml',
    'policy-validators.yml', 'post-merge-corpus-ratchet.yml', 'publish-dry-run.yml',
    'ripr.yml', 'ux-regression-gate.yml', 'vscode-current-source-linux-smoke.yml',
    'vscode-managed-binary-smoke.yml', 'workflow-contracts-advisory.yml',
    'workflow-policy.yml', 'workflow-trigger-lint.yml',
)


class ExplicitRoutingTest(unittest.TestCase):
    def test_pr_build_events_are_labeled_only(self):
        for filename in WORKFLOWS:
            with self.subTest(filename=filename):
                path = ROOT / '.github' / 'workflows' / filename
                workflow = yaml.load(path.read_text(encoding='utf-8'), Loader=yaml.BaseLoader)
                self.assertEqual(workflow['on']['pull_request']['types'], ['labeled'])

    def test_publication_contract_rechecks_body_and_head(self):
        path = ROOT / '.github' / 'workflows' / 'publication-sync-contract.yml'
        workflow = yaml.load(path.read_text(encoding='utf-8'), Loader=yaml.BaseLoader)
        self.assertEqual(workflow['on']['pull_request']['types'],
                         ['opened', 'edited', 'reopened', 'synchronize'])


if __name__ == '__main__':
    unittest.main()
