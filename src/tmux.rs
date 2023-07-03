use std::path::PathBuf;
use std::process::{Command, Stdio};

pub(crate) fn run(path: PathBuf) -> Result<(), std::io::Error> {
    let basename = path.file_name().unwrap().to_str().unwrap();
    let tmux = Tmux::new(path.to_str().unwrap().to_string(), basename.to_string());

    match (tmux.is_running()?, tmux.env()) {
        (false, false) => {
            tmux.new_session()?;
            Ok(())
        }
        (true, false) => {
            if tmux.has_session()? {
                tmux.attach()?;
            } else {
                tmux.new_session()?;
            }
            Ok(())
        }
        (true, true) => {
            if tmux.has_session()? {
                tmux.switch()?;
            } else {
                tmux.new_session_detach()?;
                tmux.switch()?;
            }
            Ok(())
        }
        _ => unreachable!(),
    }
}

struct Tmux {
    path: String,
    basename: String,
}

pub(crate) fn kill_session(path: PathBuf) -> Result<(), std::io::Error> {
    let basename = path.file_name().unwrap().to_str().unwrap();
    Command::new("tmux")
        .arg("kill-session")
        .arg("-t")
        .arg(basename)
        .output()?;

    Ok(())
}

pub(crate) fn sessions() -> Vec<String> {
    let output = Command::new("tmux")
        .arg("ls")
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to run tmux ls");

    let stdout = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| {
            line.split_once(' ')
                .unwrap()
                .0
                .strip_suffix(':')
                .unwrap()
                .to_string()
        })
        .collect::<Vec<String>>();

    stdout
}

impl Tmux {
    fn new(path: String, basename: String) -> Self {
        Self { path, basename }
    }

    fn env(&self) -> bool {
        std::env::var("TMUX").is_ok()
    }

    fn is_running(&self) -> Result<bool, std::io::Error> {
        let output = Command::new("pgrep")
            .arg("tmux")
            .stdout(Stdio::piped())
            .output()?;

        if output.status.success() {
            Ok(!String::from_utf8_lossy(&output.stdout).is_empty())
        } else {
            Ok(false)
        }
    }

    fn has_session(&self) -> Result<bool, std::io::Error> {
        let output = Command::new("tmux")
            .arg("has-session")
            .arg("-t")
            .arg(&self.basename)
            .output()?;

        Ok(output.status.success())
    }

    fn new_session(&self) -> Result<(), std::io::Error> {
        Command::new("tmux")
            .args(["new-session", "-s", &self.basename, "-c", &self.path])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()?;
        Ok(())
    }

    fn new_session_detach(&self) -> Result<(), std::io::Error> {
        Command::new("tmux")
            .args(["new-session", "-ds", &self.basename, "-c", &self.path])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()?;
        Ok(())
    }

    fn attach(&self) -> Result<(), std::io::Error> {
        Command::new("tmux")
            .args(["attach", "-t", &self.basename])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()?;
        Ok(())
    }

    fn switch(&self) -> Result<(), std::io::Error> {
        Command::new("tmux")
            .args(["switch-client", "-t", &self.basename])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()?;
        Ok(())
    }
}
