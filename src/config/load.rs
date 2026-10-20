use std::fs;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::Deserialize;

use super::{absolute, Config};
use crate::constants::{CONFIG_FILE_NAMES, PYPROJECT_FILE_NAME};
use crate::document::to_json::to_json;
use crate::document::Document;
use crate::error::{Error, Result};

const TOML_EXTENSION: &str = "toml";
const GIT_DIR: &str = ".git";

#[derive(Deserialize)]
struct PyProject {
    tool: Option<Tool>,
}

#[derive(Deserialize)]
struct Tool {
    odcslint: Option<Config>,
}

/// Searches `start` and its parents up to the git repository root. Outside a git repository only
/// `start` itself is searched, so a stray config higher up is never picked up by surprise.
pub fn find(start: &Path) -> Result<Option<PathBuf>> {
    let start = absolute(start).unwrap_or_else(|| start.to_path_buf());
    let git_root = start
        .ancestors()
        .find(|directory| directory.join(GIT_DIR).exists());
    let directories: Vec<&Path> = git_root.map_or_else(
        || vec![start.as_path()],
        |root| {
            start
                .ancestors()
                .take_while(|directory| *directory != root)
                .chain(std::iter::once(root))
                .collect()
        },
    );
    for directory in directories {
        if let Some(path) = find_in(directory)? {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

fn find_in(directory: &Path) -> Result<Option<PathBuf>> {
    let pyproject = directory.join(PYPROJECT_FILE_NAME);
    let pyproject = if pyproject.is_file() && has_tool_section(&pyproject)? {
        Some(pyproject)
    } else {
        None
    };
    let all: Vec<PathBuf> = CONFIG_FILE_NAMES
        .iter()
        .map(|name| directory.join(name))
        .filter(|path| path.is_file())
        .chain(pyproject)
        .collect();
    if all.len() > 1 {
        log::warn!("multiple config files found, using `{}`", all[0].display());
    }
    Ok(all.into_iter().next())
}

pub fn load(path: Option<&Path>) -> Result<Config> {
    let Some(path) = path else {
        return Ok(Config::default());
    };
    let source = fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let config = parse(path, &source).map_err(|message| Error::Config {
        path: path.to_path_buf(),
        message,
    })?;
    config.validate().map_err(|message| Error::Config {
        path: path.to_path_buf(),
        message,
    })?;
    let root = path.parent().and_then(absolute).unwrap_or_default();
    Ok(Config { root, ..config })
}

fn parse(path: &Path, source: &str) -> std::result::Result<Config, String> {
    if is_pyproject(path) {
        parse_toml::<PyProject>(source).map(|pyproject| {
            pyproject
                .tool
                .and_then(|tool| tool.odcslint)
                .unwrap_or_default()
        })
    } else if path
        .extension()
        .is_some_and(|extension| extension == TOML_EXTENSION)
    {
        parse_toml(source)
    } else {
        parse_yaml(source)
    }
}

fn parse_toml<T: DeserializeOwned>(source: &str) -> std::result::Result<T, String> {
    toml::from_str(source).map_err(|error| error.to_string())
}

fn parse_yaml(source: &str) -> std::result::Result<Config, String> {
    if source.trim().is_empty() {
        return Ok(Config::default());
    }
    let document = Document::parse(source).map_err(|error| {
        format!(
            "{} at line {} column {}",
            error.message, error.location.line, error.location.column
        )
    })?;
    serde_json::from_value(to_json(document.root())).map_err(|error| error.to_string())
}

fn is_pyproject(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|name| name == PYPROJECT_FILE_NAME)
}

fn has_tool_section(path: &Path) -> Result<bool> {
    let source = fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(toml::from_str::<toml::Table>(&source)
        .ok()
        .and_then(|table| table.get("tool")?.get("odcslint").cloned())
        .is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SchemaValidation;

    fn write(directory: &Path, name: &str, contents: &str) -> PathBuf {
        let path = directory.join(name);
        fs::write(&path, contents).unwrap();
        path
    }

    fn git_repo() -> tempfile::TempDir {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir(directory.path().join(GIT_DIR)).unwrap();
        directory
    }

    #[test]
    fn finds_config_in_parent_directory() {
        let repo = git_repo();
        let config = write(repo.path(), "odcslint.yml", "");
        let nested = repo.path().join("contracts/sales");
        fs::create_dir_all(&nested).unwrap();
        assert_eq!(find(&nested).unwrap(), absolute(&config));
    }

    #[test]
    fn nearest_config_wins() {
        let repo = git_repo();
        write(repo.path(), "odcslint.yml", "");
        let nested = repo.path().join("contracts");
        fs::create_dir_all(&nested).unwrap();
        let near = write(&nested, "odcslint.toml", "");
        assert_eq!(find(&nested).unwrap(), absolute(&near));
    }

    #[test]
    fn stops_at_git_root() {
        let outer = tempfile::tempdir().unwrap();
        write(outer.path(), "odcslint.yml", "");
        let repo = outer.path().join("repo");
        fs::create_dir_all(repo.join(GIT_DIR)).unwrap();
        assert_eq!(find(&repo).unwrap(), None);
    }

    #[test]
    fn outside_git_only_searches_start_directory() {
        let outer = tempfile::tempdir().unwrap();
        write(outer.path(), "odcslint.yml", "");
        let nested = outer.path().join("nested");
        fs::create_dir_all(&nested).unwrap();
        assert_eq!(find(&nested).unwrap(), None);
    }

    #[test]
    fn loaded_config_remembers_its_directory() {
        let directory = tempfile::tempdir().unwrap();
        let path = write(directory.path(), "odcslint.yml", "");
        assert_eq!(
            load(Some(&path)).unwrap().root,
            absolute(directory.path()).unwrap()
        );
    }

    #[test]
    fn finds_nothing_in_empty_directory() {
        let directory = tempfile::tempdir().unwrap();
        assert_eq!(find(directory.path()).unwrap(), None);
    }

    #[test]
    fn yml_takes_precedence_over_toml() {
        let directory = tempfile::tempdir().unwrap();
        write(directory.path(), "odcslint.toml", "");
        let yml = write(directory.path(), "odcslint.yml", "");
        assert_eq!(find(directory.path()).unwrap(), Some(yml));
    }

    #[test]
    fn pyproject_without_section_is_ignored() {
        let directory = tempfile::tempdir().unwrap();
        write(
            directory.path(),
            "pyproject.toml",
            "[project]\nname = 'x'\n",
        );
        assert_eq!(find(directory.path()).unwrap(), None);
    }

    #[test]
    fn loads_pyproject_section() {
        let directory = tempfile::tempdir().unwrap();
        let path = write(
            directory.path(),
            "pyproject.toml",
            "[tool.odcslint.settings]\nschema_validation = \"warning\"\n",
        );
        assert_eq!(find(directory.path()).unwrap(), Some(path.clone()));
        let config = load(Some(&path)).unwrap();
        assert_eq!(config.settings.schema_validation, SchemaValidation::Warning);
    }

    #[test]
    fn loads_yaml_rules() {
        let directory = tempfile::tempdir().unwrap();
        let path = write(
            directory.path(),
            "odcslint.yml",
            "rules:\n  - type: unique_names\n    scope: property\n    severity: warning\n",
        );
        let config = load(Some(&path)).unwrap();
        assert_eq!(config.rules.len(), 1);
    }

    #[test]
    fn loads_toml_rules() {
        let directory = tempfile::tempdir().unwrap();
        let path = write(
            directory.path(),
            "odcslint.toml",
            "[[rules]]\ntype = \"unique_names\"\nscope = \"property\"\n",
        );
        assert_eq!(load(Some(&path)).unwrap().rules.len(), 1);
    }

    #[test]
    fn empty_yaml_is_default_config() {
        let directory = tempfile::tempdir().unwrap();
        let path = write(directory.path(), "odcslint.yml", "\n");
        assert!(load(Some(&path)).unwrap().rules.is_empty());
    }

    #[test]
    fn invalid_regex_is_a_config_error() {
        let directory = tempfile::tempdir().unwrap();
        let path = write(
            directory.path(),
            "odcslint.yml",
            "rules:\n  - type: id_format\n    pattern: '('\n",
        );
        let error = load(Some(&path)).unwrap_err();
        assert!(matches!(error, Error::Config { .. }));
        assert!(error.to_string().contains("invalid regex `(`"));
    }

    #[test]
    fn semantic_validation_errors_name_the_rule() {
        let directory = tempfile::tempdir().unwrap();
        let path = write(
            directory.path(),
            "odcslint.yml",
            "rules:\n  - type: allowed_values\n    name: owners\n    field: domain\n",
        );
        let error = load(Some(&path)).unwrap_err();
        assert!(error.to_string().contains("rule `owners`"));
    }

    #[test]
    fn missing_file_is_a_read_error() {
        let error = load(Some(Path::new("does-not-exist.yml"))).unwrap_err();
        assert!(matches!(error, Error::Read { .. }));
    }
}
