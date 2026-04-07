#!/usr/bin/env python3
"""
Unit tests for scripts/publish-topo.py — topological publish-order computation.

These tests use synthetic workspace metadata to verify correct behaviour of:
1. Simple linear dependency chain.
2. Dev-dep that crosses SCC boundary (ordering honoured, no cycle).
3. Intra-SCC dev-dep cycle — the exact shape that caused issue #3254:
   perl-module-import dev-depends on perl-module-token, which normally
   depends on perl-module-import-deps (and so on), creating a dev-dep
   cycle in the same SCC. The topo sort must break the intra-SCC edge and
   still produce a valid order.
4. Normal dep cycle — must hard-fail (no valid publish order exists).
5. Empty allowlist — must hard-fail.
6. Allowlist crate not in workspace — must hard-fail.

Run with: python3 scripts/tests/test-publish-topo.py
Returns exit code 0 on all-pass, 1 on any failure.
"""

from __future__ import annotations

import sys
import unittest
import os

# Ensure the scripts directory is on the path so we can import publish-topo.
scripts_dir = os.path.join(os.path.dirname(__file__), "..")
sys.path.insert(0, scripts_dir)

# Import via importlib because the module name contains a hyphen.
import importlib.util

_spec = importlib.util.spec_from_file_location(
    "publish_topo",
    os.path.join(scripts_dir, "publish-topo.py"),
)
assert _spec is not None and _spec.loader is not None
publish_topo = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(publish_topo)  # type: ignore[union-attr]

compute_publish_order = publish_topo.compute_publish_order


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _pkg(name: str, version: str = "0.1.0", deps: list[dict] | None = None) -> dict:
    """Build a minimal cargo-metadata package entry."""
    return {
        "name": name,
        "version": version,
        "id": f"{name} {version} (path+file:///fake/{name})",
        "manifest_path": f"/fake/{name}/Cargo.toml",
        "dependencies": deps or [],
    }


def _dep(name: str, kind: str | None = None) -> dict:
    """Build a minimal cargo-metadata dependency entry."""
    d: dict = {"name": name}
    if kind is not None:
        d["kind"] = kind
    return d


def _meta(packages: list[dict], allowlist: list[str]) -> dict:
    """Build a minimal cargo metadata dict."""
    workspace_members = [p["id"] for p in packages]
    return {
        "workspace_members": workspace_members,
        "packages": packages,
        "metadata": {"publish": {"allow": allowlist}},
    }


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


class TestLinearChain(unittest.TestCase):
    """A -> B -> C: must publish C, then B, then A."""

    def test_order(self) -> None:
        packages = [
            _pkg("a", deps=[_dep("b")]),
            _pkg("b", deps=[_dep("c")]),
            _pkg("c"),
        ]
        meta = _meta(packages, ["a", "b", "c"])
        result = compute_publish_order(meta)
        names = [r["name"] for r in result]
        self.assertLess(names.index("c"), names.index("b"))
        self.assertLess(names.index("b"), names.index("a"))

    def test_all_listed(self) -> None:
        packages = [
            _pkg("a", deps=[_dep("b")]),
            _pkg("b", deps=[_dep("c")]),
            _pkg("c"),
        ]
        meta = _meta(packages, ["a", "b", "c"])
        result = compute_publish_order(meta)
        self.assertEqual(len(result), 3)


class TestDevDepCrossingSccBoundary(unittest.TestCase):
    """
    perl-corpus dev-depends on perl-tdd-support (issue #3236 shape):
      perl-tdd-support has no deps on perl-corpus
      => no SCC cycle; dev-dep edge is kept; perl-tdd-support must come first.
    """

    def test_order_with_cross_scc_dev_dep(self) -> None:
        packages = [
            _pkg("perl-corpus", deps=[_dep("perl-tdd-support", kind="dev")]),
            _pkg("perl-tdd-support"),
        ]
        meta = _meta(packages, ["perl-tdd-support", "perl-corpus"])
        result = compute_publish_order(meta)
        names = [r["name"] for r in result]
        self.assertLess(
            names.index("perl-tdd-support"),
            names.index("perl-corpus"),
            "perl-tdd-support must be published before perl-corpus (dev-dep constraint)",
        )


class TestIntraSccDevDepCycle(unittest.TestCase):
    """
    Exact cycle shape from issue #3254:
      perl-module-import dev-depends on perl-module-token
      perl-module-token   has NO direct dep on perl-module-import

    But imagine a third crate that creates a cycle through normal deps,
    pulling both into the same SCC:
      perl-module-token    -> perl-module-base
      perl-module-import   -> perl-module-base (normal), dev -> perl-module-token

    Here perl-module-import and perl-module-token are in different SCCs,
    so the dev edge IS kept.  This test verifies the ordering is correct.

    The true intra-SCC case requires:
      A -> B (normal), B dev-> A

    This creates A and B in the same SCC; the intra-SCC dev edge B dev-> A
    is dropped; final order is alphabetical / whatever topo gives.
    """

    def test_intra_scc_dev_cycle_does_not_raise(self) -> None:
        """
        A and B form an intra-SCC cycle via A->B (normal) and B dev->A.
        The dev edge must be dropped; a valid order must be produced.
        """
        packages = [
            _pkg("a", deps=[_dep("b")]),          # a normally depends on b
            _pkg("b", deps=[_dep("a", kind="dev")]),  # b dev-depends on a (cycle!)
        ]
        meta = _meta(packages, ["a", "b"])
        # Must not raise SystemExit.
        result = compute_publish_order(meta)
        names = [r["name"] for r in result]
        self.assertEqual(set(names), {"a", "b"})

    def test_intra_scc_dev_cycle_publishes_b_before_a(self) -> None:
        """
        Since a normally depends on b, b must come before a in publish order.
        Even though b dev-depends on a (intra-SCC), the normal constraint wins.
        """
        packages = [
            _pkg("a", deps=[_dep("b")]),
            _pkg("b", deps=[_dep("a", kind="dev")]),
        ]
        meta = _meta(packages, ["a", "b"])
        result = compute_publish_order(meta)
        names = [r["name"] for r in result]
        self.assertLess(names.index("b"), names.index("a"))

    def test_module_import_token_shape(self) -> None:
        """
        Exact shape of #3254:
          perl-module-import dev-depends on perl-module-token
          perl-module-token has no dep on perl-module-import

        Cross-SCC, so dev edge is kept. perl-module-token must come first.
        """
        packages = [
            _pkg(
                "perl-module-import",
                deps=[
                    _dep("perl-module-path"),
                    _dep("perl-module-token", kind="dev"),
                ],
            ),
            _pkg("perl-module-token", deps=[_dep("perl-module-boundary")]),
            _pkg("perl-module-path"),
            _pkg("perl-module-boundary"),
        ]
        meta = _meta(
            packages,
            [
                "perl-module-boundary",
                "perl-module-path",
                "perl-module-token",
                "perl-module-import",
            ],
        )
        result = compute_publish_order(meta)
        names = [r["name"] for r in result]
        self.assertLess(
            names.index("perl-module-token"),
            names.index("perl-module-import"),
            "perl-module-token must precede perl-module-import",
        )


class TestNormalDepCycleFails(unittest.TestCase):
    """Normal dep cycles must cause hard exit(1)."""

    def test_cycle_exits(self) -> None:
        packages = [
            _pkg("a", deps=[_dep("b")]),
            _pkg("b", deps=[_dep("a")]),
        ]
        meta = _meta(packages, ["a", "b"])
        with self.assertRaises(SystemExit) as ctx:
            compute_publish_order(meta)
        self.assertEqual(ctx.exception.code, 1)


class TestEmptyAllowlistFails(unittest.TestCase):
    def test_empty_fails(self) -> None:
        packages = [_pkg("a")]
        meta = _meta(packages, [])
        with self.assertRaises(SystemExit) as ctx:
            compute_publish_order(meta)
        self.assertEqual(ctx.exception.code, 1)


class TestAllowlistCrateNotInWorkspaceFails(unittest.TestCase):
    def test_missing_crate_fails(self) -> None:
        packages = [_pkg("a")]
        meta = _meta(packages, ["a", "nonexistent-crate"])
        with self.assertRaises(SystemExit) as ctx:
            compute_publish_order(meta)
        self.assertEqual(ctx.exception.code, 1)


class TestAllowlistFiltering(unittest.TestCase):
    """Crates not in the allowlist must be excluded from the output."""

    def test_filtered(self) -> None:
        packages = [
            _pkg("a", deps=[_dep("b")]),
            _pkg("b"),
            _pkg("internal-helper"),  # not in allowlist
        ]
        meta = _meta(packages, ["a", "b"])
        result = compute_publish_order(meta)
        names = [r["name"] for r in result]
        self.assertNotIn("internal-helper", names)
        self.assertIn("a", names)
        self.assertIn("b", names)


if __name__ == "__main__":
    unittest.main()
