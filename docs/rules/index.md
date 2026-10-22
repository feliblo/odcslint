---
hide:
  - toc
---

# Rules

Rules are off by default. To turn one on, add it to the [`rules`](../configuration.md#rules) list in your config. On top of its own options, every rule accepts `name`, `severity`, `description`, `includes` and `excludes`. Rules marked in the Autofix column are fixed by `odcslint check --fix`.

| Rule | What it checks | Autofix |
|---|---|:-:|
| [`allowed_values`](allowed_values.md) | A field only uses allowed values: owners, domains, statuses, tenants, types, classifications, tags or custom properties |  |
| [`deprecated_fields`](deprecated_fields.md) | No deprecated ODCS fields are used |  |
| [`file_naming`](file_naming.md) | File names follow a pattern and/or match the contract `id` or `name` |  |
| [`has_description`](has_description.md) | Contracts, schema objects or properties have a description |  |
| [`id_format`](id_format.md) | The contract `id` is a UUID or matches a pattern |  |
| [`naming_convention`](naming_convention.md) | Schema object and property names follow a naming convention |  |
| [`primary_key`](primary_key.md) | `primaryKey` and `primaryKeyPosition` are consistent |  |
| [`property_order`](property_order.md) | Primary keys come first and metadata columns last | :lucide-check:{ .autofix-yes title="Fixed by odcslint check --fix" } |
| [`required_fields`](required_fields.md) | Fields are present and non-empty |  |
| [`required_tags`](required_tags.md) | Tags are present |  |
| [`unique_names`](unique_names.md) | Schema object or property names are unique |  |

`allowed_values` also has [aliases](allowed_values.md#fields) such as `allowed_domains` and `allowed_owners`, which set its `field` for you.

`odcslint rules` prints the same list in your terminal, aliases included.
