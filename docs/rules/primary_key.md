# `primary_key`

The ODCS schema lets `primaryKey` and `primaryKeyPosition` disagree. This rule checks, for each schema object, that:

- a primary key has a `primaryKeyPosition` of 1 or more
- a property with a `primaryKeyPosition` is a primary key (`-1`, the schema default, is ignored)
- positions are unique
- positions run from 1 without gaps

## Options

| Option | Default | Description |
|---|---|---|
| `require_primary_key` | `false` | Every schema object must have at least one primary key. |
| `require_position` | `true` | Every primary key must have a `primaryKeyPosition`. Set to `false` to allow single-column keys without a position. |

## Example

**Config**

=== "YAML"

    ```yaml
    rules:
      - type: primary_key
        require_primary_key: true
    ```

=== "TOML"

    ```toml
    [[rules]]
    type = "primary_key"
    require_primary_key = true
    ```

**Contract**

```yaml title="orders.odcs.yaml"
schema:
  - name: orders
    properties:
      - name: order_id  # primary key without a primaryKeyPosition
        primaryKey: true
      - name: amount
        primaryKeyPosition: 2  # position on a column that isn't a primary key
```

**Findings**

```text
orders.odcs.yaml
  4:9   error    primary_key  primary key `order_id` has no primaryKeyPosition
  7:29  error    primary_key  `amount` has a primaryKeyPosition but is not a primary key
```
