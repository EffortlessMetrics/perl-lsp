#!/usr/bin/env python3
"""Exercise cargo-safe admission with fake disk/Cargo and real private flock."""

import fcntl
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import unittest


SOURCE = Path(__file__).resolve().parents[1] / "cargo-safe"
GIB_KB = 1024 * 1024


class CargoSafeAdmission(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="cargo-safe-admission-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.devplane = self.root / "dev plane"
        self.lock = self.devplane / "locks/cargo-build.lock"
        self.lock.parent.mkdir(parents=True)
        self.wrapper = self.root / "cargo-safe"
        self.wrapper.write_bytes(SOURCE.read_bytes())
        self.wrapper.chmod(0o755)
        for name in ["bash", "mkdir", "basename", "dirname", "awk", "flock"]:
            executable = shutil.which(name)
            self.assertIsNotNone(executable, f"fixture requires {name}")
            (self.bin / name).symlink_to(executable)
        self.env = {
            "PATH": str(self.bin),
            "HOME": str(self.root / "home"),
            "DEVPLANE": str(self.devplane),
            "CARGO_SAFE_TEST_ROOT": str(self.root),
            "CARGO_SAFE_TEST_STATUS": "17",
            "MIN_FREE_GB": "20",
            "MAX_USED_PCT": "95",
            "CARGO_LOCK_WAIT": "1",
        }
        self.stub("git", 'print(os.environ["CARGO_SAFE_TEST_ROOT"])')
        self.stub(
            "df",
            '''root = Path(os.environ["CARGO_SAFE_TEST_ROOT"])
state = json.loads((root / "disk.json").read_text())
with (root / "probes.jsonl").open("a") as out:
    out.write(json.dumps({"args": sys.argv[1:], "state": state}) + "\\n")
if state.get("failure"):
    sys.exit(62)
print("Filesystem 1024-blocks Used Available Capacity Mounted on")
print(f"fixture 100000000 1 {state['available_kb']} {state['used_pct']}% /fixture")
''',
        )
        self.stub(
            "cargo",
            '''import fcntl
root = Path(os.environ["CARGO_SAFE_TEST_ROOT"])
held = False
with (Path(os.environ["DEVPLANE"]) / "locks/cargo-build.lock").open("a") as lock:
    try:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        held = True
record = {"args": sys.argv[1:], "lock_held": held, "env": {
    key: os.environ.get(key) for key in ["CARGO_HOME", "CARGO_TARGET_DIR",
    "CARGO_BUILD_BUILD_DIR", "CARGO_INCREMENTAL", "CARGO_BUILD_JOBS", "TMPDIR"]}}
(root / "cargo.json").write_text(json.dumps(record))
sys.exit(int(os.environ["CARGO_SAFE_TEST_STATUS"]))
''',
        )
        self.disk(available_kb=50 * GIB_KB, used_pct=10)

    def stub(self, name, body):
        script = self.bin / name
        script.write_text(
            f"#!{sys.executable}\nimport json, os, sys\nfrom pathlib import Path\n" + body + "\n"
        )
        script.chmod(0o755)

    def disk(self, available_kb=50 * GIB_KB, used_pct=10, failure=False):
        state = self.root / "next-disk.json"
        state.write_text(json.dumps(dict(available_kb=available_kb, used_pct=used_pct, failure=failure)))
        state.replace(self.root / "disk.json")

    def probes(self):
        path = self.root / "probes.jsonl"
        return [json.loads(line) for line in path.read_text().splitlines()] if path.exists() else []

    def run_wrapper(self, *args):
        return subprocess.run(
            [str(self.wrapper), *args], cwd=self.root, env=self.env,
            capture_output=True, text=True, timeout=5,
        )

    def wait_for_probe(self, child):
        deadline = time.monotonic() + 3
        while time.monotonic() < deadline:
            if self.probes():
                return
            self.assertIsNone(child.poll(), "wrapper exited before its initial disk probe")
            time.sleep(0.01)
        self.fail("wrapper did not reach its initial disk probe")

    def contended_run(self, state, *args):
        with self.lock.open("a") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            child = subprocess.Popen(
                [str(self.wrapper), *args], cwd=self.root, env=self.env,
                stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True,
            )
            try:
                self.wait_for_probe(child)
                self.assertIsNone(child.poll())
                self.assertFalse((self.root / "cargo.json").exists(), "Cargo ran while lock was held")
                self.disk(**state)
                fcntl.flock(lock, fcntl.LOCK_UN)
                stdout, stderr = child.communicate(timeout=5)
                return child.returncode, stdout, stderr
            finally:
                if child.poll() is None:
                    os.killpg(child.pid, signal.SIGKILL)
                    child.communicate(timeout=5)

    def test_rechecks_free_space_after_waiting_for_lock(self):
        code, _, stderr = self.contended_run({"available_kb": 20 * GIB_KB - 1}, "test", "--locked")
        self.assertEqual(code, 75, stderr)
        self.assertFalse((self.root / "cargo.json").exists())
        self.assertEqual(len(self.probes()), 2)
        self.assertIn("DENY:", stderr)

    def test_rechecks_usage_after_waiting_for_lock(self):
        code, _, stderr = self.contended_run({"used_pct": 95}, "check")
        self.assertEqual(code, 75, stderr)
        self.assertFalse((self.root / "cargo.json").exists())

    def test_post_lock_success_preserves_arguments_environment_status_and_lock(self):
        overrides = {
            "CARGO_HOME": str(self.root / "custom home"),
            "CARGO_TARGET_DIR": str(self.root / "custom target"),
            "CARGO_BUILD_BUILD_DIR": str(self.root / "custom build"),
            "CARGO_INCREMENTAL": "0", "CARGO_BUILD_JOBS": "3",
            "TMPDIR": str(self.root / "custom tmp"),
        }
        self.env.update(overrides)
        args = ["test", "--locked", "--", "case with spaces", "literal;$value"]
        code, _, stderr = self.contended_run({"available_kb": 20 * GIB_KB, "used_pct": 94}, *args)
        self.assertEqual(code, 17, stderr)
        record = json.loads((self.root / "cargo.json").read_text())
        self.assertEqual(record, {"args": args, "lock_held": True, "env": overrides})
        self.assertEqual(len(self.probes()), 2)
        self.assertTrue(all(p["args"] == ["-Pk", str(self.devplane)] for p in self.probes()))

    def test_initial_denial_does_not_wait_for_lock_or_run_cargo(self):
        self.disk(available_kb=19 * GIB_KB)
        with self.lock.open("a") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            result = self.run_wrapper("build")
        self.assertEqual(result.returncode, 75, result.stderr)
        self.assertEqual(len(self.probes()), 1)
        self.assertFalse((self.root / "cargo.json").exists())

    def test_lock_timeout_does_not_run_cargo(self):
        self.env["CARGO_LOCK_WAIT"] = "0.05"
        with self.lock.open("a") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            result = self.run_wrapper("build")
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertEqual(len(self.probes()), 1)
        self.assertFalse((self.root / "cargo.json").exists())
        self.assertNotIn("DENY:", result.stderr)

    def test_invalid_lock_wait_does_not_run_cargo_or_report_disk_denial(self):
        self.env["CARGO_LOCK_WAIT"] = "not-a-duration"
        result = self.run_wrapper("build")
        self.assertNotIn(result.returncode, [0, 75], result.stderr)
        self.assertEqual(len(self.probes()), 1)
        self.assertFalse((self.root / "cargo.json").exists())

    def test_failing_initial_probe_does_not_run_cargo(self):
        self.disk(failure=True)
        result = self.run_wrapper("build")
        self.assertNotEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.root / "cargo.json").exists())

    def test_missing_disk_probe_does_not_run_cargo(self):
        (self.bin / "df").unlink()
        result = self.run_wrapper("build")
        self.assertNotEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.root / "cargo.json").exists())

    def test_failing_post_lock_probe_does_not_run_cargo(self):
        code, _, stderr = self.contended_run({"failure": True}, "build")
        self.assertNotEqual(code, 0, stderr)
        self.assertFalse((self.root / "cargo.json").exists())

    def test_absent_flock_keeps_existing_admission_and_status(self):
        (self.bin / "flock").unlink()
        result = self.run_wrapper("build")
        self.assertEqual(result.returncode, 17, result.stderr)
        self.assertEqual(len(self.probes()), 1)
        self.assertFalse(json.loads((self.root / "cargo.json").read_text())["lock_held"])

    def test_nonheavy_command_keeps_existing_unlocked_path(self):
        self.disk(available_kb=0, used_pct=100)
        result = self.run_wrapper("--version")
        self.assertEqual(result.returncode, 17, result.stderr)
        self.assertEqual(self.probes(), [])
        self.assertFalse(json.loads((self.root / "cargo.json").read_text())["lock_held"])

    def test_all_existing_heavy_commands_recheck_and_keep_the_lock(self):
        for command in ["build", "check", "test", "run", "bench", "doc", "clippy", "nextest"]:
            with self.subTest(command=command):
                before = len(self.probes())
                result = self.run_wrapper(command, "--locked")
                self.assertEqual(result.returncode, 17, result.stderr)
                self.assertEqual(len(self.probes()) - before, 2)
                self.assertTrue(json.loads((self.root / "cargo.json").read_text())["lock_held"])

    def test_default_thresholds_remain_40_gib_and_85_percent(self):
        self.env.pop("MIN_FREE_GB")
        self.env.pop("MAX_USED_PCT")
        for available, used, expected in [(40 * GIB_KB - 1, 84, 75), (40 * GIB_KB, 85, 75), (40 * GIB_KB, 84, 17)]:
            with self.subTest(available=available, used=used):
                self.disk(available_kb=available, used_pct=used)
                result = self.run_wrapper("build")
                self.assertEqual(result.returncode, expected, result.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)
