default:
    @just --list

fmt:
    cargo fmt --all

lint:
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

test:
    cargo test --workspace --all-features

test-cov:
    cargo llvm-cov nextest --workspace --all-features

# Run fmt check, clippy and tests: the pre-merge quality gate.
check:
    cargo fmt --all --check
    just lint
    just test

install:
    cargo install --path . --locked

# Check the vendored upstream ODCS examples and negative tests.
run-upstream:
    -cargo run --release -q -- check tests/fixtures/upstream/examples
    -cargo run --release -q -- check tests/fixtures/upstream/negative-tests

# Generate N copies of the upstream examples (with unique ids) in target/corpus for benchmarking.
gen-corpus n="10000":
    cargo run --release -q --example gen_corpus -- {{n}} target/corpus

# Time `odcslint check` on the generated corpus. Uses hyperfine when installed, `time` otherwise.
bench n="10000": (gen-corpus n)
    #!/usr/bin/env bash
    set -euo pipefail
    cargo build --release -q
    command="target/release/odcslint check target/corpus --exit-zero -q"
    if command -v hyperfine > /dev/null; then
        hyperfine --warmup 2 "$command"
    else
        echo "hyperfine not found (install with: brew install hyperfine); falling back to 'time'"
        $command > /dev/null
        for run in 1 2 3; do
            /usr/bin/time -p $command > /dev/null
        done
    fi

# Fail if upstream ODCS has a schema version that schemas/odcs/ or src/schema/embedded.rs lacks.
check-schemas ref="main":
    python3 -I scripts/check_schema_versions.py {{ref}}

# Compare with `datacontract lint` (datacontract-cli). Installs it into target/datacontract-venv.
compare-datacontract files="200" version="1.2.7": (gen-corpus "10000")
    cargo build --release -q
    test -x target/datacontract-venv/bin/datacontract || (uv venv -q target/datacontract-venv && VIRTUAL_ENV=target/datacontract-venv uv pip install -q datacontract-cli=={{version}})
    python3 -I scripts/compare_datacontract.py "$PWD" target/datacontract-venv/bin/datacontract {{files}}

docs:
    uvx zensical serve

docs-build:
    uvx zensical build --clean --strict

bump:
    cz bump --increment PATCH
