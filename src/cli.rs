use crate::config;
use crate::tui;
use clap::{Parser, Subcommand};
use crossterm::style;
use crossterm::style::Stylize;
use crossterm::QueueableCommand;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "tmux-fzy")]
pub(crate) struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Add {
        #[arg(long, default_value_t = 0)]
        maxdepth: u8,

        #[arg(long, default_value_t = 0)]
        mindepth: u8,

        paths: Vec<PathBuf>,
    },
    List,

    Del {
        paths: Vec<PathBuf>,
    },
}

pub(crate) fn run() {
    let mut config = config::load();
    let cli = Cli::parse();

    match cli.command {
        None => {
            tui::run(&mut config);
        }
        Some(cmd) => match cmd {
            Commands::Add {
                mindepth,
                maxdepth,
                paths,
            } => {
                for i in paths {
                    if i.is_dir() {
                        if let Err(err) = config.write_single_path((i, (mindepth, maxdepth))) {
                            eprintln!("{}", err);
                        }
                    } else {
                        eprintln!("Invalid path: {:?}", i);
                    }
                }
            }
            Commands::List => {
                for (path, (min, max)) in config.file_paths {
                    let stdout = || -> Result<(), std::io::Error> {
                        std::io::stdout()
                            .queue(style::PrintStyledContent("[min depth: ".green()))?
                            .queue(style::PrintStyledContent(min.to_string().green()))?
                            .queue(style::PrintStyledContent(", max depth: ".green()))?
                            .queue(style::PrintStyledContent(max.to_string().green()))?
                            .queue(style::PrintStyledContent("]".green()))?;

                        Ok(())
                    };

                    stdout().expect("Failed to print list");

                    print!(" => {}", path.to_str().unwrap());

                    println!();
                }
            }
            Commands::Del { paths } => {
                for path in paths {
                    config.file_paths.remove_entry(&path);
                }
                config.write_all().expect("Failed to Write to the file");
            }
        },
    }
}
