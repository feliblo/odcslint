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

## Examples

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

## Findings

```text
orders.odcs.yaml
  9:5    error    primary_key  `orders` has no primary key
  9:5    error    primary_key  primaryKeyPosition values of `orders` should be 1..2 but are 1, 3
  12:9   error    primary_key  primary key `order_id` has no primaryKeyPosition
  20:29  error    primary_key  `amount` has a primaryKeyPosition but is not a primary key
  24:9   error    primary_key  duplicate primaryKeyPosition 1 in `orders`
```
