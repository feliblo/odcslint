# `naming_convention`

Checks that schema object or property names follow a naming convention, either a built-in one or your own regex.

## Options

| Option | Description |
|---|---|
| `target` | Which names to check: `schema.name`, `schema.physicalName`, `schema.properties.name` or `schema.properties.physicalName`. The short names `schema_name`, `schema_physical_name`, `property_name` and `property_physical_name` work too. Property targets include nested properties. |
| `convention` | One of the conventions below, or `{ pattern: '<regex>' }`. |

| Convention | Example | Notes |
|---|---|---|
| `snake_case` | `order_id` | Leading underscores are allowed, so metadata columns like `_loaded_at` pass. |
| `screaming_snake_case` | `ORDER_ID` | Leading underscores are allowed. |
| `camel_case` | `orderId` | |
| `pascal_case` | `OrderId` | |
| `kebab_case` | `order-id` | |

## Example

**Config**

=== "YAML"

    ```yaml
    rules:
      - type: naming_convention
        target: schema.properties.name
        convention: snake_case

      - type: naming_convention
        target: schema.physicalName
        convention:
          pattern: '^(raw|stg|dim|fct)_[a-z0-9_]+$'
    ```

=== "TOML"

    ```toml
    [[rules]]
    type = "naming_convention"
    target = "schema.properties.name"
    convention = "snake_case"

    [[rules]]
    type = "naming_convention"
    target = "schema.physicalName"
    convention = { pattern = '^(raw|stg|dim|fct)_[a-z0-9_]+$' }
    ```

**Contract**

```yaml title="orders.odcs.yaml"
schema:
  - name: orders
    physicalName: orders  # needs a raw_, stg_, dim_ or fct_ prefix
    properties:
      - name: customerName  # not snake_case
```

**Findings**

```text
orders.odcs.yaml
  3:19  error    naming_convention  schema physicalName `orders` does not follow pattern `^(raw|stg|dim|fct)_[a-z0-9_]+$`
  5:15  error    naming_convention  property name `customerName` does not follow snake_case
```
