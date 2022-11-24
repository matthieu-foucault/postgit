use std::process::Command;

use anyhow::{bail, Result};

use crate::config::PostgresConfig;

pub fn run_migra(source: &PostgresConfig, target: &PostgresConfig) -> Result<String> {
    let output = Command::new("migra")
        .arg(source.to_url())
        .arg(target.to_url())
        .arg("--unsafe")
        .output()?;
    if !output.stderr.is_empty() {
        bail!("{}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
