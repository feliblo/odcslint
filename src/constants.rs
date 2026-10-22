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
    pub const UPGRADE: &str = "upgrade";
}

pub mod keys {
    pub const API_VERSION: &str = "apiVersion";
    pub const CLASSIFICATION: &str = "classification";
    pub const CUSTOM_PROPERTIES: &str = "customProperties";
    pub const DATA_PRODUCT: &str = "dataProduct";
    pub const DESCRIPTION: &str = "description";
    pub const DOMAIN: &str = "domain";
    pub const ELEMENT: &str = "element";
    pub const EXCLUSIVE_MAXIMUM: &str = "exclusiveMaximum";
    pub const EXCLUSIVE_MINIMUM: &str = "exclusiveMinimum";
    pub const ID: &str = "id";
    pub const ITEMS: &str = "items";
    pub const LOGICAL_TYPE: &str = "logicalType";
    pub const LOGICAL_TYPE_OPTIONS: &str = "logicalTypeOptions";
    pub const MAXIMUM: &str = "maximum";
    pub const MEMBERS: &str = "members";
    pub const METRIC: &str = "metric";
    pub const MINIMUM: &str = "minimum";
    pub const MUST_BE: &str = "mustBe";
    pub const MUST_BE_BETWEEN: &str = "mustBeBetween";
    pub const MUST_BE_GREATER_OR_EQUAL_TO: &str = "mustBeGreaterOrEqualTo";
    pub const MUST_BE_GREATER_THAN: &str = "mustBeGreaterThan";
    pub const MUST_BE_LESS_OR_EQUAL_TO: &str = "mustBeLessOrEqualTo";
    pub const MUST_BE_LESS_THAN: &str = "mustBeLessThan";
    pub const MUST_NOT_BE: &str = "mustNotBe";
    pub const MUST_NOT_BE_BETWEEN: &str = "mustNotBeBetween";
    pub const NAME: &str = "name";
    pub const PHYSICAL_NAME: &str = "physicalName";
    pub const PHYSICAL_TYPE: &str = "physicalType";
    pub const PRIMARY_KEY: &str = "primaryKey";
    pub const PRIMARY_KEY_POSITION: &str = "primaryKeyPosition";
    pub const PROPERTIES: &str = "properties";
    pub const PROPERTY: &str = "property";
    pub const PURPOSE: &str = "purpose";
    pub const QUALITY: &str = "quality";
    pub const ROLE: &str = "role";
    pub const ROLES: &str = "roles";
    pub const RULE: &str = "rule";
    pub const SCHEMA: &str = "schema";
    pub const SERVERS: &str = "servers";
    pub const SLA_DEFAULT_ELEMENT: &str = "slaDefaultElement";
    pub const SLA_PROPERTIES: &str = "slaProperties";
    pub const STAGING_DIR: &str = "stagingDir";
    pub const STAGING_DIR_LEGACY: &str = "staging_dir";
    pub const STATUS: &str = "status";
    pub const SUPPORT: &str = "support";
    pub const TAGS: &str = "tags";
    pub const TEAM: &str = "team";
    pub const TENANT: &str = "tenant";
    pub const TYPE: &str = "type";
    pub const USERNAME: &str = "username";
    pub const VALUE: &str = "value";
}
