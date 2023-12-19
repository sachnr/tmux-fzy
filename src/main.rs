use crossterm::{execute, style::Print};
use std::io::stderr;
use tmux_fzy::{get_paths, start};

fn main() {
    match get_paths() {
        Ok(mut paths) => {
            if let Err(err) = start(&mut paths) {
                execute!(
                    stderr(),
                    Print(crossterm::style::Stylize::bold(
                        crossterm::style::Stylize::red("Error: ")
                    )),
                    Print(format!("{:?}", err))
                ).expect("Failed to start");
            }
        }
        Err(err) => {
            execute!(
                stderr(),
                Print(crossterm::style::Stylize::bold(
                    crossterm::style::Stylize::red("Error: ")
                )),
                Print(format!("{:?}", err))
            ).expect("Failed");
        }
    }
}
