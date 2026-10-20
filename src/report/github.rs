use crate::diagnostic::Severity;
use crate::engine::FileReport;

const TITLE_PREFIX: &str = "odcslint";

pub fn render(reports: &[FileReport]) -> String {
    reports
        .iter()
        .flat_map(|report| {
            report.findings.iter().map(|finding| {
                let command = match finding.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                };
                format!(
                    "::{command} file={},line={},col={},title={}::{}\n",
                    escape_property(&report.path),
                    finding.location.line,
                    finding.location.column,
                    escape_property(&format!("{TITLE_PREFIX} ({})", finding.name)),
                    escape_data(&finding.message),
                )
            })
        })
        .collect()
}

fn escape_data(text: &str) -> String {
    text.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

fn escape_property(text: &str) -> String {
    escape_data(text).replace(':', "%3A").replace(',', "%2C")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::test_support::reports;

    #[test]
    fn renders_annotations() {
        insta::assert_snapshot!(render(&reports()));
    }

    #[test]
    fn escapes_workflow_command_values() {
        assert_eq!(escape_data("50%\nnext"), "50%25%0Anext");
        assert_eq!(escape_property("a:b,c"), "a%3Ab%2Cc");
    }
}
