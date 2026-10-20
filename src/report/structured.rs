use serde::Serialize;

use super::Summary;
use crate::engine::FileReport;

const CSV_HEADER: &str = "path,line,column,severity,rule,name,message,fixable";

#[derive(Debug, Serialize)]
struct Row<'a> {
    path: &'a str,
    line: usize,
    column: usize,
    severity: &'a str,
    rule: &'a str,
    name: &'a str,
    message: &'a str,
    fixable: bool,
}

#[derive(Debug, Serialize)]
struct JsonOutput<'a> {
    summary: &'a Summary,
    findings: Vec<Row<'a>>,
}

fn rows(reports: &[FileReport]) -> impl Iterator<Item = Row<'_>> {
    reports.iter().flat_map(|report| {
        report.findings.iter().map(|finding| Row {
            path: &report.path,
            line: finding.location.line,
            column: finding.location.column,
            severity: finding.severity.as_str(),
            rule: &finding.rule,
            name: &finding.name,
            message: &finding.message,
            fixable: finding.fixable,
        })
    })
}

pub fn json(reports: &[FileReport], summary: &Summary) -> String {
    let output = JsonOutput {
        summary,
        findings: rows(reports).collect(),
    };
    // Serializing plain structs of strings and numbers cannot fail.
    serde_json::to_string_pretty(&output).unwrap_or_default() + "\n"
}

pub fn ndjson(reports: &[FileReport]) -> String {
    rows(reports)
        .map(|row| serde_json::to_string(&row).unwrap_or_default() + "\n")
        .collect()
}

pub fn csv(reports: &[FileReport]) -> String {
    let lines = rows(reports).map(|row| {
        [
            escape_csv(row.path),
            row.line.to_string(),
            row.column.to_string(),
            row.severity.to_string(),
            escape_csv(row.rule),
            escape_csv(row.name),
            escape_csv(row.message),
            row.fixable.to_string(),
        ]
        .join(",")
            + "\n"
    });
    std::iter::once(format!("{CSV_HEADER}\n"))
        .chain(lines)
        .collect()
}

fn escape_csv(field: &str) -> String {
    if field.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::test_support::reports;

    #[test]
    fn renders_json() {
        let reports = reports();
        insta::assert_snapshot!(json(&reports, &Summary::new(&reports)));
    }

    #[test]
    fn renders_ndjson() {
        insta::assert_snapshot!(ndjson(&reports()));
    }

    #[test]
    fn renders_csv() {
        insta::assert_snapshot!(csv(&reports()));
    }

    #[test]
    fn escapes_csv_fields() {
        assert_eq!(escape_csv("plain"), "plain");
        assert_eq!(escape_csv("a,b"), "\"a,b\"");
        assert_eq!(escape_csv("say \"hi\""), "\"say \"\"hi\"\"\"");
    }
}
