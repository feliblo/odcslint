# `required_fields`

Checks that fields are present and not empty on every object in scope. A value counts as empty if it is null, a string with only whitespace, an empty list or an empty mapping.

## Options

| Option | Default | Description |
|---|---|---|
| `scope` | `contract` | `contract`, `schema` (each schema object) or `property` (each property, nested ones included). |
| `fields` | required | Dotted paths relative to each object, for example `description.purpose`. |

## Example

**Config**

=== "YAML"

    ```yaml
    rules:
      - type: required_fields
        fields: [domain, team, description.purpose, description.usage]

      - type: required_fields
        scope: property
        fields: [logicalType, physicalType]
    ```

=== "TOML"

    ```toml
    [[rules]]
    type = "required_fields"
    fields = ["domain", "team", "description.purpose", "description.usage"]

    [[rules]]
    type = "required_fields"
    scope = "property"
    fields = ["logicalType", "physicalType"]
    ```

**Contract**

```yaml title="orders.odcs.yaml"
domain: sales
team:
  name: sales
description:  # no purpose
  usage: Monthly revenue reporting
schema:
  - name: orders
    properties:
      - name: amount  # no physicalType
        logicalType: number
```

**Findings**

```text
orders.odcs.yaml
  1:1  error    required_fields  missing required field `description.purpose`
  9:9  error    required_fields  property `amount` is missing required field `physicalType`
```
