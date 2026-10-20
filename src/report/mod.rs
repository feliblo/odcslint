mod github;
mod structured;
mod text;

use clap::ValueEnum;

use crate::diagnostic::Severity;
use crate::engine::FileReport;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable lines and a summary.
    #[default]
    Text,
    /// A single JSON document with a summary and all findings.
    Json,
    /// One JSON object per finding.
    Ndjson,
    /// Comma-separated values with a header row.
    Csv,
    /// GitHub Actions workflow commands (annotations on pull requests).
    Github,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
pub struct Summary {
    pub files: usize,
    pub errors: usize,
    pub warnings: usize,
    pub fixable: usize,
    pub fixes_applied: usize,
}

impl Summary {
    pub fn new(reports: &[FileReport]) -> Self {
        let findings = || reports.iter().flat_map(|report| &report.findings);
        let count = |severity: Severity| {
            findings()
                .filter(|finding| finding.severity == severity)
                .count()
        };
        Self {
            files: reports.len(),
            errors: count(Severity::Error),
            warnings: count(Severity::Warning),
            fixable: findings().filter(|finding| finding.fixable).count(),
            fixes_applied: reports.iter().map(|report| report.fixes_applied).sum(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TextOptions {
    pub quiet: bool,
    pub color: bool,
}

pub fn render(format: OutputFormat, reports: &[FileReport], text: TextOptions) -> String {
    let summary = Summary::new(reports);
    match format {
        OutputFormat::Text => text::render(reports, &summary, text),
        OutputFormat::Json => structured::json(reports, &summary),
        OutputFormat::Ndjson => structured::ndjson(reports),
        OutputFormat::Csv => structured::csv(reports),
        OutputFormat::Github => github::render(reports),
    }
}

#[cfg(test)]
pub mod test_support {

    use crate::diagnostic::{Finding, Location, Severity};
    use crate::engine::FileReport;

    pub fn reports() -> Vec<FileReport> {
        vec![
            FileReport {
                path: "contracts/orders.odcs.yaml".to_string(),
                findings: vec![
                    Finding {
                        rule: "schema".to_string(),
                        name: "schema".to_string(),
                        severity: Severity::Error,
                        message: "\"id\" is a required property".to_string(),
                        location: Location::START,
                        fixable: false,
                    },
                    Finding {
                        rule: "property_order".to_string(),
                        name: "column_order".to_string(),
                        severity: Severity::Warning,
                        message: "expected `order_id` at position 1 of `orders`, found `_loaded_at`, \"quoted\"".to_string(),
                        location: Location {
                            line: 12,
                            column: 9,
                        },
                        fixable: true,
                    },
                ],
                fixes_applied: 0,
            },
            FileReport {
                path: "contracts/clean.odcs.yaml".to_string(),
                findings: vec![],
                fixes_applied: 1,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_counts_findings() {
        assert_eq!(
            Summary::new(&test_support::reports()),
            Summary {
                files: 2,
                errors: 1,
                warnings: 1,
                fixable: 1,
                fixes_applied: 1
            }
        );
    }
}
