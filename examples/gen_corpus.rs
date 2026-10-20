//! Usage: `cargo run --release --example gen_corpus -- <count> <output-dir>`

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const SOURCE: &str = "tests/fixtures/upstream/examples";
const DEFAULT_COUNT: usize = 10_000;
const DEFAULT_OUTPUT: &str = "target/corpus";
const ID_PREFIX: &str = "id:";

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let count = args
        .first()
        .and_then(|count| count.parse().ok())
        .unwrap_or(DEFAULT_COUNT);
    let output = PathBuf::from(args.get(1).map_or(DEFAULT_OUTPUT, String::as_str));

    let templates = read_examples(Path::new(SOURCE))?;
    if templates.is_empty() {
        return Err(io::Error::other(format!("no examples found in {SOURCE}")));
    }
    fs::create_dir_all(&output)?;
    (0..count).try_for_each(|index| {
        let template = &templates[index % templates.len()];
        fs::write(
            output.join(format!("contract-{index:06}.odcs.yaml")),
            with_id(template, index),
        )
    })?;
    println!("wrote {count} contracts to {}", output.display());
    Ok(())
}

fn read_examples(directory: &Path) -> io::Result<Vec<String>> {
    fs::read_dir(directory)?.try_fold(Vec::new(), |mut templates, entry| {
        let path = entry?.path();
        if path.is_dir() {
            templates.extend(read_examples(&path)?);
        } else if path.to_string_lossy().ends_with(".odcs.yaml") {
            templates.push(fs::read_to_string(&path)?);
        }
        Ok(templates)
    })
}

fn with_id(template: &str, index: usize) -> String {
    template
        .lines()
        .map(|line| {
            if line.starts_with(ID_PREFIX) {
                format!("{ID_PREFIX} 00000000-0000-4000-8000-{index:012x}")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}
