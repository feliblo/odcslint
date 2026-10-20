# Configuration

odcslint looks for a config file in the directory you run it from and uses the first one it finds, in this order:

1. `odcslint.yml`
2. `odcslint.yaml`
3. `odcslint.toml`
4. `pyproject.toml`, if it has a `[tool.odcslint]` section

Pass `--config <FILE>` to use a specific file instead. Without a config file, odcslint only runs [schema validation](schema-validation.md). Rules stay off until you list them.

## Structure

=== "YAML"

    ```yaml
    settings:
      include: ["**/*.odcs.yaml", "**/*.odcs.yml"]
      exclude: ["examples/**"]
      schema_validation: error   # error | warning | off

    rules:
      - type: property_order
      - type: allowed_values
        name: approved_owners
        severity: error
        description: Owners must be on the data governance list.
        field: team.members.username
        role: owner
        values: [alice, bob]
        excludes: ["sandbox/**"]
    ```

=== "TOML"

    ```toml
    [settings]
    include = ["**/*.odcs.yaml", "**/*.odcs.yml"]
    exclude = ["examples/**"]
    schema_validation = "error"

    [[rules]]
    type = "property_order"

    [[rules]]
    type = "allowed_values"
    name = "approved_owners"
    severity = "error"
    description = "Owners must be on the data governance list."
    field = "team.members.username"
    role = "owner"
    values = ["alice", "bob"]
    excludes = ["sandbox/**"]
    ```

=== "pyproject.toml"

    ```toml
    [tool.odcslint.settings]
    exclude = ["examples/**"]

    [[tool.odcslint.rules]]
    type = "property_order"

    [[tool.odcslint.rules]]
    type = "allowed_values"
    name = "approved_owners"
    field = "team.members.username"
    role = "owner"
    values = ["alice", "bob"]
    ```

## `settings`

| Key | Default | Description |
|---|---|---|
| `include` | `["**/*.odcs.yaml", "**/*.odcs.yml"]` | Globs a file must match to be checked when a directory is walked. |
| `exclude` | `[]` | Globs to skip when walking. With `--force-exclude`, also applied to files passed explicitly. |
| `schema_validation` | `error` | Severity of [schema findings](schema-validation.md), or `off`. |

Globs are matched against paths relative to the working directory, with `/` as the separator. `*` stays within one directory, while `**` matches across directories.

## `rules`

`rules` is a list. Every entry accepts the keys below, plus the options of its `type`:

| Key | Default | Description |
|---|---|---|
| `type` | required | The rule type. See the [rules](rules/index.md). |
| `name` | the type | Display name in output. Useful when the same type is configured more than once. |
| `severity` | `error` | `error` or `warning`. Only errors make `odcslint check` exit with `1`. |
| `description` | | Free text that documents why the rule exists. |
| `includes` | all files | Only run the rule on files matching these globs. |
| `excludes` | none | Skip files matching these globs. |

You can list the same rule type more than once, for example with different values, a different `name`, or different `includes` and `excludes`.

odcslint rejects unknown keys and invalid regexes or globs. It prints a message naming the problem and exits with code `2`.
