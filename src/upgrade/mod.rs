//! Moves a contract to a newer ODCS version.
//!
//! Every step between the declared `apiVersion` and the target runs its migrations and bumps
//! `apiVersion`; the result is then validated against the target's JSON Schema, and whatever still
//! fails is reported for the user to fix.

pub mod migrations;
pub mod ops;

use serde_json::{json, Value};

use crate::constants::{builtin, keys, MAX_FIX_PASSES};
use crate::diagnostic::{Finding, Location, Severity};
use crate::document::node::{as_str, get};
use crate::document::pointer::resolve;
use crate::document::to_json::to_json;
use crate::document::Document;
use crate::error::Result;
use crate::fix::apply_edits;
use crate::schema::{self, SchemaVersion};
use migrations::Migration;
use ops::{child, Op};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Already at (or past) the target version.
    UpToDate,
    /// Converted and valid against the target schema.
    Upgraded,
    /// Converted as far as possible; the blockers need manual changes.
    Blocked,
    /// A conversion didn't produce the expected content, so nothing may be written.
    Failed,
}

#[derive(Debug, Clone)]
pub struct UpgradeOutcome {
    pub from: Option<String>,
    pub to: SchemaVersion,
    pub status: Status,
    /// The upgraded text, when anything changed and it is safe to write.
    pub source: Option<String>,
    pub applied: Vec<&'static str>,
    /// Locations refer to the upgraded text.
    pub blockers: Vec<Finding>,
}

const API_VERSION_STEP: &str = "api_version";

pub fn upgrade(source: &str, target: SchemaVersion) -> Result<UpgradeOutcome> {
    let outcome = |from: Option<String>, status, blockers| UpgradeOutcome {
        from,
        to: target,
        status,
        source: None,
        applied: Vec::new(),
        blockers,
    };
    let document = match Document::parse(source) {
        Ok(document) => document,
        Err(error) => {
            return Ok(outcome(
                None,
                Status::Blocked,
                vec![blocker(error.message, error.location)],
            ));
        }
    };
    let declared = get(document.root(), keys::API_VERSION).and_then(as_str);
    let Some(from) = declared.and_then(SchemaVersion::from_api_version) else {
        let message = declared.map_or_else(
            || {
                "the contract has no `apiVersion`, so its version is unknown; add it first"
                    .to_string()
            },
            |value| format!("apiVersion `{value}` can't be upgraded automatically"),
        );
        return Ok(outcome(
            declared.map(str::to_string),
            Status::Blocked,
            vec![blocker(message, Location::START)],
        ));
    };
    if from >= target {
        return Ok(outcome(
            Some(from.api_version().to_string()),
            Status::UpToDate,
            Vec::new(),
        ));
    }

    let mut state = State {
        source: source.to_string(),
        applied: Vec::new(),
        blockers: Vec::new(),
    };
    for step in SchemaVersion::steps(from, target) {
        for migration in migrations::for_step(step) {
            if !state.run(migration.name, migration.plan)? {
                return Ok(UpgradeOutcome {
                    blockers: vec![failed_finding(migration)],
                    ..outcome(
                        Some(from.api_version().to_string()),
                        Status::Failed,
                        Vec::new(),
                    )
                });
            }
        }
        if !state.run(API_VERSION_STEP, bump_to(step))? {
            return Ok(outcome(
                Some(from.api_version().to_string()),
                Status::Failed,
                Vec::new(),
            ));
        }
    }

    let schema_findings = {
        let upgraded = Document::parse(&state.source).map_err(|_| crate::error::Error::Schema {
            version: target.api_version(),
            message: "upgraded contract no longer parses".to_string(),
        })?;
        schema::validate(
            &upgraded,
            &to_json(upgraded.root()),
            target,
            Severity::Error,
        )?
    };
    let blockers: Vec<Finding> = state.blockers.into_iter().chain(schema_findings).collect();
    Ok(UpgradeOutcome {
        from: Some(from.api_version().to_string()),
        to: target,
        status: if blockers.is_empty() {
            Status::Upgraded
        } else {
            Status::Blocked
        },
        source: Some(state.source),
        applied: state.applied,
        blockers,
    })
}

fn bump_to(version: SchemaVersion) -> impl Fn(&Value) -> Result<Vec<Op>> {
    move |json: &Value| {
        let current = json.get(keys::API_VERSION).and_then(Value::as_str);
        Ok((current != Some(version.api_version()))
            .then(|| Op::Set {
                pointer: child("", keys::API_VERSION),
                value: json!(version.api_version()),
            })
            .into_iter()
            .collect())
    }
}

struct State {
    source: String,
    applied: Vec<&'static str>,
    blockers: Vec<Finding>,
}

/// One pass of a migration over `source`.
enum Pass {
    /// Nothing (left) to do.
    Done,
    /// Operations applied; every edit matched the migration's specification.
    Applied(String),
    /// The edited text doesn't match the specification.
    Mismatch,
}

impl State {
    /// Runs one migration until it has nothing left to do. Returns `false` when an edit didn't
    /// produce the content the migration specified.
    fn run(
        &mut self,
        name: &'static str,
        plan: impl Fn(&Value) -> Result<Vec<Op>>,
    ) -> Result<bool> {
        let mut skipped: Vec<Op> = Vec::new();
        for _ in 0..MAX_FIX_PASSES {
            let (pass, manual) = pass(&self.source, name, &plan, &skipped)?;
            self.blockers
                .extend(manual.iter().map(|(_, finding)| finding.clone()));
            let made_progress = !manual.is_empty();
            skipped.extend(manual.into_iter().map(|(op, _)| op));
            match pass {
                Pass::Done if made_progress => {}
                Pass::Done => break,
                Pass::Mismatch => return Ok(false),
                Pass::Applied(next) => {
                    self.source = next;
                    if name != API_VERSION_STEP && !self.applied.contains(&name) {
                        self.applied.push(name);
                    }
                }
            }
        }
        Ok(true)
    }
}

/// Applies the migration's planned operations one at a time. Each edit is checked on its own: the
/// re-parsed text must equal the JSON with just that operation applied.
fn pass(
    source: &str,
    name: &'static str,
    plan: &impl Fn(&Value) -> Result<Vec<Op>>,
    skipped: &[Op],
) -> Result<(Pass, Vec<(Op, Finding)>)> {
    let Ok(document) = Document::parse(source) else {
        return Ok((Pass::Mismatch, Vec::new()));
    };
    let ops: Vec<Op> = plan(&to_json(document.root()))?
        .into_iter()
        .filter(|op| !skipped.contains(op))
        .collect();
    let mut current = source.to_string();
    let mut manual = Vec::new();
    let mut changed = false;
    for op in ops {
        match apply_op(&current, &op) {
            Applied::Text(next) => {
                current = next;
                changed = true;
            }
            Applied::Manual(location) => {
                let message = format!(
                    "{name}: couldn't {} automatically; please do it by hand",
                    op.describe()
                );
                manual.push((op, blocker(message, location)));
            }
            Applied::Mismatch => return Ok((Pass::Mismatch, manual)),
        }
    }
    Ok((
        if changed {
            Pass::Applied(current)
        } else {
            Pass::Done
        },
        manual,
    ))
}

enum Applied {
    Text(String),
    Manual(Location),
    Mismatch,
}

fn apply_op(source: &str, op: &Op) -> Applied {
    let Ok(document) = Document::parse(source) else {
        return Applied::Mismatch;
    };
    let Some(edit) = op.edit(&document) else {
        return Applied::Manual(document.location(resolve(document.root(), op.pointer()).anchor()));
    };
    let mut expected = to_json(document.root());
    if op.apply(&mut expected).is_none() {
        return Applied::Mismatch;
    }
    let next = apply_edits(source, [&edit]);
    let actual = Document::parse(&next)
        .ok()
        .map(|fixed| to_json(fixed.root()));
    if actual.as_ref() == Some(&expected) {
        Applied::Text(next)
    } else {
        Applied::Mismatch
    }
}

fn blocker(message: String, location: Location) -> Finding {
    Finding {
        rule: builtin::UPGRADE.to_string(),
        name: builtin::UPGRADE.to_string(),
        severity: Severity::Error,
        message,
        location,
        fixable: false,
    }
}

fn failed_finding(migration: &Migration) -> Finding {
    blocker(
        format!(
            "{}: the conversion didn't produce the expected content, so the file was left untouched; please report this",
            migration.name
        ),
        Location::START,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const V3_0_2: &str = "\
apiVersion: v3.0.2
kind: DataContract
id: 53581432-6c55-4ba2-a65f-72344a91553c
version: 1.0.0
status: active
";

    #[test]
    fn bumps_an_additive_step() {
        let source = V3_0_2.replace("v3.0.2", "v3.1.0");
        let outcome = upgrade(&source, SchemaVersion::V3_2_0).unwrap();
        assert_eq!(outcome.status, Status::Upgraded);
        assert_eq!(outcome.source.unwrap(), source.replace("v3.1.0", "v3.2.0"));
        assert_eq!(outcome.applied, Vec::<&str>::new());
    }

    #[test]
    fn up_to_date_contracts_are_untouched() {
        let outcome = upgrade(&V3_0_2.replace("v3.0.2", "v3.2.0"), SchemaVersion::V3_2_0).unwrap();
        assert_eq!(outcome.status, Status::UpToDate);
        assert!(outcome.source.is_none());
    }

    #[test]
    fn converts_breaking_changes_and_keeps_comments() {
        let source = format!(
            "{V3_0_2}team:\n  # data owners\n  - username: alice\n    role: owner\nschema:\n  - name: orders\n    properties:\n      - name: amount\n        logicalType: number\n        logicalTypeOptions:\n          maximum: 100\n          exclusiveMaximum: true\n"
        );
        let outcome = upgrade(&source, SchemaVersion::V3_2_0).unwrap();
        assert_eq!(outcome.status, Status::Upgraded, "{:?}", outcome.blockers);
        assert_eq!(outcome.applied, vec!["team_members", "exclusive_bounds"]);
        assert_eq!(
            outcome.source.unwrap(),
            "apiVersion: v3.2.0\nkind: DataContract\nid: 53581432-6c55-4ba2-a65f-72344a91553c\nversion: 1.0.0\nstatus: active\nteam:\n  members:\n    # data owners\n    - username: alice\n      role: owner\nschema:\n  - name: orders\n    properties:\n      - name: amount\n        logicalType: number\n        logicalTypeOptions:\n          exclusiveMaximum: 100\n"
        );
    }

    #[test]
    fn reports_what_needs_manual_work() {
        let source = format!("{V3_0_2}schema:\n  - name: t\n    quality:\n      - rule: validValues\n        mustBe: 0\n");
        let outcome = upgrade(&source, SchemaVersion::V3_2_0).unwrap();
        assert_eq!(outcome.status, Status::Blocked);
        assert!(outcome.source.unwrap().starts_with("apiVersion: v3.2.0"));
        assert!(!outcome.blockers.is_empty(), "{:?}", outcome.blockers);
    }

    #[test]
    fn converts_every_key_of_a_check_that_starts_an_item() {
        let source = format!(
            "{V3_0_2}schema:\n  - name: t\n    quality:\n      - rule: rowCount\n        mustBeGreaterOrEqualTo: 1\n        mustBeLessOrEqualTo: 10\n"
        );
        let outcome = upgrade(&source, SchemaVersion::V3_2_0).unwrap();
        assert_eq!(outcome.status, Status::Upgraded, "{:?}", outcome.blockers);
        assert!(outcome
            .source
            .unwrap()
            .ends_with("    quality:\n      - metric: rowCount\n        mustBeBetween: [1, 10]\n"));
    }

    #[test]
    fn flow_style_is_reported_instead_of_converted() {
        let source = format!("{V3_0_2}team: [{{username: alice}}]\n");
        let outcome = upgrade(&source, SchemaVersion::V3_2_0).unwrap();
        assert_eq!(outcome.status, Status::Blocked);
        assert!(outcome.blockers[0]
            .message
            .starts_with("team_members: couldn't move `/team` under `members`"));
    }

    #[test]
    fn missing_api_version_is_blocked() {
        let outcome = upgrade("kind: DataContract\n", SchemaVersion::V3_2_0).unwrap();
        assert_eq!(outcome.status, Status::Blocked);
        assert!(outcome.source.is_none());
    }

    #[test]
    fn upgrading_twice_changes_nothing() {
        let source = format!("{V3_0_2}team:\n  - username: alice\n");
        let once = upgrade(&source, SchemaVersion::V3_2_0)
            .unwrap()
            .source
            .unwrap();
        assert_eq!(
            upgrade(&once, SchemaVersion::V3_2_0).unwrap().status,
            Status::UpToDate
        );
    }
}
