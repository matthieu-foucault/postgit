use clap::Parser;
use postgit::{Cli, Commands, Config};
use std::process;

fn main() {
    let cli = Cli::parse();
    let config = match Config::build() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error while loading the config: {e}");
            process::exit(1);
        }
    };

    match &cli.command {
        Commands::Diff(args) => match postgit::get_diff_string(args, &config) {
            Ok(diff_string) => {
                println!("{diff_string}");
            }
            Err(e) => {
                eprintln!("Application error: {e}");
                process::exit(1);
            }
        },
        Commands::Push(args) => {
            if let Err(e) = postgit::apply_diff(args, &config) {
                eprintln!("Application error: {e}");
                process::exit(1);
            }
        }
        Commands::Watch(args) => {
            if let Err(e) = postgit::watch(args, &config) {
                eprintln!("Application error: {e}");
                process::exit(1);
            }
        }
    }
}
