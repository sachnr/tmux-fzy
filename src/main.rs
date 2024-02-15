use crossterm::style::Stylize;
use crossterm::{execute, style::Print};
use startup::run;

mod cli;
mod config;
mod startup;
mod tmux;
mod tui;
mod tui_components;

fn main() -> Result<(), anyhow::Error> {
    if let Err(err) = run() {
        execute!(std::io::stderr(), Print("Error: ".red()))?;
        for cause in err.chain() {
            execute!(std::io::stderr(), Print(cause), Print("\n"))?;
        }
    }
    Ok(())
}
