use std::fs;
use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;

use crate::common::{FIXTURES, VALID_CONTRACT};

fn odcslint(directory: &Path) -> Command {
    let mut command = Command::cargo_bin("odcslint").unwrap();
    command.current_dir(directory).env("NO_COLOR", "1");
    command
}

fn workspace(files: &[(&str, &str)]) -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    for (name, contents) in files {
        let path = directory.path().join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
    directory
}

#[test]
fn valid_contract_exits_zero() {
    let directory = workspace(&[("contracts/a.odcs.yaml", VALID_CONTRACT)]);
    odcslint(directory.path())
        .arg("check")
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            "All checks passed (1 file checked).",
        ));
}

#[test]
fn schema_error_exits_one_with_location() {
    let directory = workspace(&[("a.odcs.yaml", "apiVersion: v3.1.0\nkind: DataContract\n")]);
    odcslint(directory.path())
        .arg("check")
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "a.odcs.yaml\n  1:1  error    schema",
        ));
}

#[test]
fn exit_zero_ignores_errors() {
    let directory = workspace(&[("a.odcs.yaml", "apiVersion: v3.1.0\n")]);
    odcslint(directory.path())
        .args(["check", "--exit-zero"])
        .assert()
        .code(0);
}

#[test]
fn missing_path_exits_two() {
    let directory = workspace(&[]);
    odcslint(directory.path())
        .args(["check", "nope"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("path `nope` does not exist"));
}

#[test]
fn no_files_exits_two_unless_allowed() {
    let directory = workspace(&[("readme.md", "x")]);
    odcslint(directory.path()).arg("check").assert().code(2);
    odcslint(directory.path())
        .args(["check", "--allow-empty"])
        .assert()
        .code(0);
}

#[test]
fn invalid_config_exits_two() {
    let directory = workspace(&[
        ("a.odcs.yaml", VALID_CONTRACT),
        ("odcslint.yml", "rules:\n  - type: not_a_rule\n"),
    ]);
    odcslint(directory.path())
        .arg("check")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("invalid config"));
}

#[test]
fn configured_rule_reports_findings() {
    let directory = workspace(&[
        ("a.odcs.yaml", &format!("{VALID_CONTRACT}domain: hr\n")),
        (
            "odcslint.yml",
            "rules:\n  - type: allowed_values\n    field: domain\n    values: [sales]\n",
        ),
    ]);
    odcslint(directory.path())
        .arg("check")
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "allowed_values  domain `hr` is not allowed; allowed values: sales",
        ));
}

#[test]
fn fix_rewrites_file_and_exits_zero() {
    let directory = workspace(&[
        ("orders.odcs.yaml", &fs::read_to_string(format!("{FIXTURES}/fix/orders.in.yaml")).unwrap()),
        (
            "odcslint.yml",
            "settings:\n  schema_validation: off\nrules:\n  - type: property_order\n    excludes: ['never/**']\n",
        ),
    ]);
    let path = directory.path().join("orders.odcs.yaml");
    odcslint(directory.path())
        .args(["check", "--fix"])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Applied 2 fixes."));
    assert_eq!(
        fs::read_to_string(path).unwrap(),
        fs::read_to_string(format!("{FIXTURES}/fix/orders.out.yaml")).unwrap()
    );
}

#[test]
fn json_output_is_valid_json() {
    let directory = workspace(&[("a.odcs.yaml", "apiVersion: v3.1.0\n")]);
    let output = odcslint(directory.path())
        .args(["check", "--output-format", "json"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json["summary"]["errors"].as_u64().unwrap() > 0);
}

#[test]
fn github_output_emits_annotations() {
    let directory = workspace(&[("a.odcs.yaml", "apiVersion: v3.1.0\n")]);
    odcslint(directory.path())
        .args(["check", "--output-format", "github"])
        .assert()
        .stdout(predicate::str::starts_with(
            "::error file=a.odcs.yaml,line=1,col=1",
        ));
}

#[test]
fn output_file_receives_output() {
    let directory = workspace(&[("a.odcs.yaml", VALID_CONTRACT)]);
    odcslint(directory.path())
        .args([
            "check",
            "--output-format",
            "ndjson",
            "--output-file",
            "out.ndjson",
        ])
        .assert()
        .code(0)
        .stdout("");
    assert!(directory.path().join("out.ndjson").exists());
}

#[test]
fn force_exclude_skips_excluded_explicit_files() {
    let directory = workspace(&[
        ("sandbox/a.odcs.yaml", "apiVersion: v3.1.0\n"),
        ("odcslint.yml", "settings:\n  exclude: ['sandbox/**']\n"),
    ]);
    odcslint(directory.path())
        .args(["check", "sandbox/a.odcs.yaml"])
        .assert()
        .code(1);
    odcslint(directory.path())
        .args([
            "check",
            "--force-exclude",
            "--allow-empty",
            "sandbox/a.odcs.yaml",
        ])
        .assert()
        .code(0);
}

#[test]
fn glob_arguments_select_files() {
    let directory = workspace(&[
        ("domains/sales/a.odcs.yaml", VALID_CONTRACT),
        ("domains/hr/b.odcs.yaml", "apiVersion: v3.1.0\n"),
    ]);
    odcslint(directory.path())
        .args(["check", "domains/sales/**/*.yaml"])
        .assert()
        .code(0);
}

#[test]
fn hide_warnings_omits_warnings() {
    let directory = workspace(&[
        ("a.odcs.yaml", "apiVersion: v3.1.0\n"),
        ("odcslint.yml", "settings:\n  schema_validation: warning\n"),
    ]);
    odcslint(directory.path())
        .args(["check", "--hide-warnings"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("All checks passed"));
}

#[test]
fn init_writes_config_that_check_accepts() {
    let directory = workspace(&[("a.odcs.yaml", VALID_CONTRACT)]);
    odcslint(directory.path()).arg("init").assert().code(0);
    assert!(directory.path().join("odcslint.yml").exists());
    odcslint(directory.path()).arg("check").assert().code(0);
    odcslint(directory.path()).arg("init").assert().code(2);
}

#[test]
fn config_in_parent_directory_is_used_from_subdirectory() {
    let directory = workspace(&[
        (".git/HEAD", "ref: refs/heads/main\n"),
        (
            "contracts/sales/a.odcs.yaml",
            &format!("{VALID_CONTRACT}domain: hr\n"),
        ),
        (
            "odcslint.yml",
            "rules:\n  - type: allowed_values\n    field: domain\n    values: [sales]\n",
        ),
    ]);
    odcslint(&directory.path().join("contracts/sales"))
        .arg("check")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("domain `hr` is not allowed"));
}

#[test]
fn globs_in_parent_config_are_relative_to_the_config() {
    let directory = workspace(&[
        (".git/HEAD", "ref: refs/heads/main\n"),
        ("contracts/sandbox/a.odcs.yaml", "apiVersion: v3.1.0\n"),
        ("contracts/sales/b.odcs.yaml", VALID_CONTRACT),
        (
            "odcslint.yml",
            "settings:\n  exclude: ['contracts/sandbox/**']\n",
        ),
    ]);
    odcslint(&directory.path().join("contracts"))
        .arg("check")
        .assert()
        .code(0)
        .stdout(predicate::str::contains("1 file checked"));
    odcslint(&directory.path().join("contracts/sales"))
        .args([
            "check",
            "--force-exclude",
            "--allow-empty",
            "../sandbox/a.odcs.yaml",
        ])
        .assert()
        .code(0);
}

#[test]
fn output_paths_stay_relative_to_working_directory() {
    let directory = workspace(&[
        (".git/HEAD", "ref: refs/heads/main\n"),
        ("contracts/a.odcs.yaml", "apiVersion: v3.1.0\n"),
        ("odcslint.yml", ""),
    ]);
    odcslint(&directory.path().join("contracts"))
        .arg("check")
        .assert()
        .stdout(predicate::str::starts_with("a.odcs.yaml\n"));
}

#[test]
fn rules_lists_rule_types() {
    let directory = workspace(&[]);
    odcslint(directory.path())
        .arg("rules")
        .assert()
        .code(0)
        .stdout(predicate::str::contains("property_order"));
}
