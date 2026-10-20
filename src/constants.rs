pub const CONFIG_FILE_NAMES: [&str; 3] = ["odcslint.yml", "odcslint.yaml", "odcslint.toml"];

pub const PYPROJECT_FILE_NAME: &str = "pyproject.toml";

pub const DEFAULT_INCLUDE: [&str; 2] = ["**/*.odcs.yaml", "**/*.odcs.yml"];

pub const CONTRACT_SUFFIXES: [&str; 4] = [".odcs.yaml", ".odcs.yml", ".yaml", ".yml"];

pub const MAX_FIX_PASSES: usize = 10;

pub mod exit_code {
    pub const SUCCESS: u8 = 0;
    pub const FINDINGS: u8 = 1;
    pub const TOOL_ERROR: u8 = 2;
}

pub mod builtin {
    pub const YAML_SYNTAX: &str = "yaml_syntax";
    pub const SCHEMA: &str = "schema";
    pub const UNSUPPORTED_API_VERSION: &str = "unsupported_api_version";
    pub const FIX_FAILED: &str = "fix_failed";
}

pub mod keys {
    pub const API_VERSION: &str = "apiVersion";
    pub const CLASSIFICATION: &str = "classification";
    pub const CUSTOM_PROPERTIES: &str = "customProperties";
    pub const DATA_PRODUCT: &str = "dataProduct";
    pub const DESCRIPTION: &str = "description";
    pub const DOMAIN: &str = "domain";
    pub const ID: &str = "id";
    pub const ITEMS: &str = "items";
    pub const LOGICAL_TYPE: &str = "logicalType";
    pub const MEMBERS: &str = "members";
    pub const NAME: &str = "name";
    pub const PHYSICAL_NAME: &str = "physicalName";
    pub const PHYSICAL_TYPE: &str = "physicalType";
    pub const PRIMARY_KEY: &str = "primaryKey";
    pub const PRIMARY_KEY_POSITION: &str = "primaryKeyPosition";
    pub const PROPERTIES: &str = "properties";
    pub const PROPERTY: &str = "property";
    pub const PURPOSE: &str = "purpose";
    pub const ROLE: &str = "role";
    pub const SCHEMA: &str = "schema";
    pub const SLA_DEFAULT_ELEMENT: &str = "slaDefaultElement";
    pub const STATUS: &str = "status";
    pub const TAGS: &str = "tags";
    pub const TEAM: &str = "team";
    pub const TENANT: &str = "tenant";
    pub const USERNAME: &str = "username";
    pub const VALUE: &str = "value";
}
