mod diagnostics;
mod embedded;
mod split;

use std::collections::BTreeSet;
use std::sync::OnceLock;

use jsonschema::Validator;
use serde_json::Value;
use strum::{EnumIter, IntoEnumIterator};

use self::diagnostics::{is_within, issue, Issue};
use self::split::{Cut, Definition, Part};
use crate::diagnostic::{Finding, Severity};
use crate::document::Document;
use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, EnumIter)]
pub enum SchemaVersion {
    V3_0_0,
    V3_0_1,
    V3_0_2,
    V3_1_0,
    V3_2_0,
}

impl SchemaVersion {
    pub const LATEST: Self = Self::V3_2_0;

    pub const fn api_version(self) -> &'static str {
        match self {
            Self::V3_0_0 => "v3.0.0",
            Self::V3_0_1 => "v3.0.1",
            Self::V3_0_2 => "v3.0.2",
            Self::V3_1_0 => "v3.1.0",
            Self::V3_2_0 => "v3.2.0",
        }
    }

    pub fn from_api_version(api_version: &str) -> Option<Self> {
        Self::iter().find(|version| version.api_version() == api_version)
    }

    /// Every version after `from`, up to and including `to`, oldest first.
    pub fn steps(from: Self, to: Self) -> impl Iterator<Item = Self> {
        Self::iter().filter(move |version| from < *version && *version <= to)
    }

    const fn index(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaSelection {
    Supported(SchemaVersion),
    Unsupported(String),
}

pub fn select(api_version: Option<&str>) -> SchemaSelection {
    api_version.map_or(SchemaSelection::Supported(SchemaVersion::LATEST), |value| {
        SchemaVersion::from_api_version(value).map_or_else(
            || SchemaSelection::Unsupported(value.to_string()),
            SchemaSelection::Supported,
        )
    })
}

pub fn validate(
    document: &Document<'_>,
    json: &Value,
    version: SchemaVersion,
    severity: Severity,
) -> Result<Vec<Finding>> {
    let root = Part {
        value: json,
        definition: Definition::Contract,
    };
    Ok(diagnostics::findings(
        document,
        part_issues(version, "", root)?,
        severity,
    ))
}

/// See [`split`] for why nested properties are validated as separate parts.
fn part_issues(version: SchemaVersion, pointer: &str, part: Part<'_>) -> Result<Vec<Issue>> {
    let validator = validator(version, part.definition)?;
    let cuts = split::cuts(part);
    let stand_in = split::with_stand_ins(part.value, &cuts);
    let own = validator
        .iter_errors(&stand_in)
        .map(|error| issue(&error, pointer));
    let probed: Vec<String> = if cuts.is_empty() {
        Vec::new()
    } else {
        validator
            .iter_errors(&split::with_probes(part.value, &cuts))
            .map(|error| error.instance_path().as_str().to_string())
            .collect()
    };
    let nested = cuts
        .iter()
        .filter(|cut| probed.iter().any(|path| is_within(path, &cut.pointer)))
        .flat_map(Cut::children)
        .map(|(child_pointer, child)| {
            part_issues(version, &format!("{pointer}{child_pointer}"), child)
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(own.chain(nested.into_iter().flatten()).collect())
}

type CompiledSchema = std::result::Result<Validator, String>;
type ParsedSchema = std::result::Result<Value, String>;

static VALIDATORS: [[OnceLock<CompiledSchema>; Definition::COUNT]; 5] =
    [const { [const { OnceLock::new() }; Definition::COUNT] }; 5];
static DOCUMENTS: [OnceLock<ParsedSchema>; 5] = [const { OnceLock::new() }; 5];

const DEFINITIONS_PREFIX: &str = "#/$defs/";
const REF_KEY: &str = "$ref";
const PROPERTIES_KEY: &str = "properties";
const COMBINATOR_KEYS: [&str; 6] = ["allOf", "anyOf", "oneOf", "if", "then", "else"];
const DEFINITIONS_KEY: &str = "$defs";
const META_SCHEMA_KEY: &str = "$schema";

/// Keys that an object of `definition` (e.g. `Server`) may have in `version`, following `$ref`,
/// combinators and conditionals. The union over all branches errs towards treating keys as known.
pub fn declared_keys(version: SchemaVersion, definition: &str) -> Result<BTreeSet<String>> {
    let schema = schema_document(version)?;
    let start = format!("{DEFINITIONS_PREFIX}{definition}");
    Ok(collect_keys(
        schema,
        &reference(schema, &start),
        &mut BTreeSet::new(),
    ))
}

fn reference(schema: &Value, target: &str) -> Value {
    target
        .strip_prefix('#')
        .and_then(|pointer| schema.pointer(pointer))
        .cloned()
        .unwrap_or(Value::Null)
}

fn collect_keys(schema: &Value, node: &Value, visited: &mut BTreeSet<String>) -> BTreeSet<String> {
    let own: BTreeSet<String> = node
        .get(PROPERTIES_KEY)
        .and_then(Value::as_object)
        .map(|properties| properties.keys().cloned().collect())
        .unwrap_or_default();
    let referenced = node
        .get(REF_KEY)
        .and_then(Value::as_str)
        .filter(|target| visited.insert((*target).to_string()))
        .map(|target| collect_keys(schema, &reference(schema, target), visited))
        .unwrap_or_default();
    let combined: Vec<Value> = COMBINATOR_KEYS
        .iter()
        .filter_map(|key| node.get(key))
        .flat_map(|branch| match branch {
            Value::Array(branches) => branches.clone(),
            other => vec![other.clone()],
        })
        .collect();
    let nested = combined
        .iter()
        .flat_map(|branch| collect_keys(schema, branch, visited))
        .collect::<BTreeSet<_>>();
    own.into_iter().chain(referenced).chain(nested).collect()
}

fn schema_document(version: SchemaVersion) -> Result<&'static Value> {
    DOCUMENTS[version.index()]
        .get_or_init(|| {
            serde_json::from_str(embedded::source(version)).map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(|message| Error::Schema {
            version: version.api_version(),
            message: message.clone(),
        })
}

fn validator(version: SchemaVersion, definition: Definition) -> Result<&'static Validator> {
    let schema = schema_document(version)?;
    VALIDATORS[version.index()][definition.index()]
        .get_or_init(|| compile(&definition_schema(schema, definition)))
        .as_ref()
        .map_err(|message| Error::Schema {
            version: version.api_version(),
            message: message.clone(),
        })
}

/// A definition is validated through a schema that keeps the shared `$defs` and refers to it.
fn definition_schema(schema: &Value, definition: Definition) -> Value {
    definition.name().map_or_else(
        || schema.clone(),
        |name| {
            let kept = [META_SCHEMA_KEY, DEFINITIONS_KEY]
                .into_iter()
                .filter_map(|key| {
                    schema
                        .get(key)
                        .map(|value| (key.to_string(), value.clone()))
                });
            let reference = (
                REF_KEY.to_string(),
                Value::String(format!("{DEFINITIONS_PREFIX}{name}")),
            );
            Value::Object(kept.chain(std::iter::once(reference)).collect())
        },
    )
}

fn compile(schema: &Value) -> CompiledSchema {
    jsonschema::draft201909::options()
        .should_validate_formats(true)
        .should_ignore_unknown_formats(true)
        .build(schema)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::Location;
    use crate::document::to_json::to_json;

    const NESTED: &str = "\
apiVersion: v3.2.0
kind: DataContract
id: abc
version: 1.0.0
status: active
schema:
  - name: t
    properties:
      - name: address
        logicalType: object
        properties:
          - name: lines
            logicalType: array
            items:
              logicalType: object
              properties:
                - name: geo
                  logicalType: map
                  map:
                    key:
                      logicalType: string
                    value:
                      logicalType: object
                      properties:
                        - name: lat
                          logicalType: number
";

    fn findings(source: &str) -> Vec<(String, Location)> {
        let document = Document::parse(source).unwrap();
        validate(
            &document,
            &to_json(document.root()),
            SchemaVersion::V3_2_0,
            Severity::Error,
        )
        .unwrap()
        .into_iter()
        .map(|finding| (finding.message, finding.location))
        .collect()
    }

    #[test]
    fn nested_contract_through_every_edge_is_valid() {
        assert_eq!(findings(NESTED), vec![]);
    }

    #[test]
    fn deep_error_points_at_its_own_line_without_ancestor_noise() {
        let source = NESTED.replace(
            "logicalType: number\n",
            "logicalType: number\n                          unknownKey: oops\n",
        );
        assert_eq!(
            findings(&source),
            vec![(
                "properties[0]: unexpected property `unknownKey`".to_string(),
                Location {
                    line: 27,
                    column: 27
                }
            )]
        );
    }

    #[test]
    fn misplaced_properties_are_reported_and_not_descended_into() {
        let source = NESTED
            .replace(
                "        logicalType: object\n        properties:",
                "        logicalType: string\n        properties:",
            )
            .replace("logicalType: number", "logicalType: nope");
        assert_eq!(
            findings(&source),
            vec![(
                "properties[0]: unexpected property `properties`".to_string(),
                Location {
                    line: 11,
                    column: 9
                }
            )]
        );
    }

    #[test]
    fn deep_nesting_validates_in_linear_time() {
        let depth = 40;
        let levels = (0..depth)
            .map(|level| {
                let indent = " ".repeat(6 + level * 6);
                format!(
                    "{indent}- name: level{level}\n{indent}  logicalType: array\n{indent}  items:\n{indent}    logicalType: object\n{indent}    properties:\n"
                )
            })
            .collect::<Vec<_>>()
            .concat();
        let leaf = format!("{}- name: leaf\n", " ".repeat(6 + depth * 6));
        let source = format!(
            "apiVersion: v3.2.0\nkind: DataContract\nid: abc\nversion: 1.0.0\nstatus: active\nschema:\n  - name: t\n    properties:\n{levels}{leaf}"
        );
        assert_eq!(findings(&source), vec![]);
    }

    #[test]
    fn every_embedded_schema_compiles() {
        let failures: Vec<_> = SchemaVersion::iter()
            .flat_map(|version| Definition::iter().map(move |definition| (version, definition)))
            .filter(|(version, definition)| validator(*version, *definition).is_err())
            .collect();
        assert!(failures.is_empty(), "failed to compile: {failures:?}");
    }

    #[test]
    fn steps_are_ordered_and_exclusive_of_the_start() {
        let steps: Vec<SchemaVersion> =
            SchemaVersion::steps(SchemaVersion::V3_0_2, SchemaVersion::V3_2_0).collect();
        assert_eq!(steps, vec![SchemaVersion::V3_1_0, SchemaVersion::V3_2_0]);
        assert_eq!(
            SchemaVersion::steps(SchemaVersion::V3_2_0, SchemaVersion::V3_1_0).count(),
            0
        );
    }

    #[test]
    fn declared_keys_follow_references_and_conditionals() {
        let server = declared_keys(SchemaVersion::V3_1_0, "Server").unwrap();
        assert!(server.contains("customProperties"));
        assert!(
            server.contains("stagingDir"),
            "type-specific keys come from if/then branches"
        );
        assert!(!server.contains("staging_dir"));
        let property = declared_keys(SchemaVersion::V3_1_0, "SchemaProperty").unwrap();
        assert!(property.contains("primaryKey"));
        assert!(
            property.contains("customProperties"),
            "inherited from SchemaElement"
        );
    }

    #[test]
    fn selects_schema_by_api_version() {
        assert_eq!(
            select(Some("v3.0.2")),
            SchemaSelection::Supported(SchemaVersion::V3_0_2)
        );
    }

    #[test]
    fn missing_api_version_selects_latest() {
        assert_eq!(
            select(None),
            SchemaSelection::Supported(SchemaVersion::LATEST)
        );
    }

    #[test]
    fn unknown_api_version_is_unsupported() {
        assert_eq!(
            select(Some("v2.2.2")),
            SchemaSelection::Unsupported("v2.2.2".to_string())
        );
    }

    #[test]
    fn valid_contract_has_no_findings() {
        let source =
            "apiVersion: v3.1.0\nkind: DataContract\nid: abc\nversion: 1.0.0\nstatus: active\n";
        let document = Document::parse(source).unwrap();
        let findings = validate(
            &document,
            &to_json(document.root()),
            SchemaVersion::V3_1_0,
            Severity::Error,
        )
        .unwrap();
        assert_eq!(findings, vec![]);
    }
}
