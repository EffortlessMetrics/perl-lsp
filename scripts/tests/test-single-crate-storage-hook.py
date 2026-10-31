#!/usr/bin/env python3
"""Exercise the real pre-push hook with private Git/Cargo/storage fixtures."""

import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import tempfile
import time
import unittest


ROOT = Path(__file__).resolve().parents[2]
PARSER = argparse.ArgumentParser()
PARSER.add_argument("--xtask", required=True)
ARGS, REST = PARSER.parse_known_args()

CARGO = r'''#!/usr/bin/env python3
import fcntl,json,os,signal,sys,time
from pathlib import Path
a=sys.argv[1:]
trace=Path(os.environ['FIXTURE_TRACE'])
with trace.open('a') as out:
 out.write(json.dumps({'argv':a,'cwd':os.getcwd(),'git':{k:v for k,v in os.environ.items() if k.startswith('GIT_')},'routing':{k:os.environ.get(k) for k in ['CARGO_TARGET_DIR','CARGO_BUILD_BUILD_DIR','CARGO_PROFILE_DEV_DEBUG','CARGO_PROFILE_TEST_DEBUG','CARGO_PKG_NAME','CARGO_MANIFEST_DIR']}})+'\n')
if a==['--version']: print('cargo fixture');sys.exit(0)
if a[:2]==['xtask','change-set']: print('crates/probe/src/lib.rs');sys.exit(0)
if a[:2]==['xtask','resolve-package-name']: print(os.environ.get('FIXTURE_PACKAGE','probe'));sys.exit(0)
if a[:5]==['run','--package','xtask','--locked','--']:
 os.execv(os.environ['FIXTURE_XTASK'],[os.environ['FIXTURE_XTASK']]+a[5:])
if not a or a[0] not in ['fmt','clippy','test']: sys.exit(91)
lock_path=Path(os.environ['DEVPLANE'])/'locks/cargo-build.lock'
lock_path.parent.mkdir(parents=True,exist_ok=True)
lock=open(lock_path,'a')
try:
 fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
 Path(os.environ['FIXTURE_UNLOCKED']).write_text('substantive check ran without lock')
except BlockingIOError: pass
mode=os.environ['FIXTURE_MODE']
if mode.startswith('receipt-error:') and a[0]=='fmt':
 receipt=Path.cwd()/'target/receipts/single-crate-hook/receipt.json'
 receipt.rename(receipt.with_suffix('.before.json'));receipt.mkdir()
 print('fixture check finished')
 sys.exit(int(mode.split(':')[1]))
if mode.startswith('output-error:') and a[0]=='fmt':
 print('fixture output',flush=True)
 Path(os.environ['FIXTURE_STAGE_STARTED']).write_text('ready')
 deadline=time.monotonic()+10
 while not Path(os.environ['FIXTURE_RELEASE']).exists() and time.monotonic()<deadline:time.sleep(.01)
 sys.exit(int(mode.split(':')[1]))
if mode=='volume-loss' and a[0]=='clippy':
 route=Path(os.environ['CARGO_BUILD_BUILD_DIR']);route.unlink();route.symlink_to(route.parent/'absent-volume')
 signal.signal(signal.SIGTERM,signal.SIG_IGN)
 child=os.fork()
 if child==0:
  signal.signal(signal.SIGTERM,signal.SIG_IGN)
  deadline=time.monotonic()+10
  while time.monotonic()<deadline: time.sleep(.05)
  os._exit(92)
 Path(os.environ['FIXTURE_PIDS']).write_text(json.dumps([os.getpid(),child]))
 deadline=time.monotonic()+10
 while time.monotonic()<deadline: time.sleep(.05)
 sys.exit(92)
if mode=='large-output' and a[0]=='test':
 sys.stdout.write('x'*(5*1024*1024));sys.stdout.write('\nFINAL-OUTPUT-MARKER\n')
if mode=='signal' and a[0]=='clippy': os.kill(os.getpid(),signal.SIGTERM)
if mode.startswith(a[0]+':'): sys.exit(int(mode.split(':')[1]))
sys.exit(0)
'''


def git_environment():
    # Isolation applies only to disposable fixture repositories, never the real hook.
    env = os.environ.copy()
    for key in list(env):
        if key.startswith("GIT_"):
            del env[key]
    return env


def snapshot(git_dir):
    return {
        str(p.relative_to(git_dir)): hashlib.sha256(p.read_bytes()).hexdigest()
        for p in git_dir.rglob("*")
        if p.is_file() and p.relative_to(git_dir).parts[0] in
        {"HEAD", "index", "config", "objects", "refs", "logs"}
    }


def fixture_temp_parent(env):
    # Match cargo-safe's managed storage default for this source checkout. A
    # direct normal hook need not have inherited cargo-safe's exported TMPDIR.
    # Never lower admission or fall back to another volume after a denial.
    cache = Path(env.get("XDG_CACHE_HOME") or Path.home() / ".cache")
    devplane = Path(env.get("DEVPLANE") or cache / "devplane" / ROOT.name)
    return devplane / "tmp"


class FixtureStorageRouting(unittest.TestCase):
    def test_managed_and_default_roots_do_not_depend_on_tmpdir(self):
        with tempfile.TemporaryDirectory() as root:
            base = Path(root)
            env = {"DEVPLANE": str(base / "managed"), "XDG_CACHE_HOME": str(base / "cache"),
                   "TMPDIR": str(base / "unrelated")}
            self.assertEqual(fixture_temp_parent(env), base / "managed/tmp")
            del env["DEVPLANE"]
            self.assertEqual(fixture_temp_parent(env), base / "cache/devplane" / ROOT.name / "tmp")
            del env["XDG_CACHE_HOME"]
            self.assertEqual(fixture_temp_parent(env), Path.home() / ".cache/devplane" / ROOT.name / "tmp")


class SingleCrateHook(unittest.TestCase):
    def setUp(self):
        parent = fixture_temp_parent(os.environ)
        parent.mkdir(parents=True, exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(prefix="single-crate-hook-", dir=parent)
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.assertEqual(self.root.parent.resolve(), parent.resolve())
        self.assertEqual(self.root.stat().st_dev, parent.stat().st_dev)
        self.caller = self.root / "caller"
        self.receiver = self.root / "receiver.git"
        self.env = git_environment()
        self.run_git("init", "-q", str(self.caller), cwd=self.root)
        self.run_git("init", "-q", "--bare", str(self.receiver), cwd=self.root)
        self.run_git("config", "user.name", "Storage Hook Fixture")
        self.run_git("config", "user.email", "storage-hook@example.invalid")
        source = self.caller / "crates/probe/src/lib.rs"
        source.parent.mkdir(parents=True)
        source.write_text("pub fn probe() {}\n")
        self.run_git("add", ".")
        self.run_git("commit", "-qm", "fixture base")
        self.run_git("push", "-q", str(self.receiver), "HEAD:refs/heads/base")
        source.write_text("pub fn probe() { let _value = 1; }\n")
        self.run_git("add", ".")
        self.run_git("commit", "-qm", "single-crate fixture candidate")
        for rel in ["hooks/pre-push", "scripts/cargo-safe"]:
            dest = self.caller / rel
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(ROOT / rel, dest)
        shutil.copy2(ROOT / "hooks/pre-push", self.caller / ".git/hooks/pre-push")
        self.bin = self.root / "bin"
        self.bin.mkdir()
        (self.bin / "cargo").write_text(CARGO)
        (self.bin / "cargo").chmod(0o755)
        self.devplane = self.root / "devplane"
        self.devplane.mkdir()
        route_target = self.root / "build-volume"
        route_target.mkdir()
        self.route = self.root / "build-route"
        self.route.symlink_to(route_target)
        self.trace = self.root / "cargo.jsonl"
        self.push_count = 0
        self.env.update(
            PATH=str(self.bin) + os.pathsep + self.env["PATH"],
            DEVPLANE=str(self.devplane),
            CARGO_HOME=str(self.devplane / "cargo-home"),
            CARGO_TARGET_DIR=str(self.devplane / "target"),
            CARGO_BUILD_BUILD_DIR=str(self.route),
            MIN_FREE_GB="20", MAX_USED_PCT="95", CARGO_LOCK_WAIT="3",
            CARGO_PROFILE_DEV_DEBUG="fixture-dev", CARGO_PROFILE_TEST_DEBUG="fixture-test",
            CARGO_PKG_NAME="outer-package", CARGO_MANIFEST_DIR="outer-manifest",
            FIXTURE_XTASK=str(Path(ARGS.xtask).resolve()), FIXTURE_TRACE=str(self.trace),
            FIXTURE_UNLOCKED=str(self.root / "unlocked"),
            FIXTURE_PIDS=str(self.root / "pids.json"), FIXTURE_MODE="pass",
        )
        self.env.pop("SCCACHE_DIR", None)
        self.env.pop("RUSTC_WRAPPER", None)
        # A real caller selector survives production supervision. Only fixture setup
        # above sanitized inherited selectors so it could create its own repositories.
        self.env["GIT_DIR"] = str(self.caller / ".git")
        self.env["GIT_WORK_TREE"] = str(self.caller)
        self.env["GIT_INDEX_FILE"] = str(self.caller / ".git/index")
        self.env["GIT_CONFIG_COUNT"] = "1"
        self.env["GIT_CONFIG_KEY_0"] = "storage.fixture"
        self.env["GIT_CONFIG_VALUE_0"] = "preserved"

    def run_git(self, *args, cwd=None):
        return subprocess.run(["git", *args], cwd=cwd or self.caller, env=self.env,
                              text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True)

    def push(self, mode="pass", supervised=True):
        self.env["FIXTURE_MODE"] = mode
        self.push_count += 1
        before = snapshot(self.caller / ".git")
        process = subprocess.Popen(
            ["git", "push", str(self.receiver), "HEAD:refs/heads/probe-" + str(self.push_count)],
            cwd=self.caller, env=self.env, stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT, text=True, start_new_session=True,
        )
        try:
            output, _ = process.communicate(timeout=25)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, 9)
            output, _ = process.communicate()
            self.fail("fixture push exceeded bounded test deadline: " + output[-4000:])
        self.assertEqual(snapshot(self.caller / ".git"), before, "hook mutated caller Git state")
        rows = [json.loads(line) for line in self.trace.read_text().splitlines()]
        self.assertEqual(any(r["argv"][:5] == ["run", "--package", "xtask", "--locked", "--"]
                             for r in rows), supervised,
                         "single-crate checks bypassed the storage supervisor or used the wrong route")
        receipt_path = self.caller / "target/receipts/single-crate-hook/receipt.json"
        receipt = json.loads(receipt_path.read_text()) if receipt_path.is_file() else None
        return process.returncode, output, rows, receipt

    def test_exact_commands_lock_context_and_receipt(self):
        status, output, rows, receipt = self.push()
        self.assertEqual(status, 0, output[-4000:])
        commands = [r for r in rows if r["argv"][0] in ["fmt", "clippy", "test"]]
        self.assertEqual([r["argv"] for r in commands], [
            ["fmt", "-p", "probe", "--", "--check"],
            ["clippy", "-p", "probe", "--", "-D", "warnings"], ["test", "-p", "probe"],
        ])
        bootstrap = next(r for r in rows if r["argv"][0] == "run")
        for row in commands:
            self.assertEqual(row["cwd"], str(self.caller))
            self.assertEqual(row["git"], bootstrap["git"])
            for key in ["CARGO_TARGET_DIR", "CARGO_BUILD_BUILD_DIR", "CARGO_PROFILE_DEV_DEBUG", "CARGO_PROFILE_TEST_DEBUG"]:
                self.assertEqual(row["routing"][key], bootstrap["routing"][key])
            self.assertIsNone(row["routing"]["CARGO_PKG_NAME"])
            self.assertIsNone(row["routing"]["CARGO_MANIFEST_DIR"])
        self.assertFalse((self.root / "unlocked").exists())
        self.assertEqual(receipt["summary"]["overall_status"], "pass")
        self.assertEqual(receipt["metadata"]["git_sha"], self.run_git("rev-parse", "HEAD").stdout.strip())
        with (self.devplane / "locks/cargo-build.lock").open("a") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)

    def test_failures_stop_later_checks_and_preserve_status(self):
        for command, code in [("fmt", 17), ("clippy", 124), ("test", 75)]:
            with self.subTest(command=command):
                if self.trace.exists(): self.trace.unlink()
                status, output, rows, receipt = self.push(command + ":" + str(code))
                self.assertNotEqual(status, 0)
                self.assertIn("exit " + str(code), output)
                gates = receipt["gates"]
                index = ["fmt", "clippy", "test"].index(command)
                self.assertEqual(gates[index]["exit_code"], code)
                self.assertEqual(gates[index]["status"], "fail")
                self.assertEqual([g["status"] for g in gates[index+1:]], ["skip"] * (2-index))
                self.assertEqual([r["argv"][0] for r in rows if r["argv"][0] in ["fmt", "clippy", "test"]], ["fmt", "clippy", "test"][:index+1])

    def test_signal_exit_is_preserved(self):
        status, output, _, receipt = self.push("signal")
        self.assertNotEqual(status, 0)
        self.assertIn("exit 143", output)
        self.assertEqual(receipt["gates"][1]["exit_code"], 143)
        self.assertEqual(receipt["gates"][1]["status"], "fail")

    def test_runtime_volume_loss_stops_resistant_group_and_retains_evidence(self):
        status, output, rows, receipt = self.push("volume-loss")
        self.assertNotEqual(status, 0)
        self.assertIn("exit 75", output)
        self.assertEqual(receipt["gates"][1]["status"], "error")
        self.assertEqual(receipt["gates"][2]["status"], "skip")
        self.assertFalse(any(r["argv"][0] == "test" for r in rows))
        evidence = list((self.caller / "target/receipts/single-crate-hook").glob("gate-storage-*.json"))
        self.assertEqual(len(evidence), 1)
        self.assertEqual(json.loads(evidence[0].read_text())["status"], "error")
        for pid in json.loads((self.root / "pids.json").read_text()):
            state = Path("/proc") / str(pid) / "stat"
            self.assertTrue(not state.exists() or state.read_text().split()[2] == "Z",
                            "known resistant fixture child still running")

    def test_large_output_is_bounded(self):
        status, output, _, receipt = self.push("large-output")
        self.assertEqual(status, 0, output[-2000:])
        log = self.caller / "target/receipts/single-crate-hook/logs/test.log"
        self.assertLessEqual(log.stat().st_size, 4 * 1024 * 1024)
        self.assertIn("FINAL-OUTPUT-MARKER", log.read_text())
        self.assertEqual(receipt["summary"]["overall_status"], "pass")

    def test_post_lock_admission_denial_never_starts_checks(self):
        counter = self.root / "df-count"
        script = self.bin / "df"
        script.write_text("#!/usr/bin/env python3\nfrom pathlib import Path\np=Path(" + repr(str(counter)) + ")\nn=int(p.read_text())+1 if p.exists() else 1\np.write_text(str(n))\nprint('Filesystem 1024-blocks Used Available Capacity Mounted on')\nprint('fixture 104857600 1000 '+str((40 if n==1 else 19)*1024*1024)+' 1% /fixture')\n")
        script.chmod(0o755)
        status, output, rows, receipt = self.push(supervised=False)
        self.assertNotEqual(status, 0)
        self.assertIn("exit 75", output)
        self.assertEqual(counter.read_text(), "2")
        self.assertFalse(any(r["argv"][0] in ["fmt", "clippy", "test"] for r in rows))
        self.assertIsNone(receipt, "supervisor did not start; no runtime receipt is claimed")
        with (self.devplane / "locks/cargo-build.lock").open("a") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)

    def test_non_linux_keeps_original_commands(self):
        uname = self.bin / "uname"
        uname.write_text("#!/bin/sh\nprintf 'Darwin\\n'\n")
        uname.chmod(0o755)
        status, output, rows, receipt = self.push(supervised=False)
        self.assertEqual(status, 0, output[-4000:])
        self.assertIsNone(receipt)
        self.assertEqual([r["argv"] for r in rows if r["argv"][0] in ["fmt", "clippy", "test"]], [
            ["fmt", "-p", "probe", "--", "--check"],
            ["clippy", "-p", "probe", "--", "-D", "warnings"], ["test", "-p", "probe"],
        ])

    def test_package_metacharacters_remain_one_literal_argument(self):
        marker = self.root / "injected"
        package = "probe; touch " + str(marker)
        self.env["FIXTURE_PACKAGE"] = package
        status, output, rows, receipt = self.push()
        self.assertEqual(status, 0, output[-4000:])
        self.assertFalse(marker.exists())
        for row in rows:
            if row["argv"][0] in ["fmt", "clippy", "test"]:
                self.assertEqual(row["argv"][2], package)
        self.assertEqual(receipt["summary"]["overall_status"], "pass")

    def test_platform_detection_failure_does_not_run_unsupervised_checks(self):
        uname = self.bin / "uname"
        for program in ["exit 1", "exit 0", "printf 'unknown-kernel\\n'"]:
            with self.subTest(program=program):
                uname.write_text("#!/bin/sh\n" + program + "\n")
                uname.chmod(0o755)
                if self.trace.exists(): self.trace.unlink()
                status, output, rows, receipt = self.push(supervised=False)
                self.assertNotEqual(status, 0)
                self.assertIn("exit 70", output)
                self.assertFalse(any(r["argv"][0] in ["fmt", "clippy", "test"] for r in rows))
                self.assertIsNone(receipt)

    def test_receipt_error_keeps_failed_check_status_and_blocks_after_success(self):
        for child_code, expected in [(17, 17), (0, 70)]:
            with self.subTest(child_code=child_code):
                if self.trace.exists(): self.trace.unlink()
                directory = self.caller / "target/receipts/single-crate-hook"
                receipt_path = directory / "receipt.json"
                if receipt_path.is_dir(): receipt_path.rmdir()
                status, output, rows, receipt = self.push("receipt-error:" + str(child_code))
                self.assertNotEqual(status, 0)
                self.assertIn("exit " + str(expected), output)
                self.assertIn("Cannot persist single-crate hook receipt", output)
                self.assertIsNone(receipt)
                before = json.loads((directory / "receipt.before.json").read_text())
                self.assertEqual(before["summary"]["overall_status"], "fail")
                self.assertFalse(any(r["argv"][0] in ["clippy", "test"] for r in rows))

    def test_output_error_keeps_failed_check_status_and_records_success_as_error(self):
        # The same wrapper entry used by the hook, with a deterministically closed
        # output consumer after the substantive check starts. No real disk fill.
        for child_code, expected in [(17, 17), (0, 70)]:
            with self.subTest(child_code=child_code):
                started, release = self.root / "started", self.root / "release"
                for path in [started, release]: path.unlink(missing_ok=True)
                self.env.update(FIXTURE_MODE="output-error:" + str(child_code),
                                FIXTURE_STAGE_STARTED=str(started), FIXTURE_RELEASE=str(release))
                before = snapshot(self.caller / ".git")
                with (self.root / "output-error.log").open("w") as log:
                    process = subprocess.Popen(
                        [str(self.caller / "scripts/cargo-safe"), "run", "--package", "xtask",
                         "--locked", "--", "single-crate-hook", "probe"],
                        cwd=self.caller, env=self.env, stdout=subprocess.PIPE, stderr=log,
                        start_new_session=True,
                    )
                    try:
                        deadline = time.monotonic() + 8
                        while not started.exists() and process.poll() is None and time.monotonic() < deadline:
                            time.sleep(.01)
                        self.assertTrue(started.exists(), "check did not start: " + (self.root / "output-error.log").read_text())
                        process.stdout.close()
                        release.write_text("finish")
                        status = process.wait(timeout=8)
                    finally:
                        process.stdout.close()
                        release.write_text("finish")
                        if process.poll() is None:
                            os.killpg(process.pid, signal.SIGKILL)
                            process.wait()
                self.assertEqual(status, expected, (self.root / "output-error.log").read_text())
                self.assertEqual(snapshot(self.caller / ".git"), before)
                receipt = json.loads((self.caller / "target/receipts/single-crate-hook/receipt.json").read_text())
                self.assertEqual(receipt["gates"][0]["exit_code"], expected)
                self.assertEqual(receipt["gates"][0]["status"], "fail" if child_code else "error")
                self.assertEqual(receipt["summary"]["overall_status"], "fail")


if __name__ == "__main__":
    unittest.main(argv=[__file__, *REST])
