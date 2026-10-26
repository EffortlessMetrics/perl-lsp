#!/usr/bin/env python3
"""Exercise agent entrypoints with real just, but no native Cargo workloads.

Run with: python3 scripts/tests/test-agent-cargo-recipes.py -v
"""

import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
RECIPES = {
    "agent-check": ["check", "--workspace", "--all-targets", "--profile", "agent", "--locked"],
    "agent-test": ["test", "--workspace", "--all-targets", "--profile", "agent", "--locked"],
    "agent-clippy": [
        "clippy", "--workspace", "--all-targets", "--profile", "agent", "--locked",
        "--", "-D", "warnings", "-A", "missing_docs",
    ],
    "agent-nextest": ["nextest", "run", "--workspace", "--profile", "agent"],
    "agent-pr-fast": ["xtask", "gates", "--tier", "pr-fast", "--receipt"],
}


class AgentCargoRecipesTest(unittest.TestCase):
    def setUp(self):
        self.just = shutil.which("just")
        self.assertIsNotNone(self.just, "real just is required; tests must not silently skip")
        temporary = tempfile.TemporaryDirectory(prefix="agent-cargo-recipes-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        (self.root / "scripts").mkdir()
        self.bin = self.root / "bin"
        self.bin.mkdir()
        shutil.copyfile(REPO_ROOT / "justfile", self.root / "justfile")
        self.log = self.root / "call.json"
        self.env = os.environ.copy()
        for key in list(self.env):
            if key.startswith(("CARGO_", "SCCACHE_", "RUSTC_", "FAKE_")) or key in {
                "MIN_FREE_GB", "MAX_USED_PCT", "DEVPLANE", "TMPDIR", "JUST_UNSTABLE",
            }:
                self.env.pop(key)
        self.env.update(
            PATH=f"{self.bin}{os.pathsep}{os.environ['PATH']}",
            DEVPLANE=str(self.root / "devplane"),
            FAKE_REPO_ROOT=str(self.root),
            FAKE_CALL_LOG=str(self.log),
            FAKE_USED_PCT="90",
            FAKE_AVAIL_GB="29",
            FAKE_DF_LOG=str(self.root / "df.json"),
            FAKE_LOCK_LOG=str(self.root / "lock.json"),
        )
        self.write_program(self.root / "scripts/cargo-safe", """
import json, os, sys
from pathlib import Path
Path(os.environ['FAKE_CALL_LOG']).write_text(json.dumps({
    'args': sys.argv[1:], 'env': dict(os.environ),
}))
sys.exit(int(os.environ.get('FAKE_CARGO_EXIT', '0')))
""")

    def write_program(self, path, body):
        path.write_text(f"#!{sys.executable}\n{body}")
        path.chmod(0o755)

    def run_recipe(self, recipe, **overrides):
        return subprocess.run(
            [self.just, "--justfile", str(self.root / "justfile"), recipe],
            cwd=self.root, env={**self.env, **overrides}, capture_output=True, text=True,
        )

    def read_call(self):
        return json.loads(self.log.read_text())

    def use_real_wrapper(self):
        shutil.copyfile(REPO_ROOT / "scripts/cargo-safe", self.root / "scripts/cargo-safe")
        # Only the Cargo executable is stubbed; actual cargo-safe handles admission,
        # environment routing, and the decision to call flock.
        self.write_program(self.bin / "cargo", """
import json, os, sys
from pathlib import Path
Path(os.environ['FAKE_CALL_LOG']).write_text(json.dumps({
    'args': sys.argv[1:], 'env': dict(os.environ),
}))
sys.exit(int(os.environ.get('FAKE_CARGO_EXIT', '0')))
""")
        self.write_program(self.bin / "git", """
import os
print(os.environ['FAKE_REPO_ROOT'])
""")
        self.write_program(self.bin / "df", """
import json, os, sys
from pathlib import Path
Path(os.environ['FAKE_DF_LOG']).write_text(json.dumps(sys.argv[1:]))
print('Filesystem 1024-blocks Used Available Capacity Mounted on')
print('fixture 999999999 0', int(os.environ['FAKE_AVAIL_GB']) * 1024 * 1024,
      os.environ['FAKE_USED_PCT'] + '%', '/')
""")
        self.write_program(self.bin / "flock", """
import json, os, sys
from pathlib import Path
Path(os.environ['FAKE_LOCK_LOG']).write_text(json.dumps(sys.argv[1:]))
if os.environ.get('FAKE_FLOCK_EXIT'):
    sys.exit(int(os.environ['FAKE_FLOCK_EXIT']))
os.execvp(sys.argv[4], sys.argv[4:])
""")
        self.write_program(self.bin / "sccache", "raise SystemExit('unexpected execution')\n")

    def test_all_agent_recipes_forward_defaults_and_arguments(self):
        for recipe, arguments in RECIPES.items():
            with self.subTest(recipe=recipe):
                result = self.run_recipe(recipe)
                self.assertEqual(result.returncode, 0, result.stderr)
                call = self.read_call()
                self.assertEqual(call["args"], arguments)
                self.assertEqual(call["env"]["MIN_FREE_GB"], "20")
                self.assertEqual(call["env"]["MAX_USED_PCT"], "95")

    def test_caller_thresholds_override_independently(self):
        for recipe in RECIPES:
            for overrides, expected in (
                ({"MIN_FREE_GB": "45"}, ("45", "95")),
                ({"MAX_USED_PCT": "80"}, ("20", "80")),
                ({"MIN_FREE_GB": "45", "MAX_USED_PCT": "80"}, ("45", "80")),
                ({"MIN_FREE_GB": "", "MAX_USED_PCT": ""}, ("20", "95")),
            ):
                with self.subTest(recipe=recipe, overrides=overrides):
                    result = self.run_recipe(recipe, **overrides)
                    self.assertEqual(result.returncode, 0, result.stderr)
                    env = self.read_call()["env"]
                    self.assertEqual((env["MIN_FREE_GB"], env["MAX_USED_PCT"]), expected)

    def test_agent_recipes_propagate_wrapper_failure(self):
        for recipe in RECIPES:
            with self.subTest(recipe=recipe):
                result = self.run_recipe(recipe, FAKE_CARGO_EXIT="37")
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("37", result.stderr)

    def test_non_agent_recipe_does_not_set_agent_thresholds(self):
        result = self.run_recipe("check-agent-capabilities")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.read_call()["args"], ["xtask", "check-agent-capabilities"])
        env = self.read_call()["env"]
        self.assertNotIn("MIN_FREE_GB", env)
        self.assertNotIn("MAX_USED_PCT", env)

    def test_real_wrapper_admits_agent_fixture_and_keeps_storage_and_lock(self):
        self.use_real_wrapper()
        for recipe in ("agent-check", "agent-test", "agent-clippy", "agent-nextest"):
            with self.subTest(recipe=recipe):
                result = self.run_recipe(recipe)
                self.assertEqual(result.returncode, 0, result.stderr)
                call = self.read_call()
                self.assertEqual(call["args"], RECIPES[recipe])
                devplane = self.env["DEVPLANE"]
                for key, suffix in (
                    ("CARGO_HOME", "cargo-home"), ("CARGO_TARGET_DIR", "target"),
                    ("CARGO_BUILD_BUILD_DIR", "build"), ("TMPDIR", "tmp"),
                    ("SCCACHE_DIR", "sccache"),
                ):
                    self.assertEqual(call["env"][key], f"{devplane}/{suffix}")
                for key, expected in (
                    ("CARGO_INCREMENTAL", "0"), ("CARGO_BUILD_JOBS", "2"),
                    ("RUSTC_WRAPPER", "sccache"), ("SCCACHE_CACHE_SIZE", "15G"),
                ):
                    self.assertEqual(call["env"][key], expected)
                self.assertEqual(json.loads((self.root / "df.json").read_text()), ["-Pk", devplane])
                self.assertEqual(
                    json.loads((self.root / "lock.json").read_text()),
                    ["-w", "180", f"{devplane}/locks/cargo-build.lock", "cargo", *RECIPES[recipe]],
                )

    def test_real_wrapper_preserves_storage_and_lock_overrides(self):
        self.use_real_wrapper()
        overrides = {
            "CARGO_HOME": str(self.root / "custom cargo"),
            "CARGO_TARGET_DIR": str(self.root / "custom target"),
            "CARGO_BUILD_BUILD_DIR": str(self.root / "custom build"),
            "CARGO_INCREMENTAL": "0", "CARGO_BUILD_JOBS": "1",
            "TMPDIR": str(self.root / "custom tmp"), "CARGO_LOCK_WAIT": "23",
            "SCCACHE_DIR": str(self.root / "custom cache"),
            "SCCACHE_CACHE_SIZE": "2G", "RUSTC_WRAPPER": "custom-wrapper",
        }
        result = self.run_recipe("agent-check", **overrides)
        self.assertEqual(result.returncode, 0, result.stderr)
        env = self.read_call()["env"]
        for key, expected in overrides.items():
            self.assertEqual(env[key], expected)
        self.assertEqual(json.loads((self.root / "lock.json").read_text())[:2], ["-w", "23"])

    def test_agent_refusal_preserved_at_disk_boundaries_and_stricter_overrides(self):
        self.use_real_wrapper()
        for recipe in ("agent-check", "agent-test", "agent-clippy", "agent-nextest"):
            for overrides in (
                {"FAKE_AVAIL_GB": "19", "FAKE_USED_PCT": "1"},
                {"FAKE_AVAIL_GB": "29", "FAKE_USED_PCT": "95"},
                {"MIN_FREE_GB": "30"}, {"MAX_USED_PCT": "90"},
            ):
                with self.subTest(recipe=recipe, overrides=overrides):
                    result = self.run_recipe(recipe, **overrides)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn("DENY:", result.stderr)
                    self.assertIn("75", result.stderr)
                    self.assertFalse(self.log.exists(), "refusal must precede Cargo")
                    self.assertFalse((self.root / "lock.json").exists())

    def test_direct_wrapper_retains_non_agent_fallback(self):
        self.use_real_wrapper()
        for free, used, expected in (("29", "1", 75), ("50", "85", 75), ("40", "84", 0)):
            with self.subTest(free=free, used=used):
                self.log.unlink(missing_ok=True)
                result = subprocess.run(
                    [str(self.root / "scripts/cargo-safe"), "check", "--locked"],
                    cwd=self.root,
                    env={**self.env, "FAKE_AVAIL_GB": free, "FAKE_USED_PCT": used},
                    capture_output=True, text=True,
                )
                self.assertEqual(result.returncode, expected, result.stderr)
                self.assertEqual(self.log.exists(), expected == 0)

    def test_real_wrapper_preserves_cargo_and_lock_failures(self):
        self.use_real_wrapper()
        result = self.run_recipe("agent-check", FAKE_FLOCK_EXIT="73")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("73", result.stderr)
        self.assertFalse(self.log.exists())
        result = self.run_recipe("agent-check", FAKE_CARGO_EXIT="37")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("37", result.stderr)
        self.assertEqual(self.read_call()["args"], RECIPES["agent-check"])


if __name__ == "__main__":
    unittest.main()
