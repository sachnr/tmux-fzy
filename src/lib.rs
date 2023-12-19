mod cli;
mod config;
#[allow(dead_code)]
mod tmux;
mod tui;

use std::path::PathBuf;

pub use cli::start;
pub use config::{get_paths, AppColors, Paths};
pub use tui::*;

#[derive(thiserror::Error)]
pub enum Error {
    #[error("Failed to parse config file: {0}")]
    ParseError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

pub fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}

pub fn start_tmux(path: &str) -> Result<(), Error> {
    let pathbuf = PathBuf::from(path);
    let session_name = pathbuf
        .file_name()
        .ok_or(anyhow::anyhow!("Failed to get session_name from filepath."))?
        .to_str()
        .ok_or(anyhow::anyhow!("session_name is not a valid utf8 string"))?;

    let tmux_running = tmux::status()?;
    let tmux_env = tmux::env();
    let tmux_has_session = tmux::has_session(session_name)?;

    match (tmux_running, tmux_env) {
        (false, false) => tmux::new_session(session_name, path)?,
        (true, false) => {
            if tmux_has_session {
                tmux::attach(session_name)?;
            } else {
                tmux::new_session(session_name, path)?;
            }
        }
        (true, true) => {
            if tmux_has_session {
                tmux::switch_client(session_name)?;
            } else {
                tmux::new_session_detach(session_name, path)?;
                tmux::switch_client(session_name)?;
            }
        }
        (false, true) => {}
    }

    Ok(())
}

pub fn switch_sessions(session_name: &str) -> Result<(), Error> {
    let tmux_env = tmux::env();

    if tmux_env {
        tmux::switch_client(session_name)?;
    } else {
        tmux::attach(session_name)?;
    }

    Ok(())
}
