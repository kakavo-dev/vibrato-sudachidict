use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "sudachi-vibrato-converter")]
#[command(about = "Convert SudachiDict resources to Vibrato/jpreprocess-compatible format")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Convert(ConvertArgs),
}

#[derive(Debug, Args)]
pub struct ConvertArgs {
    #[arg(long)]
    pub lex_in: PathBuf,
    #[arg(long)]
    pub lex_out: PathBuf,
    #[arg(long)]
    pub unk_in: PathBuf,
    #[arg(long)]
    pub unk_out: PathBuf,
    #[arg(long)]
    pub char_in: PathBuf,
    #[arg(long)]
    pub char_out: PathBuf,
    #[arg(long)]
    pub stats_out: PathBuf,
    #[arg(long = "lex-append")]
    pub lex_append: Vec<PathBuf>,
    #[arg(long = "char-append")]
    pub char_append: Vec<PathBuf>,
    #[arg(long = "unk-append")]
    pub unk_append: Vec<PathBuf>,
    #[arg(long, requires = "rewrite_out")]
    pub rewrite_in: Option<PathBuf>,
    #[arg(long, requires = "rewrite_in")]
    pub rewrite_out: Option<PathBuf>,
    #[arg(long = "rewrite-append", requires_all = ["rewrite_in", "rewrite_out"])]
    pub rewrite_append: Vec<PathBuf>,
}
