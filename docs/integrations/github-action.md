# GitHub Action

The action installs odcslint and runs `odcslint check`. It uses the `github` output format by default, so findings show up as annotations on the pull request.

```yaml title=".github/workflows/contracts.yml"
name: Data contracts
on: [pull_request]

jobs:
  odcslint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - uses: feliblo/odcslint@v0.1.0
        with:
          paths: contracts/
```

## Inputs

| Input | Default | Description |
|---|---|---|
| `paths` | `.` | Space-separated files, directories or globs to check. |
| `config-file` | | Config file. Discovered in the working directory when empty. |
| `output-format` | `github` | `text`, `json`, `ndjson`, `csv` or `github`. |
| `output-file` | | Write output to a file, for example to upload it as an artifact. |
| `args` | | Extra arguments for `odcslint check`, e.g. `--hide-warnings`. |
| `version` | `latest` | odcslint version to install, e.g. `v0.1.0`. |

The step fails if odcslint reports any error-severity finding.
