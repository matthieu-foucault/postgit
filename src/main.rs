use clap::Parser;
use postgres_schema_vcs::Args;
use std::process;

fn main() {
    let args = Args::parse();

    println!(
        "Migrating in {} from {}@{} to {}@{}!",
        args.repo_path, args.source_path, args.source_ref, args.target_path, args.target_ref
    );

    if let Err(e) = postgres_schema_vcs::run(&args) {
        eprintln!("Application error: {e}");
        process::exit(1);
    }
}
