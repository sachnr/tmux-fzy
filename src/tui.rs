use std::{
    io::{self, stderr, stdout},
    time::{Duration, Instant},
};

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    },
    execute,
    style::{Print, Stylize},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use fuzzy_matcher::FuzzyMatcher;
use once_cell::sync::Lazy;
use ratatui::{
    prelude::{Backend, Constraint, Corner, CrosstermBackend, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Padding, Paragraph, Tabs},
    Frame, Terminal,
};

use crate::{config::Configuration, start_tmux, switch_sessions, tmux, Error};

static HINT1: Lazy<Line<'static>> = Lazy::new(|| {
    Line::from(vec![
        Span::styled("Movement: ", Style::default().fg(Color::Blue)),
        Span::styled("<C-j>, <C-k>, Up, Down", Style::default().fg(Color::White)),
        Span::styled(" | ", Style::default().fg(Color::White)),
        Span::styled("Tabs: ", Style::default().fg(Color::Blue)),
        Span::styled("<Tab>, Right, Left", Style::default().fg(Color::White)),
        Span::styled(" | ", Style::default().fg(Color::White)),
        Span::styled("Scroll: ", Style::default().fg(Color::Blue)),
        Span::styled(
            "<C-u>, <C-d>, <C-up>, <C-down>",
            Style::default().fg(Color::White),
        ),
        Span::styled(" | ", Style::default().fg(Color::White)),
        Span::styled("Exit: ", Style::default().fg(Color::Blue)),
        Span::styled("<C-c>, <Esc>", Style::default().fg(Color::White)),
    ])
});

static HINT2: Lazy<Line<'static>> = Lazy::new(|| {
    Line::from(vec![
        Span::styled("Kill Session: ", Style::default().fg(Color::Blue)),
        Span::styled("<C-x>, <Delete>", Style::default().fg(Color::White)),
    ])
});

enum InputMode {
    Editing,
    Command,
}

struct App<'a> {
    curr_tab: usize,
    tabs: [Tab<'a>; 2],
    input_mode: InputMode,
    message: Line<'a>,
}

impl<'a> App<'a> {
    fn new(paths: Vec<String>, active_sessions: Vec<String>) -> Self {
        let tabs = [Tab::new("All", paths), Tab::new("Active", active_sessions)];
        Self {
            curr_tab: 0,
            tabs,
            input_mode: InputMode::Editing,
            message: HINT1.to_owned(),
        }
    }

    fn get_selected(&self) -> Option<String> {
        let selected = self.tabs[self.curr_tab].list.state.selected();
        selected.map(|selected| self.tabs[self.curr_tab].list.items[selected].path.clone())
    }
}

struct Tab<'a> {
    name: &'a str,
    user_input: String,
    list: StatefullList,
    orignal_list: Vec<String>,
    cursor_position: usize,
}

impl<'a> Tab<'a> {
    fn new(name: &'a str, paths: Vec<String>) -> Tab<'a> {
        let items: Vec<Item> = paths.iter().map(|s| Item::make_item(s.clone())).collect();
        Self {
            name,
            user_input: String::new(),
            list: StatefullList::with_items(items),
            orignal_list: paths,
            cursor_position: 0,
        }
    }

    fn update(&mut self) {
        let items = self.fuzzy_matcher();
        self.list.update(items);
    }

    fn enter_char(&mut self, c: char) {
        self.user_input.push(c);
        self.cursor_position += 1;
        self.update();
    }

    fn del_char(&mut self) {
        if self.user_input.pop().is_some() {
            self.cursor_position -= 1;
            self.update();
        };
    }

    fn fuzzy_matcher(&mut self) -> Vec<Item> {
        let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();
        let mut items = Vec::new();
        for s in self.orignal_list.iter() {
            if let Some((_, indices)) = matcher.fuzzy_indices(s.as_str(), &self.user_input) {
                items.push(Item {
                    path: s.clone(),
                    indices,
                })
            }
        }
        items
    }
}

struct StatefullList {
    items: Vec<Item>,
    state: ListState,
}

struct Item {
    path: String,
    indices: Vec<usize>,
}

impl Item {
    fn make_item(path: String) -> Self {
        Self {
            path,
            indices: Vec::new(),
        }
    }
}

impl StatefullList {
    fn with_items(items: Vec<Item>) -> StatefullList {
        StatefullList {
            items,
            state: ListState::default(),
        }
    }

    fn next(&mut self) {
        let len = self.items.len();
        let selected = self.state.selected();
        if let Some(selected) = selected {
            if selected < len.saturating_sub(1) {
                self.state.select(Some(selected + 1))
            }
        }
    }

    fn previous(&mut self) {
        let selected = self.state.selected();
        if let Some(selected) = selected {
            if selected > 0 {
                self.state.select(Some(selected.saturating_sub(1)))
            }
        }
    }

    fn scroll_up(&mut self) {
        let selected = self.state.selected();
        if let Some(selected) = selected {
            if selected > 5 {
                self.state.select(Some(selected - 5))
            } else {
                self.state.select(Some(0))
            }
        }
    }

    fn scroll_down(&mut self) {
        let len = self.items.len();
        let selected = self.state.selected();
        if let Some(selected) = selected {
            let new_selected = selected + 5;
            if new_selected >= len {
                self.state.select(Some(len.saturating_sub(1)))
            } else {
                self.state.select(Some(new_selected))
            }
        }
    }

    fn update(&mut self, list: Vec<Item>) {
        let len = list.len();
        match self.state.selected() {
            None => {
                if len > 0 {
                    self.items = list;
                    self.state.select(Some(0));
                }
            }
            Some(curr) => {
                if len < curr {
                    self.state.select(Some(0))
                }
                self.items = list;
            }
        }
    }
}

// type Result<T> = std::result::Result<T, Box<dyn Error>>;
type TuiError<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn render(config: &Configuration) -> TuiError<()> {
    Lazy::force(&HINT1);
    Lazy::force(&HINT2);
    let original_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |panic| {
        reset_terminal().unwrap();
        original_hook(panic);
    }));

    enable_raw_mode().map_err(|e| Error::UnexpectedError(e.into()))?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .map_err(|e| Error::UnexpectedError(e.into()))?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|e| Error::UnexpectedError(e.into()))?;

    let tick_rate = Duration::from_millis(250);
    let paths = config.expand_paths();
    let active_sessions = tmux::list_sessions()?;
    let app = App::new(paths, active_sessions);
    let res = run_app(&mut terminal, app, tick_rate);

    reset_terminal()?;

    if let Err(err) = res {
        execute!(
            stderr(),
            Print("Error: ".red().bold()),
            Print(format!("{:?}", err))
        )?;
    }

    Ok(())
}

fn reset_terminal() -> TuiError<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    tick_rate: Duration,
) -> Result<(), Error> {
    let mut last_tick = Instant::now();
    if !app.tabs[app.curr_tab].list.items.is_empty() {
        app.tabs[app.curr_tab].list.state.select(Some(0));
    }
    loop {
        terminal
            .draw(|f| ui(f, &mut app))
            .map_err(|e| Error::UnexpectedError(e.into()))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout).map_err(|e| Error::UnexpectedError(e.into()))? {
            if let Event::Key(KeyEvent {
                code,
                modifiers,
                kind: _,
                state: _,
            }) = event::read().map_err(|e| Error::UnexpectedError(e.into()))?
            {
                match app.input_mode {
                    InputMode::Editing => match (code, modifiers) {
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) => return Ok(()),
                        (KeyCode::Esc, KeyModifiers::NONE) => app.input_mode = InputMode::Command,
                        (KeyCode::Char('j'), KeyModifiers::CONTROL)
                        | (KeyCode::Down, KeyModifiers::NONE) => app.tabs[app.curr_tab].list.next(),
                        (KeyCode::Char('k'), KeyModifiers::CONTROL)
                        | (KeyCode::Up, KeyModifiers::NONE) => {
                            app.tabs[app.curr_tab].list.previous()
                        }
                        (KeyCode::Char('d'), KeyModifiers::CONTROL)
                        | (KeyCode::Down, KeyModifiers::CONTROL) => {
                            app.tabs[app.curr_tab].list.scroll_down()
                        }
                        (KeyCode::Char('u'), KeyModifiers::CONTROL)
                        | (KeyCode::Up, KeyModifiers::CONTROL) => {
                            app.tabs[app.curr_tab].list.scroll_up()
                        }
                        (KeyCode::Char(c), KeyModifiers::NONE) => {
                            app.tabs[app.curr_tab].enter_char(c)
                        }
                        (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                            app.tabs[app.curr_tab].enter_char(c.to_ascii_uppercase())
                        }
                        (KeyCode::Backspace, KeyModifiers::NONE) => {
                            app.tabs[app.curr_tab].del_char()
                        }
                        (KeyCode::Enter, KeyModifiers::NONE) => {
                            if let Some(path) = app.get_selected() {
                                if app.curr_tab == 0 {
                                    start_tmux(&path)?;
                                } else {
                                    switch_sessions(&path)?;
                                }
                                return Ok(());
                            }
                        }
                        (KeyCode::Delete, KeyModifiers::NONE)
                        | (KeyCode::Char('x'), KeyModifiers::CONTROL) => {
                            if app.curr_tab == 1 {
                                app.input_mode = InputMode::Command;
                                app.message =
                                    Line::from("Are you sure you want to kill this session? y/n")
                            }
                        }
                        (KeyCode::Tab, KeyModifiers::NONE)
                        | (KeyCode::Left, KeyModifiers::NONE)
                        | (KeyCode::Right, KeyModifiers::NONE) => {
                            if app.curr_tab == 0 {
                                app.curr_tab = 1;
                                app.message = HINT2.to_owned();
                            } else {
                                app.curr_tab = 0;
                                app.message = HINT1.to_owned();
                            }
                            app.tabs[app.curr_tab].update();
                        }
                        _ => {}
                    },
                    InputMode::Command => match (code, modifiers) {
                        (KeyCode::Char('y'), KeyModifiers::NONE) => {
                            if let Some(session) = app.get_selected() {
                                tmux::kill_session(&session)?;
                                app.tabs[app.curr_tab].orignal_list = tmux::list_sessions()?;
                                app.tabs[app.curr_tab].update();
                                if let Some(selected) = app.tabs[app.curr_tab].list.state.selected()
                                {
                                    let len = app.tabs[app.curr_tab].list.items.len();
                                    if selected >= len {
                                        app.tabs[app.curr_tab]
                                            .list
                                            .state
                                            .select(Some(len.saturating_sub(1)))
                                    }
                                }
                            }
                            app.input_mode = InputMode::Editing;
                            app.message = HINT1.to_owned();
                        }
                        (KeyCode::Char('n'), KeyModifiers::NONE) => {
                            app.input_mode = InputMode::Editing;
                            app.message = HINT2.to_owned();
                        }
                        (KeyCode::Esc, KeyModifiers::NONE) => return Ok(()),
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) => return Ok(()),
                        _ => {}
                    },
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Min(3),
                Constraint::Min(3),
                Constraint::Percentage(100),
            ]
            .as_ref(),
        )
        .split(f.size());

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(16), Constraint::Percentage(100)].as_ref())
        .split(chunks[0]);

    let tabs = get_tabs(app);
    let items = get_items(app);
    let input = get_inputs(app);
    let message = get_message(app);

    f.render_widget(tabs, top[0]);
    f.render_widget(message, top[1]);
    f.render_widget(input, chunks[1]);
    if let InputMode::Editing = app.input_mode {
        f.set_cursor(
            chunks[1].x + app.tabs[app.curr_tab].cursor_position as u16 + 1,
            // Move one line down, from the border to the input line
            chunks[1].y + 1,
        );
    }
    f.render_stateful_widget(items, chunks[2], &mut app.tabs[app.curr_tab].list.state);
}

fn get_message<'a>(app: &'a App) -> Paragraph<'a> {
    Paragraph::new(app.message.to_owned())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue))
                .padding(Padding::new(2, 0, 0, 0))
                .title("Info")
                .title_style(Style::default().fg(Color::Blue)),
        )
        .style(Style::default().fg(Color::White))
}

fn get_tabs<'a>(app: &'a App) -> Tabs<'a> {
    let titles = app.tabs.iter().map(|tab| Line::from(tab.name)).collect();

    Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue))
                .title_style(Style::default().fg(Color::Blue))
                .title("Sessions"),
        )
        .select(app.curr_tab)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Blue),
        )
}

fn get_inputs<'a>(app: &'a App) -> Paragraph<'a> {
    Paragraph::new(app.tabs[app.curr_tab].user_input.as_str())
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .style(Style::default().fg(Color::LightRed))
                .borders(Borders::BOTTOM)
                .title(" Input")
                .padding(Padding::new(1, 0, 0, 0)),
        )
}

fn get_items<'a>(app: &App) -> List<'a> {
    let items: Vec<ListItem> = app.tabs[app.curr_tab]
        .list
        .items
        .iter()
        .enumerate()
        .filter_map(|(line_index, Item { path, indices })| {
            if let Some(selected) = app.tabs[app.curr_tab].list.state.selected() {
                let spans = path
                    .chars()
                    .enumerate()
                    .map(|(char_index, char)| {
                        let (fg, bg) = {
                            let contains = indices.contains(&char_index);
                            let focused = line_index == selected;
                            match (focused, contains) {
                                (true, true) => (Color::Yellow, Color::Blue),
                                (true, false) => (Color::Black, Color::Blue),
                                (false, true) => (Color::Yellow, Color::default()),
                                (false, false) => (Color::White, Color::default()),
                            }
                        };
                        Span::styled(char.to_string(), Style::default().fg(fg).bg(bg))
                    })
                    .collect::<Vec<_>>();
                Some(ListItem::new(Line::from(spans)))
            } else {
                None
            }
        })
        .collect();

    List::new(items)
        .block(
            Block::default()
                .padding(Padding::new(1, 5, 0, 2))
                .title("Results")
                .borders(Borders::LEFT)
                .style(Style::default().fg(Color::Blue)),
        )
        .start_corner(Corner::TopLeft)
}
