use clap::Parser;
use postgres_schema_vcs::Args;
use std::process;

fn main() {
    let args = Args::parse();

    if let Err(e) = postgres_schema_vcs::run(&args) {
        eprintln!("Application error: {e}");
        process::exit(1);
    }
}
