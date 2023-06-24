use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;
use thiserror::Error;
use walkdir::WalkDir;

pub(crate) fn load() -> Config {
    let mut config = Config::new();
    match config.read() {
        Ok(config) => config,
        Err(err) => eprintln!("{}", err),
    };
    config
}

#[derive(Debug, Error)]
pub(crate) enum ConfErrs {
    #[error("Failed to determine the configuration directory")]
    ConfigDir,

    #[error("Failed to open or create the file: {0}")]
    #[from(io::Error)]
    FileOpen(#[source] std::io::Error),

    #[error("Failed to write to the file: {0}")]
    #[from(io::Error)]
    FileWrite(#[source] std::io::Error),

    #[error("Failed to read the file: {0}")]
    #[from(io::Error)]
    Read(#[source] std::io::Error),

    #[error("Failed to deserialize the data: {0}")]
    #[from(toml::de::Error)]
    Deserialization(#[source] toml::de::Error),

    #[error("Failed to serialize the data: {0}")]
    #[from(toml::ser::Error)]
    Serialization(#[source] toml::ser::Error),
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub(crate) struct Config {
    pub(crate) file_paths: HashMap<PathBuf, (u8, u8)>,
}

impl Config {
    fn new() -> Self {
        Self::default()
    }

    pub fn read(&mut self) -> Result<(), ConfErrs> {
        let config_path = dirs::config_dir()
            .ok_or(ConfErrs::ConfigDir)?
            .join("tmux-fzy/config.toml");

        if !config_path.exists() {
            std::fs::create_dir_all(config_path.parent().unwrap()).map_err(ConfErrs::FileOpen)?;
            let mut file = File::create(&config_path).map_err(ConfErrs::FileOpen)?;
            let empty = Config::new();
            let empty_to_string =
                toml::to_string_pretty(&empty).map_err(ConfErrs::Serialization)?;
            file.write_all(empty_to_string.as_bytes())
                .map_err(ConfErrs::FileWrite)?;
        }

        let mut file = File::open(&config_path).map_err(ConfErrs::FileOpen)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(ConfErrs::Read)?;
        let config: Config = toml::from_str(&contents).map_err(ConfErrs::Deserialization)?;
        self.file_paths = config.file_paths;
        Ok(())
    }

    pub(crate) fn write_single_path(&mut self, path: (PathBuf, (u8, u8))) -> Result<(), ConfErrs> {
        let config_path = dirs::config_dir()
            .ok_or(ConfErrs::ConfigDir)?
            .join("tmux-fzy/config.toml");

        let dir_path = path.0.canonicalize().map_err(ConfErrs::FileWrite)?;
        if self.file_paths.get(&dir_path).is_none() {
            self.file_paths.insert(dir_path, (path.1 .0, path.1 .1));
        }

        let mut file = OpenOptions::new()
            .append(false)
            .write(true)
            .open(config_path)
            .map_err(ConfErrs::FileOpen)?;

        let content = toml::to_string_pretty(&self).map_err(ConfErrs::Serialization)?;
        file.write_all(content.as_bytes())
            .map_err(ConfErrs::FileWrite)?;
        Ok(())
    }

    pub(crate) fn write_all(&mut self) -> Result<(), ConfErrs> {
        let config_path = dirs::config_dir()
            .ok_or(ConfErrs::ConfigDir)?
            .join("tmux-fzy/config.toml");

        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(config_path)
            .map_err(ConfErrs::FileOpen)?;

        let content = toml::to_string_pretty(&self).map_err(ConfErrs::Serialization)?;
        file.write_all(content.as_bytes())
            .map_err(ConfErrs::FileWrite)?;
        Ok(())
    }

    pub(crate) fn expand(&self) -> Vec<PathBuf> {
        let mut directories = Vec::new();
        for (paths, (min, max)) in &self.file_paths {
            directories.extend(
                WalkDir::new(paths)
                    .min_depth(*min as usize)
                    .max_depth(*max as usize)
                    .into_iter()
                    .filter_map(|entry| {
                        let entry = entry.ok()?;
                        let path = entry.path().to_owned(); // Clone the PathBuf
                        if entry.file_type().is_dir() {
                            Some(path)
                        } else {
                            None
                        }
                    }),
            );
        }
        directories
    }
}
