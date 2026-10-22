use std::fmt::Write as _;

use strum::IntoEnumIterator;

use super::{load_and_discover, write_output, Selection, UpgradeArgs};
use crate::constants::exit_code;
use crate::engine::{upgrade_files, UpgradeReport};
use crate::error::{Error, Result};
use crate::schema::SchemaVersion;
use crate::upgrade::Status;

const VERSION_PREFIX: char = 'v';

pub fn run(args: &UpgradeArgs) -> Result<u8> {
    let target = target_version(args.to.as_deref())?;
    let (_, paths) = load_and_discover(&Selection {
        paths: &args.paths,
        config: args.config.as_ref(),
        force_exclude: args.force_exclude,
        no_ignore: args.no_ignore,
        allow_empty: args.allow_empty,
    })?;
    let reports = upgrade_files(&paths, target, !args.dry_run, args.allow_partial)?;
    write_output(None, &render(&reports, args.dry_run))?;
    let needs_work = reports
        .iter()
        .any(|report| matches!(report.outcome.status, Status::Blocked | Status::Failed));
    Ok(if needs_work {
        exit_code::FINDINGS
    } else {
        exit_code::SUCCESS
    })
}

fn target_version(requested: Option<&str>) -> Result<SchemaVersion> {
    requested.map_or(Ok(SchemaVersion::LATEST), |value| {
        let normalized = if value.starts_with(VERSION_PREFIX) {
            value.to_string()
        } else {
            format!("{VERSION_PREFIX}{value}")
        };
        SchemaVersion::from_api_version(&normalized).ok_or_else(|| Error::UnknownVersion {
            value: value.to_string(),
            supported: SchemaVersion::iter()
                .map(SchemaVersion::api_version)
                .collect::<Vec<_>>()
                .join(", "),
        })
    })
}

fn render(reports: &[UpgradeReport], dry_run: bool) -> String {
    let mut output = String::new();
    for report in reports {
        let outcome = &report.outcome;
        let from = outcome.from.as_deref().unwrap_or("unknown version");
        let to = outcome.to.api_version();
        let changes = if outcome.applied.is_empty() {
            String::new()
        } else {
            format!(" ({})", outcome.applied.join(", "))
        };
        let line = match (outcome.status, report.written, dry_run) {
            (Status::UpToDate, ..) => format!("{from}  up to date"),
            (Status::Upgraded, _, true) => format!("{from} → {to}  would be upgraded{changes}"),
            (Status::Upgraded, ..) => format!("{from} → {to}  upgraded{changes}"),
            (Status::Blocked, true, _) => {
                format!("{from} → {to}  partly upgraded{changes}, needs manual changes:")
            }
            (Status::Blocked, false, _) if outcome.source.is_none() => {
                format!("{from}  not upgraded:")
            }
            (Status::Blocked, false, _) => {
                format!("{from} → {to}  not upgraded, needs manual changes:")
            }
            (Status::Failed, ..) => format!("{from} → {to}  not upgraded:"),
        };
        let _ = writeln!(output, "{}  {line}", report.path);
        for blocker in &outcome.blockers {
            let _ = writeln!(
                output,
                "  {}:{}  {}",
                blocker.location.line, blocker.location.column, blocker.message
            );
        }
    }
    let count = |status: Status| {
        reports
            .iter()
            .filter(|report| report.outcome.status == status)
            .count()
    };
    let upgraded = count(Status::Upgraded);
    let blocked = count(Status::Blocked) + count(Status::Failed);
    let up_to_date = count(Status::UpToDate);
    let files = if reports.len() == 1 { "file" } else { "files" };
    let _ = writeln!(
        output,
        "\n{upgraded} upgraded, {blocked} need manual changes, {up_to_date} up to date ({} {files}).",
        reports.len()
    );
    let unwritten_blocked = reports
        .iter()
        .any(|report| report.outcome.status == Status::Blocked && !report.written);
    if dry_run {
        output.push_str("Dry run: no files were written.\n");
    } else if unwritten_blocked {
        output.push_str(
            "Files that need manual changes were left untouched; line numbers refer to the upgraded version.\n\
             Run with --allow-partial to write the automatic changes and finish the rest by hand.\n",
        );
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_defaults_to_latest() {
        assert_eq!(target_version(None).unwrap(), SchemaVersion::LATEST);
    }

    #[test]
    fn target_accepts_versions_with_or_without_prefix() {
        assert_eq!(
            target_version(Some("v3.1.0")).unwrap(),
            SchemaVersion::V3_1_0
        );
        assert_eq!(
            target_version(Some("3.1.0")).unwrap(),
            SchemaVersion::V3_1_0
        );
    }

    #[test]
    fn unknown_target_is_an_error() {
        let error = target_version(Some("v4.0.0")).unwrap_err();
        assert!(error
            .to_string()
            .starts_with("unknown ODCS version `v4.0.0`"));
    }
}
