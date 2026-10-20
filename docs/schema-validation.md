# Schema validation

odcslint validates each contract against the official [ODCS JSON Schema](https://github.com/bitol-io/open-data-contract-standard/tree/main/schema) for its `apiVersion`.

| `apiVersion` | Embedded schema |
|---|---|
| `v3.0.0` | `odcs-json-schema-v3.0.0.json` |
| `v3.0.1` | `odcs-json-schema-v3.0.1.json` |
| `v3.0.2` | `odcs-json-schema-v3.0.2.json` |
| `v3.1.0` | `odcs-json-schema-v3.1.0-20260908.json` (dated snapshot) |
| `v3.2.0` | `odcs-json-schema-v3.2.0.json` |
| missing or not a string | the newest schema above |
| anything else (e.g. `v2.2.2`) | not validated; reported as `unsupported_api_version` |

The schemas are copied into the repository under `schemas/odcs/` and compiled into the binary. `schemas/odcs/SOURCE.md` records the upstream commit they came from. They are licensed under Apache-2.0 (see `licenses/`).

## What is checked

The schemas are written in JSON Schema Draft 2019-09, and odcslint enforces the whole draft. In practice that means required fields such as `apiVersion`, `kind`, `id` and `version`, no unknown properties (`unevaluatedProperties: false`), enum values such as `logicalType`, server `type` and quality `dimension`, the extra fields each server type requires, and formats such as `uri`, `date` and `date-time`.

## Findings

Schema findings use the rule name `schema`, point at the offending key, and name the field they are about:

```text
orders.odcs.yaml
  9:1   error    schema  unexpected property `unknownRootField`
  14:9  error    schema  logicalType: `binary` is not one of: string, date, timestamp, time, number, integer, object, array, boolean, map, vector
  15:5  error    schema  servers[0]: missing required property `host`
  18:9  error    schema  deprecated: expected boolean, found `maybe`
```

Under the 2019-09 rules, an object that fails validation also gets its other, valid keys reported as "unevaluated". odcslint hides those follow-on errors while a more specific error exists at the same object or deeper. Once you fix the reported error, run odcslint again to see anything that was hidden.

Files that can't be parsed get a `yaml_syntax` finding instead. That covers syntax errors, duplicate keys, empty files and files with more than one YAML document.

## Severity

Set `settings.schema_validation` to `warning` to report schema findings without failing the run, or to `off` to run only your rules.
