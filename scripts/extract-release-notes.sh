#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

raw_version="${1:-}"
if [[ -z "$raw_version" ]]; then
  echo "Usage: scripts/extract-release-notes.sh <version>" >&2
  exit 2
fi

VERSION="${raw_version#v}"

python3 - "$VERSION" <<'PY'
import re
import sys
from pathlib import Path

version = sys.argv[1]
text = Path("CHANGELOG.md").read_text()
match = re.search(rf"^## \[{re.escape(version)}\][^\n]*\n(?P<body>.*?)(?=\n## \[|\Z)", text, flags=re.M | re.S)
if not match:
    raise SystemExit(f"Could not find CHANGELOG.md notes for {version}")
notes = match.group("body").strip()
if not notes:
    raise SystemExit(f"CHANGELOG.md notes for {version} are empty")
print(notes)
PY
