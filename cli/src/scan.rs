use std::{path::PathBuf, process::ExitCode};

use anyhow::Result;

#[derive(clap::Parser, Debug, Clone, Eq, PartialEq)]
#[command()]
#[group(id = "scan")]
pub struct Scan {
    #[arg(id = "input", long = "input")]
    pub input: PathBuf,
    #[arg(id = "output", long = "output")]
    pub output: Option<PathBuf>,
}

impl Scan {
    pub async fn run(self) -> Result<ExitCode> {
        Ok(ExitCode::SUCCESS)
    }
}
