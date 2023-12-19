use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    env,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
    str::FromStr,
    sync::Mutex,
};

use ratatui::style::Color;
use walkdir::WalkDir;

use crate::Error;

#[derive(Debug)]
pub struct Paths(pub HashMap<String, Flags>);

#[derive(Debug)]
pub struct Flags {
    pub min_depth: usize,
    pub max_depth: usize,
}

impl Paths {
    pub fn insert_row(&mut self, path: String, min_depth: usize, max_depth: usize) {
        self.0.entry(path).or_insert(Flags {
            min_depth,
            max_depth,
        });
    }

    pub fn save_configuration(&self) -> Result<(), Error> {
        let paths_dir = get_paths_dir(".cache")
            .ok_or(anyhow::anyhow!("Failed to locate the config directory."))?;
        let file_path = paths_dir.join(".tmux-fzy");

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

impl FromStr for Paths {
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
        Ok(Paths(map))
    }
}

impl ToString for Paths {
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

pub fn get_paths_dir(from_home: &str) -> Option<PathBuf> {
    env::var_os("XDG_CACHE_HOME")
        .and_then(is_absolute_path)
        .or_else(|| {
            env::var_os("HOME")
                .map(PathBuf::from)
                .map(|h| h.join(from_home))
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

pub fn get_paths() -> Result<Paths, Error> {
    let config_dir =
        get_paths_dir(".cache").ok_or(anyhow::anyhow!("Failed to locate the config directory."))?;

    let file_path = config_dir.join(".tmux-fzy");
    if !file_path.exists() {
        init_config(&file_path)?;
    }

    let mut file = File::open(&file_path).map_err(|e| Error::UnexpectedError(e.into()))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|e| Error::UnexpectedError(e.into()))?;
    let paths = Paths::from_str(&contents).map_err(Error::ParseError)?;
    Ok(paths)
}

struct Colors {
    fg: Color,
    border: Color,
    inactive: Color,
    active: Color,
    selection: Color,
}

impl Colors {
    fn new() -> Colors {
        Colors {
            fg: Color::White,
            border: Color::White,
            inactive: Color::DarkGray,
            active: Color::Red,
            selection: Color::Green,
        }
    }

    fn border(&mut self, border: u8) {
        if let Some(value) = int_to_ansi_colors(border) {
            self.border = value;
        }
    }

    fn inactive(&mut self, inactive: u8) {
        if let Some(value) = int_to_ansi_colors(inactive) {
            self.inactive = value;
        }
    }

    fn active(&mut self, active: u8) {
        if let Some(value) = int_to_ansi_colors(active) {
            self.active = value;
        }
    }

    fn selection(&mut self, selection: u8) {
        if let Some(value) = int_to_ansi_colors(selection) {
            self.selection = value;
        }
    }

    fn fg(&mut self, fg: u8) {
        if let Some(value) = int_to_ansi_colors(fg) {
            self.fg = value;
        }
    }
}

fn int_to_ansi_colors(i: u8) -> Option<Color> {
    match i {
        0 => Some(Color::Black),
        1 => Some(Color::Red),
        2 => Some(Color::Green),
        3 => Some(Color::Yellow),
        4 => Some(Color::Blue),
        5 => Some(Color::Magenta),
        6 => Some(Color::Cyan),
        7 => Some(Color::Gray),
        8 => Some(Color::DarkGray),
        9 => Some(Color::LightRed),
        10 => Some(Color::LightGreen),
        11 => Some(Color::LightYellow),
        12 => Some(Color::LightBlue),
        13 => Some(Color::LightMagenta),
        14 => Some(Color::LightCyan),
        15 => Some(Color::White),
        _ => None,
    }
}

fn get_or_init_config() -> Colors {
    let mut colors = Colors::new();
    let config_dir = {
        if let Some(path) = get_paths_dir(".config/tmux-fzy") {
            path
        } else {
            return colors;
        }
    };

    let file_path = config_dir.join("config");
    if !file_path.exists() {
        return colors;
    }

    let mut file = {
        if let Ok(file) = File::open(&file_path) {
            file
        } else {
            return colors;
        }
    };

    let mut contents = String::new();
    if file.read_to_string(&mut contents).is_err() {
        return colors;
    };

    for line in contents.lines() {
        if line.is_empty() {
            continue;
        }
        let parts = line.split_once('=');
        match parts {
            Some((name, val)) => {
                let name = name.trim();
                let val = val.trim();
                if let Ok(value) = val.parse::<u8>() {
                    match name {
                        "fg" => colors.fg(value),
                        "border" => colors.border(value),
                        "inactive" => colors.inactive(value),
                        "active" => colors.active(value),
                        "selection" => colors.selection(value),
                        _ => continue,
                    }
                } else {
                    continue;
                }
            }
            None => continue,
        }
    }

    colors
}

static CONFIG: Lazy<Mutex<Colors>> = Lazy::new(|| Mutex::new(get_or_init_config()));

pub enum AppColors {
    Fg,
    Border,
    Active,
    Inactive,
    Selection,
}

impl AppColors {
    pub fn get(&self) -> Color {
        match self {
            AppColors::Fg => CONFIG.lock().unwrap().fg,
            AppColors::Border => CONFIG.lock().unwrap().border,
            AppColors::Active => CONFIG.lock().unwrap().active,
            AppColors::Inactive => CONFIG.lock().unwrap().inactive,
            AppColors::Selection => CONFIG.lock().unwrap().selection,
        }
    }
}
