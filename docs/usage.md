# Usage

## `odcslint check`

```bash
odcslint check [PATHS]... [OPTIONS]
```

`PATHS` can be any mix of files, directories and glob patterns. Without arguments, odcslint checks the current directory.

| Input | Behaviour |
|---|---|
| `contracts/` | Walked recursively. Files are checked if they match `settings.include` (by default `**/*.odcs.yaml` and `**/*.odcs.yml`) and don't match `settings.exclude`. `.gitignore` is respected. |
| `contracts/orders.odcs.yaml` | Always checked, even if it doesn't match `include`. The one exception: with `--force-exclude`, a file that matches `exclude` is skipped. |
| `"domains/**/*.yaml"` | Walked from the part before the first wildcard (`domains/`), checking files that match the pattern. Quote globs so your shell doesn't expand them. |

```bash
odcslint check                                   # everything below the current directory
odcslint check contracts/ other/orders.odcs.yaml # several inputs
odcslint check "domains/*/contracts/**/*.yaml"   # a glob
```

### Options

| Option | Description |
|---|---|
| `--fix` | Apply fixes in place. See [Autofix](autofix.md). |
| `-c, --config <FILE>` | Use this config file instead of discovering one. |
| `--output-format <FORMAT>` | `text` (default), `json`, `ndjson`, `csv` or `github`. |
| `--output-file <FILE>` | Write the output to a file instead of stdout. |
| `-q, --quiet` | Only print findings, without the summary line. |
| `--hide-warnings` | Do not report warnings. |
| `--exit-zero` | Exit with `0` even when errors are found. |
| `--force-exclude` | Also apply `settings.exclude` to files passed explicitly. Use this for prek hooks. |
| `--no-ignore` | Do not respect `.gitignore`, `.ignore` and hidden-file rules. |
| `--allow-empty` | Succeed when no contract files are found. |
| `-v, --verbose` | Print debug information: which config is used, and timings. |

### Exit codes

| Code | Meaning |
|---|---|
| `0` | No error-severity findings. Warnings are allowed. |
| `1` | At least one error-severity finding. |
| `2` | Tool error: invalid config, unreadable file, path not found, or no contracts found. |

### Output formats

`text`
:   Findings grouped by file, one aligned line each (`line:col  severity  rule  message`), followed by a summary. A trailing `[*]` marks findings that `--fix` can resolve. Output is colored when stdout is a terminal and `NO_COLOR` isn't set.

`json`
:   A single document with a `summary` object and a `findings` array.

`ndjson`
:   One JSON object per finding, for streaming into other tools.

`csv`
:   A header row, then one row per finding.

`github`
:   [GitHub Actions workflow commands](https://docs.github.com/en/actions/reference/workflow-commands-for-github-actions). Findings show up as annotations on pull requests.

In the structured formats, each finding has the fields `path`, `line`, `column`, `severity`, `rule`, `name`, `message` and `fixable`. `rule` is the rule type, or `schema`, `yaml_syntax` or `unsupported_api_version` for the built-in checks. `name` is the rule's configured name.

## `odcslint upgrade`

Moves contracts to a newer ODCS version and converts the breaking changes between versions. See [Upgrading contracts](upgrade.md).

```bash
odcslint upgrade [PATHS]... [--to <VERSION>] [--dry-run] [--allow-partial]
```

## `odcslint init`

Writes a commented starter config. By default it goes in the current directory.

```bash
odcslint init                 # odcslint.yml
odcslint init --format toml   # odcslint.toml
odcslint init --location configs/ --force
```

## `odcslint rules`

Prints every rule type with a short description, marking the ones `--fix` can fix with `[*]`, followed by the [aliases](rules/allowed_values.md#fields) of `allowed_values` and the field each one checks.
