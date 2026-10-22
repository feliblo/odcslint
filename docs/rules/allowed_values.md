# `allowed_values`

Checks that a field only uses values from an allow-list, or values that match a regex. This is the rule for approved owners, known domains, agreed statuses, supported types and similar lists.

## Fields

Each `allowed_values` rule checks one field of the contract. There are two ways to say which one:

- **Alias**: a ready-made rule type, such as `allowed_domains`, that already knows which field to check. You only add the allowed values.
- **Long form**: `type: allowed_values` with `field:` set to the field you want, by its ODCS path (`team.members.username`) or its short name (`team_members_username`). This works for every field, including the ones without an alias.

<div class="alias-picker" markdown>

=== "Alias"

    ```yaml
    - type: allowed_owners
      values: [alice, bob]
    ```

    :lucide-mouse-pointer-click: Click a row to show it in the example above.
    { .alias-picker__hint }

    | Alias | Field | Example values |
    |---|---|---|
    | `allowed_api_versions` | `apiVersion` | v3.1.0, v3.2.0 |
    | `allowed_domains` | `domain` | finance, sales |
    | `allowed_statuses` | `status` | active, draft |
    | `allowed_tenants` | `tenant` | emea, us |
    | `allowed_data_products` | `dataProduct` | orders, customers |
    | `allowed_tags` | `tags` of the contract | finance, pii |
    | `allowed_teams` | `team.name` | data-platform, finance |
    | `allowed_members` | `team.members.username` | alice, bob, carol |
    | `allowed_owners` | `team.members.username` with `role: owner` | alice, bob |
    | `allowed_roles` | `team.members.role` | owner, engineer, analyst |
    | `allowed_schema_types` | `schema.physicalType` | table, view |
    | `allowed_logical_types` | `schema.properties.logicalType` | string, integer, date |
    | `allowed_physical_types` | `schema.properties.physicalType` | varchar, bigint, timestamp |
    | `allowed_classifications` | `schema.properties.classification` | public, internal, confidential |
    | `allowed_property_tags` | `schema.properties.tags` | pii, metric |

    An alias sets `field` (and `role` for `allowed_owners`), so you can't set those yourself. Findings use the alias as the rule name unless you set `name`. Schema object names and tags and custom properties have no alias; use the long form for them.

=== "Long form"

    ```yaml
    - type: allowed_values
      field: team.members.username
      role: owner
      values: [alice, bob]
    ```

    :lucide-mouse-pointer-click: Click a row to show it in the example above.
    { .alias-picker__hint }

    | Field | Checks | Example values |
    |---|---|---|
    | `apiVersion` or `api_version` | the contract's `apiVersion` | v3.1.0, v3.2.0 |
    | `domain` | the contract's `domain` | finance, sales |
    | `status` | the contract's `status` | active, draft |
    | `tenant` | the contract's `tenant` | emea, us |
    | `dataProduct` or `data_product` | the contract's `dataProduct` | orders, customers |
    | `tags` or `contract_tags` | each entry of the contract's `tags` | finance, pii |
    | `team.name` or `team_name` | the team's `name` | data-platform, finance |
    | `team.members.username` or `team_members_username` | `username` of each team member (`team.members`, or the deprecated `team` list) | alice, bob, carol |
    | `team.members.role` or `team_members_role` | `role` of each team member | owner, engineer, analyst |
    | `customProperties: my_custom_property` or `custom_property: my_custom_property` | `value` of the contract-level `customProperties` entry whose `property` is `my_custom_property` | gold, silver |
    | `schema.name` or `schema_name` | `name` of each schema object | orders, customers |
    | `schema.physicalType` or `schema_physical_type` | `physicalType` of each schema object | table, view |
    | `schema.tags` or `schema_tags` | `tags` of each schema object | finance, pii |
    | `schema.properties.logicalType` or `property_logical_type` | `logicalType` of each property, nested ones included | string, integer, date |
    | `schema.properties.physicalType` or `property_physical_type` | `physicalType` of each property | varchar, bigint, timestamp |
    | `schema.properties.classification` or `property_classification` | `classification` of each property | public, internal, confidential |
    | `schema.properties.tags` or `property_tags` | `tags` of each property | pii, metric |

</div>

## Options

| Option | Default | Description |
|---|---|---|
| `field` | required, unless you use an alias | The field to check (see [Fields](#fields)). |
| `values` | `[]` | Allowed values. |
| `pattern` | | A regex. Values matching it are also allowed. At least one of `values` and `pattern` is required. |
| `role` | | Only for the team member fields (`team.members.username`, `team.members.role`): only check members with this role. |
| `required` | `false` | Report objects where the field is missing or empty. |
| `case_sensitive` | `true` | Compare values case-sensitively. |

With `required: true`, every object that owns the field must have it: every property needs a `logicalType`, for instance, or the contract needs at least one owner.

## Example

**Config**

=== "YAML"

    ```yaml
    rules:
      # Only approved owners, and every contract must have one.
      - type: allowed_owners
        values: [alice, bob, data-platform-bot]
        required: true

      # Known domains only.
      - type: allowed_domains
        values: [finance, marketing, sales]

      # Values of a custom property must look like first.last.
      - type: allowed_values
        field: { customProperties: my_custom_property }
        pattern: '^[a-z]+\.[a-z]+$'

      # Restrict physical types.
      - type: allowed_physical_types
        pattern: '^(varchar\(\d+\)|integer|bigint|date|timestamp_ntz|decimal\(\d+,\s*\d+\))$'
        case_sensitive: false
    ```

=== "TOML"

    ```toml
    [[rules]]
    type = "allowed_owners"
    values = ["alice", "bob", "data-platform-bot"]
    required = true

    [[rules]]
    type = "allowed_domains"
    values = ["finance", "marketing", "sales"]

    [[rules]]
    type = "allowed_values"
    field = { customProperties = "my_custom_property" }
    pattern = '^[a-z]+\.[a-z]+$'
    ```

**Contract**

```yaml title="orders.odcs.yaml"
domain: hr  # not an allowed domain
team:
  members:
    - username: mallory  # not an approved owner
      role: owner
```

**Findings**

```text
orders.odcs.yaml
  1:9   error    allowed_domains  domain `hr` is not allowed; allowed values: finance, marketing, sales
  4:17  error    allowed_owners   team member username `mallory` is not allowed; allowed values: alice, bob, data-platform-bot
```
