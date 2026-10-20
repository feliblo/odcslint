use std::path::{Path, PathBuf};
use std::sync::mpsc;

use ignore::{WalkBuilder, WalkState};

use crate::config::pattern::{glob, GlobList};
use crate::config::{normalize_path, relative_path};
use crate::error::{Error, Result};

const GLOB_CHARACTERS: [char; 4] = ['*', '?', '[', '{'];
const CURRENT_DIRECTORY: &str = ".";

#[derive(Debug, Clone, Copy)]
pub struct Discovery<'a> {
    pub include: &'a GlobList,
    pub exclude: &'a GlobList,
    pub force_exclude: bool,
    pub respect_ignore: bool,
    /// Directory `include` and `exclude` are relative to (the config's directory).
    pub root: &'a Path,
}

pub fn discover(inputs: &[String], options: &Discovery<'_>) -> Result<Vec<PathBuf>> {
    let mut paths: Vec<PathBuf> = inputs
        .iter()
        .map(|input| discover_input(input, options))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn discover_input(input: &str, options: &Discovery<'_>) -> Result<Vec<PathBuf>> {
    let config_path = |path: &Path| relative_path(options.root, path);
    let excluded = |path: &Path| options.exclude.is_match(&config_path(path));
    if is_glob(input) {
        let pattern = normalize_path(Path::new(input));
        let matcher = glob(&pattern)
            .map_err(|message| Error::Glob {
                pattern: input.to_string(),
                message,
            })?
            .compile_matcher();
        Ok(walk(&glob_root(&pattern), options.respect_ignore, |path| {
            matcher.is_match(normalize_path(path)) && !excluded(path)
        }))
    } else {
        let path = Path::new(input);
        if path.is_file() {
            let skip = options.force_exclude && excluded(path);
            Ok(if skip {
                Vec::new()
            } else {
                vec![path.to_path_buf()]
            })
        } else if path.is_dir() {
            Ok(walk(path, options.respect_ignore, |path| {
                options.include.is_match(&config_path(path)) && !excluded(path)
            }))
        } else {
            Err(Error::PathNotFound(path.to_path_buf()))
        }
    }
}

fn is_glob(input: &str) -> bool {
    input.contains(GLOB_CHARACTERS)
}

fn glob_root(pattern: &str) -> PathBuf {
    let literal: Vec<&str> = pattern
        .split('/')
        .take_while(|component| !component.contains(GLOB_CHARACTERS))
        .collect();
    if literal.is_empty() {
        PathBuf::from(CURRENT_DIRECTORY)
    } else if literal == [""] {
        PathBuf::from("/")
    } else {
        PathBuf::from(literal.join("/"))
    }
}

fn walk(root: &Path, respect_ignore: bool, keep: impl Fn(&Path) -> bool + Sync) -> Vec<PathBuf> {
    let (sender, receiver) = mpsc::channel();
    WalkBuilder::new(root)
        .standard_filters(respect_ignore)
        .build_parallel()
        .run(|| {
            let sender = sender.clone();
            let keep = &keep;
            Box::new(move |entry| {
                match entry {
                    Ok(entry) => {
                        let is_file = entry.file_type().is_some_and(|kind| kind.is_file());
                        if is_file && keep(entry.path()) {
                            // The receiver outlives the walk, so sending cannot fail.
                            let _ = sender.send(entry.into_path());
                        }
                    }
                    Err(error) => log::warn!("skipping unreadable path: {error}"),
                }
                WalkState::Continue
            })
        });
    drop(sender);
    receiver.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::constants::DEFAULT_INCLUDE;

    fn tree() -> tempfile::TempDir {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        fs::create_dir_all(root.join("domains/sales")).unwrap();
        fs::create_dir_all(root.join("domains/sandbox")).unwrap();
        for file in [
            "a.odcs.yaml",
            "domains/sales/orders.odcs.yaml",
            "domains/sales/notes.yaml",
            "domains/sandbox/tmp.odcs.yml",
        ] {
            fs::write(root.join(file), "id: x\n").unwrap();
        }
        directory
    }

    fn run(root: &Path, inputs: &[&str], exclude: &[&str], force_exclude: bool) -> Vec<String> {
        let include = GlobList::new(&DEFAULT_INCLUDE).unwrap();
        let exclude = GlobList::new(exclude).unwrap();
        let options = Discovery {
            include: &include,
            exclude: &exclude,
            force_exclude,
            respect_ignore: false,
            root,
        };
        let inputs: Vec<String> = inputs
            .iter()
            .map(|input| {
                if *input == CURRENT_DIRECTORY {
                    root.display().to_string()
                } else {
                    format!("{}/{input}", root.display())
                }
            })
            .collect();
        let prefix = format!("{}/", normalize_path(root));
        discover(&inputs, &options)
            .unwrap()
            .iter()
            .map(|path| normalize_path(path).replace(&prefix, ""))
            .collect()
    }

    #[test]
    fn walks_directories_with_default_include() {
        let directory = tree();
        assert_eq!(
            run(directory.path(), &["."], &[], false),
            vec![
                "a.odcs.yaml",
                "domains/sales/orders.odcs.yaml",
                "domains/sandbox/tmp.odcs.yml"
            ]
        );
    }

    #[test]
    fn applies_exclude_while_walking() {
        let directory = tree();
        let found = run(directory.path(), &["domains"], &["**/sandbox/**"], false);
        assert_eq!(found, vec!["domains/sales/orders.odcs.yaml"]);
    }

    #[test]
    fn expands_glob_inputs() {
        let directory = tree();
        let found = run(directory.path(), &["domains/**/*.yaml"], &[], false);
        assert_eq!(
            found,
            vec!["domains/sales/notes.yaml", "domains/sales/orders.odcs.yaml"]
        );
    }

    #[test]
    fn explicit_files_bypass_include() {
        let directory = tree();
        let found = run(directory.path(), &["domains/sales/notes.yaml"], &[], false);
        assert_eq!(found, vec!["domains/sales/notes.yaml"]);
    }

    #[test]
    fn force_exclude_applies_to_explicit_files() {
        let directory = tree();
        let found = run(
            directory.path(),
            &["domains/sandbox/tmp.odcs.yml"],
            &["**/sandbox/**"],
            true,
        );
        assert!(found.is_empty());
    }

    #[test]
    fn deduplicates_overlapping_inputs() {
        let directory = tree();
        let found = run(
            directory.path(),
            &["domains/sales", "domains/sales/orders.odcs.yaml"],
            &[],
            false,
        );
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn missing_path_is_an_error() {
        let include = GlobList::empty();
        let options = Discovery {
            include: &include,
            exclude: &include,
            force_exclude: false,
            respect_ignore: true,
            root: Path::new(""),
        };
        let error = discover(&["does/not/exist".to_string()], &options).unwrap_err();
        assert!(matches!(error, Error::PathNotFound(_)));
    }

    #[test]
    fn glob_root_is_literal_prefix() {
        assert_eq!(glob_root("domains/*/x.yaml"), PathBuf::from("domains"));
        assert_eq!(glob_root("**/*.yaml"), PathBuf::from("."));
        assert_eq!(glob_root("/abs/**"), PathBuf::from("/abs"));
    }
}
