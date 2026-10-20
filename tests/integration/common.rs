use std::fs;
use std::path::{Path, PathBuf};

use odcslint::config::Config;

pub const FIXTURES: &str = "tests/fixtures";
pub const UPSTREAM_EXAMPLES: &str = "tests/fixtures/upstream/examples";
pub const UPSTREAM_NEGATIVE: &str = "tests/fixtures/upstream/negative-tests";

pub const VALID_CONTRACT: &str = "\
apiVersion: v3.1.0
kind: DataContract
id: 53581432-6c55-4ba2-a65f-72344a91553c
version: 1.0.0
status: active
";

pub fn contracts(directory: &str) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = walk(Path::new(directory))
        .into_iter()
        .filter(|path| path.to_string_lossy().ends_with(".odcs.yaml"))
        .collect();
    paths.sort();
    paths
}

fn walk(directory: &Path) -> Vec<PathBuf> {
    fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .flat_map(|path| {
            if path.is_dir() {
                walk(&path)
            } else {
                vec![path]
            }
        })
        .collect()
}

pub fn config(yaml: &str) -> Config {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("odcslint.yml");
    fs::write(&path, yaml).unwrap();
    odcslint::config::load(Some(&path)).unwrap()
}
