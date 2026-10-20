use owo_colors::OwoColorize;

use super::{Summary, TextOptions};
use crate::diagnostic::{Finding, Severity};
use crate::engine::FileReport;

const FIXABLE_MARKER: &str = " [*]";
const SEVERITY_WIDTH: usize = 7;

pub fn render(reports: &[FileReport], summary: &Summary, options: TextOptions) -> String {
    let files: Vec<String> = reports
        .iter()
        .filter(|report| !report.findings.is_empty())
        .map(|report| render_file(report, options.color))
        .collect();
    let body = files.join("\n");
    match (options.quiet, body.is_empty()) {
        (true, _) => body,
        (false, true) => format!("{}\n", summary_line(summary)),
        (false, false) => format!("{body}\n{}\n", summary_line(summary)),
    }
}

fn render_file(report: &FileReport, color: bool) -> String {
    let paint = |text: String, style: fn(&str) -> String| if color { style(&text) } else { text };
    let positions: Vec<String> = report
        .findings
        .iter()
        .map(|finding| format!("{}:{}", finding.location.line, finding.location.column))
        .collect();
    let position_width = positions.iter().map(String::len).max().unwrap_or(0);
    let name_width = report
        .findings
        .iter()
        .map(|finding| finding.name.len())
        .max()
        .unwrap_or(0);
    let rows = report
        .findings
        .iter()
        .zip(&positions)
        .map(|(finding, position)| {
            let severity = format!("{:SEVERITY_WIDTH$}", finding.severity.as_str());
            let severity = match finding.severity {
                Severity::Error => paint(severity, |text| text.red().bold().to_string()),
                Severity::Warning => paint(severity, |text| text.yellow().bold().to_string()),
            };
            format!(
                "  {}  {severity}  {}  {}",
                paint(format!("{position:position_width$}"), |text| text
                    .dimmed()
                    .to_string()),
                paint(format!("{:name_width$}", finding.name), |text| text
                    .cyan()
                    .to_string()),
                message(finding),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "{}\n{rows}\n",
        paint(report.path.clone(), |text| text
            .bold()
            .underline()
            .to_string())
    )
}

fn message(finding: &Finding) -> String {
    let marker = if finding.fixable { FIXABLE_MARKER } else { "" };
    format!("{}{marker}", finding.message)
}

fn plural(count: usize, singular: &str, plural: &str) -> String {
    format!("{count} {}", if count == 1 { singular } else { plural })
}

fn summary_line(summary: &Summary) -> String {
    let checked = format!("{} checked", plural(summary.files, "file", "files"));
    let result = if summary.errors + summary.warnings == 0 {
        format!("All checks passed ({checked}).")
    } else {
        format!(
            "Found {} and {} ({checked}).",
            plural(summary.errors, "error", "errors"),
            plural(summary.warnings, "warning", "warnings")
        )
    };
    let fixable = (summary.fixable > 0).then(|| {
        format!(
            " {} marked [*] can be fixed with `odcslint check --fix`.",
            plural(summary.fixable, "finding", "findings")
        )
    });
    let applied = (summary.fixes_applied > 0).then(|| {
        format!(
            " Applied {}.",
            plural(summary.fixes_applied, "fix", "fixes")
        )
    });
    format!(
        "{result}{}{}",
        fixable.unwrap_or_default(),
        applied.unwrap_or_default()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::test_support::reports;

    #[test]
    fn renders_findings_and_summary() {
        let reports = reports();
        insta::assert_snapshot!(render(
            &reports,
            &Summary::new(&reports),
            TextOptions::default()
        ));
    }

    #[test]
    fn quiet_omits_summary() {
        let reports = reports();
        let output = render(
            &reports,
            &Summary::new(&reports),
            TextOptions {
                quiet: true,
                color: false,
            },
        );
        assert!(!output.contains("Found"));
    }

    #[test]
    fn clean_run_says_all_checks_passed() {
        assert_eq!(
            summary_line(&Summary {
                files: 3,
                ..Summary::default()
            }),
            "All checks passed (3 files checked)."
        );
    }

    #[test]
    fn color_adds_ansi_codes() {
        let reports = reports();
        let output = render(
            &reports,
            &Summary::new(&reports),
            TextOptions {
                quiet: true,
                color: true,
            },
        );
        assert!(output.contains("\u{1b}["));
    }
}
