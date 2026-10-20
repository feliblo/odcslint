mod diagnostics;
mod embedded;

use std::sync::OnceLock;

use jsonschema::Validator;
use serde_json::Value;
use strum::{EnumIter, IntoEnumIterator};

use crate::diagnostic::{Finding, Severity};
use crate::document::Document;
use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter)]
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
    let validator = validator(version)?;
    Ok(diagnostics::findings(
        document,
        validator.iter_errors(json),
        severity,
    ))
}

type CompiledSchema = std::result::Result<Validator, String>;

static VALIDATORS: [OnceLock<CompiledSchema>; 5] = [const { OnceLock::new() }; 5];

fn validator(version: SchemaVersion) -> Result<&'static Validator> {
    VALIDATORS[version.index()]
        .get_or_init(|| compile(embedded::source(version)))
        .as_ref()
        .map_err(|message| Error::Schema {
            version: version.api_version(),
            message: message.clone(),
        })
}

fn compile(source: &str) -> CompiledSchema {
    let schema: Value = serde_json::from_str(source).map_err(|error| error.to_string())?;
    jsonschema::draft201909::options()
        .should_validate_formats(true)
        .should_ignore_unknown_formats(true)
        .build(&schema)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::to_json::to_json;

    #[test]
    fn every_embedded_schema_compiles() {
        let failures: Vec<_> = SchemaVersion::iter()
            .filter(|version| validator(*version).is_err())
            .collect();
        assert!(failures.is_empty(), "failed to compile: {failures:?}");
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
