use clap::Parser;
use postgit::{Cli, Commands};
use std::process;

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Diff(args) => {
            if let Err(e) = postgit::run(args) {
                eprintln!("Application error: {e}");
                process::exit(1);
            }
        }
    }
}
