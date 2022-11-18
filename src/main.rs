use clap::Parser;
use postgit::Args;
use std::process;

fn main() {
    let args = Args::parse();

    if let Err(e) = postgit::run(&args) {
        eprintln!("Application error: {e}");
        process::exit(1);
    }
}
