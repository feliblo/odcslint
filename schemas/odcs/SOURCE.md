# Vendored ODCS JSON Schemas

Source: https://github.com/bitol-io/open-data-contract-standard/tree/dev/schema
Commit: `8532d9c129be76cfd97e5720e96f16ca52d66c0b` (vendored 2026-10-19)
License: Apache-2.0, see `licenses/ODCS-LICENSE` and `licenses/NOTICE`.

| File | apiVersion |
|---|---|
| `odcs-json-schema-v3.0.0.json` | v3.0.0 |
| `odcs-json-schema-v3.0.1.json` | v3.0.1 |
| `odcs-json-schema-v3.0.2.json` | v3.0.2 |
| `odcs-json-schema-v3.1.0-20260908.json` | v3.1.0 (dated snapshot) |
| `odcs-json-schema-v3.2.0.json` | v3.2.0 (rolling minor) |

The files are unmodified. To update, download the files from a newer upstream commit, update the
commit above, and run `cargo test` (the upstream example and negative-test fixtures in
`tests/fixtures/upstream/` should be refreshed from the same commit).
