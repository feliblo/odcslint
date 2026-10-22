mod init;
mod upgrade;

use std::fs;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use clap::{Args, Parser, Subcommand, ValueEnum};
use owo_colors::OwoColorize;
use strum::IntoEnumIterator;

use crate::config;
use crate::constants::exit_code;
use crate::diagnostic::Severity;
use crate::discovery::{discover, Discovery};
use crate::engine::{check_files, FileReport};
use crate::error::{Error, Result};
use crate::report::{render, OutputFormat, Summary, TextOptions, FIXABLE_MARKER};
use crate::rules::allowed_values::{Alias, ALIASES};
use crate::rules::RuleType;

const NO_COLOR_ENV: &str = "NO_COLOR";
const CURRENT_DIRECTORY: &str = ".";

/// A fast linter and fixer for Open Data Contract Standard (ODCS) contracts.
#[derive(Debug, Parser)]
#[command(name = "odcslint", author, version, about, propagate_version = true)]
pub struct Cli {
    /// Print debug information (config used, timings).
    #[arg(short, long, global = true)]
    pub verbose: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Validate contracts against the ODCS schema and the configured rules.
    Check(CheckArgs),
    /// Write a starter config file.
    Init(InitArgs),
    /// List the available rule types.
    Rules,
    /// Move contracts to a newer ODCS version, converting breaking changes where possible.
    Upgrade(UpgradeArgs),
}

#[derive(Debug, Args)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each bool is an independent CLI flag"
)]
pub struct CheckArgs {
    /// Files, directories or glob patterns to check.
    #[arg(default_value = CURRENT_DIRECTORY)]
    pub paths: Vec<String>,
    /// Apply fixes in place (formatting and comments are preserved).
    #[arg(long)]
    pub fix: bool,
    /// Config file to use instead of discovering one in the current directory.
    #[arg(short, long)]
    pub config: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t)]
    pub output_format: OutputFormat,
    /// Write output to a file instead of stdout.
    #[arg(long)]
    pub output_file: Option<PathBuf>,
    /// Only print findings, no summary.
    #[arg(short, long)]
    pub quiet: bool,
    /// Do not report warnings.
    #[arg(long)]
    pub hide_warnings: bool,
    /// Exit with 0 even if errors are found.
    #[arg(long)]
    pub exit_zero: bool,
    /// Apply `settings.exclude` to files passed explicitly (useful for prek hooks).
    #[arg(long)]
    pub force_exclude: bool,
    /// Do not respect .gitignore, .ignore and hidden-file rules.
    #[arg(long)]
    pub no_ignore: bool,
    /// Succeed when no contract files are found.
    #[arg(long)]
    pub allow_empty: bool,
}

#[derive(Debug, Args)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each bool is an independent CLI flag"
)]
pub struct UpgradeArgs {
    /// Files, directories or glob patterns to upgrade.
    #[arg(default_value = CURRENT_DIRECTORY)]
    pub paths: Vec<String>,
    /// ODCS version to upgrade to, e.g. v3.2.0 (default: the newest supported version).
    #[arg(long)]
    pub to: Option<String>,
    /// Show what would change without writing any file.
    #[arg(long)]
    pub dry_run: bool,
    /// Also write files that still need manual changes, and list those changes.
    #[arg(long)]
    pub allow_partial: bool,
    /// Config file to use instead of discovering one in the current directory.
    #[arg(short, long)]
    pub config: Option<PathBuf>,
    /// Apply `settings.exclude` to files passed explicitly.
    #[arg(long)]
    pub force_exclude: bool,
    /// Do not respect .gitignore, .ignore and hidden-file rules.
    #[arg(long)]
    pub no_ignore: bool,
    /// Succeed when no contract files are found.
    #[arg(long)]
    pub allow_empty: bool,
}

#[derive(Debug, Args)]
pub struct InitArgs {
    /// Directory to write the config file to.
    #[arg(long, default_value = CURRENT_DIRECTORY)]
    pub location: PathBuf,
    #[arg(long, value_enum, default_value_t)]
    pub format: ConfigFormat,
    /// Overwrite an existing config file.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum ConfigFormat {
    #[default]
    Yaml,
    Toml,
}

pub fn run(cli: &Cli) -> Result<u8> {
    match &cli.command {
        Command::Check(args) => check(args),
        Command::Init(args) => init::run(args),
        Command::Rules => {
            write_output(None, &rules_table(stdout_has_color()))?;
            Ok(exit_code::SUCCESS)
        }
        Command::Upgrade(args) => upgrade::run(args),
    }
}

/// Inputs shared by commands that work on discovered contracts.
struct Selection<'a> {
    paths: &'a [String],
    config: Option<&'a PathBuf>,
    force_exclude: bool,
    no_ignore: bool,
    allow_empty: bool,
}

fn load_and_discover(selection: &Selection<'_>) -> Result<(config::Config, Vec<PathBuf>)> {
    let config_path = match selection.config {
        Some(path) => Some(path.clone()),
        None => {
            config::find(Path::new(CURRENT_DIRECTORY))?.map(|found| relative_to_current(&found))
        }
    };
    log::debug!(
        "config: {}",
        config_path
            .as_deref()
            .map_or_else(|| "defaults".to_string(), |path| path.display().to_string())
    );
    let config = config::load(config_path.as_deref())?;
    let paths = discover(
        selection.paths,
        &Discovery {
            include: &config.settings.include,
            exclude: &config.settings.exclude,
            force_exclude: selection.force_exclude,
            respect_ignore: !selection.no_ignore,
            root: &config.root,
        },
    )?;
    if paths.is_empty() && !selection.allow_empty {
        return Err(Error::NoFiles);
    }
    Ok((config, paths))
}

fn check(args: &CheckArgs) -> Result<u8> {
    let (config, paths) = load_and_discover(&Selection {
        paths: &args.paths,
        config: args.config.as_ref(),
        force_exclude: args.force_exclude,
        no_ignore: args.no_ignore,
        allow_empty: args.allow_empty,
    })?;

    let started = Instant::now();
    let reports = check_files(&paths, &config, args.fix)?;
    log::debug!("checked {} files in {:?}", paths.len(), started.elapsed());

    let reports = if args.hide_warnings {
        without_warnings(reports)
    } else {
        reports
    };
    let color = args.output_file.is_none() && stdout_has_color();
    let output = render(
        args.output_format,
        &reports,
        TextOptions {
            quiet: args.quiet,
            color,
        },
    );
    write_output(args.output_file.as_deref(), &output)?;

    let failed = Summary::new(&reports).errors > 0 && !args.exit_zero;
    Ok(if failed {
        exit_code::FINDINGS
    } else {
        exit_code::SUCCESS
    })
}

/// `path` relative to the working directory, using `..` when the config lives in a parent.
fn relative_to_current(path: &Path) -> PathBuf {
    let Some(current) = config::absolute(Path::new(CURRENT_DIRECTORY)) else {
        return path.to_path_buf();
    };
    current
        .ancestors()
        .find(|ancestor| path.starts_with(ancestor))
        .and_then(|common| {
            let ups = current.strip_prefix(common).ok()?.components().count();
            let rest = path.strip_prefix(common).ok()?;
            Some(
                std::iter::repeat_n(Path::new(".."), ups)
                    .collect::<PathBuf>()
                    .join(rest),
            )
        })
        .unwrap_or_else(|| path.to_path_buf())
}

fn without_warnings(reports: Vec<FileReport>) -> Vec<FileReport> {
    reports
        .into_iter()
        .map(|report| FileReport {
            findings: report
                .findings
                .into_iter()
                .filter(|finding| finding.severity == Severity::Error)
                .collect(),
            ..report
        })
        .collect()
}

fn write_output(file: Option<&Path>, output: &str) -> Result<()> {
    match file {
        Some(path) => fs::write(path, output).map_err(|source| Error::Write {
            path: path.to_path_buf(),
            source,
        }),
        None => Ok(std::io::stdout().lock().write_all(output.as_bytes())?),
    }
}

fn stdout_has_color() -> bool {
    std::io::stdout().is_terminal() && std::env::var_os(NO_COLOR_ENV).is_none()
}

fn rules_table(color: bool) -> String {
    const WIDTH: usize = 25;
    let heading = |text: &str| {
        if color {
            format!("{}\n", text.bold())
        } else {
            format!("{text}\n")
        }
    };
    let dim = |text: String| {
        if color {
            text.dimmed().to_string()
        } else {
            text
        }
    };
    let rules = RuleType::iter().map(|kind| {
        let fix = if kind.fixable() { FIXABLE_MARKER } else { "" };
        format!(
            "  {:WIDTH$}{}{fix}\n",
            kind.name(),
            dim(kind.summary().to_string())
        )
    });
    let aliases = ALIASES
        .iter()
        .map(|alias| format!("  {:WIDTH$}{}\n", alias.name, dim(alias_target(alias))));
    std::iter::once(heading("Rules"))
        .chain(rules)
        .chain(std::iter::once(format!(
            "\n{}",
            heading(&format!("Aliases of {}", RuleType::AllowedValues.name()))
        )))
        .chain(aliases)
        .chain(std::iter::once(format!(
            "\n{} can be fixed with `odcslint check --fix`\n",
            FIXABLE_MARKER.trim_start()
        )))
        .collect()
}

fn alias_target(alias: &Alias) -> String {
    let role = alias
        .role
        .map(|role| format!(" (role: {role})"))
        .unwrap_or_default();
    format!("{}{role}", alias.field.path().unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_table_lists_every_rule() {
        let table = rules_table(false);
        assert!(RuleType::iter().all(|kind| table.contains(kind.name())));
    }

    #[test]
    fn rules_table_lists_every_alias() {
        let table = rules_table(false);
        assert!(ALIASES.iter().all(|alias| table.contains(alias.name)));
    }

    #[test]
    fn cli_definition_is_valid() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
