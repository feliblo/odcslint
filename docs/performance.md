# Performance

odcslint walks directories and checks files in parallel. Each contract is parsed once, and each ODCS schema is compiled at most once per run.

## Throughput

Measured on 2026-10-19 on an Apple M4 (10 cores), using the `dist` build profile. The corpus is 10,000 contracts generated from the upstream ODCS examples (`just gen-corpus`):

| Run | Wall time |
|---|---|
| 10,000 contracts, schema validation only | 0.40 s |
| 10,000 contracts, schema validation + 5 rules | 0.43 to 0.47 s |
| 1 contract (process start, schema compile, check) | 7 ms |

The `dist` binary is 5.1 MiB.

## Compared with `datacontract lint`

[datacontract-cli](https://github.com/datacontract/datacontract-cli) is the established toolkit for data contracts. It tests contracts against real data, imports and exports many formats, and much more. One of its commands, `lint`, validates a contract against the ODCS JSON Schema. That is where the two tools overlap, so this comparison covers schema validation only.

### Speed

| | `datacontract lint` | `odcslint check` |
|---|---|---|
| 1 contract | 301 ms | 7.3 ms |
| 200 contracts, one call per file | 58 s | 1.4 s |
| 200 contracts, one call | not supported (one file per call) | 0.018 s |
| 10,000 contracts | about 48 min (estimated from 290 ms per file) | 0.40 s |

The validation itself is quick in both tools: `datacontract lint` reports about 0.03 s per contract. Most of its wall time is Python starting up, and since it takes one file per call, that start-up happens once per contract. The difference matters mostly when you check many contracts at once, for example in a pre-commit hook or CI.

### Correctness on the upstream ODCS fixtures

| | `datacontract lint` | `odcslint check` |
|---|---|---|
| Invalid contracts rejected (44 negative tests) | 41 | 44 |
| Valid examples accepted (42 examples) | 38 | 39 |

The differences come from deliberate choices rather than bugs. Two negative tests use `kind: DataModel` and `apiVersion: v4.0.0`. datacontract still supports its older Data Contract Specification format, so when a file isn't recognised as ODCS it converts it and warns instead of failing. A third has an unknown `element` field in a quality check, which datacontract currently reports as a warning; it has announced this becomes an error in January 2027.

datacontract also flags one upstream example, `schema/relationships.odcs.yaml` (v3.1.0), because the v3.1.0 schema it ships predates a later upstream update.

Both tools reject three upstream examples that declare `apiVersion: v3.0.2` but use features from later versions (see [Schema validation](schema-validation.md)).

### Method

The comparison used datacontract-cli 1.2.7 and odcslint 0.1.0, on 2026-10-19 on an Apple M4 (10 cores).

datacontract ran as `datacontract lint --no-inline-references --all-errors <file>`. `--no-inline-references` keeps it offline (it then uses its bundled schemas and makes no network calls), and `--all-errors` makes it report every schema error instead of only the first. odcslint ran as `odcslint check -q <files>`, using a release build and the default config, which means schema validation only.

Times are wall-clock time around each process, start-up included:

- Single contract: the median of 15 runs on the same file, after one warm-up run.
- 200 contracts: the first 200 files of the generated corpus, with one process per file for both tools, plus one odcslint run over all 200 files. odcslint batch timings are the median of 5 runs.
- 10,000 contracts: for datacontract this is an estimate from the measured 290 ms per file. We didn't run it.

A contract counts as rejected when the tool exits with a non-zero code.

## Reproduce

```bash
just bench 10000                  # odcslint on 10,000 generated contracts (uses hyperfine if installed)
just compare-datacontract 200     # installs datacontract-cli 1.2.7 into target/ and runs the comparison
```

The comparison script is `scripts/compare_datacontract.py`.
