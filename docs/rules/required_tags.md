# `required_tags`

Checks that objects in scope carry all of the configured tags, or at least one of them.

## Options

| Option | Default | Description |
|---|---|---|
| `scope` | `contract` | `contract`, `schema` or `property`. |
| `tags` | required | The tags to look for. |
| `criteria` | `all` | `all`: every tag is required. `any`: at least one of them is. |

## Example

**Config**

=== "YAML"

    ```yaml
    rules:
      - type: required_tags
        tags: [pii-reviewed]
      - type: required_tags
        scope: schema
        tags: [bronze, silver, gold]
        criteria: any
    ```

=== "TOML"

    ```toml
    [[rules]]
    type = "required_tags"
    tags = ["pii-reviewed"]

    [[rules]]
    type = "required_tags"
    scope = "schema"
    tags = ["bronze", "silver", "gold"]
    criteria = "any"
    ```

**Contract**

```yaml title="orders.odcs.yaml"
tags: [finance]  # no pii-reviewed tag
schema:
  - name: orders  # none of bronze, silver or gold
    tags: [raw]
```

**Findings**

```text
orders.odcs.yaml
  1:1  error    required_tags  contract is missing required tags: pii-reviewed
  3:5  error    required_tags  schema `orders` needs at least one of the tags: bronze, silver, gold
```
