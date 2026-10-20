use globset::{Glob, GlobBuilder, GlobMatcher, GlobSet, GlobSetBuilder};
use regex::Regex;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(try_from = "String")]
pub struct Pattern(Regex);

impl Pattern {
    pub fn is_match(&self, text: &str) -> bool {
        self.0.is_match(text)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<String> for Pattern {
    type Error = String;

    fn try_from(pattern: String) -> Result<Self, Self::Error> {
        Regex::new(&pattern)
            .map(Self)
            .map_err(|error| format!("invalid regex `{pattern}`: {error}"))
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(try_from = "Vec<String>")]
pub struct GlobList {
    patterns: Vec<String>,
    set: GlobSet,
}

impl GlobList {
    pub fn new(patterns: &[&str]) -> Result<Self, String> {
        Self::try_from(patterns.iter().map(ToString::to_string).collect::<Vec<_>>())
    }

    pub const fn empty() -> Self {
        Self {
            patterns: Vec::new(),
            set: GlobSet::empty(),
        }
    }

    pub fn is_match(&self, path: &str) -> bool {
        self.set.is_match(path)
    }

    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }

    pub fn patterns(&self) -> &[String] {
        &self.patterns
    }
}

impl TryFrom<Vec<String>> for GlobList {
    type Error = String;

    fn try_from(patterns: Vec<String>) -> Result<Self, Self::Error> {
        let set = patterns
            .iter()
            .map(|pattern| glob(pattern))
            .try_fold(GlobSetBuilder::new(), |mut builder, glob| {
                builder.add(glob?);
                Ok::<_, String>(builder)
            })?
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Self { patterns, set })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(try_from = "String")]
pub struct GlobPattern {
    pattern: String,
    matcher: GlobMatcher,
}

impl GlobPattern {
    pub fn is_match(&self, text: &str) -> bool {
        self.matcher.is_match(text)
    }

    pub fn as_str(&self) -> &str {
        &self.pattern
    }
}

impl TryFrom<String> for GlobPattern {
    type Error = String;

    fn try_from(pattern: String) -> Result<Self, Self::Error> {
        let matcher = glob(&pattern)?.compile_matcher();
        Ok(Self { pattern, matcher })
    }
}

pub fn glob(pattern: &str) -> Result<Glob, String> {
    GlobBuilder::new(pattern)
        .literal_separator(true)
        .build()
        .map_err(|error| format!("invalid glob `{pattern}`: {}", error.kind()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_star_matches_root_and_nested_files() {
        let globs = GlobList::new(&["**/*.odcs.yaml"]).unwrap();
        assert!(globs.is_match("a.odcs.yaml"));
        assert!(globs.is_match("x/y/a.odcs.yaml"));
        assert!(!globs.is_match("a.yaml"));
    }

    #[test]
    fn single_star_does_not_cross_directories() {
        let globs = GlobList::new(&["contracts/*.yaml"]).unwrap();
        assert!(globs.is_match("contracts/a.yaml"));
        assert!(!globs.is_match("contracts/sub/a.yaml"));
    }

    #[test]
    fn invalid_glob_is_rejected() {
        let error = GlobList::new(&["a/[b"]).unwrap_err();
        assert!(error.starts_with("invalid glob `a/[b`"));
    }

    #[test]
    fn invalid_regex_is_rejected() {
        let error = Pattern::try_from("(".to_string()).unwrap_err();
        assert!(error.starts_with("invalid regex `(`"));
    }
}
