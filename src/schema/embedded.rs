use super::SchemaVersion;

pub const fn source(version: SchemaVersion) -> &'static str {
    match version {
        SchemaVersion::V3_0_0 => include_str!("../../schemas/odcs/odcs-json-schema-v3.0.0.json"),
        SchemaVersion::V3_0_1 => include_str!("../../schemas/odcs/odcs-json-schema-v3.0.1.json"),
        SchemaVersion::V3_0_2 => include_str!("../../schemas/odcs/odcs-json-schema-v3.0.2.json"),
        SchemaVersion::V3_1_0 => {
            include_str!("../../schemas/odcs/odcs-json-schema-v3.1.0-20260908.json")
        }
        SchemaVersion::V3_2_0 => include_str!("../../schemas/odcs/odcs-json-schema-v3.2.0.json"),
    }
}
