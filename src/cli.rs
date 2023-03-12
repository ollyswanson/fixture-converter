use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
pub struct Cli {
    /// The directory containing XML files to convert to JSON.
    pub input: PathBuf,
    /// The directory to output JSON to.
    pub output: PathBuf,
}

pub fn get_args() -> Cli {
    Cli::parse()
}
