use std::{
    collections::HashMap,
    env,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
    str::FromStr,
};

use walkdir::WalkDir;

use crate::Error;

#[derive(Debug)]
pub struct Configuration(pub HashMap<String, Flags>);

#[derive(Debug)]
pub struct Flags {
    pub min_depth: usize,
    pub max_depth: usize,
}

impl Configuration {
    pub fn insert_row(&mut self, path: String, min_depth: usize, max_depth: usize) {
        self.0.entry(path).or_insert(Flags {
            min_depth,
            max_depth,
        });
    }

    pub fn save_configuration(&self) -> Result<(), Error> {
        let config_dir =
            get_config_dir().ok_or(anyhow::anyhow!("Failed to locate the config directory."))?;
        let file_path = config_dir.join(".tmux-fzy");

        let c = self.to_string();

        let mut file = OpenOptions::new()
            .append(false)
            .write(true)
            .truncate(true)
            .open(file_path)
            .map_err(|e| Error::UnexpectedError(e.into()))?;

        file.write_all(c.as_bytes())
            .map_err(|e| Error::UnexpectedError(e.into()))?;

        Ok(())
    }

    pub fn expand_paths(&self) -> Vec<String> {
        let mut dirs = Vec::new();
        for (
            path,
            Flags {
                min_depth,
                max_depth,
            },
        ) in self.0.iter()
        {
            let directories = WalkDir::new(path)
                .min_depth(*min_depth)
                .max_depth(*max_depth)
                .into_iter()
                .filter_map(|entry| {
                    let entry = entry.ok()?;
                    let path = entry.path().to_owned();
                    if entry.file_type().is_dir() {
                        Some(path.to_str().unwrap().to_owned())
                    } else {
                        None
                    }
                });
            dirs.extend(directories);
        }
        dirs
    }
}

impl FromStr for Configuration {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut map = HashMap::new();
        for (i, line) in s.lines().enumerate() {
            let values: Vec<&str> = line.split(":|:").collect();

            if values.len() != 3 {
                return Err("Invalid number of values".into());
            }

            let _ = PathBuf::from_str(values[1])
                .map_err(|_| format!("Error on line {}, invalid path", i))?;
            let min_depth: usize = values[1]
                .parse()
                .map_err(|_| format!("Error on line {}, invalid min_depth", i))?;
            let max_depth: usize = values[2]
                .parse()
                .map_err(|_| format!("Error on line {}, invalid max_depth", i))?;
            map.entry(values[0].to_string()).or_insert(Flags {
                min_depth,
                max_depth,
            });
        }
        Ok(Configuration(map))
    }
}

impl ToString for Configuration {
    fn to_string(&self) -> String {
        self.0
            .iter()
            .map(|(key, value)| format!("{}:|:{}:|:{}", key, value.min_depth, value.max_depth))
            .collect::<Vec<String>>()
            .join("\n")
    }
}

pub fn is_absolute_path(path: OsString) -> Option<PathBuf> {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        Some(path)
    } else {
        None
    }
}

pub fn get_config_dir() -> Option<PathBuf> {
    env::var_os("XDG_CACHE_HOME")
        .and_then(is_absolute_path)
        .or_else(|| {
            env::var_os("HOME")
                .map(PathBuf::from)
                .map(|h| h.join(".cache"))
        })
}

pub fn init_config(path: &PathBuf) -> Result<(), anyhow::Error> {
    let dir = path.parent().unwrap();
    if !dir.exists() {
        fs::create_dir(dir).map_err(|e| anyhow::anyhow!(e))?;
    }
    File::create(path).map_err(|e| anyhow::anyhow!(e))?;
    Ok(())
}

pub fn get_configuration() -> Result<Configuration, Error> {
    let config_dir =
        get_config_dir().ok_or(anyhow::anyhow!("Failed to locate the config directory."))?;

    let file_path = config_dir.join(".tmux-fzy");
    if !file_path.exists() {
        init_config(&file_path)?;
    }

    let mut file = File::open(&file_path).map_err(|e| Error::UnexpectedError(e.into()))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|e| Error::UnexpectedError(e.into()))?;
    let config = Configuration::from_str(&contents).map_err(Error::ParseError)?;
    Ok(config)
}
