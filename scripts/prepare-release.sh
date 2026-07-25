#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE' >&2
Usage: scripts/prepare-release.sh <version>

Prepares a Gavin release by updating:
  - Cargo.toml and Cargo.lock
  - package.json and package-lock.json
  - charts/gavin/Chart.yaml
  - CHANGELOG.md

Pass versions without the leading "v" ("v1.2.3" is accepted and normalized).
Set ALLOW_DIRTY=1 to run with existing uncommitted changes.
USAGE
}

raw_version="${1:-}"
if [[ -z "$raw_version" ]]; then
  usage
  exit 2
fi

VERSION="${raw_version#v}"
TAG="v${VERSION}"
RELEASE_DATE="${RELEASE_DATE:-$(date -u +%F)}"

if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
  echo "Invalid semantic version: $raw_version" >&2
  echo "Expected something like 1.2.3 or 1.2.3-rc.1" >&2
  exit 2
fi

for cmd in cargo npm python3; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "Missing required command: $cmd" >&2
    exit 1
  fi
done

if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  if [[ "${ALLOW_DIRTY:-0}" != "1" ]]; then
    if ! git diff --quiet -- . || ! git diff --cached --quiet -- .; then
      echo "Working tree has uncommitted changes. Commit/stash them or set ALLOW_DIRTY=1." >&2
      exit 1
    fi
  fi
fi

if grep -Eq "^## \[${VERSION//./\.}\]" CHANGELOG.md; then
  echo "CHANGELOG.md already contains a section for $VERSION" >&2
  exit 1
fi

if git rev-parse --git-dir >/dev/null 2>&1 && git tag --list "$TAG" | grep -qx "$TAG"; then
  echo "Local git tag already exists: $TAG" >&2
  exit 1
fi

echo "Updating npm package files to $VERSION..."
npm version --no-git-tag-version "$VERSION"

echo "Updating Rust, Helm, and changelog files..."
python3 - "$VERSION" "$RELEASE_DATE" <<'PY'
import re
import sys
from pathlib import Path

version = sys.argv[1]
release_date = sys.argv[2]
repo_url = "https://github.com/restanrm/gavin"


def write_if_changed(path: str, old: str, new: str) -> None:
    if old != new:
        Path(path).write_text(new)


# Cargo.toml package version
path = Path("Cargo.toml")
text = path.read_text()
new_text, count = re.subn(
    r'(?ms)^(\[package\]\n(?:[^\[]*?\n)*?version = ")[^"]+(")',
    rf'\g<1>{version}\2',
    text,
    count=1,
)
if count != 1:
    raise SystemExit("Could not update Cargo.toml package version")
write_if_changed(str(path), text, new_text)

# Helm chart version and appVersion
path = Path("charts/gavin/Chart.yaml")
text = path.read_text()
new_text, version_count = re.subn(r"^version: .*$", f"version: {version}", text, count=1, flags=re.M)
new_text, app_count = re.subn(r"^appVersion: .*$", f'appVersion: "v{version}"', new_text, count=1, flags=re.M)
if version_count != 1 or app_count != 1:
    raise SystemExit("Could not update charts/gavin/Chart.yaml version/appVersion")
write_if_changed(str(path), text, new_text)

# CHANGELOG.md: promote Unreleased content into the new version section and update links.
path = Path("CHANGELOG.md")
text = path.read_text()
if re.search(rf"^## \[{re.escape(version)}\](?:\s|$)", text, flags=re.M):
    raise SystemExit(f"CHANGELOG.md already contains {version}")

unreleased_heading = "## [Unreleased]"
start = text.find(unreleased_heading)
if start == -1:
    raise SystemExit("Could not find CHANGELOG.md Unreleased section")
body_start = text.find("\n", start)
if body_start == -1:
    raise SystemExit("Malformed CHANGELOG.md Unreleased section")
body_start += 1
next_heading = re.search(r"\n## \[[^\]]+\]", text[body_start:])
if not next_heading:
    raise SystemExit("Could not find next CHANGELOG.md version section")
next_index = body_start + next_heading.start()

unreleased_body = text[body_start:next_index].strip()
if not unreleased_body:
    raise SystemExit("CHANGELOG.md Unreleased section is empty; add release notes first")

previous_match = re.match(r"\n## \[([^\]]+)\]", text[next_index:])
previous_version = previous_match.group(1) if previous_match else ""
if previous_version.lower() == "unreleased":
    previous_version = ""

new_release_section = f"\n\n## [{version}] - {release_date}\n\n{unreleased_body}\n"
new_text = text[:start + len(unreleased_heading)] + new_release_section + text[next_index:]

unreleased_link = f"[Unreleased]: {repo_url}/compare/v{version}...HEAD"
if previous_version:
    version_link = f"[{version}]: {repo_url}/compare/v{previous_version}...v{version}"
else:
    version_link = f"[{version}]: {repo_url}/releases/tag/v{version}"

new_text, link_count = re.subn(
    r"^\[Unreleased\]: .*$",
    f"{unreleased_link}\n{version_link}",
    new_text,
    count=1,
    flags=re.M,
)
if link_count != 1:
    raise SystemExit("Could not update CHANGELOG.md release links")

write_if_changed(str(path), text, new_text)
PY

echo "Updating Cargo.lock..."
cargo check

cat <<EOF

Prepared Gavin release $TAG.

Review the changes, then run the validation/release flow or commit and tag manually:
  git diff
  git add Cargo.toml Cargo.lock package.json package-lock.json charts/gavin/Chart.yaml CHANGELOG.md
  git commit -m "chore: release $TAG"
  git tag -a "$TAG" -m "Release $TAG"
EOF
