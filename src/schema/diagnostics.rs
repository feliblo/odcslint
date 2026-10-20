use std::collections::BTreeSet;

use jsonschema::error::{TypeKind, ValidationErrorKind};
use jsonschema::ValidationError;
use serde_json::Value;

use crate::constants::builtin;
use crate::diagnostic::{Finding, Severity};
use crate::document::node::entry;
use crate::document::pointer::resolve;
use crate::document::Document;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Issue {
    pointer: String,
    message: String,
    key: Option<String>,
    unevaluated: bool,
}

/// `unevaluatedProperties` errors are dropped when another error exists at the same object or
/// below it: a failing `if`/`then` or `$ref` branch discards its annotations, which makes every
/// key it covers look unevaluated.
pub fn findings<'e>(
    document: &Document<'_>,
    errors: impl Iterator<Item = ValidationError<'e>>,
    severity: Severity,
) -> Vec<Finding> {
    let issues: BTreeSet<Issue> = errors.map(|error| issue(&error)).collect();
    let primary: Vec<&str> = issues
        .iter()
        .filter(|issue| !issue.unevaluated)
        .map(|issue| issue.pointer.as_str())
        .collect();
    issues
        .iter()
        .filter(|issue| {
            !issue.unevaluated
                || !primary
                    .iter()
                    .any(|pointer| is_within(pointer, &issue.pointer))
        })
        .map(|issue| {
            let target = resolve(document.root(), &issue.pointer);
            let anchor = issue
                .key
                .as_deref()
                .and_then(|key| entry(target.node, key))
                .map_or_else(|| target.anchor(), |(key, _)| key);
            Finding {
                rule: builtin::SCHEMA.to_string(),
                name: builtin::SCHEMA.to_string(),
                severity,
                message: field_label(&issue.pointer).map_or_else(
                    || issue.message.clone(),
                    |label| format!("{label}: {}", issue.message),
                ),
                location: document.location(anchor),
                fixable: false,
            }
        })
        .collect()
}

fn is_within(pointer: &str, parent: &str) -> bool {
    pointer
        .strip_prefix(parent)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

/// The finding already points at the line, so only the innermost field is named
/// (`logicalType`, `servers[0]`) instead of the whole JSON pointer.
fn field_label(pointer: &str) -> Option<String> {
    let segments: Vec<String> = pointer
        .split('/')
        .skip(1)
        .map(|segment| segment.replace("~1", "/").replace("~0", "~"))
        .collect();
    let is_index = |segment: &str| segment.parse::<usize>().is_ok();
    match segments.as_slice() {
        [.., key, index] if is_index(index) && !is_index(key) => Some(format!("{key}[{index}]")),
        [.., last] if !is_index(last) => Some(last.clone()),
        _ => None,
    }
}

fn issue(error: &ValidationError<'_>) -> Issue {
    let pointer = error.instance_path().as_str().to_string();
    let (message, key) = match error.kind() {
        ValidationErrorKind::AdditionalProperties { unexpected }
        | ValidationErrorKind::UnevaluatedProperties { unexpected } => (
            unexpected
                .iter()
                .map(|key| format!("unexpected property `{key}`"))
                .collect::<Vec<_>>()
                .join(", "),
            unexpected.first().cloned(),
        ),
        ValidationErrorKind::Required { property } => (
            format!("missing required property `{}`", plain(property)),
            None,
        ),
        ValidationErrorKind::Type { kind } => {
            let expected = match kind {
                TypeKind::Single(single) => single.to_string(),
                TypeKind::Multiple(set) => set
                    .iter()
                    .map(|json_type| json_type.to_string())
                    .collect::<Vec<_>>()
                    .join(" or "),
            };
            let found = match error.instance().as_ref() {
                Value::Object(_) => "an object".to_string(),
                Value::Array(_) => "an array".to_string(),
                scalar => format!("`{}`", plain(scalar)),
            };
            (format!("expected {expected}, found {found}"), None)
        }
        ValidationErrorKind::Enum { options } => (
            format!(
                "`{}` is not one of: {}",
                plain(error.instance()),
                options_list(options)
            ),
            None,
        ),
        _ => {
            let text = error.to_string();
            let message = match error.instance().as_ref() {
                instance @ (Value::Object(_) | Value::Array(_)) => {
                    text.replacen(&instance.to_string(), "value", 1)
                }
                _ => text,
            };
            (message, None)
        }
    };
    Issue {
        pointer,
        message,
        key,
        unevaluated: matches!(
            error.kind(),
            ValidationErrorKind::UnevaluatedProperties { .. }
        ),
    }
}

fn plain(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

fn options_list(options: &Value) -> String {
    const SHOWN: usize = 12;
    let options: Vec<String> = options
        .as_array()
        .map(|values| values.iter().map(plain).collect())
        .unwrap_or_default();
    if options.len() > SHOWN {
        format!(
            "{}, ... ({} total)",
            options[..SHOWN - 2].join(", "),
            options.len()
        )
    } else {
        options.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::Location;
    use crate::document::to_json::to_json;
    use crate::schema::{validate, SchemaVersion};

    fn check(source: &str) -> Vec<Finding> {
        let document = Document::parse(source).unwrap();
        validate(
            &document,
            &to_json(document.root()),
            SchemaVersion::V3_2_0,
            Severity::Error,
        )
        .unwrap()
    }

    const BASE: &str = "apiVersion: v3.2.0\nkind: DataContract\nid: abc\nversion: 1.0.0\n";

    #[test]
    fn reports_missing_required_property_at_root() {
        let findings = check("apiVersion: v3.2.0\nkind: DataContract\nversion: 1.0.0\n");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].message, "missing required property `id`");
        assert_eq!(findings[0].location, Location::START);
    }

    #[test]
    fn points_at_unexpected_key() {
        let findings = check(&format!("{BASE}bogus: 1\n"));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].message, "unexpected property `bogus`");
        assert_eq!(findings[0].location, Location { line: 5, column: 1 });
    }

    #[test]
    fn names_the_field_and_lists_allowed_values() {
        let findings = check(&format!(
            "{BASE}schema:\n  - name: t\n    properties:\n      - name: c\n        logicalType: nope\n"
        ));
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].message,
            "logicalType: `nope` is not one of: string, date, timestamp, time, number, integer, object, array, boolean, map, vector"
        );
        assert_eq!(findings[0].location, Location { line: 9, column: 9 });
    }

    #[test]
    fn suppresses_unevaluated_cascade_below_a_primary_error() {
        let findings = check(&format!(
            "{BASE}schema:\n  - name: t\n    properties:\n      - name: c\n        deprecated: maybe\n"
        ));
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].message,
            "deprecated: expected boolean, found `maybe`"
        );
    }

    #[test]
    fn labels_use_the_innermost_field() {
        assert_eq!(field_label(""), None);
        assert_eq!(field_label("/servers/0"), Some("servers[0]".to_string()));
        assert_eq!(
            field_label("/schema/0/properties/1/logicalType"),
            Some("logicalType".to_string())
        );
    }

    #[test]
    fn long_option_lists_are_truncated() {
        let options = Value::Array((1..=20).map(Value::from).collect());
        assert_eq!(
            options_list(&options),
            "1, 2, 3, 4, 5, 6, 7, 8, 9, 10, ... (20 total)"
        );
    }

    #[test]
    fn is_within_matches_descendants_only() {
        assert!(is_within("/a/b", "/a"));
        assert!(is_within("/a", "/a"));
        assert!(!is_within("/ab", "/a"));
        assert!(is_within("/a", ""));
    }

    #[test]
    fn does_not_embed_objects_in_messages() {
        let findings = check(&format!("{BASE}tags:\n  a: 1\n"));
        assert!(findings
            .iter()
            .all(|finding| !finding.message.contains('{')));
    }
}
