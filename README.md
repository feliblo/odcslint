# odcslint

A fast linter and fixer for [Open Data Contract Standard (ODCS)](https://bitol-io.github.io/open-data-contract-standard/) contracts, written in Rust.

- **Schema validation.** Every contract is checked against the official ODCS JSON Schema for its `apiVersion`. The schemas are embedded in the binary ([supported versions](https://feliblo.github.io/odcslint/schema-validation/)).
- **Your rules.** Opt-in governance rules: primary keys first and metadata columns last, allowed owners, domains and types, naming conventions, required fields, tags and descriptions, and more.
- **Fixes that keep formatting.** `--fix` moves the original text, so comments and formatting are preserved.
- **Precise and fast.** Every finding has a line and column, and files are checked in parallel.
- **Runs anywhere:** as a CLI, a prek hook or a GitHub Action, with `text`, `json`, `ndjson`, `csv` and `github` output.

```console
$ odcslint check contracts/
contracts/orders.odcs.yaml
  13:17  error    approved_owners  team member username `mallory` is not allowed; allowed values: alice, bob
  20:9   error    property_order   expected `order_id` at position 1 of `orders`, found `_loaded_at` [*]
  40:13  error    property_order   expected `street` at position 1 of `shipping_address`, found `_source_hash` [*]

contracts/payments.odcs.yaml
  6:1  error    schema  unexpected property `owner`

Found 4 errors and 0 warnings (2 files checked). 2 findings marked [*] can be fixed with `odcslint check --fix`.
```

Checking 10,000 contracts takes 0.4 s. For a comparison with `datacontract lint`, the schema check in the [datacontract-cli](https://github.com/datacontract/datacontract-cli) toolkit, see [Performance](https://feliblo.github.io/odcslint/performance/).

## Installation

```bash
pip install odcslint                 # or: uv tool install odcslint
brew install feliblo/tap/odcslint
curl -LsSf https://github.com/feliblo/odcslint/releases/latest/download/odcslint-installer.sh | sh
```

## Usage

```bash
odcslint init                          # write a starter odcslint.yml
odcslint check                         # every *.odcs.yaml below the current directory
odcslint check contracts/ "domains/**/*.odcs.yaml"
odcslint check --fix                   # apply fixes in place
odcslint rules                         # list the rule types
```

Exit codes: `0` means no errors, `1` means errors were found, `2` means a tool error (invalid config, path not found, no contracts).

## Configuration

`odcslint.yml` (or `odcslint.toml`, or `[tool.odcslint]` in `pyproject.toml`):

```yaml
settings:
  exclude: ["examples/**"]

rules:
  - type: property_order            # fixable
    metadata_columns:
      prefixes: ["_"]

  - type: allowed_values
    name: approved_owners
    field: team.members.username
    role: owner
    values: [alice, bob]
    required: true

  - type: allowed_values
    field: domain
    values: [finance, marketing, sales]

  - type: naming_convention
    target: schema.properties.name
    convention: snake_case
```

See the [documentation](https://feliblo.github.io/odcslint/) for all rules and options.

## prek

```yaml
repos:
  - repo: https://github.com/feliblo/odcslint
    rev: v0.1.0
    hooks:
      - id: odcslint-fix   # or `odcslint` to only check
```

No separate install needed: prek builds odcslint on first use. The hooks also work with pre-commit.

## GitHub Action

```yaml
- uses: feliblo/odcslint@v0.1.0
  with:
    paths: contracts/
```

## License

MIT. The embedded ODCS JSON Schemas are © The Bitol Contributors, licensed under Apache-2.0 (see [`licenses/`](licenses/)).
