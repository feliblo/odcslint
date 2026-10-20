use std::fs;
use std::path::Path;

use odcslint::config::Config;
use odcslint::engine::{check_files, lint};
use odcslint::report::{render, OutputFormat, TextOptions};

use crate::common::{contracts, UPSTREAM_EXAMPLES, UPSTREAM_NEGATIVE};

/// Upstream examples that declare `apiVersion: v3.0.2` but use features of later versions.
/// Upstream only validates its examples against the latest schema, which hides this.
const KNOWN_VERSION_MISMATCHES: [&str; 3] = [
    "data-types/all-data-types.odcs.yaml",
    "quality/column-completeness.odcs.yaml",
    "stakeholders/basic-four-dpo.odcs.yaml",
];

fn is_known_mismatch(path: &str) -> bool {
    KNOWN_VERSION_MISMATCHES
        .iter()
        .any(|suffix| Path::new(path).ends_with(suffix))
}

fn findings_for(path: &std::path::Path) -> Vec<String> {
    let source = fs::read_to_string(path).unwrap();
    lint(&source, &Config::default(), &path.to_string_lossy())
        .unwrap()
        .into_iter()
        .map(|finding| finding.message)
        .collect()
}

#[test]
fn upstream_examples_are_valid_for_their_declared_version() {
    let failures: Vec<(String, Vec<String>)> = contracts(UPSTREAM_EXAMPLES)
        .iter()
        .map(|path| (path.to_string_lossy().into_owned(), findings_for(path)))
        .filter(|(path, findings)| !findings.is_empty() && !is_known_mismatch(path))
        .collect();
    assert!(failures.is_empty(), "unexpected findings: {failures:#?}");
}

#[test]
fn known_version_mismatches_are_still_reported() {
    let still_mismatched = contracts(UPSTREAM_EXAMPLES)
        .iter()
        .filter(|path| is_known_mismatch(&path.to_string_lossy()))
        .filter(|path| !findings_for(path).is_empty())
        .count();
    assert_eq!(still_mismatched, KNOWN_VERSION_MISMATCHES.len());
}

#[test]
fn every_upstream_negative_test_fails() {
    let passing: Vec<String> = contracts(UPSTREAM_NEGATIVE)
        .iter()
        .filter(|path| findings_for(path).is_empty())
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    assert!(
        passing.is_empty(),
        "negative tests without findings: {passing:#?}"
    );
}

#[test]
fn negative_test_findings_snapshot() {
    let reports = check_files(&contracts(UPSTREAM_NEGATIVE), &Config::default(), false).unwrap();
    let output = render(
        OutputFormat::Text,
        &reports,
        TextOptions {
            quiet: false,
            color: false,
        },
    );
    insta::assert_snapshot!(output);
}
