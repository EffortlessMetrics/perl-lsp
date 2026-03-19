#!/usr/bin/env python3
"""Validate the machine-readable swarm agent roster contract."""

from __future__ import annotations

import json
import re
import sys
from datetime import date
from pathlib import Path

import yaml


ROOT = Path(__file__).resolve().parent.parent
AGENTS_DIR = ROOT / ".claude" / "agents"
ROSTER_PATH = AGENTS_DIR / "agent-roster.json"
COMMANDS_DIR = ROOT / ".claude" / "commands"
SKILLS_DIR = ROOT / ".claude" / "skills"

ALLOWED_ROOT_KEYS = {"schema_version", "last_updated", "agents"}
ALLOWED_AGENT_KEYS = {
    "name",
    "class",
    "category",
    "file",
    "spawned_by",
    "handoff_to",
    "first_entrypoints",
    "owns",
    "description",
}
ALLOWED_CLASSES = {"coordinator", "reusable_worker", "specialist_worker"}
ALLOWED_CATEGORIES = {
    "docs_devex",
    "explore",
    "implementation",
    "quality",
    "quality_ops",
    "review",
    "scout",
}
NAME_RE = re.compile(r"^[a-z0-9-]+$")


def fail(message: str) -> None:
    print(f"ERROR: {message}", file=sys.stderr)
    raise SystemExit(1)


def ensure_nonempty_string(value: object, field: str) -> str:
    if not isinstance(value, str) or not value.strip():
        fail(f"{field} must be a non-empty string")
    return value


def ensure_string_list(value: object, field: str) -> list[str]:
    if not isinstance(value, list) or not value:
        fail(f"{field} must be a non-empty array")
    result: list[str] = []
    for index, item in enumerate(value):
        result.append(ensure_nonempty_string(item, f"{field}[{index}]"))
    return result


def parse_date(raw: str, field: str) -> date:
    try:
        return date.fromisoformat(raw)
    except ValueError as exc:
        fail(f"{field} must be ISO date YYYY-MM-DD: {exc}")
    raise AssertionError("unreachable")


def load_frontmatter(path: Path) -> dict[str, object]:
    text = path.read_text(encoding="utf-8")
    if not text.startswith("---\n"):
        fail(f"{path} must start with YAML frontmatter")
    try:
        _empty, frontmatter, _rest = text.split("---", 2)
    except ValueError:
        fail(f"{path} must contain opening and closing frontmatter markers")
    try:
        data = yaml.safe_load(frontmatter)
    except yaml.YAMLError as exc:
        fail(f"{path} has invalid YAML frontmatter: {exc}")
    if not isinstance(data, dict):
        fail(f"{path} frontmatter must parse to a mapping")
    return data


def entrypoint_exists(entrypoint: str) -> bool:
    name = entrypoint.removeprefix("/")
    return (COMMANDS_DIR / f"{name}.md").exists() or (SKILLS_DIR / name / "SKILL.md").exists()


def main() -> None:
    data = json.loads(ROSTER_PATH.read_text(encoding="utf-8"))

    extra_root = set(data) - ALLOWED_ROOT_KEYS
    if extra_root:
        fail(f"unexpected root keys: {sorted(extra_root)}")
    if data.get("schema_version") != 1:
        fail("schema_version must be 1")
    parse_date(ensure_nonempty_string(data.get("last_updated"), "last_updated"), "last_updated")

    agents = data.get("agents")
    if not isinstance(agents, list) or not agents:
        fail("agents must be a non-empty array")

    agent_files = {
        path.name
        for path in AGENTS_DIR.glob("*.md")
        if path.name not in {"README.md", "AGENT_CATALOG.md"}
    }
    if not agent_files:
        fail(f"no agent definition files found in {AGENTS_DIR}")

    seen_names: set[str] = set()
    seen_files: set[str] = set()
    roster_files: set[str] = set()

    for index, agent in enumerate(agents):
        if not isinstance(agent, dict):
            fail(f"agent #{index + 1} must be an object")
        extra_agent = set(agent) - ALLOWED_AGENT_KEYS
        if extra_agent:
            fail(f"agent #{index + 1} has unexpected keys: {sorted(extra_agent)}")

        name = ensure_nonempty_string(agent.get("name"), f"agent #{index + 1}.name")
        if not NAME_RE.fullmatch(name):
            fail(f"{name} must match [a-z0-9-]+")
        if name in seen_names:
            fail(f"duplicate agent name: {name}")
        seen_names.add(name)

        agent_class = ensure_nonempty_string(agent.get("class"), f"{name}.class")
        if agent_class not in ALLOWED_CLASSES:
            fail(f"{name}.class must be one of {sorted(ALLOWED_CLASSES)}")

        category = agent.get("category")
        if agent_class == "specialist_worker":
            category_value = ensure_nonempty_string(category, f"{name}.category")
            if category_value not in ALLOWED_CATEGORIES:
                fail(f"{name}.category must be one of {sorted(ALLOWED_CATEGORIES)}")
        elif category is not None:
            fail(f"{name}.category is only allowed for specialist_worker entries")

        agent_file = ensure_nonempty_string(agent.get("file"), f"{name}.file")
        if agent_file in seen_files:
            fail(f"duplicate agent file: {agent_file}")
        seen_files.add(agent_file)
        roster_files.add(agent_file)

        agent_path = AGENTS_DIR / agent_file
        if not agent_path.exists():
            fail(f"{name}.file does not exist: {agent_file}")

        ensure_string_list(agent.get("spawned_by"), f"{name}.spawned_by")
        ensure_string_list(agent.get("handoff_to"), f"{name}.handoff_to")
        first_entrypoints = ensure_string_list(agent.get("first_entrypoints"), f"{name}.first_entrypoints")
        description = ensure_nonempty_string(agent.get("description"), f"{name}.description")

        if agent_class == "coordinator":
            ensure_nonempty_string(agent.get("owns"), f"{name}.owns")
        elif agent.get("owns") is not None:
            fail(f"{name}.owns is only allowed for coordinator entries")

        frontmatter = load_frontmatter(agent_path)
        frontmatter_name = ensure_nonempty_string(frontmatter.get("name"), f"{agent_file} frontmatter.name")
        if frontmatter_name != name:
            fail(f"{agent_file} frontmatter.name ({frontmatter_name}) does not match roster name ({name})")

        frontmatter_description = ensure_nonempty_string(
            frontmatter.get("description"),
            f"{agent_file} frontmatter.description",
        )
        if frontmatter_description != description:
            fail(f"{agent_file} description does not match agent-roster.json")

        skills = frontmatter.get("skills")
        if skills is not None:
            for skill in ensure_string_list(skills, f"{agent_file} frontmatter.skills"):
                skill_path = SKILLS_DIR / skill / "SKILL.md"
                if not skill_path.exists():
                    fail(f"{agent_file} references missing skill: {skill}")

        for entrypoint in first_entrypoints:
            if not entrypoint.startswith("/"):
                fail(f"{name}.first_entrypoints entries must start with '/': {entrypoint}")
            if not entrypoint_exists(entrypoint):
                fail(f"{name}.first_entrypoints references missing command/skill: {entrypoint}")

    if roster_files != agent_files:
        fail(
            "agent-roster.json file set does not match .claude/agents surface: "
            f"roster={sorted(roster_files)} agents={sorted(agent_files)}"
        )

    print(f"Validated {len(agents)} agents in {ROSTER_PATH}")


if __name__ == "__main__":
    main()
