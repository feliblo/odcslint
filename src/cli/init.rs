use std::fs;

use super::{ConfigFormat, InitArgs};
use crate::constants::{exit_code, CONFIG_FILE_NAMES};
use crate::error::{Error, Result};

const YAML_FILE_NAME: &str = CONFIG_FILE_NAMES[0];
const TOML_FILE_NAME: &str = CONFIG_FILE_NAMES[2];

const YAML_TEMPLATE: &str = r#"# odcslint configuration: https://feliblo.github.io/odcslint/configuration/
settings:
  include: ["**/*.odcs.yaml", "**/*.odcs.yml"]
  exclude: []
  schema_validation: error # error | warning | off

rules:
  # Primary keys first, metadata columns last. Fixable with `odcslint check --fix`.
  - type: property_order
    metadata_columns:
      prefixes: ["_"]

  # primaryKey and primaryKeyPosition must agree.
  - type: primary_key

  # No duplicate property names within a table.
  - type: unique_names
    scope: property

  # Only approved owners may own a contract.
  # - type: allowed_values
  #   name: approved_owners
  #   field: team.members.username
  #   role: owner
  #   values: [alice, bob]

  # Only known domains.
  # - type: allowed_values
  #   field: domain
  #   values: [finance, marketing, sales]
  #   required: true

  # - type: naming_convention
  #   target: schema.properties.name
  #   convention: snake_case

  # - type: has_description
  #   scope: property
  #   min_length: 10

  # - type: required_fields
  #   scope: contract
  #   fields: [domain, description.purpose]
"#;

const TOML_TEMPLATE: &str = r#"# odcslint configuration: https://feliblo.github.io/odcslint/configuration/
[settings]
include = ["**/*.odcs.yaml", "**/*.odcs.yml"]
exclude = []
schema_validation = "error" # error | warning | off

# Primary keys first, metadata columns last. Fixable with `odcslint check --fix`.
[[rules]]
type = "property_order"
metadata_columns = { prefixes = ["_"] }

# primaryKey and primaryKeyPosition must agree.
[[rules]]
type = "primary_key"

# No duplicate property names within a table.
[[rules]]
type = "unique_names"
scope = "property"

# Only approved owners may own a contract.
# [[rules]]
# type = "allowed_values"
# name = "approved_owners"
# field = "team.members.username"
# role = "owner"
# values = ["alice", "bob"]

# [[rules]]
# type = "naming_convention"
# target = "schema.properties.name"
# convention = "snake_case"
"#;

pub fn run(args: &InitArgs) -> Result<u8> {
    let (file_name, template) = match args.format {
        ConfigFormat::Yaml => (YAML_FILE_NAME, YAML_TEMPLATE),
        ConfigFormat::Toml => (TOML_FILE_NAME, TOML_TEMPLATE),
    };
    let path = args.location.join(file_name);
    if path.exists() && !args.force {
        return Err(Error::AlreadyExists { path });
    }
    fs::write(&path, template).map_err(|source| Error::Write {
        path: path.clone(),
        source,
    })?;
    println!("Created {}", path.display());
    Ok(exit_code::SUCCESS)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::config::load;

    fn init(location: &Path, format: ConfigFormat, force: bool) -> Result<u8> {
        run(&InitArgs {
            location: location.to_path_buf(),
            format,
            force,
        })
    }

    #[test]
    fn yaml_template_is_a_valid_config() {
        let directory = tempfile::tempdir().unwrap();
        init(directory.path(), ConfigFormat::Yaml, false).unwrap();
        let config = load(Some(&directory.path().join(YAML_FILE_NAME))).unwrap();
        assert_eq!(config.rules.len(), 3);
    }

    #[test]
    fn toml_template_is_a_valid_config() {
        let directory = tempfile::tempdir().unwrap();
        init(directory.path(), ConfigFormat::Toml, false).unwrap();
        let config = load(Some(&directory.path().join(TOML_FILE_NAME))).unwrap();
        assert_eq!(config.rules.len(), 3);
    }

    #[test]
    fn refuses_to_overwrite_without_force() {
        let directory = tempfile::tempdir().unwrap();
        init(directory.path(), ConfigFormat::Yaml, false).unwrap();
        let error = init(directory.path(), ConfigFormat::Yaml, false).unwrap_err();
        assert!(matches!(error, Error::AlreadyExists { .. }));
        assert!(init(directory.path(), ConfigFormat::Yaml, true).is_ok());
    }
}
