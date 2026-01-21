pub mod cli;
pub mod diagnostics;
pub mod emit_flow;
pub mod graph;
pub mod ir;
pub mod scan;
pub mod tools;
pub mod workspace;

use anyhow::Result;
use cli::{Cli, Commands};

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Generate(args) => workspace::generate(&args),
    }
}
