# `has_description`

Checks that objects in scope have a description of at least a minimum length. For the `contract` scope, that's `description.purpose`. For `schema` and `property`, it's the `description` string.

## Options

| Option | Default | Description |
|---|---|---|
| `scope` | `property` | `contract`, `schema` or `property`. |
| `min_length` | `1` | Minimum number of characters, after trimming whitespace. |

## Example

**Config**

=== "YAML"

    ```yaml
    rules:
      - type: has_description
        scope: property
        min_length: 10
    ```

=== "TOML"

    ```toml
    [[rules]]
    type = "has_description"
    scope = "property"
    min_length = 10
    ```

**Contract**

```yaml title="orders.odcs.yaml"
schema:
  - name: orders
    properties:
      - name: amount  # no description
      - name: note
        description: A note  # shorter than 10 characters
```

**Findings**

```text
orders.odcs.yaml
  4:9  error    has_description  property `amount` has no description
  5:9  error    has_description  description of property `note` is shorter than 10 characters
```
