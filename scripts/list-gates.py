#!/usr/bin/env python3
import tomllib
from pathlib import Path

def main():
    gate_toml = Path(".ci/GATE_REGISTRY.toml")
    if not gate_toml.exists():
        print(f"❌ Gate registry not found: {gate_toml}")
        return

    with open(gate_toml, "rb") as f:
        data = tomllib.load(f)

    print("📋 Registered Gates")
    print("===================")
    for gate in data.get("gate", []):
        blocking = "🔴 BLOCKING" if gate.get("blocking", False) else "🟢 OPTIONAL"
        print(f"{blocking} {gate['id']:20s} - {gate['name']}")

if __name__ == "__main__":
    main()
