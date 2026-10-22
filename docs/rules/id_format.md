# `id_format`

Checks that the contract `id` is a UUID, or that it matches a regex you choose. If the `id` is missing altogether, [schema validation](../schema-validation.md) reports it, not this rule.

## Options

| Option | Default | Description |
|---|---|---|
| `pattern` | | A regex the id must match. When omitted, the id must be a UUID. |

## Example

**Config**

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

**Contract**

```yaml title="orders.odcs.yaml"
id: orders  # not a UUID, and not urn:datacontract:...
```

**Findings**

```text
orders.odcs.yaml
  1:5  error    id_format  contract id `orders` is not a UUID
  1:5  error    urn_ids    contract id `orders` does not match pattern `^urn:datacontract:[a-z]+:[a-z0-9-]+$`
```
