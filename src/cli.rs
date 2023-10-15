use clap::{Parser, Subcommand};
use crossterm::{
    execute,
    style::{Print, Stylize},
};
use std::{
    io::{stderr, stdout},
    path::PathBuf,
};

use crate::{config::Configuration, render, Error};

#[derive(Parser)]
#[command(name = "tmux-fzy")]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
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

pub fn start(config: &mut Configuration) -> Result<(), Error> {
    let cli = Cli::parse();

    match cli.command {
        None => {
            let res = render(config);
            if let Err(err) = res {
                execute!(
                    stderr(),
                    Print(crossterm::style::Stylize::bold(
                        crossterm::style::Stylize::red("Error: ")
                    )),
                    Print(format!("{:?}", err))
                )
                .map_err(|e| Error::UnexpectedError(e.into()))?
            }

            Ok(())
        }
        Some(cmd) => match cmd {
            Commands::Add {
                maxdepth,
                mindepth,
                paths,
            } => {
                for path in paths {
                    let path = path
                        .canonicalize()
                        .map_err(|e| Error::UnexpectedError(e.into()))?
                        .to_str()
                        .unwrap()
                        .to_string();

                    config.insert_row(path, mindepth, maxdepth);
                }
                config.save_configuration().expect("Failed to save");
                Ok(())
            }
            Commands::Del { paths } => {
                for path in paths {
                    let path = path
                        .canonicalize()
                        .map_err(|e| Error::UnexpectedError(e.into()))?;
                    let path = path.to_str().unwrap();
                    _ = config.0.remove(path);
                    config.save_configuration()?;
                }
                Ok(())
            }
            Commands::List => {
                for (i, (path, values)) in config.0.iter().enumerate() {
                    execute!(
                        stdout(),
                        Print(format!("{}: ", i + 1).blue()),
                        Print(path),
                        Print("\tsearch_depth:[".blue()),
                        Print("min:"),
                        Print(values.min_depth),
                        Print(", max:"),
                        Print(values.max_depth),
                        Print("]".blue()),
                        Print("\n"),
                    )
                    .map_err(|e| Error::UnexpectedError(e.into()))?;
                }
                Ok(())
            }
        },
    }
}
