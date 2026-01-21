use clap::Parser;
use greentic_cards2pack::cli::Cli;
use greentic_cards2pack::run;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    run(cli)
}
