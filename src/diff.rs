use std::process::Command;

use anyhow::{bail, Result};

use crate::config::DiffEngineConfig;

fn run_migra(source: &String, target: &String) -> Result<String> {
    let output = Command::new("migra")
        .arg(source)
        .arg(target)
        .arg("--unsafe")
        .output()?;
    if !output.stderr.is_empty() {
        bail!("{}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn run_diff_command(config: &DiffEngineConfig) -> Result<String> {
    let source = config.source.to_url();
    let target = config.target.to_url();

    match &config.command {
        Some(command) => {
            let output = Command::new("sh")
                .arg("-c")
                .arg(command)
                .arg("postgit") // The "command_name", i.e. $0
                .arg(source)
                .arg(target)
                .output()?;
            if !output.stderr.is_empty() {
                bail!("{}", String::from_utf8_lossy(&output.stderr));
            }
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        }
        None => run_migra(&source, &target),
    }
}
