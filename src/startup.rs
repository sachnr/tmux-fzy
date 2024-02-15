use clap::Parser;
use crossterm::{
    execute,
    style::{Print, Stylize},
};

use crate::{
    cli::{Cli, Commands},
    tui::{reset_terminal, start_tui},
};

pub fn run() -> Result<(), anyhow::Error> {
    let colors = crate::config::init_colors();
    let mut pathlist = crate::config::get_paths()?;
    let cli = Cli::parse();

    match cli.command {
        None => {
            if let Err(err) = start_tui(pathlist, colors) {
                reset_terminal()?;
                execute!(std::io::stderr(), Print("Error: ".red()))?;
                for cause in err.chain() {
                    execute!(std::io::stderr(), Print(cause), Print("\n"))?;
                }
            } else {
                reset_terminal()?;
            }
        }

        Some(Commands::List) => {
            for (i, entry) in pathlist.entries.iter().enumerate() {
                let i = format!("{}:", i);
                execute!(
                    std::io::stdout(),
                    Print(i.blue()),
                    Print(entry.path.to_string_lossy()),
                    Print(", min_depth: ".green()),
                    Print(entry.min_depth),
                    Print(", max_depth: ".green()),
                    Print(entry.max_depth),
                    Print("\n")
                )?;
            }
        }

        Some(Commands::Add {
            maxdepth,
            mindepth,
            paths,
        }) => {
            for path in paths {
                let full_path = path.canonicalize()?;
                pathlist.insert_row(full_path, mindepth, maxdepth)
            }
            pathlist.save_configuration()?;
        }

        Some(Commands::Del { paths }) => {
            pathlist.remove_paths(paths)?;
            pathlist.save_configuration()?;
        }
    }
    Ok(())
}
