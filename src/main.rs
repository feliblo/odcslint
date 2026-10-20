use std::process::ExitCode;

use clap::Parser;
use odcslint::cli::{run, Cli};
use odcslint::constants::exit_code;

const DEFAULT_LOG_LEVEL: &str = "warn";
const VERBOSE_LOG_LEVEL: &str = "debug";

fn main() -> ExitCode {
    let cli = Cli::parse();
    let level = if cli.verbose {
        VERBOSE_LOG_LEVEL
    } else {
        DEFAULT_LOG_LEVEL
    };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(level))
        .format_timestamp(None)
        .init();
    match run(&cli) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(exit_code::TOOL_ERROR)
        }
    }
}
