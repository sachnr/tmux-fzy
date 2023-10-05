use crossterm::{execute, style::Print};
use std::io::stderr;
use tmux_fzy::{get_configuration, start};

fn main() {
    let mut config = get_configuration().unwrap();

    if let Err(err) = start(&mut config) {
        execute!(
            stderr(),
            Print(crossterm::style::Stylize::bold(
                crossterm::style::Stylize::red("Error: ")
            )),
            Print(format!("{:?}", err))
        )
        .expect("Failed to start.");
    }
}
