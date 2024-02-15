use std::{collections::BinaryHeap, path::PathBuf, time::Duration};

use crossterm::{
    event::{KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use fuzzy_matcher::FuzzyMatcher;
use jwalk::{
    rayon::iter::{IntoParallelRefIterator, ParallelBridge, ParallelIterator},
    WalkDir,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::ListState,
    Frame, Terminal,
};

use crate::{
    config::{Colors, PathList},
    tmux,
    tui_components::{get_input_bar, get_list, get_total_item_no},
};

pub struct PathItem<'a> {
    pub path: &'a str,
    pub fullpath: &'a str,
    pub score: i64,
    pub indices: Vec<usize>,
}

#[derive(Default)]
struct StatefulList<'a> {
    state: ListState,
    items: BinaryHeap<PathItem<'a>>,
    history: Vec<BinaryHeap<PathItem<'a>>>,
}

struct App<'a> {
    running: bool,
    input: String,
    cursor_pos: usize,
    total_items: usize,
    colors: Colors,
    list: StatefulList<'a>,
}

type Term = Terminal<CrosstermBackend<std::io::Stdout>>;

pub fn start_tui(paths: PathList, colors: Colors) -> Result<(), anyhow::Error> {
    let mut terminal = init_terminal()?;
    let paths = expand_paths(paths);
    let statefullist = StatefulList::from(&paths);
    let mut app = App::new(statefullist, colors, paths.len());

    while app.running {
        let timeout = Duration::from_millis(200);
        if crossterm::event::poll(timeout)? {
            match crossterm::event::read()? {
                crossterm::event::Event::Key(KeyEvent {
                    code, modifiers, ..
                }) => match (code, modifiers) {
                    (KeyCode::Char(c), KeyModifiers::NONE) => {
                        app.input.push(c);
                        app.cursor_pos += 1;
                        app.refresh();
                    }
                    (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                        app.input.push(c.to_ascii_uppercase());
                        app.cursor_pos += 1;
                        app.refresh();
                    }
                    (KeyCode::Backspace, KeyModifiers::NONE) => {
                        _ = app.input.pop();
                        app.cursor_pos = app.cursor_pos.saturating_sub(1);
                        app.undo();
                    }
                    (KeyCode::Esc, KeyModifiers::NONE) => app.running = false,
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => app.running = false,

                    (KeyCode::Char('j'), KeyModifiers::CONTROL)
                    | (KeyCode::Down, KeyModifiers::NONE) => app.list.next(),

                    (KeyCode::Char('k'), KeyModifiers::CONTROL)
                    | (KeyCode::Up, KeyModifiers::NONE) => app.list.prev(),

                    (KeyCode::Char('d'), KeyModifiers::CONTROL)
                    | (KeyCode::Down, KeyModifiers::CONTROL) => app.list.scroll_next(),

                    (KeyCode::Char('u'), KeyModifiers::CONTROL)
                    | (KeyCode::Up, KeyModifiers::CONTROL) => app.list.scroll_prev(),

                    (KeyCode::Enter, KeyModifiers::NONE) => {
                        if let Some(i) = app.list.state.selected() {
                            if let Some(item) = app.list.items.iter().nth(i) {
                                app.running = false;
                                start_tmux(item.fullpath)?;
                            } else {
                                return Err(anyhow::anyhow!("Indexing Failed"));
                            }
                        }
                    }

                    _ => {}
                },
                crossterm::event::Event::Resize(_, _) => terminal.autoresize()?,
                _ => {}
            }
        }
        terminal.draw(|f| render_frame(f, &mut app))?;
    }

    Ok(())
}

fn render_frame(f: &mut Frame<'_>, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(2), Constraint::Percentage(100)].as_ref())
        .split(f.size());

    let top = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Min(1)].as_ref())
        .split(chunks[0]);

    let rows = chunks[1].height;
    let curr_row = app.list.state.selected();

    let input_bar = get_input_bar(&app.input, &app.colors);
    let items = get_list(&app.list.items, rows, curr_row, &app.colors);
    let status = get_total_item_no(app.total_items, items.len(), &app.colors);

    f.render_widget(input_bar, top[0]);
    f.render_widget(status, top[1]);
    f.render_stateful_widget(items, chunks[1], &mut app.list.state);

    f.set_cursor(top[0].x + app.cursor_pos as u16 + 3, top[0].y);
}

fn expand_paths(paths: PathList) -> Vec<(String, String)> {
    let mut path_items = Vec::new();
    for path in paths.entries {
        let dirs: Vec<(String, String)> = WalkDir::new(path.path)
            .min_depth(path.min_depth)
            .max_depth(path.max_depth)
            .into_iter()
            .par_bridge()
            .filter_map(|item| {
                let entry = item.ok()?;
                let path = entry.path().to_owned();
                if entry.file_type().is_dir() {
                    let full_path = path.to_str()?.to_string();
                    let dir_name = path.file_name()?.to_str()?.to_string();
                    Some((full_path, dir_name))
                } else {
                    None
                }
            })
            .collect();

        path_items.extend(dirs);
    }
    path_items
}

fn init_terminal() -> Result<Term, anyhow::Error> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

pub fn reset_terminal() -> Result<(), anyhow::Error> {
    disable_raw_mode()?;
    execute!(std::io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}

impl<'a> From<&'a Vec<(String, String)>> for StatefulList<'a> {
    fn from(value: &'a Vec<(String, String)>) -> Self {
        let mut list = StatefulList::default();
        for item in value {
            list.items.push(PathItem {
                path: &item.1,
                fullpath: &item.0,
                score: 0,
                indices: vec![],
            });
        }
        if !list.items.is_empty() {
            list.state.select(Some(0))
        }
        list
    }
}

impl<'a> Eq for PathItem<'a> {}
impl<'a> PartialEq for PathItem<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.score.eq(&other.score)
    }
}

impl<'a> Ord for PathItem<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.score.cmp(&other.score)
    }
}
impl<'a> PartialOrd for PathItem<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.score.cmp(&other.score))
    }
}

impl<'a> App<'a> {
    fn new(list: StatefulList<'a>, colors: Colors, len: usize) -> Self {
        App {
            running: true,
            input: String::new(),
            cursor_pos: 0,
            total_items: len,
            list,
            colors,
        }
    }

    fn refresh(&mut self) {
        let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();

        let new_items: BinaryHeap<PathItem> = self
            .list
            .items
            .par_iter()
            .filter_map(|item| {
                if let Some((score, indices)) = matcher.fuzzy_indices(item.path, &self.input) {
                    return Some(PathItem {
                        path: item.path,
                        fullpath: item.fullpath,
                        score,
                        indices,
                    });
                }
                None
            })
            .collect();

        let items = std::mem::take(&mut self.list.items);
        self.list.history.push(items);
        self.list.items = new_items;

        let len = self.list.items.len();
        match len {
            0 => self.list.state.select(None),
            i if i >= len => self.list.state.select(Some(0)),
            _ => {}
        }
    }

    fn undo(&mut self) {
        if let Some(items) = self.list.history.pop() {
            let len = items.len();
            if len != 0 {
                self.list.state.select(Some(0))
            }
            self.list.items = items;
        }
    }
}

impl<'a> StatefulList<'a> {
    fn next(&mut self) {
        if let Some(i) = self.state.selected() {
            if i < self.items.len() - 1 {
                self.state.select(Some(i + 1));
            }
        }
    }

    fn scroll_next(&mut self) {
        if let Some(i) = self.state.selected() {
            if i < self.items.len() - 5 {
                self.state.select(Some(i + 5));
            } else {
                self.state.select(Some(self.items.len() - 1))
            }
        }
    }

    fn prev(&mut self) {
        if let Some(i) = self.state.selected() {
            if i != 0 {
                self.state.select(Some(i - 1));
            }
        }
    }

    fn scroll_prev(&mut self) {
        if let Some(i) = self.state.selected() {
            if i > 5 {
                self.state.select(Some(i - 5));
            } else {
                self.state.select(Some(0))
            }
        }
    }
}

pub fn start_tmux(path: &str) -> Result<(), anyhow::Error> {
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
