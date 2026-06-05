#!/usr/bin/env python3
"""Focused tests for scripts/ci/check_perl_lsp_rs_core_package.py."""

from __future__ import annotations

import importlib.util
import io
import json
import tarfile
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


SCRIPT_PATH = Path(__file__).with_name("check_perl_lsp_rs_core_package.py")
SPEC = importlib.util.spec_from_file_location("check_perl_lsp_rs_core_package", SCRIPT_PATH)
assert SPEC is not None
core_package = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(core_package)


def metadata_fixture(root: Path) -> str:
    return json.dumps(
        {
            "workspace_members": [
                "path+file:///repo#perl-lsp-rs-core@0.1.0",
                "path+file:///repo#perl-parser@0.1.0",
                "path+file:///repo#perl-tdd-support@0.1.0",
                "path+file:///repo#external-shadow@0.1.0",
            ],
            "packages": [
                {
                    "id": "path+file:///repo#perl-lsp-rs-core@0.1.0",
                    "name": "perl-lsp-rs-core",
                    "version": "0.1.0",
                    "manifest_path": str(root / "crates" / "perl-lsp-rs-core" / "Cargo.toml"),
                    "dependencies": [
                        {"name": "perl-parser", "kind": None},
                        {"name": "perl-tdd-support", "kind": "dev"},
                        {"name": "serde", "kind": None},
                    ],
                },
                {
                    "id": "path+file:///repo#perl-parser@0.1.0",
                    "name": "perl-parser",
                    "version": "0.1.0",
                    "manifest_path": str(root / "crates" / "perl-parser" / "Cargo.toml"),
                    "dependencies": [],
                },
                {
                    "id": "path+file:///repo#perl-tdd-support@0.1.0",
                    "name": "perl-tdd-support",
                    "version": "0.1.0",
                    "manifest_path": str(root / "crates" / "perl-tdd-support" / "Cargo.toml"),
                    "dependencies": [],
                },
                {
                    "id": "path+file:///repo#external-shadow@0.1.0",
                    "name": "external-shadow",
                    "version": "0.1.0",
                    "manifest_path": str(root / "vendor" / "external-shadow" / "Cargo.toml"),
                    "dependencies": [],
                },
                {
                    "id": "registry+https://example.invalid#serde@1.0.0",
                    "name": "serde",
                    "version": "1.0.0",
                    "manifest_path": str(root / "registry" / "serde" / "Cargo.toml"),
                    "dependencies": [],
                },
            ],
        }
    )


class CorePackageValidatorTests(unittest.TestCase):
    def test_workspace_package_version_finds_workspace_member(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            with patch.object(core_package, "run", return_value=metadata_fixture(Path(tmp))):
                version = core_package.workspace_package_version("perl-lsp-rs-core")

        self.assertEqual("0.1.0", version)

    def test_workspace_package_version_rejects_unknown_member(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            with patch.object(core_package, "run", return_value=metadata_fixture(Path(tmp))):
                with self.assertRaises(SystemExit) as raised:
                    core_package.workspace_package_version("missing-crate")

        self.assertIn("workspace package not found: missing-crate", str(raised.exception))

    def test_workspace_patch_args_include_only_transitive_normal_deps_by_default(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            with patch.object(core_package, "run", return_value=metadata_fixture(Path(tmp))):
                patch_args = core_package.workspace_patch_args(
                    "perl-lsp-rs-core",
                    include_dev_deps=False,
                )

        self.assertEqual(1, len(patch_args))
        self.assertIn("patch.crates-io.perl-parser.path", patch_args[0])
        self.assertNotIn("perl-tdd-support", "\n".join(patch_args))
        self.assertNotIn("serde", "\n".join(patch_args))

    def test_workspace_patch_args_can_include_dev_dependency_workspace_members(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            with patch.object(core_package, "run", return_value=metadata_fixture(Path(tmp))):
                patch_args = core_package.workspace_patch_args(
                    "perl-lsp-rs-core",
                    include_dev_deps=True,
                )

        rendered = "\n".join(patch_args)
        self.assertIn("patch.crates-io.external-shadow.path", rendered)
        self.assertIn("patch.crates-io.perl-parser.path", rendered)
        self.assertIn("patch.crates-io.perl-tdd-support.path", rendered)
        self.assertNotIn("patch.crates-io.perl-lsp-rs-core.path", rendered)
        self.assertNotIn("serde", rendered)

    def test_safe_extract_rejects_archive_path_traversal(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            archive = root / "malicious.tar"
            with tarfile.open(archive, "w") as tar:
                payload = b"escape"
                member = tarfile.TarInfo("../escape.txt")
                member.size = len(payload)
                tar.addfile(member, io.BytesIO(payload))

            with self.assertRaises(SystemExit) as raised:
                core_package.safe_extract(archive, root / "out")

        self.assertIn("refusing to extract path outside destination", str(raised.exception))

    def test_strip_dev_dependencies_removes_dev_tables_only(self) -> None:
        manifest_text = """[package]
name = "demo"

[dependencies]
serde = "1"

[dev-dependencies]
tempfile = "3"

[build-dependencies]
cc = "1"
"""
        with tempfile.TemporaryDirectory() as tmp:
            manifest = Path(tmp) / "Cargo.toml"
            manifest.write_text(manifest_text, encoding="utf-8")

            core_package.strip_dev_dependencies(manifest)

            stripped = manifest.read_text(encoding="utf-8")

        self.assertIn("[package]", stripped)
        self.assertIn("[dependencies]", stripped)
        self.assertIn("[build-dependencies]", stripped)
        self.assertNotIn("[dev-dependencies]", stripped)
        self.assertNotIn("tempfile", stripped)

    def test_package_args_preserve_patch_verify_and_dirty_flags(self) -> None:
        args = core_package.package_args(
            "--list",
            allow_dirty=True,
            patch_args=["--config=patch.crates-io.foo.path=\"/tmp/foo\""],
            no_verify=True,
        )

        self.assertEqual(
            [
                "cargo",
                "package",
                "-p",
                "perl-lsp-rs-core",
                "--locked",
                "--config=patch.crates-io.foo.path=\"/tmp/foo\"",
                "--list",
                "--no-verify",
                "--allow-dirty",
            ],
            args,
        )


if __name__ == "__main__":
    unittest.main()
