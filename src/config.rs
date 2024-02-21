use std::{
    env,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
    str::FromStr,
};

use ratatui::style::Color;

pub struct Entry {
    pub path: PathBuf,
    pub min_depth: usize,
    pub max_depth: usize,
}

pub struct PathList {
    pub entries: Vec<Entry>,
}

pub struct Colors {
    pub fg: Color,
    pub border: Color,
    pub inactive: Color,
    pub active: Color,
    pub selection: Color,
}

impl FromStr for PathList {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut entries = Vec::new();
        for (i, line) in s.lines().enumerate() {
            let values: Vec<&str> = line.split(":|:").collect();

            if values.len() != 3 {
                return Err(anyhow::anyhow!("Invalid number of values"));
            }

            let path = PathBuf::from_str(values[0]).map_err(|err| anyhow::anyhow!(err))?;
            let min_depth: usize = values[1]
                .parse()
                .map_err(|_| anyhow::anyhow!("Error on line {}, invalid min_depth", i))?;
            let max_depth: usize = values[2]
                .parse()
                .map_err(|_| anyhow::anyhow!("Error on line {}, invalid max_depth", i))?;

            if path.is_dir() {
                let path = PathBuf::from_str(values[0])?;
                entries.push(Entry {
                    path,
                    min_depth,
                    max_depth,
                })
            }
        }
        Ok(PathList { entries })
    }
}

impl ToString for PathList {
    fn to_string(&self) -> String {
        self.entries
            .iter()
            .map(|entry| {
                format!(
                    "{}:|:{}:|:{}",
                    entry.path.to_str().unwrap(),
                    entry.min_depth,
                    entry.max_depth
                )
            })
            .collect::<Vec<String>>()
            .join("\n")
    }
}

impl PathList {
    pub fn insert_row(&mut self, path: PathBuf, min_depth: usize, max_depth: usize) {
        self.entries.push(Entry {
            path,
            min_depth,
            max_depth,
        })
    }

    pub fn save_configuration(&self) -> Result<(), anyhow::Error> {
        let paths_dir = get_paths_dir(".cache")
            .ok_or(anyhow::anyhow!("Failed to locate the config directory."))?;
        let file_path = paths_dir.join(".tmux-fzy");

        let c = self.to_string();

        let mut file = OpenOptions::new()
            .append(false)
            .write(true)
            .truncate(true)
            .open(file_path)
            .map_err(|e| anyhow::anyhow!(e))?;

        file.write_all(c.as_bytes())
            .map_err(|e| anyhow::anyhow!(e))?;

        Ok(())
    }

    pub fn remove_paths(&mut self, path: Vec<PathBuf>) -> Result<(), anyhow::Error> {
        self.entries.retain(|entry| !path.contains(&entry.path));
        Ok(())
    }
}

impl Colors {
    fn default() -> Colors {
        Colors {
            fg: Color::White,
            border: Color::White,
            inactive: Color::DarkGray,
            active: Color::LightGreen,
            selection: Color::LightYellow,
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

fn is_absolute_path(path: OsString) -> Option<PathBuf> {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        Some(path)
    } else {
        None
    }
}

fn get_paths_dir(from_home: &str) -> Option<PathBuf> {
    env::var_os("XDG_CACHE_HOME")
        .and_then(is_absolute_path)
        .or_else(|| {
            env::var_os("HOME")
                .map(PathBuf::from)
                .map(|h| h.join(from_home))
        })
}

fn init_config(path: &PathBuf) -> Result<(), anyhow::Error> {
    let dir = path.parent().unwrap();
    if !dir.exists() {
        fs::create_dir(dir).map_err(|e| anyhow::anyhow!(e))?;
    }
    File::create(path).map_err(|e| anyhow::anyhow!(e))?;
    Ok(())
}

pub fn get_paths() -> Result<PathList, anyhow::Error> {
    let config_dir =
        get_paths_dir(".cache").ok_or(anyhow::anyhow!("Failed to locate the config directory."))?;

    let file_path = config_dir.join(".tmux-fzy");
    if !file_path.exists() {
        init_config(&file_path)?;
    }

    let mut file = File::open(&file_path).map_err(|e| anyhow::anyhow!(e))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|e| anyhow::anyhow!(e))?;
    let paths = PathList::from_str(&contents).map_err(|e| anyhow::anyhow!(e))?;
    Ok(paths)
}

pub fn init_colors() -> Colors {
    let mut colors = Colors::default();
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
        match File::open(&file_path) {
            Ok(file) => file,
            Err(_) => return colors,
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
        if let Some((name, val)) = parts {
            let name = name.trim();
            let val = val.trim();
            if let Ok(value) = val.parse::<u8>() {
                let value = int_to_ansi_colors(value);
                if let Some(value) = value {
                    match name {
                        "fg" => colors.fg = value,
                        "border" => colors.border = value,
                        "inactive" => colors.inactive = value,
                        "active" => colors.active = value,
                        "selection" => colors.selection = value,
                        _ => {}
                    }
                }
            }
        }
    }

    colors
}
