use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Args)]
pub struct DiffArgs {
    /// Path to the root of the git repository
    #[arg(long, short, default_value = ".")]
    pub repo_path: String,

    /// Git revision where the source schema can be found.
    /// This may be omitted for the first migration, when the database is empty
    #[arg(long, short)]
    pub from: Option<String>,

    /// Git revision where the target schema can be found.
    #[arg(long, short)]
    pub to: String,

    /// Path to the source schema at the source ref, if different from the target path
    #[arg(long)]
    pub source_path: Option<String>,

    /// Path to the schema file or directory, relative to the repo root
    pub path: String,
}

#[derive(Args)]
pub struct WatchArgs {
    /// Path to the directory to watch
    pub path: String,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Shows the migration diff between two schemas
    Diff(DiffArgs),
    /// Calculates the migration diff between two schemas and applies it to the target database
    Push(DiffArgs),
    /// Watches a directory and applies the migrations to the target database
    Watch(WatchArgs),
}
