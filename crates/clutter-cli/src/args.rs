use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Parser)]
#[command(name = "clutter", about = "Clutter UI compiler")]
pub struct Args {
    /// Source .clutter file to compile
    pub file: PathBuf,

    /// Output directory (defaults to the source file's directory)
    #[arg(long)]
    pub out: Option<PathBuf>,

    /// Explicit path to tokens.json (auto-discovered if omitted)
    #[arg(long)]
    pub tokens: Option<PathBuf>,

    /// Compilation target
    #[arg(long, default_value = "vue")]
    pub target: Target,
}

#[derive(ValueEnum, Clone)]
pub enum Target {
    Vue,
    Html,
}
