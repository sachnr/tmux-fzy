use std::process::{Command, Output, Stdio};

use crate::Error;

/// Check if tmux is running
pub fn status() -> Result<bool, Error> {
    let status = Command::new("pgrep")
        .arg("tmux")
        .output()
        .map_err(|e| Error::UnexpectedError(e.into()))?
        .status
        .success();

    Ok(status)
}

/// Check if the 'TMUX' env variable is set
pub fn env() -> bool {
    std::env::var("TMUX").is_ok()
}

pub fn has_session(session_name: &str) -> Result<bool, Error> {
    let status = CommandBuilder::new()
        .args(vec!["has-session", "-t", session_name])
        .run()?;

    Ok(status)
}

pub fn kill_session(session_name: &str) -> Result<(), Error> {
    CommandBuilder::new()
        .args(vec!["kill-session", "-t", session_name])
        .run()?;

    Ok(())
}

/// lists all active sessions
pub fn list_sessions() -> Result<Vec<String>, Error> {
    let output = CommandBuilder::new()
        .args(vec!["ls", "-F", "'#{session_name}'"])
        .run_capture_output()?;

    let sessions = output
        .lines()
        .map(|line| {
            let session = line.trim();
            let prefix = session.strip_prefix('\'').unwrap();
            let suffix = prefix.strip_suffix('\'').unwrap();
            suffix.to_string()
        })
        .collect::<Vec<String>>();

    Ok(sessions)
}

/// Detach from the current session and start a new session, useful when
/// you are inside a tmux session
pub fn switch_client(session_name: &str) -> Result<(), Error> {
    CommandBuilder::new()
        .args(vec!["switch-client", "-t", session_name])
        .run_inherit_stdio()?;

    Ok(())
}

/// attach to a new session, useful when you are outside a tmux session
pub fn attach(session_name: &str) -> Result<(), Error> {
    CommandBuilder::new()
        .args(vec!["attach", "-t", session_name])
        .run_inherit_stdio()?;

    Ok(())
}

pub fn new_session(session_name: &str, path: &str) -> Result<(), Error> {
    CommandBuilder::new()
        .args(vec!["new-session", "-s", session_name, "-c", path])
        .run_inherit_stdio()?;

    Ok(())
}

/// don't attach new session to current terminal
pub fn new_session_detach(session_name: &str, path: &str) -> Result<(), Error> {
    CommandBuilder::new()
        .args(vec!["new-session", "-ds", session_name, "-c", path])
        .run_inherit_stdio()?;

    Ok(())
}

/// Helper to run tmux commands
///
/// Example
///
/// ```no_run
/// pub fn ls() -> Result<(), Error> {
///     CommandBuilder::new()
///         .args(vec!["ls"])
///         .run()?;
///     Ok(())
/// }
/// ```
pub struct CommandBuilder<'a> {
    args: Vec<&'a str>,
}

impl<'a> CommandBuilder<'a> {
    pub fn new() -> CommandBuilder<'a> {
        CommandBuilder { args: Vec::new() }
    }

    pub fn arg(mut self, s: &'a str) -> Self {
        self.args.push(s);
        self
    }

    pub fn args(mut self, s: Vec<&'a str>) -> Self {
        self.args.extend(s);
        self
    }

    pub fn run(self) -> Result<bool, Error> {
        let command = Command::new("tmux")
            .args(self.args)
            .output()
            .map_err(|e| Error::UnexpectedError(e.into()))?
            .status
            .success();

        Ok(command)
    }

    pub fn run_capture_output(self) -> Result<String, Error> {
        let command = Command::new("tmux")
            .args(self.args)
            .stdout(Stdio::piped())
            .output()
            .map_err(|e| Error::UnexpectedError(e.into()))?;

        let stdout = String::from_utf8_lossy(&command.stdout);
        let output = stdout.to_string();

        Ok(output)
    }

    pub fn run_inherit_stdio(self) -> Result<Output, Error> {
        let command = Command::new("tmux")
            .args(self.args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| Error::UnexpectedError(e.into()))?;
        Ok(command)
    }
}
