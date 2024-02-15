use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "tmux-fzy")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Add {
        #[arg(long, default_value_t = 0)]
        maxdepth: usize,
        #[arg(long, default_value_t = 0)]
        mindepth: usize,
        paths: Vec<PathBuf>,
    },

    List,

    Del {
        paths: Vec<PathBuf>,
    },
}
