# Upgrading contracts

```bash
odcslint upgrade                   # upgrade every contract to the newest supported ODCS version
odcslint upgrade --to v3.1.0       # or to a specific version
odcslint upgrade --dry-run         # show what would change, write nothing
```

`odcslint upgrade` moves contracts to a newer ODCS version and converts the breaking changes between versions for you. It works like `check`: it takes files, directories or globs, uses the same config and `.gitignore` rules, and edits the original text, so comments and formatting survive.

## How it works

The upgrade walks through every version between the contract's `apiVersion` and the target. For each step it applies the conversions that step needs and updates `apiVersion`. Most releases only add things, so for them updating `apiVersion` is all there is to do.

Afterwards, the result is validated against the official JSON Schema of the target version. If anything still fails, the file needs manual changes: by default it is left untouched and the problems are listed with their line and column. Line numbers refer to the upgraded version of the file.

Every edit is checked before it's kept: the edited file is parsed again and must contain exactly the data the conversion intended. If it doesn't, the file is left untouched and the error is reported.

## Automatic conversions

| Version | Conversion |
|---|---|
| v3.0.2 | Athena servers: `staging_dir` is renamed to `stagingDir`. |
| v3.1.0 | A `team` list becomes `team.members`. |
| v3.1.0 | `exclusiveMinimum: true` with `minimum: X` becomes `exclusiveMinimum: X` (same for maximum); `false` is removed. |
| v3.1.0 | DuckDB `schema` numbers become strings. |
| v3.1.0 | `slaDefaultElement` is copied to every `slaProperties` entry without an `element`, then removed. |
| v3.1.0 | Quality rules with an exact metric: `rule: rowCount` becomes `metric: rowCount`, `rule: duplicateCount` becomes `metric: duplicateValues`. |
| v3.1.0 | `mustBeGreaterOrEqualTo: a` with `mustBeLessOrEqualTo: b` becomes `mustBeBetween: [a, b]` (or `mustBe: a` when they're equal). |
| v3.1.0 | A `customProperties` entry without `value` gets `value: null`. |
| v3.1.0 | Keys the v3.1.0 schema doesn't know are moved into the object's `customProperties`, on servers, teams and members, roles, support channels, schema objects, properties and quality checks. |

Some changes can't be made without guessing, so they're reported instead: a team member without a `username`, quality rules whose meaning changed (such as `validValues`), checks without an operator, and unknown keys on objects that have no `customProperties`. Flow-style YAML (`[...]`, `{...}`) in a place that needs converting is also reported.

## Options

| Option | Description |
|---|---|
| `--to <VERSION>` | Target ODCS version, e.g. `v3.1.0`. Defaults to the newest version odcslint supports. Contracts already at or past it are left alone. |
| `--dry-run` | Report what would change without writing any file. |
| `--allow-partial` | Also write files that still need manual changes. The automatic conversions and the new `apiVersion` are written, and the remaining problems are listed. |
| `-c, --config <FILE>` | Use this config file instead of discovering one. |
| `--force-exclude`, `--no-ignore`, `--allow-empty` | Same as for [`check`](usage.md). |

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Every contract was upgraded or already up to date. |
| `1` | At least one contract needs manual changes (left untouched, or written with `--allow-partial`). |
| `2` | Tool error, such as an unknown `--to` version or an invalid config. |
