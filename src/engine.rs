use std::fs;
use std::path::Path;

use rayon::prelude::*;
use strum::IntoEnumIterator;

use crate::config::{normalize_path, Config};
use crate::constants::{builtin, keys, MAX_FIX_PASSES};
use crate::diagnostic::{Finding, Location, Severity};
use crate::document::node::{as_str, get};
use crate::document::to_json::to_json;
use crate::document::{Document, SyntaxError};
use crate::error::{Error, Result};
use crate::fix::{self, Permutation};
use crate::rules::{self, Context};
use crate::schema::{self, SchemaSelection, SchemaVersion};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileReport {
    pub path: String,
    pub findings: Vec<Finding>,
    pub fixes_applied: usize,
}

pub fn check_files(
    paths: &[impl AsRef<Path> + Sync],
    config: &Config,
    fix: bool,
) -> Result<Vec<FileReport>> {
    let mut reports = paths
        .par_iter()
        .map(|path| check_file(path.as_ref(), config, fix))
        .collect::<Result<Vec<_>>>()?;
    reports.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(reports)
}

fn check_file(path: &Path, config: &Config, fix: bool) -> Result<FileReport> {
    let display = normalize_path(path);
    let original = fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let outcome = if fix {
        fix_source(&original, config, &display)
    } else {
        FixOutcome::default()
    };
    if let Some(fixed) = &outcome.source {
        write_atomically(path, fixed)?;
    }
    let source = outcome.source.as_deref().unwrap_or(&original);
    let mut findings = lint(source, config, &display)?;
    findings.extend(outcome.failure);
    findings.sort_by(|left, right| {
        (left.location, &left.name, &left.message).cmp(&(
            right.location,
            &right.name,
            &right.message,
        ))
    });
    Ok(FileReport {
        path: display,
        findings,
        fixes_applied: outcome.applied,
    })
}

pub fn lint(source: &str, config: &Config, path: &str) -> Result<Vec<Finding>> {
    let document = match Document::parse(source) {
        Ok(document) => document,
        Err(error) => return Ok(vec![syntax_finding(error)]),
    };
    let schema_findings = schema_findings(&document, config)?;
    let context = Context::new(&document, path);
    let rule_findings = rules::run(&config.rules, &context)
        .into_iter()
        .map(|(finding, _)| finding);
    Ok(schema_findings.into_iter().chain(rule_findings).collect())
}

fn syntax_finding(error: SyntaxError) -> Finding {
    Finding {
        rule: builtin::YAML_SYNTAX.to_string(),
        name: builtin::YAML_SYNTAX.to_string(),
        severity: Severity::Error,
        message: error.message,
        location: error.location,
        fixable: false,
    }
}

fn schema_findings(document: &Document<'_>, config: &Config) -> Result<Vec<Finding>> {
    let Some(severity) = config.settings.schema_validation.severity() else {
        return Ok(Vec::new());
    };
    let api_version = get(document.root(), keys::API_VERSION);
    match schema::select(api_version.and_then(as_str)) {
        SchemaSelection::Supported(version) => {
            schema::validate(document, &to_json(document.root()), version, severity)
        }
        SchemaSelection::Unsupported(value) => {
            let supported: Vec<&str> = SchemaVersion::iter()
                .map(SchemaVersion::api_version)
                .collect();
            Ok(vec![Finding {
                rule: builtin::UNSUPPORTED_API_VERSION.to_string(),
                name: builtin::UNSUPPORTED_API_VERSION.to_string(),
                severity,
                message: format!(
                    "apiVersion `{value}` is not supported; supported versions: {}",
                    supported.join(", ")
                ),
                location: api_version.map_or(Location::START, |node| document.location(node)),
                fixable: false,
            }])
        }
    }
}

#[derive(Debug, Default)]
struct FixOutcome {
    source: Option<String>,
    applied: usize,
    failure: Option<Finding>,
}

enum Step {
    Done,
    Applied { source: String, count: usize },
    Rejected,
}

fn fix_source(original: &str, config: &Config, path: &str) -> FixOutcome {
    let mut current: Option<String> = None;
    let mut applied = 0;
    for _ in 0..MAX_FIX_PASSES {
        match fix_step(current.as_deref().unwrap_or(original), config, path) {
            Step::Done => break,
            Step::Applied { source, count } => {
                applied += count;
                current = Some(source);
            }
            Step::Rejected => {
                return FixOutcome {
                    source: current,
                    applied,
                    failure: Some(fix_failed_finding()),
                }
            }
        }
    }
    FixOutcome {
        source: current,
        applied,
        failure: None,
    }
}

fn fix_step(source: &str, config: &Config, path: &str) -> Step {
    let Ok(document) = Document::parse(source) else {
        return Step::Done;
    };
    let fixes = fix::non_overlapping(
        rules::run(&config.rules, &Context::new(&document, path))
            .into_iter()
            .filter_map(|(_, fix)| fix)
            .collect(),
    );
    if fixes.is_empty() {
        return Step::Done;
    }
    let next = fix::apply(source, &fixes);
    let permutations: Vec<&Permutation> = fixes.iter().map(|fix| &fix.permutation).collect();
    let expected = fix::permuted(&to_json(document.root()), &permutations);
    let actual = Document::parse(&next)
        .ok()
        .map(|fixed| to_json(fixed.root()));
    if expected.is_some() && expected == actual {
        Step::Applied {
            source: next,
            count: fixes.len(),
        }
    } else {
        Step::Rejected
    }
}

fn fix_failed_finding() -> Finding {
    Finding {
        rule: builtin::FIX_FAILED.to_string(),
        name: builtin::FIX_FAILED.to_string(),
        severity: Severity::Warning,
        message: "an autofix was discarded because it would have changed the contract's content; \
                  please report this"
            .to_string(),
        location: Location::START,
        fixable: false,
    }
}

fn write_atomically(path: &Path, contents: &str) -> Result<()> {
    let file_name = path
        .file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
    let temporary = path.with_file_name(format!(".{file_name}.odcslint-tmp"));
    let result = fs::metadata(path)
        .and_then(|metadata| {
            fs::write(&temporary, contents)?;
            fs::set_permissions(&temporary, metadata.permissions())?;
            fs::rename(&temporary, path)
        })
        .map_err(|source| Error::Write {
            path: path.to_path_buf(),
            source,
        });
    if result.is_err() && temporary.exists() {
        // Best effort cleanup; the write error itself is returned to the caller.
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(yaml: &str) -> Config {
        let document = Document::parse(yaml).unwrap();
        serde_json::from_value(to_json(document.root())).unwrap()
    }

    const VALID: &str = "apiVersion: v3.1.0\nkind: DataContract\nid: 53581432-6c55-4ba2-a65f-72344a91553c\nversion: 1.0.0\nstatus: active\n";

    #[test]
    fn valid_contract_has_no_findings() {
        assert!(lint(VALID, &Config::default(), "c.odcs.yaml")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn syntax_errors_become_findings() {
        let findings = lint("a: [\n", &Config::default(), "c.odcs.yaml").unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule, builtin::YAML_SYNTAX);
    }

    #[test]
    fn unsupported_api_version_is_reported_at_its_value() {
        let findings = lint("apiVersion: v2.2.2\n", &Config::default(), "c.odcs.yaml").unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule, builtin::UNSUPPORTED_API_VERSION);
        assert_eq!(
            findings[0].location,
            Location {
                line: 1,
                column: 13
            }
        );
    }

    #[test]
    fn schema_validation_can_be_downgraded() {
        let config = config("settings:\n  schema_validation: warning\n");
        let findings = lint("apiVersion: v3.2.0\n", &config, "c.odcs.yaml").unwrap();
        assert!(findings
            .iter()
            .all(|finding| finding.severity == Severity::Warning));
        assert!(!findings.is_empty());
    }

    #[test]
    fn schema_validation_can_be_disabled() {
        let config = config("settings:\n  schema_validation: off\n");
        assert!(lint("apiVersion: v3.2.0\n", &config, "c.odcs.yaml")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn rules_use_display_name_and_severity() {
        let config =
            config("rules:\n  - type: id_format\n    name: uuid_ids\n    severity: warning\n");
        let source = VALID.replace("53581432-6c55-4ba2-a65f-72344a91553c", "orders");
        let findings = lint(&source, &config, "c.odcs.yaml").unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].name, "uuid_ids");
        assert_eq!(findings[0].rule, "id_format");
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn rule_excludes_skip_files() {
        let config = config("rules:\n  - type: id_format\n    excludes: ['legacy/**']\n");
        let source = VALID.replace("53581432-6c55-4ba2-a65f-72344a91553c", "orders");
        assert!(lint(&source, &config, "legacy/c.odcs.yaml")
            .unwrap()
            .is_empty());
    }

    const UNORDERED: &str = "\
schema:
  - name: orders
    properties:
      - name: _loaded_at # metadata
      - name: amount
      - name: id
        primaryKey: true
        primaryKeyPosition: 1
        properties:
          - name: _x
          - name: y
";

    #[test]
    fn fix_reorders_nested_lists_over_multiple_passes() {
        let config =
            config("settings:\n  schema_validation: off\nrules:\n  - type: property_order\n");
        let outcome = fix_source(UNORDERED, &config, "c.odcs.yaml");
        assert_eq!(outcome.applied, 2);
        assert!(outcome.failure.is_none());
        assert_eq!(
            outcome.source.unwrap(),
            "\
schema:
  - name: orders
    properties:
      - name: id
        primaryKey: true
        primaryKeyPosition: 1
        properties:
          - name: y
          - name: _x
      - name: amount
      - name: _loaded_at # metadata
"
        );
    }

    #[test]
    fn fix_is_idempotent() {
        let config = config("rules:\n  - type: property_order\n");
        let fixed = fix_source(UNORDERED, &config, "c.odcs.yaml")
            .source
            .unwrap();
        let again = fix_source(&fixed, &config, "c.odcs.yaml");
        assert!(again.source.is_none());
        assert_eq!(again.applied, 0);
    }

    #[test]
    fn nothing_to_fix_leaves_source_untouched() {
        let config = config("rules:\n  - type: property_order\n");
        assert!(fix_source(VALID, &config, "c.odcs.yaml").source.is_none());
    }

    #[test]
    fn check_files_writes_fixes_and_reports_remaining_findings() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("c.odcs.yaml");
        fs::write(&path, UNORDERED).unwrap();
        let config =
            config("settings:\n  schema_validation: off\nrules:\n  - type: property_order\n");
        let reports = check_files(&[&path], &config, true).unwrap();
        assert_eq!(reports[0].fixes_applied, 2);
        assert!(reports[0].findings.is_empty());
        assert!(fs::read_to_string(&path)
            .unwrap()
            .contains("      - name: id\n"));
        assert!(!directory.path().join(".c.odcs.yaml.odcslint-tmp").exists());
    }

    #[test]
    fn check_without_fix_does_not_write() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("c.odcs.yaml");
        fs::write(&path, UNORDERED).unwrap();
        let config =
            config("settings:\n  schema_validation: off\nrules:\n  - type: property_order\n");
        let reports = check_files(&[&path], &config, false).unwrap();
        assert_eq!(reports[0].findings.len(), 2);
        assert!(reports[0].findings.iter().all(|finding| finding.fixable));
        assert_eq!(fs::read_to_string(&path).unwrap(), UNORDERED);
    }

    #[test]
    fn unreadable_file_is_an_error() {
        let error =
            check_files(&[Path::new("missing.odcs.yaml")], &Config::default(), false).unwrap_err();
        assert!(matches!(error, Error::Read { .. }));
    }
}
