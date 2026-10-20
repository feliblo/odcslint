# `allowed_values`

Checks that a field only uses values from an allow-list, or values that match a regex. This is the rule for approved owners, known domains, agreed statuses, supported types and similar lists.

## Options

| Option | Default | Description |
|---|---|---|
| `field` | required | The field to check (see below). |
| `values` | `[]` | Allowed values. |
| `pattern` | | A regex. Values matching it are also allowed. At least one of `values` and `pattern` is required. |
| `role` | | Only for the team member fields (`team.members.username`, `team.members.role`): only check members with this role. |
| `required` | `false` | Report objects where the field is missing or empty. |
| `case_sensitive` | `true` | Compare values case-sensitively. |

### Fields

You can name a field by its ODCS path or by its short name; both mean the same thing.

| ODCS path | Short name | Checks |
|---|---|---|
| `apiVersion` | `api_version` | `apiVersion` |
| `domain` | `domain` | `domain` |
| `status` | `status` | `status` |
| `tenant` | `tenant` | `tenant` |
| `dataProduct` | `data_product` | `dataProduct` |
| `tags` | `contract_tags` | each entry of the contract's `tags` |
| `team.name` | `team_name` | `team.name` |
| `team.members.username` | `team_members_username` | `username` of each team member (`team.members`, or the deprecated `team` list) |
| `team.members.role` | `team_members_role` | `role` of each team member |
| `customProperties: <key>` | `custom_property: <key>` | `value` of the contract-level `customProperties` entry whose `property` is `<key>` |
| `schema.name` | `schema_name` | `name` of each schema object |
| `schema.physicalType` | `schema_physical_type` | `physicalType` of each schema object |
| `schema.tags` | `schema_tags` | `tags` of each schema object |
| `schema.properties.logicalType` | `property_logical_type` | `logicalType` of each property, nested ones included |
| `schema.properties.physicalType` | `property_physical_type` | `physicalType` of each property |
| `schema.properties.classification` | `property_classification` | `classification` of each property |
| `schema.properties.tags` | `property_tags` | `tags` of each property |

With `required: true`, every object that owns the field must have it: every property needs a `logicalType`, for instance, or the contract needs at least one owner.

## Examples

=== "YAML"

    ```yaml
    rules:
      # Only approved owners, and every contract must have one.
      - type: allowed_values
        name: approved_owners
        field: team.members.username
        role: owner
        values: [alice, bob, data-platform-bot]
        required: true

      # Known domains only.
      - type: allowed_values
        field: domain
        values: [finance, marketing, sales]

      # Owners stored as a custom property must look like first.last.
      - type: allowed_values
        field: { customProperties: owner }
        pattern: '^[a-z]+\.[a-z]+$'

      # Restrict physical types.
      - type: allowed_values
        field: schema.properties.physicalType
        pattern: '^(varchar\(\d+\)|integer|bigint|date|timestamp_ntz|decimal\(\d+,\s*\d+\))$'
        case_sensitive: false
    ```

=== "TOML"

    ```toml
    [[rules]]
    type = "allowed_values"
    name = "approved_owners"
    field = "team.members.username"
    role = "owner"
    values = ["alice", "bob", "data-platform-bot"]
    required = true

    [[rules]]
    type = "allowed_values"
    field = "domain"
    values = ["finance", "marketing", "sales"]

    [[rules]]
    type = "allowed_values"
    field = { customProperties = "owner" }
    pattern = '^[a-z]+\.[a-z]+$'
    ```

## Findings

```text
orders.odcs.yaml
  1:1   error    approved_owners  no team member username found
  3:9   error    allowed_values   domain `hr` is not allowed; allowed values: finance, marketing, sales
  6:17  error    approved_owners  team member username `mallory` is not allowed; allowed values: alice, bob, data-platform-bot
```
