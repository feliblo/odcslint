use std::fs;

use assert_cmd::Command;
use odcslint::engine::upgrade_files;
use odcslint::schema::SchemaVersion;
use odcslint::upgrade::{upgrade, Status};
use predicates::prelude::*;

use crate::common::{contracts, FIXTURES, UPSTREAM_EXAMPLES};

const QUALITY_NEEDING_REVIEW: &str = "\
apiVersion: v3.0.2
kind: DataContract
id: 53581432-6c55-4ba2-a65f-72344a91553c
version: 1.0.0
status: active
schema:
  - name: orders
    quality:
      - rule: validValues
        mustBe: 0
";

fn odcslint(directory: &std::path::Path) -> Command {
    let mut command = Command::cargo_bin("odcslint").unwrap();
    command.current_dir(directory).env("NO_COLOR", "1");
    command
}

#[test]
fn golden_orders_contract_is_upgraded_with_comments_kept() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("orders.odcs.yaml");
    fs::copy(format!("{FIXTURES}/upgrade/orders.in.yaml"), &path).unwrap();

    let reports = upgrade_files(&[&path], SchemaVersion::LATEST, true, false).unwrap();

    assert_eq!(reports[0].outcome.status, Status::Upgraded);
    assert!(reports[0].written);
    let expected = fs::read_to_string(format!("{FIXTURES}/upgrade/orders.out.yaml")).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), expected);
}

#[test]
fn golden_output_is_up_to_date() {
    let source = fs::read_to_string(format!("{FIXTURES}/upgrade/orders.out.yaml")).unwrap();
    assert_eq!(
        upgrade(&source, SchemaVersion::LATEST).unwrap().status,
        Status::UpToDate
    );
}

#[test]
fn every_older_upstream_example_upgrades_cleanly_and_once() {
    let failures: Vec<String> = contracts(UPSTREAM_EXAMPLES)
        .iter()
        .filter_map(|path| {
            let source = fs::read_to_string(path).unwrap();
            let outcome = upgrade(&source, SchemaVersion::LATEST).unwrap();
            match outcome.status {
                Status::UpToDate => None,
                Status::Upgraded => {
                    let again = upgrade(&outcome.source.unwrap(), SchemaVersion::LATEST).unwrap();
                    (again.status != Status::UpToDate)
                        .then(|| format!("{}: not up to date after upgrading", path.display()))
                }
                status => Some(format!(
                    "{}: {status:?} {:?}",
                    path.display(),
                    outcome.blockers
                )),
            }
        })
        .collect();
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn dry_run_writes_nothing() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("orders.odcs.yaml");
    fs::copy(format!("{FIXTURES}/upgrade/orders.in.yaml"), &path).unwrap();
    odcslint(directory.path())
        .args(["upgrade", "--dry-run"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("would be upgraded"))
        .stdout(predicate::str::contains("Dry run: no files were written."));
    let original = fs::read_to_string(format!("{FIXTURES}/upgrade/orders.in.yaml")).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
}

#[test]
fn files_needing_manual_changes_are_left_untouched_by_default() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("orders.odcs.yaml");
    fs::write(&path, QUALITY_NEEDING_REVIEW).unwrap();
    odcslint(directory.path())
        .arg("upgrade")
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "not upgraded, needs manual changes",
        ))
        .stdout(predicate::str::contains("--allow-partial"));
    assert_eq!(fs::read_to_string(&path).unwrap(), QUALITY_NEEDING_REVIEW);
}

#[test]
fn allow_partial_writes_the_automatic_changes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("orders.odcs.yaml");
    fs::write(&path, QUALITY_NEEDING_REVIEW).unwrap();
    odcslint(directory.path())
        .args(["upgrade", "--allow-partial"])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("partly upgraded"));
    assert!(fs::read_to_string(&path)
        .unwrap()
        .starts_with("apiVersion: v3.2.0\n"));
}

#[test]
fn upgrades_to_a_chosen_version() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("orders.odcs.yaml");
    fs::copy(format!("{FIXTURES}/upgrade/orders.in.yaml"), &path).unwrap();
    odcslint(directory.path())
        .args(["upgrade", "--to", "v3.1.0"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("v3.0.2 → v3.1.0  upgraded"));
    assert!(fs::read_to_string(&path)
        .unwrap()
        .contains("apiVersion: v3.1.0\n"));
}

#[test]
fn unknown_target_version_is_a_tool_error() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("orders.odcs.yaml"),
        QUALITY_NEEDING_REVIEW,
    )
    .unwrap();
    odcslint(directory.path())
        .args(["upgrade", "--to", "v9.9.9"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("unknown ODCS version `v9.9.9`"));
}
