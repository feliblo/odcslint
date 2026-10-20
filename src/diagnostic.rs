use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    #[default]
    Error,
    Warning,
}

impl Severity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize)]
pub struct Location {
    pub line: usize,
    pub column: usize,
}

impl Location {
    pub const START: Self = Self { line: 1, column: 1 };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub rule: String,
    pub name: String,
    pub severity: Severity,
    pub message: String,
    pub location: Location,
    pub fixable: bool,
}
