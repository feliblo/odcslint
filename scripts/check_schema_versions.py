"""Check that every upstream ODCS schema version (v3 and later) is vendored and embedded.

Usage: python3 scripts/check_schema_versions.py [git-ref]

Exits with 1 when upstream has a version that `schemas/odcs/` lacks, or when a vendored schema is
not referenced by `src/schema/embedded.rs`. A newer dated snapshot of a vendored version is only a
warning, because the vendored snapshot is still a valid schema for that version.
"""

import json
import os
import re
import sys
import urllib.request

UPSTREAM_API = "https://api.github.com/repos/bitol-io/open-data-contract-standard/contents/schema?ref={ref}"
DEFAULT_REF = "main"
VENDORED_DIR = "schemas/odcs"
EMBEDDED_RS = "src/schema/embedded.rs"
# v2 contracts are intentionally unsupported.
MIN_MAJOR = 3
SCHEMA_FILE = re.compile(r"^odcs-json-schema-v(?P<version>\d+\.\d+\.\d+)(?:-(?P<date>\d{8}))?\.json$")
IN_GITHUB_ACTIONS = os.environ.get("GITHUB_ACTIONS") == "true"


def report(level, message, file=None):
    if IN_GITHUB_ACTIONS:
        location = f" file={file}" if file else ""
        print(f"::{level}{location}::{message}")
    else:
        print(f"{level}: {message}")


def versions(names):
    """Map each version to its newest snapshot date ("" for the undated rolling file)."""
    found = {}
    for name in names:
        match = SCHEMA_FILE.match(name)
        if not match or int(match["version"].split(".")[0]) < MIN_MAJOR:
            continue
        found.setdefault(match["version"], set()).add(match["date"] or "")
    return found


def upstream_names(ref):
    request = urllib.request.Request(UPSTREAM_API.format(ref=ref), headers={"Accept": "application/vnd.github+json"})
    token = os.environ.get("GITHUB_TOKEN")
    if token:
        request.add_header("Authorization", f"Bearer {token}")
    with urllib.request.urlopen(request, timeout=30) as response:
        return [entry["name"] for entry in json.load(response)]


def main():
    ref = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_REF
    vendored_names = sorted(os.listdir(VENDORED_DIR))
    vendored = versions(vendored_names)
    upstream = versions(upstream_names(ref))
    embedded = open(EMBEDDED_RS).read()
    errors = 0

    for version in sorted(set(upstream) - set(vendored)):
        report("error", f"upstream ({ref}) has ODCS v{version}, but {VENDORED_DIR}/ has no schema for it")
        errors += 1

    for version in sorted(set(upstream) & set(vendored)):
        newest_upstream = max(upstream[version])
        newest_vendored = max(vendored[version])
        if newest_vendored and newest_upstream > newest_vendored:
            report(
                "warning",
                f"upstream has a newer v{version} snapshot ({newest_upstream}) than the vendored one ({newest_vendored})",
                file=f"{VENDORED_DIR}/odcs-json-schema-v{version}-{newest_vendored}.json",
            )

    for name in vendored_names:
        if SCHEMA_FILE.match(name) and name not in embedded:
            report("error", f"{VENDORED_DIR}/{name} is vendored but not embedded in {EMBEDDED_RS}", file=EMBEDDED_RS)
            errors += 1

    if errors == 0:
        print(f"All upstream ODCS v{MIN_MAJOR}+ schema versions on {ref} are vendored and embedded.")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
