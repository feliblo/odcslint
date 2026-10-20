# `deprecated_fields`

Reports fields that ODCS has deprecated, along with what to use instead:

| Field | Replacement |
|---|---|
| `dataProduct` | Describe the data product outside the contract. |
| `slaDefaultElement` | Set `element` on each `slaProperties` entry. |
| `team` as a list | Use the `team` object with `members` (ODCS v3.1+). |

## Options

This rule has no options.

## Examples

=== "YAML"

    ```yaml
    rules:
      - type: deprecated_fields
        severity: warning
    ```

=== "TOML"

    ```toml
    [[rules]]
    type = "deprecated_fields"
    severity = "warning"
    ```

## Findings

```text
orders.odcs.yaml
  12:1  warning  deprecated_fields  `team` as a list is deprecated; use `team.members`
```
