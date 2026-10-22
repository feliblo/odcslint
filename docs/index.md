---
hide:
  - navigation
  - toc
---

<div class="hero" markdown>

# odcslint

<p class="hero-tagline">Lint your data contracts against the standard <em>and</em> your own rules.</p>

odcslint is a linter for [Open Data Contract Standard (ODCS)](https://bitol-io.github.io/open-data-contract-standard/) contracts, written in Rust. It checks every contract against the official schema and against the conventions your team agrees on. Where it can, it fixes problems for you and leaves your formatting alone. When a new ODCS version comes out, `odcslint upgrade` moves your contracts to it.

[Get started](installation.md){ .md-button .md-button--primary }
[Browse the rules](rules/index.md){ .md-button }

</div>

## What it checks

<div class="grid cards pillars" markdown>

-   :lucide-shield-check:{ .lg .middle } __Schema validation__

    ---

    Each contract is validated against the official ODCS JSON Schema for its `apiVersion` (see the [supported versions](schema-validation.md)). The schemas ship inside the binary, so nothing is downloaded, and every error points at a line and column.

    ```text
    payments.odcs.yaml
      6:1  error    schema  unexpected property `owner`
      9:9  error    schema  logicalType: `binary` is not one of: string, date, ...
    ```

    [:lucide-arrow-right: Schema validation](schema-validation.md)

-   :lucide-scale:{ .lg .middle } __Your own rules__

    ---

    Rules turn team conventions into checks: approved owners, known domains, column order, naming, and required descriptions and tags. There are 11 rules to choose from, and you enable them in `odcslint.yml`.

    ```yaml
    - type: allowed_values
      field: team.members.username
      role: owner
      values: [alice, bob]
    ```

    [:lucide-arrow-right: All rules](rules/index.md)

</div>

## Quick start

1.  Install odcslint:

    === "pip / uv"

        ```bash
        pip install odcslint
        ```

    === "Homebrew"

        ```bash
        brew install feliblo/tap/odcslint
        ```

2.  Check every `*.odcs.yaml` below the current directory against the ODCS schema:

    ```bash
    odcslint check
    ```

3.  Write a starter `odcslint.yml`, then enable and tune the rules you want:

    ```bash
    odcslint init
    ```

4.  Fix what can be fixed:

    ```bash
    odcslint check --fix
    ```

5.  Move your contracts to the newest ODCS version, after checking what would change:

    ```bash
    odcslint upgrade --dry-run
    odcslint upgrade
    ```

## Autofix

`odcslint check --fix` repairs fixable findings in place. Instead of rewriting the file, it moves the original text, so comments, block scalars and blank lines stay where you put them.

```diff
     properties:
-      # Load metadata, added by the ingestion framework.
-      - name: _loaded_at
       - name: order_id
         primaryKey: true
         primaryKeyPosition: 1
       - name: amount
+      # Load metadata, added by the ingestion framework.
+      - name: _loaded_at
```

[:lucide-arrow-right: How autofix works](autofix.md)

## Upgrades

`odcslint upgrade` moves contracts to a newer ODCS version and converts the breaking changes along the way. Like autofix, it edits the original text, so your comments stay. Run it with `--dry-run` first to see what would change.

```diff
-apiVersion: v3.0.2
+apiVersion: v3.2.0
 team:
-  # Owners of this contract
-  - username: alice
-    role: owner
+  members:
+    # Owners of this contract
+    - username: alice
+      role: owner
 ...
         logicalTypeOptions:
-          minimum: 0
-          exclusiveMinimum: true
+          exclusiveMinimum: 0
```

[:lucide-arrow-right: Upgrading contracts](upgrade.md)

## Example output

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

## Speed, output and integrations

<div class="grid cards features" markdown>

-   :lucide-zap:{ .lg .middle } __Fast__

    ---

    10,000 contracts take 0.4 s. Files are checked in parallel, and each schema is compiled only once.

    [:lucide-arrow-right: Performance](performance.md)

-   :lucide-crosshair:{ .lg .middle } __Precise__

    ---

    Every finding has a line and column. Messages stay short and never print whole objects from your contract.

-   :lucide-git-pull-request:{ .lg .middle } __Runs everywhere__

    ---

    The same binary works on the command line, as a [prek hook](integrations/prek.md) and as a [GitHub Action](integrations/github-action.md) that annotates pull requests.

-   :lucide-file-json:{ .lg .middle } __Machine-readable__

    ---

    Output comes as `text`, `json`, `ndjson`, `csv` or `github`, and the exit codes are documented.

    [:lucide-arrow-right: Usage](usage.md)

</div>
