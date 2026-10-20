use serde::Deserialize;

use super::{Context, Violation};
use crate::config::pattern::GlobPattern;
use crate::constants::{keys, CONTRACT_SUFFIXES};
use crate::diagnostic::Location;
use crate::document::node::scalar_text;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StemField {
    Id,
    Name,
}

impl StemField {
    const fn key(self) -> &'static str {
        match self {
            Self::Id => keys::ID,
            Self::Name => keys::NAME,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileNaming {
    pub pattern: Option<GlobPattern>,
    pub match_field: Option<StemField>,
}

impl FileNaming {
    pub(super) fn validate(&self) -> Result<(), String> {
        if self.pattern.is_none() && self.match_field.is_none() {
            Err("`pattern` or `match_field` is required".to_string())
        } else {
            Ok(())
        }
    }
}

fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn stem(name: &str) -> &str {
    CONTRACT_SUFFIXES
        .iter()
        .find_map(|suffix| name.strip_suffix(suffix))
        .unwrap_or(name)
}

pub fn check(options: &FileNaming, context: &Context<'_>) -> Vec<Violation> {
    let name = file_name(context.path);
    let pattern = options.pattern.as_ref().and_then(|pattern| {
        (!pattern.is_match(name)).then(|| {
            Violation::new(
                format!("file name `{name}` does not match `{}`", pattern.as_str()),
                Location::START,
            )
        })
    });
    let field = options.match_field.and_then(|field| {
        let node = context.contract.field(field.key())?;
        let value = scalar_text(node)?;
        (stem(name) != value).then(|| {
            Violation::new(
                format!(
                    "file name `{name}` does not match contract {} `{value}`",
                    field.key()
                ),
                context.location(node),
            )
        })
    });
    pattern.into_iter().chain(field).collect()
}

#[cfg(test)]
mod tests {
    use crate::rules::test_support::{rule, violations_at};

    fn messages_at(yaml: &str, contract: &str, path: &str) -> Vec<String> {
        violations_at(&rule(yaml), contract, path)
            .into_iter()
            .map(|violation| violation.message)
            .collect()
    }

    #[test]
    fn checks_file_name_pattern() {
        let yaml = "type: file_naming\npattern: '*.odcs.yaml'\n";
        assert!(messages_at(yaml, "id: x\n", "a/orders.odcs.yaml").is_empty());
        assert_eq!(
            messages_at(yaml, "id: x\n", "a/orders.odcs.yml"),
            vec!["file name `orders.odcs.yml` does not match `*.odcs.yaml`"]
        );
    }

    #[test]
    fn stem_must_match_name() {
        let yaml = "type: file_naming\nmatch_field: name\n";
        assert!(messages_at(yaml, "name: orders\n", "orders.odcs.yaml").is_empty());
        assert_eq!(
            messages_at(yaml, "name: sales\n", "x/orders.odcs.yaml"),
            vec!["file name `orders.odcs.yaml` does not match contract name `sales`"]
        );
    }
}
