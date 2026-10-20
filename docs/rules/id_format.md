# `id_format`

Checks that the contract `id` is a UUID, or that it matches a regex you choose. If the `id` is missing altogether, [schema validation](../schema-validation.md) reports it, not this rule.

## Options

| Option | Default | Description |
|---|---|---|
| `pattern` | | A regex the id must match. When omitted, the id must be a UUID. |

## Examples

=== "YAML"

    ```yaml
    rules:
      - type: id_format
      - type: id_format
        name: urn_ids
        pattern: '^urn:datacontract:[a-z]+:[a-z0-9-]+$'
    ```

=== "TOML"

    ```toml
    [[rules]]
    type = "id_format"
    ```

## Findings

```text
orders.odcs.yaml
  4:5  error    id_format  contract id `orders` is not a UUID
```
