"""Compare odcslint with `datacontract lint` on correctness (upstream fixtures) and speed.

Usage: python3 scripts/compare_datacontract.py <repo> <datacontract-binary> [files] [--full]

`datacontract lint` checks one file per call, so it is also timed with one process per CPU core
running in parallel. `--full` additionally times that on the whole generated corpus.
"""

import glob
import os
import statistics
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor

REPO, DATACONTRACT = sys.argv[1], sys.argv[2]
OPTIONS = sys.argv[3:]
FILES = next((int(option) for option in OPTIONS if option.isdigit()), 200)
FULL = "--full" in OPTIONS
WORKERS = os.cpu_count() or 1
ODCSLINT = os.path.join(REPO, "target/release/odcslint")
EXAMPLES = sorted(glob.glob(f"{REPO}/tests/fixtures/upstream/examples/**/*.odcs.yaml", recursive=True))
NEGATIVE = sorted(glob.glob(f"{REPO}/tests/fixtures/upstream/negative-tests/*.odcs.yaml"))
CORPUS = sorted(glob.glob(f"{REPO}/target/corpus/*.odcs.yaml"))


def datacontract(path):
    # Without --no-inline-references, datacontract may resolve references over the network.
    command = [DATACONTRACT, "lint", "--no-inline-references", "--all-errors", path]
    return subprocess.run(command, capture_output=True, text=True)


def odcslint(*paths):
    return subprocess.run([ODCSLINT, "check", "-q", *paths], capture_output=True, text=True, cwd=REPO)


def seconds(function, *args):
    start = time.perf_counter()
    function(*args)
    return time.perf_counter() - start


def in_parallel(function, paths):
    with ThreadPoolExecutor(max_workers=WORKERS) as pool:
        list(pool.map(function, paths))


def relative(paths):
    return [os.path.relpath(path, REPO) for path in paths]


print("== Correctness on the upstream ODCS fixtures")
for label, files, valid in [("examples", EXAMPLES, True), ("negative-tests", NEGATIVE, False)]:
    wrong_datacontract = [f for f in files if (datacontract(f).returncode == 0) != valid]
    wrong_odcslint = [f for f in files if (odcslint(f).returncode == 0) != valid]
    print(f"{label}: {len(files)} files")
    print(f"  datacontract wrong: {len(wrong_datacontract)} {relative(wrong_datacontract)}")
    print(f"  odcslint wrong:     {len(wrong_odcslint)} {relative(wrong_odcslint)}")

print("\n== One contract, median of 15 runs (includes process start)")
sample = EXAMPLES[0]
for label, function in [("datacontract lint", datacontract), ("odcslint check", odcslint)]:
    function(sample)
    median = statistics.median(seconds(function, sample) for _ in range(15))
    print(f"  {label:18} {median * 1000:7.1f} ms")

files = CORPUS[:FILES]
print(f"\n== {len(files)} contracts from target/corpus")
per_file_datacontract = seconds(lambda: [datacontract(path) for path in files])
parallel_datacontract = seconds(in_parallel, datacontract, files)
per_file_odcslint = seconds(lambda: [odcslint(path) for path in files])
batch_odcslint = statistics.median(seconds(odcslint, *files) for _ in range(5))
corpus_odcslint = statistics.median(seconds(odcslint, f"{REPO}/target/corpus") for _ in range(5))
print(f"  datacontract lint, one call per file: {per_file_datacontract:8.2f} s")
print(f"  datacontract lint, {WORKERS} in parallel:       {parallel_datacontract:8.2f} s")
print(f"  odcslint, one call per file:          {per_file_odcslint:8.2f} s")
print(f"  odcslint, one call:                   {batch_odcslint:8.3f} s")
print(f"  odcslint, whole corpus ({len(CORPUS)} files): {corpus_odcslint:8.3f} s")

if FULL:
    print(f"\n== Whole corpus ({len(CORPUS)} files)")
    full_datacontract = seconds(in_parallel, datacontract, CORPUS)
    print(f"  datacontract lint, {WORKERS} in parallel:       {full_datacontract:8.2f} s")
    print(f"  odcslint, one call:                   {corpus_odcslint:8.3f} s")
