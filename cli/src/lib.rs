use anyhow::Result;
use std::process::ExitCode;

use crate::{
    auth::{Login, Logout},
    scan::Scan,
};

mod auth;
mod scan;

#[derive(clap::Args, Debug)]
pub struct Run {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(clap::Subcommand, Debug)]
pub enum Command {
    Login(Login),
    Logout(Logout),
    Scan(Scan),
}

impl Run {
    pub async fn run(self) -> Result<ExitCode> {
        match self.command {
            Command::Login(login) => login.run().await,
            Command::Logout(logout) => logout.run().await,
            Command::Scan(scan) => scan.run().await,
        }
    }
}
