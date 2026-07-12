#!/usr/bin/env python3
"""One-shot guarded fixes for PR #9966 review findings."""

from pathlib import Path


def replace_once(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one exact match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


validator = Path("scripts/check_release_channel_actuals.py")
replace_once(
    validator,
    '        if raw == "channels:":\n',
    '        if raw.rstrip() == "channels:":\n',
    "channels header whitespace",
)
replace_once(
    validator,
    '''        if (
            not isinstance(release_url, str)
            or release_url
            != f"https://github.com/EffortlessMetrics/perl-lsp/releases/tag/{tag}"
        ):
''',
    '''        repository = data.get("repository")
        if (
            not isinstance(release_url, str)
            or not isinstance(repository, str)
            or not isinstance(tag, str)
            or release_url != f"https://github.com/{repository}/releases/tag/{tag}"
        ):
''',
    "manifest-derived release URL",
)

tests = Path("scripts/tests/test_release_channel_actuals.py")
replace_once(
    tests,
    '''from check_release_channel_actuals import (  # noqa: E402
    load_manifest,
''',
    '''from check_release_channel_actuals import (  # noqa: E402
    ChannelActualsError,
    load_manifest,
''',
    "specific exception import",
)
replace_once(
    tests,
    '''            with self.assertRaises(Exception):
                load_manifest(path)
''',
    '''            with self.assertRaises(ChannelActualsError):
                load_manifest(path)
''',
    "specific exception assertion",
)
