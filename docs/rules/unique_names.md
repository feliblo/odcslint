# `unique_names`

The ODCS schema allows duplicate names; this rule doesn't. With `scope: schema` it checks the names of top-level schema objects. With `scope: property` it checks property names within each `properties` list, and treats nested lists as separate lists.

## Options

| Option | Default | Description |
|---|---|---|
| `scope` | required | `schema` or `property`. |
| `case_sensitive` | `false` | When `false`, `id` and `ID` are duplicates (most databases treat identifiers this way). |

## Example

**Config**

=== "YAML"

    ```yaml
    rules:
      - type: unique_names
        scope: property
      - type: unique_names
        scope: schema
    ```

=== "TOML"

    ```toml
    [[rules]]
    type = "unique_names"
    scope = "property"
    ```

**Contract**

```yaml title="orders.odcs.yaml"
schema:
  - name: orders
    properties:
      - name: id
      - name: ID  # same name as id, ignoring case
```

**Findings**

```text
orders.odcs.yaml
  5:15  error    unique_names  duplicate property name `ID`
```
