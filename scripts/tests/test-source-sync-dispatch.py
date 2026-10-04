#!/usr/bin/env python3
"""Real-Git controls for the staged, fail-closed source packet dispatcher."""
from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def load_module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


legacy = load_module('legacy_sync_fixture', ROOT / 'scripts/tests/test-publication-sync-contract.py')
check = load_module('source_dispatch', ROOT / 'scripts/publication_sync_check.py')


class SourceDispatchTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='source-sync-dispatch-')
        self.addCleanup(self.temp.cleanup)
        self.repo = legacy.Repo(Path(self.temp.name) / 'repo')
        self.fixture = legacy.SyncFixture(self.repo)
        self.body = Path(self.temp.name) / 'body.md'
        self.body.write_text(legacy.BODY_YES, encoding='utf-8')
        digest = 'sha256:' + 'a' * 64
        self.packet = {
            'schema_version': 'source_sync.v1',
            'direction': 'swarm_to_public',
            'source_repository': 'EffortlessMetrics/perl-lsp-swarm',
            'source_ref': 'refs/heads/main',
            'destination_repository': 'EffortlessMetrics/perl-lsp',
            'destination_ref': 'refs/heads/master',
            'reconciliation_base_sha': self.fixture.r,
            'destination_base_sha': self.fixture.r,
            'swarm_source_sha': self.fixture.s,
            'sync_join_sha': self.fixture.core_join,
            'expected_projected_tree': self.repo.make_tree(legacy.PROJECTION_CONTENT),
            'expected_join_parents': [self.fixture.r, self.fixture.s],
            'published_channels': [],
            'release_cut': False,
            'projection_policy_digest': digest,
            'reconciliation_digest': digest,
            'publication_sync_manifest_digest': digest,
            'product_proof_digest': digest,
        }

    def head(self, packet):
        files = dict(legacy.PROJECTION_CONTENT)
        files[legacy.PACKET_PATH] = legacy.json_bytes(packet)
        files[legacy.LEDGER_PATH] = legacy.json_bytes(legacy.ledger_payload())
        files[legacy.MANIFEST_PATH] = legacy.json_bytes(legacy.manifest_payload())
        return self.repo.make_commit(files, [self.fixture.r, self.fixture.s], legacy.JOIN_MESSAGE)

    def pr(self, head, base=None):
        return legacy.run_check([
            'pr', '--repo', str(self.repo.path),
            '--base-sha', base or self.fixture.r, '--head-sha', head,
            '--pr-body-file', str(self.body),
        ])

    def reject(self, proc, reason):
        self.assertEqual(proc.returncode, 1, proc.stdout + proc.stderr)
        self.assertIn(reason, proc.stdout)
        self.assertNotIn('publication-sync: pass', proc.stdout)
        self.assertNotIn('publication-sync: not_applicable', proc.stdout)
        self.assertNotIn('instrument failure', proc.stdout)

    def legacy_candidate(self, policy, *, head_policy=None):
        policy_path = '.ci/policies/publication-sync-admission.v1.json'
        base_files = {'README.md': b'engineering base\n'}
        if policy is not None:
            base_files[policy_path] = legacy.json_bytes(policy)
        base = self.repo.make_commit(base_files, [], 'trusted engineering R\n')
        packet = json.loads(self.repo.git('show', f'{self.fixture.j}:{legacy.PACKET_PATH}'))
        projection = dict(legacy.PROJECTION_CONTENT)
        manifest = legacy.manifest_payload()
        if head_policy is not None:
            projection[policy_path] = legacy.json_bytes(head_policy)
            manifest['translations'].append({'from': policy_path, 'to': policy_path})
        ledger_bytes = legacy.json_bytes(legacy.ledger_payload())
        manifest_bytes = legacy.json_bytes(manifest)
        tree = self.repo.make_tree(projection)
        core = self.repo.git('commit-tree', tree, '-p', base, '-p', self.fixture.s,
                             input_bytes=legacy.JOIN_MESSAGE.encode())
        packet.update(release_base_sha=base, sync_join_sha=core,
                      expected_join_parents=[base, self.fixture.s], expected_projected_tree=tree,
                      publication_sync_manifest_digest=legacy.hashlib.sha256(manifest_bytes).hexdigest())
        packet['publication_sync_manifest_digest'] = 'sha256:' + packet['publication_sync_manifest_digest']
        files = dict(projection)
        files[legacy.PACKET_PATH] = legacy.json_bytes(packet)
        files[legacy.LEDGER_PATH] = ledger_bytes
        files[legacy.MANIFEST_PATH] = manifest_bytes
        head = self.repo.make_commit(files, [base, self.fixture.s], legacy.JOIN_MESSAGE)
        return base, head

    def test_legacy_envelope_cannot_substitute_for_unapproved_source(self):
        policy = {'schema_version': 'publication_sync_admission_policy.v1',
                  'historical_release_subjects': []}
        base, head = self.legacy_candidate(policy)
        self.reject(self.pr(head, base), 'not authorized by trusted-base policy')

    def test_candidate_policy_cannot_authorize_legacy_envelope(self):
        policy = {'schema_version': 'publication_sync_admission_policy.v1',
                  'historical_release_subjects': []}
        forged = dict(policy, historical_release_subjects=[self.fixture.s])
        base, head = self.legacy_candidate(policy, head_policy=forged)
        self.reject(self.pr(head, base), 'not authorized by trusted-base policy')

    def test_missing_trusted_policy_cannot_authorize_legacy_envelope(self):
        base, head = self.legacy_candidate(None)
        self.reject(self.pr(head, base), 'trusted-base admission policy missing or unreadable')

    def test_cross_subject_trusted_authorization_does_not_reuse(self):
        other = self.repo.make_commit(dict(legacy.S_CONTENT), [], 'different exact subject\n')
        policy = {'schema_version': 'publication_sync_admission_policy.v1',
                  'historical_release_subjects': [other]}
        base, head = self.legacy_candidate(policy)
        self.reject(self.pr(head, base), 'not authorized by trusted-base policy')

    def test_landed_wrapper_cannot_read_candidate_policy_as_authority(self):
        policy = {'schema_version': 'publication_sync_admission_policy.v1',
                  'historical_release_subjects': []}
        forged = dict(policy, historical_release_subjects=[self.fixture.s])
        base, head = self.legacy_candidate(policy, head_policy=forged)
        tree = self.repo.git('rev-parse', head + '^{tree}')
        merge = self.repo.git('commit-tree', tree, '-p', base, '-p', head,
                              input_bytes=b'wrapper with candidate policy\n')
        self.repo.git('update-ref', 'refs/heads/master', merge)
        proc = legacy.run_check(['post-merge', '--repo', str(self.repo.path), '--merge-sha', merge])
        self.reject(proc, 'not authorized by trusted-base policy')

    def test_source_shape_needs_no_release_inputs_but_admission_is_not_proven(self):
        self.assertEqual(check.validate_packet_shape(self.packet), self.packet)
        head = self.head(self.packet)
        self.assertEqual(self.repo.git('rev-list', '--parents', '-n', '1', head).split(),
                         [head, self.fixture.r, self.fixture.s])
        self.reject(self.pr(head), 'source_sync.v1 NOT_PROVEN: producer-backed admission')

    def test_historical_release_subject_still_passes(self):
        proc = self.pr(self.fixture.j)
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertIn('publication-sync: pass', proc.stdout)

    def test_only_changing_release_profile_cannot_escape_requirements(self):
        packet = json.loads(self.repo.git('show', f'{self.fixture.j}:{legacy.PACKET_PATH}'))
        packet['schema_version'] = 'source_sync.v1'
        self.reject(self.pr(self.head(packet)), 'source packet keys: missing')

    def test_unknown_profile_does_not_fall_through_ordinary(self):
        packet = dict(self.packet, schema_version='source_sync.v2')
        self.reject(self.pr(self.head(packet)), 'unsupported packet schema_version')

    def test_missing_profile_does_not_fall_through_ordinary(self):
        packet = dict(self.packet)
        del packet['schema_version']
        self.reject(self.pr(self.head(packet)), 'unsupported packet schema_version')

    def test_source_post_merge_cannot_claim_landing_proof(self):
        head = self.head(self.packet)
        tree = self.repo.git('rev-parse', head + '^{tree}')
        merge = self.repo.git('commit-tree', tree, '-p', self.fixture.r, '-p', head,
                              input_bytes=b'protected wrapper fixture\n')
        self.repo.git('update-ref', 'refs/heads/master', merge)
        proc = legacy.run_check(['post-merge', '--repo', str(self.repo.path), '--merge-sha', merge])
        self.reject(proc, 'source_sync.v1 NOT_PROVEN')

    def test_unmarked_source_packet_cannot_arrive_as_ordinary(self):
        self.body.write_text(legacy.BODY_NO, encoding='utf-8')
        self.reject(self.pr(self.head(self.packet)), 'without the PR-template marker')

    def test_unchanged_inherited_source_packet_remains_ordinary(self):
        base = self.head(self.packet)
        files = dict(legacy.PROJECTION_CONTENT)
        for path in (legacy.PACKET_PATH, legacy.LEDGER_PATH, legacy.MANIFEST_PATH):
            files[path] = self.repo.git('show', f'{base}:{path}').encode() + b'\n'
        files['README.md'] = b'ordinary follow-up\n'
        head = self.repo.make_commit(files, [base], 'ordinary follow-up\n')
        self.body.write_text(legacy.BODY_NO, encoding='utf-8')
        proc = self.pr(head, base)
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertIn('publication-sync: not_applicable', proc.stdout)

    def test_candidate_cannot_select_weaker_proof_sets_or_claim_pass(self):
        for key, value in (
            ('required_invariants', []), ('policy', 'weaker'),
            ('live_controls', {'status': 'pass'}), ('product_proof', {'status': 'pass'}),
            ('release', '0.18.0-rc.1'), ('prepared_swarm_sha', self.fixture.s),
        ):
            with self.subTest(key=key):
                self.reject(self.pr(self.head(dict(self.packet, **{key: value}))), 'unknown')

    def test_every_required_source_binding_is_mandatory(self):
        for key in check.SOURCE_PACKET_KEYS:
            with self.subTest(key=key):
                packet = dict(self.packet)
                del packet[key]
                reason = ('unsupported packet schema_version' if key == 'schema_version'
                          else 'source packet keys: missing')
                with self.assertRaisesRegex(check.ContractFailure, reason):
                    check.validate_packet_shape(packet)

    def test_cross_repository_subjects_and_publication_claims_reject(self):
        for key, value in (
            ('source_repository', 'attacker/fork'), ('source_ref', 'refs/heads/release'),
            ('destination_repository', 'attacker/public'), ('destination_ref', 'refs/heads/main'),
            ('direction', 'public_to_swarm'), ('published_channels', ['crates.io']),
            ('release_cut', True), ('release_cut', 0),
            ('expected_join_parents', [self.fixture.s, self.fixture.r]),
            ('product_proof_digest', 'pass'), ('swarm_source_sha', 'not-a-sha'),
            ('swarm_source_sha', self.fixture.s + '\n'),
            ('product_proof_digest', 'sha256:' + 'a' * 64 + '\n'),
        ):
            with self.subTest(key=key, value=value):
                self.reject(self.pr(self.head(dict(self.packet, **{key: value}))), 'FAILED')

    def test_schema_and_executable_envelope_agree(self):
        schema = json.loads((ROOT / 'schemas/source_sync.v1.schema.json').read_text(encoding='utf-8'))
        self.assertEqual(set(schema['required']), check.SOURCE_PACKET_KEYS)
        self.assertEqual(set(schema['properties']), check.SOURCE_PACKET_KEYS)
        self.assertIs(schema['additionalProperties'], False)
        self.assertEqual(schema['properties']['schema_version']['const'], check.SOURCE_SCHEMA_VERSION)
        for key, value in check.SOURCE_IDENTITIES.items():
            self.assertEqual(schema['properties'][key]['const'], value)
        for key in check.SOURCE_SHA_KEYS:
            self.assertEqual(schema['properties'][key]['pattern'], check.HEX40.pattern)
        for key in check.SOURCE_DIGEST_KEYS:
            self.assertEqual(schema['properties'][key]['pattern'], check.DIGEST.pattern)


if __name__ == '__main__':
    unittest.main(verbosity=2)
