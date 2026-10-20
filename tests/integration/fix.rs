use std::fs;

use odcslint::constants::builtin;
use odcslint::engine::check_files;

use crate::common::{config, contracts, FIXTURES, UPSTREAM_EXAMPLES};

const PROPERTY_ORDER: &str =
    "settings:\n  schema_validation: off\nrules:\n  - type: property_order\n";

#[test]
fn golden_orders_contract() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("orders.odcs.yaml");
    fs::copy(format!("{FIXTURES}/fix/orders.in.yaml"), &path).unwrap();

    let reports = check_files(&[&path], &config(PROPERTY_ORDER), true).unwrap();

    let expected = fs::read_to_string(format!("{FIXTURES}/fix/orders.out.yaml")).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), expected);
    assert_eq!(reports[0].fixes_applied, 2);
    let remaining: Vec<bool> = reports[0]
        .findings
        .iter()
        .map(|finding| finding.fixable)
        .collect();
    assert_eq!(
        remaining,
        vec![false],
        "only the flow-style list remains, unfixable"
    );
}

#[test]
fn golden_output_is_stable() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("orders.odcs.yaml");
    fs::copy(format!("{FIXTURES}/fix/orders.out.yaml"), &path).unwrap();

    let reports = check_files(&[&path], &config(PROPERTY_ORDER), true).unwrap();

    assert_eq!(reports[0].fixes_applied, 0);
    let expected = fs::read_to_string(format!("{FIXTURES}/fix/orders.out.yaml")).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), expected);
}

#[test]
fn fixing_upstream_examples_is_safe_and_idempotent() {
    let config = config(PROPERTY_ORDER);
    let directory = tempfile::tempdir().unwrap();
    let copies: Vec<_> = contracts(UPSTREAM_EXAMPLES)
        .iter()
        .enumerate()
        .map(|(index, path)| {
            let copy = directory.path().join(format!("{index}.odcs.yaml"));
            fs::copy(path, &copy).unwrap();
            copy
        })
        .collect();

    let first = check_files(&copies, &config, true).unwrap();
    let failed: Vec<&str> = first
        .iter()
        .filter(|report| {
            report
                .findings
                .iter()
                .any(|finding| finding.rule == builtin::FIX_FAILED)
        })
        .map(|report| report.path.as_str())
        .collect();
    assert!(
        failed.is_empty(),
        "fixes rejected by the content check: {failed:?}"
    );
    assert!(
        first.iter().any(|report| report.fixes_applied > 0),
        "corpus should need fixes"
    );

    let second = check_files(&copies, &config, true).unwrap();
    assert!(second.iter().all(|report| report.fixes_applied == 0));
}

#[test]
fn fixed_upstream_examples_stay_schema_valid() {
    let directory = tempfile::tempdir().unwrap();
    let copies: Vec<_> = contracts(UPSTREAM_EXAMPLES)
        .iter()
        .enumerate()
        .map(|(index, path)| {
            let copy = directory.path().join(format!("{index}.odcs.yaml"));
            fs::copy(path, &copy).unwrap();
            copy
        })
        .collect();
    let schema_only = config("rules: []\n");
    let errors_before: usize = check_files(&copies, &schema_only, false)
        .unwrap()
        .iter()
        .map(|report| report.findings.len())
        .sum();

    check_files(&copies, &config("rules:\n  - type: property_order\n"), true).unwrap();

    let errors_after: usize = check_files(&copies, &schema_only, false)
        .unwrap()
        .iter()
        .map(|report| report.findings.len())
        .sum();
    assert_eq!(errors_before, errors_after);
}
