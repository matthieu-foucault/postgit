use std::error::Error;

use clap::Parser;

/// Migrate from source to target postgres schemas
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to the root of the git repository
    #[arg(long, short)]
    pub repo_path: String,

    /// Git ref where the source schema can be found
    #[arg(long, short)]
    pub source_ref: String,

    /// Git ref where the target schema can be found
    #[arg(long, short)]
    pub target_ref: String,

    /// Path to the source schema at the source ref
    #[arg(long)]
    pub source_path: String,

    /// Path to the target schema at the target ref
    #[arg(long)]
    pub target_path: String,
}

pub fn run(args: &Args) -> Result<(), Box<dyn Error>> {
    Ok(())
}
