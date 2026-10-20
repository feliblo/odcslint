# Tests

- **Unit tests** live at the bottom of each source file. Rules are tested with real inline YAML via `rules::test_support`; no mocks.
- **Integration tests** are modules of a single test binary (`tests/integration/main.rs`) to keep build times low. Add a module under `tests/integration/` instead of a new top-level test file.
  - `corpus.rs`: upstream ODCS examples must pass for their declared `apiVersion` (except the documented `KNOWN_VERSION_MISMATCHES`), upstream negative tests must fail, plus a snapshot of their findings.
  - `fix.rs`: golden files in `tests/fixtures/fix/` and safety/idempotence of `--fix` over every upstream example.
  - `cli.rs`: the binary end to end (exit codes, output formats, `--fix`, `init`, `rules`).
  - `docs.rs`: every rule has a docs page and an index row.
- **Snapshots** (`insta`) only for reporter output and the negative-test findings. Review changes with `cargo insta review`.
- `tests/fixtures/upstream/` is vendored from bitol-io/open-data-contract-standard (see its `SOURCE.md`); do not edit those files.
- Golden fix files: `<name>.in.yaml` is the input, `<name>.out.yaml` the expected output; the out file must be stable under a second `--fix`.
