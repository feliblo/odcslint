# odcslint

[![ODCS v3](https://img.shields.io/badge/ODCS-v3.x-0f766e)](https://feliblo.github.io/odcslint/schema-validation/)
[![PyPI](https://img.shields.io/pypi/v/odcslint)](https://pypi.org/project/odcslint/)
[![CI](https://github.com/feliblo/odcslint/actions/workflows/ci.yml/badge.svg)](https://github.com/feliblo/odcslint/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

A fast, Rust-powered linter for [Open Data Contract Standard (ODCS)](https://bitol-io.github.io/open-data-contract-standard/) contracts.
Full documentation: **[feliblo.github.io/odcslint](https://feliblo.github.io/odcslint/)**

- **Fast validation against the official ODCS schema**
  - **40-3,200×** faster than `datacontract lint` (see the [benchmarks](https://feliblo.github.io/odcslint/performance/))
- **Custom business rules in simple YAML**, with autofix
  - approved owners and known domains
  - column order: primary keys first, metadata columns last
  - naming conventions, required fields and tags

## Installation

<details open>
<summary><b>uv</b></summary>

```bash
uv tool install odcslint     # as a standalone tool
uv add --dev odcslint        # as a dev dependency of your project
```

</details>

<details>
<summary><b>pip</b></summary>

```bash
pip install odcslint
```

</details>

<details>
<summary><b>Homebrew</b></summary>

```bash
brew install feliblo/tap/odcslint
```

</details>

<details>
<summary><b>Shell (macOS / Linux)</b></summary>

```bash
curl -LsSf https://github.com/feliblo/odcslint/releases/latest/download/odcslint-installer.sh | sh
```

</details>

<details>
<summary><b>PowerShell (Windows)</b></summary>

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/feliblo/odcslint/releases/latest/download/odcslint-installer.ps1 | iex"
```

</details>

<details>
<summary><b>prek / pre-commit</b></summary>

```yaml
repos:
  - repo: https://github.com/feliblo/odcslint
    rev: v0.2.0
    hooks:
      - id: odcslint-fix   # or `odcslint` to only check
```

</details>

<details>
<summary><b>GitHub Action</b></summary>

```yaml
- uses: feliblo/odcslint@v0.2.0
  with:
    paths: contracts/
```

</details>

<details>
<summary><b>From source</b></summary>

```bash
cargo install --git https://github.com/feliblo/odcslint
```

</details>

## Quickstart

```bash
odcslint init           # write a starter odcslint.yml
odcslint check          # check every *.odcs.yaml below the current directory
odcslint check --fix    # apply fixes in place
odcslint upgrade        # move contracts to the newest ODCS version
```

```console
$ odcslint check contracts/
contracts/orders.odcs.yaml
  13:17  error    approved_owners  team member username `mallory` is not allowed; allowed values: alice, bob
  20:9   error    property_order   expected `order_id` at position 1 of `orders`, found `_loaded_at` [*]

contracts/payments.odcs.yaml
  6:1  error    schema  unexpected property `owner`

Found 3 errors and 0 warnings (2 files checked). 1 finding marked [*] can be fixed with `odcslint check --fix`.
```

Enable rules in `odcslint.yml`:

```yaml
rules:
  - type: property_order          # primary keys first, metadata columns last (fixable)
  - type: allowed_values
    field: team.members.username
    role: owner
    values: [alice, bob]
```

See the [rules](https://feliblo.github.io/odcslint/rules/) and [configuration](https://feliblo.github.io/odcslint/configuration/) docs for everything else.

## Acknowledgements

odcslint stands on the work of the [Bitol](https://bitol.io/) community and the [LF AI & Data Foundation](https://lfaidata.foundation/), who develop and maintain the Open Data Contract Standard in the open. Their JSON Schemas are embedded in odcslint, and their examples and negative tests make up much of its test suite. Thank you to everyone who contributes to the standard.

## License

odcslint is released under the [MIT license](LICENSE). The embedded ODCS JSON Schemas are © The Bitol Contributors and licensed under Apache-2.0 (see [`licenses/`](licenses/)).
