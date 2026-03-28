#!/usr/bin/env python3
"""Manage reusable git worktree slots for swarm sessions."""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_STATE_FILE = REPO_ROOT / ".ops-perl-lsp" / "worktree-manager" / "state.json"
DEFAULT_MANAGED_ROOT = REPO_ROOT.parent / f"{REPO_ROOT.name}-worktrees"
DEFAULT_BRANCH_BASES = ("origin/master", "master", "origin/main", "main")
OWNER_ENV_VARS = (
    "WORKTREE_MANAGER_OWNER",
    "CLAUDE_SUBAGENT_NAME",
    "CLAUDE_AGENT_NAME",
    "CLAUDE_TEAMMATE_NAME",
    "AGENT_NAME",
    "SUBAGENT_NAME",
)


def utc_now() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def slugify(value: str) -> str:
    pieces: list[str] = []
    dash = False
    for char in value.lower():
        if char.isalnum():
            pieces.append(char)
            dash = False
        elif not dash:
            pieces.append("-")
            dash = True
    slug = "".join(pieces).strip("-")
    return slug or "slot"


def render_state_path(path: Path) -> str:
    try:
        return str(path.relative_to(REPO_ROOT))
    except ValueError:
        return str(path)


def run(cmd: list[str], *, cwd: Path | None = None, check: bool = True) -> subprocess.CompletedProcess[str]:
    proc = subprocess.run(
        cmd,
        cwd=str(cwd) if cwd is not None else None,
        text=True,
        capture_output=True,
        check=False,
    )
    if check and proc.returncode != 0:
        raise RuntimeError(
            f"command failed ({proc.returncode}): {' '.join(cmd)}\nstdout:\n{proc.stdout}\nstderr:\n{proc.stderr}"
        )
    return proc


def git(args: list[str], *, cwd: Path | None = None, check: bool = True) -> subprocess.CompletedProcess[str]:
    return run(["git", *args], cwd=cwd, check=check)


def gh_available() -> bool:
    return shutil.which("gh") is not None


def load_json(path: Path) -> dict[str, Any]:
    if not path.exists():
        return {}
    return json.loads(path.read_text(encoding="utf-8"))


def save_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def default_state() -> dict[str, Any]:
    return {
        "version": 1,
        "managed_root": render_state_path(DEFAULT_MANAGED_ROOT),
        "updated_at": None,
        "slots": [],
    }


def normalize_owner(value: str | None) -> str | None:
    if value is None:
        return None
    owner = value.strip()
    return owner or None


def owner_from_env() -> str | None:
    for key in OWNER_ENV_VARS:
        owner = normalize_owner(os.environ.get(key))
        if owner:
            return owner
    return None


def resolve_owner(explicit: str | None) -> str | None:
    owner = normalize_owner(explicit)
    if owner:
        return owner
    return owner_from_env()


def set_owner(slot: dict[str, Any], owner: str | None, owner_source: str | None) -> None:
    slot["owner"] = owner
    slot["owner_set_at"] = utc_now() if owner else None
    if owner and owner_source:
        slot["owner_source"] = owner_source
    else:
        slot.pop("owner_source", None)


def state_path_from_args(args: argparse.Namespace) -> Path:
    if args.state_file:
        return Path(args.state_file)
    override = os.environ.get("WORKTREE_MANAGER_STATE_FILE")
    if override:
        return Path(override)
    return DEFAULT_STATE_FILE


def managed_root_from_args(args: argparse.Namespace) -> Path:
    if args.managed_root:
        root = Path(args.managed_root)
        return root if root.is_absolute() else REPO_ROOT / root
    return DEFAULT_MANAGED_ROOT


def normalize_branch(branch: str) -> str:
    if branch.startswith("refs/heads/"):
        return branch.removeprefix("refs/heads/")
    return branch


def parse_worktree_list() -> list[dict[str, str]]:
    proc = git(["worktree", "list", "--porcelain"])
    entries: list[dict[str, str]] = []
    current: dict[str, str] = {}
    for line in proc.stdout.splitlines():
        if not line.strip():
            if current:
                entries.append(current)
                current = {}
            continue
        key, _, value = line.partition(" ")
        current[key] = value
    if current:
        entries.append(current)
    return entries


def base_ref() -> str:
    for candidate in DEFAULT_BRANCH_BASES:
        if git(["rev-parse", "--verify", f"{candidate}^{{commit}}"], check=False).returncode == 0:
            return candidate
    return "HEAD"


def branch_exists_elsewhere(branch: str, entries: list[dict[str, str]], slot_id: str | None = None) -> str | None:
    wanted = normalize_branch(branch)
    for entry in entries:
        current_branch = normalize_branch(entry.get("branch", ""))
        if current_branch == wanted:
            path = entry.get("worktree", "")
            if not slot_id or Path(path).name != slot_id:
                return path
    return None


def worktree_dirty(path: Path) -> bool:
    if not path.exists():
        return False
    return bool(git(["-C", str(path), "status", "--porcelain"], check=False).stdout.strip())


def branch_merged(branch: str) -> bool:
    wanted = normalize_branch(branch)
    if not wanted or wanted == "HEAD":
        return False
    base = base_ref()
    if base == "HEAD":
        return False
    return git(["merge-base", "--is-ancestor", wanted, base], check=False).returncode == 0


def open_pr_number(branch: str) -> str | None:
    if not gh_available():
        return None
    wanted = normalize_branch(branch)
    if not wanted or wanted == "HEAD":
        return None
    proc = run(
        ["gh", "pr", "list", "--head", wanted, "--state", "open", "--json", "number", "--jq", ".[0].number"],
        check=False,
    )
    number = proc.stdout.strip()
    return number or None


def load_state(path: Path) -> dict[str, Any]:
    state = default_state()
    state.update(load_json(path))
    state.setdefault("slots", [])
    return state


def save_state(path: Path, state: dict[str, Any]) -> None:
    state["updated_at"] = utc_now()
    save_json(path, state)


def slot_lookup(state: dict[str, Any]) -> dict[str, dict[str, Any]]:
    slots: dict[str, dict[str, Any]] = {}
    for slot in state.get("slots", []):
        slot_id = slot.get("slot_id")
        if slot_id:
            slots[slot_id] = slot
    return slots


def sync_state(state: dict[str, Any], managed_root: Path) -> None:
    entries = parse_worktree_list()
    slots = slot_lookup(state)
    known_paths: set[Path] = set()

    for entry in entries:
        path = Path(entry["worktree"])
        if managed_root not in path.parents and path != managed_root:
            continue

        slot_id = path.name
        branch = normalize_branch(entry.get("branch", "HEAD"))
        slot = slots.get(slot_id)
        if slot is None:
            slot = {
                "slot_id": slot_id,
                "path": render_state_path(path),
                "branch": branch,
                "owner": None,
                "owner_set_at": None,
                "status": "active" if branch != "HEAD" else "detached",
                "reuse_count": 0,
                "last_used_at": utc_now(),
                "last_released_at": None,
                "notes": [],
            }
            state["slots"].append(slot)
            slots[slot_id] = slot
        else:
            slot["path"] = render_state_path(path)
            slot["branch"] = branch
            slot.setdefault("owner", None)
            slot.setdefault("owner_set_at", None)
            if path.exists():
                if worktree_dirty(path):
                    slot["status"] = "dirty"
                elif slot.get("status") not in {"idle", "retired"}:
                    slot["status"] = "active" if branch != "HEAD" else "detached"
            else:
                slot["status"] = "missing"
            slot["last_seen_at"] = utc_now()
        known_paths.add(path)

    for slot in state.get("slots", []):
        slot_path = slot.get("path")
        if not slot_path:
            continue
        abs_path = REPO_ROOT / slot_path
        if abs_path not in known_paths and not abs_path.exists() and slot.get("status") not in {"retired", "missing"}:
            slot["status"] = "missing"


def printable_status(slot: dict[str, Any]) -> str:
    status = slot.get("status", "unknown")
    branch = slot.get("branch", "HEAD")
    if status == "idle":
        return "idle/reusable"
    if status == "active" and branch_merged(branch):
        return "active/merged"
    if status == "dirty":
        return "dirty"
    if status == "missing":
        return "missing"
    if status == "detached":
        return "detached"
    return status


def print_query(state: dict[str, Any]) -> None:
    slots = state.get("slots", [])
    if not slots:
        print("No managed worktree slots recorded.")
        return

    print(f"{'SLOT':20} {'STATUS':15} {'OWNER':20} {'BRANCH':28} {'REUSE':5} PATH")
    print(f"{'-' * 20} {'-' * 15} {'-' * 20} {'-' * 28} {'-' * 5} {'-' * 30}")
    for slot in sorted(slots, key=lambda item: item.get("slot_id", "")):
        owner = normalize_owner(slot.get("owner")) or "(unowned)"
        print(
            f"{slot.get('slot_id', '')[:20]:20} "
            f"{printable_status(slot)[:15]:15} "
            f"{owner[:20]:20} "
            f"{normalize_branch(slot.get('branch', 'HEAD'))[:28]:28} "
            f"{str(slot.get('reuse_count', 0))[:5]:5} "
            f"{slot.get('path', '')}"
        )


def json_query(state: dict[str, Any]) -> None:
    print(json.dumps(state, indent=2, sort_keys=True))


def ensure_branch_not_in_use(branch: str, entries: list[dict[str, str]], slot_id: str) -> None:
    conflict = branch_exists_elsewhere(branch, entries, slot_id=slot_id)
    if conflict:
        raise RuntimeError(f"branch {branch!r} is already checked out at {conflict}")


def allocate(args: argparse.Namespace, state: dict[str, Any], state_path: Path, managed_root: Path) -> None:
    entries = parse_worktree_list()
    ensure_branch_not_in_use(args.branch, entries, args.slot)

    slots = slot_lookup(state)
    slot = slots.get(args.slot)
    slot_path = managed_root / args.slot
    slot_rel = render_state_path(slot_path)
    owner = resolve_owner(args.owner)
    owner_source = "flag" if normalize_owner(args.owner) else "env" if owner else None

    if slot is None:
        slot = {
            "slot_id": args.slot,
            "path": slot_rel,
            "branch": normalize_branch(args.branch),
            "owner": owner,
            "owner_set_at": utc_now() if owner else None,
            "status": "idle",
            "reuse_count": 0,
            "last_used_at": None,
            "last_released_at": None,
            "notes": [],
        }
        state["slots"].append(slot)
        slots[args.slot] = slot

    if slot.get("status") not in {"idle", "missing", "retired"} and not args.force:
        raise RuntimeError(f"slot {args.slot!r} is currently {slot.get('status')!r}; use --force to reallocate")

    base = args.ref or base_ref()
    action = "reuse" if slot_path.exists() else "create"

    if args.dry_run:
        print(f"would {action} slot={args.slot} path={slot_rel} branch={args.branch} ref={base}")
        return

    slot_path.parent.mkdir(parents=True, exist_ok=True)
    if slot_path.exists():
        if worktree_dirty(slot_path) and not args.force:
            raise RuntimeError(f"slot {args.slot!r} is dirty; clean it before allocate or use --force")
        git(["-C", str(slot_path), "checkout", "-B", args.branch, base])
    else:
        git(["worktree", "add", "-B", args.branch, str(slot_path), base])

    slot["branch"] = normalize_branch(args.branch)
    slot["path"] = slot_rel
    slot["status"] = "active"
    slot["reuse_count"] = int(slot.get("reuse_count", 0)) + 1
    slot["last_used_at"] = utc_now()
    set_owner(slot, owner, owner_source)
    save_state(state_path, state)
    print(f"allocated slot={args.slot} path={slot_rel} branch={args.branch} ref={base}")


def release(args: argparse.Namespace, state: dict[str, Any], state_path: Path) -> None:
    slots = slot_lookup(state)
    slot = slots.get(args.slot)
    if slot is None:
        raise RuntimeError(f"unknown slot {args.slot!r}")

    slot_path = REPO_ROOT / slot.get("path", "")
    if slot_path.exists() and worktree_dirty(slot_path) and not args.force:
        raise RuntimeError(f"slot {args.slot!r} is dirty; clean it before release or use --force")

    if args.dry_run:
        print(f"would release slot={args.slot} path={slot.get('path', '')}")
        return

    owner = resolve_owner(args.owner)
    recorded_owner = normalize_owner(slot.get("owner"))
    if owner and recorded_owner and owner != recorded_owner and not args.force:
        raise RuntimeError(
            f"slot {args.slot!r} is owned by {recorded_owner!r}; release as that owner or use --force"
        )
    slot["status"] = "retired" if args.retire else "idle"
    slot["last_released_at"] = utc_now()
    set_owner(slot, None, None)
    if args.note:
        slot.setdefault("notes", []).append(args.note)
    save_state(state_path, state)
    print(f"released slot={args.slot} status={slot['status']}")


def stale_slot(slot: dict[str, Any], stale_days: int) -> bool:
    if slot.get("status") not in {"idle", "active", "dirty"}:
        return False
    last_released = slot.get("last_released_at")
    if not last_released:
        return False
    try:
        released = datetime.fromisoformat(last_released.replace("Z", "+00:00"))
    except ValueError:
        return False
    age = datetime.now(timezone.utc) - released.astimezone(timezone.utc)
    if age.days < stale_days:
        return False
    branch = normalize_branch(slot.get("branch", "HEAD"))
    return branch == "HEAD" or branch_merged(branch)


def cleanup(args: argparse.Namespace, state: dict[str, Any], state_path: Path) -> None:
    slots = state.get("slots", [])
    kept: list[dict[str, Any]] = []
    removed = 0
    pruned = 0

    if args.run_low_level:
        script = REPO_ROOT / "scripts" / "cleanup-completed-worktrees.sh"
        cmd = ["bash", str(script)]
        if args.dry_run:
            cmd.append("--dry-run")
        proc = run(cmd, check=False)
        sys.stdout.write(proc.stdout)
        sys.stderr.write(proc.stderr)
        if proc.returncode != 0:
            raise RuntimeError("low-level cleanup script failed")

    for slot in slots:
        slot_path = REPO_ROOT / slot.get("path", "")
        if not slot.get("path") or not slot_path.exists():
            removed += 1
            continue

        branch = normalize_branch(slot.get("branch", "HEAD"))
        should_prune = slot.get("status") == "retired" or stale_slot(slot, args.stale_days)
        if not should_prune and slot.get("status") == "idle" and branch != "HEAD":
            should_prune = branch_merged(branch) and open_pr_number(branch) is None

        if should_prune:
            if args.dry_run:
                print(f"would prune slot={slot.get('slot_id')} path={slot.get('path')} branch={branch}")
            else:
                git(["worktree", "remove", "--force", str(slot_path)], check=False)
                if branch != "HEAD":
                    git(["branch", "-D", branch], check=False)
            pruned += 1
            continue

        kept.append(slot)

    state["slots"] = kept
    if not args.dry_run:
        save_state(state_path, state)

    print(f"cleanup complete: kept={len(kept)} removed_state={removed} pruned={pruned}")


def build_parser() -> argparse.ArgumentParser:
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--state-file", help="Override the runtime state file path")
    common.add_argument("--managed-root", help="Override the managed worktree root")

    parser = argparse.ArgumentParser(description="Manage reusable worktree slots.", parents=[common])

    sub = parser.add_subparsers(dest="command", required=True)

    query = sub.add_parser("query", help="Show the current pool state", parents=[common])
    query.add_argument("--json", action="store_true", help="Emit JSON instead of a table")

    allocate_cmd = sub.add_parser("allocate", help="Allocate or reuse a worktree slot", parents=[common])
    allocate_cmd.add_argument("--slot", required=True, help="Named slot to allocate")
    allocate_cmd.add_argument("--branch", required=True, help="Branch to check out")
    allocate_cmd.add_argument("--ref", help="Base ref to check out from")
    allocate_cmd.add_argument(
        "--owner",
        help="Owner label for the slot (defaults to WORKTREE_MANAGER_OWNER / CLAUDE_* env)",
    )
    allocate_cmd.add_argument("--dry-run", action="store_true", help="Show the action without mutating anything")
    allocate_cmd.add_argument("--force", action="store_true", help="Reallocate even if the slot is active")

    release_cmd = sub.add_parser("release", help="Mark a slot reusable", parents=[common])
    release_cmd.add_argument("--slot", required=True, help="Named slot to release")
    release_cmd.add_argument(
        "--owner",
        help="Owner label to record on release (defaults to WORKTREE_MANAGER_OWNER / CLAUDE_* env)",
    )
    release_cmd.add_argument("--note", help="Optional note to append to the slot history")
    release_cmd.add_argument("--retire", action="store_true", help="Retire the slot instead of leaving it idle")
    release_cmd.add_argument("--dry-run", action="store_true", help="Show the action without mutating anything")
    release_cmd.add_argument("--force", action="store_true", help="Release even if the worktree is dirty")

    cleanup_cmd = sub.add_parser("cleanup", help="Prune stale managed slots", parents=[common])
    cleanup_cmd.add_argument("--dry-run", action="store_true", help="Show the action without mutating anything")
    cleanup_cmd.add_argument("--stale-days", type=int, default=7, help="Idle age threshold before pruning")
    cleanup_cmd.add_argument(
        "--run-low-level",
        action="store_true",
        help="Also run scripts/cleanup-completed-worktrees.sh before state reconciliation",
    )

    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    state_path = state_path_from_args(args)
    managed_root = managed_root_from_args(args)
    state = load_state(state_path)
    state["managed_root"] = render_state_path(managed_root)
    sync_state(state, managed_root)

    if args.command == "query":
        if args.json:
            json_query(state)
        else:
            print_query(state)
        save_state(state_path, state)
        return 0
    if args.command == "allocate":
        allocate(args, state, state_path, managed_root)
        return 0
    if args.command == "release":
        release(args, state, state_path)
        return 0
    if args.command == "cleanup":
        cleanup(args, state, state_path)
        return 0

    parser.error(f"unknown command: {args.command}")
    return 2


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except RuntimeError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        raise SystemExit(1)
