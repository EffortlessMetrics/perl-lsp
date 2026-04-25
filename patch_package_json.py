import json

with open('vscode-extension/package.json', 'r') as f:
    data = json.load(f)

commands = [
  'perl-lsp.restart',
  'perl-lsp.reinstall'
]

for cmd in commands:
    ev = f"onCommand:{cmd}"
    if ev not in data.get("activationEvents", []):
        data.setdefault("activationEvents", []).append(ev)

with open('vscode-extension/package.json', 'w') as f:
    json.dump(data, f, indent=2)
