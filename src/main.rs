use clap::Parser;
use miette::{Context, IntoDiagnostic, Result};
use std::fs;
use std::path::PathBuf;

mod validator;

use validator::validate_json;

#[derive(Parser)]
#[command(name = "ferrite")]
#[command(about = "json validator that actually tells you how to fix your mistakes", long_about = None)]
struct Cli {
    #[arg(value_name = "FILE")]
    file: PathBuf,

    #[arg(short, long, default_value_t = 2)]
    context: usize,
}

fn main() -> Result<()> {
    miette::set_panic_hook();
    let cli = Cli::parse();
    let content = fs::read_to_string(&cli.file)
        .into_diagnostic()
        .wrap_err_with(|| format!("Failed to read file '{}'", cli.file.display()))?;
    validate_json(
        &content,
        cli.file.to_string_lossy().to_string(),
        cli.context,
    )?;
    println!("{} is valid!", cli.file.display());
    Ok(())
}
