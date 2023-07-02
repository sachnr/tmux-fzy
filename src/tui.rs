use crate::config::Config;
use crate::tmux;
use crossterm::{
    event::{self, Event, KeyEvent},
    execute,
    terminal::{self, disable_raw_mode, enable_raw_mode, EnterAlternateScreen},
};
use fuzzy_matcher::FuzzyMatcher;
use std::path::PathBuf;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};

pub(crate) fn run(config: &mut Config) {
    if let Err(e) = render(config) {
        eprintln!("{e}");
    }
}

fn render(config: &mut Config) -> Result<(), std::io::Error> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();

    execute!(
        stdout,
        EnterAlternateScreen,
        terminal::Clear(terminal::ClearType::All),
        crossterm::event::EnableMouseCapture,
    )?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = App::new(config.expand());

    if let Err(e) = run_app(&mut terminal, app) {
        eprintln!("{e}");
    }

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

struct App {
    user_input: String,
    paths: Vec<PathBuf>,
    // is active, index, highlighted text
    filter: Vec<(bool, usize, Vec<usize>)>,
    active: Vec<String>,
    list_state: ListState,
}

impl App {
    fn new(paths: Vec<PathBuf>) -> Self {
        Self {
            user_input: String::new(),
            filter: Vec::new(),
            active: tmux::sessions(),
            paths,
            list_state: ListState::default(),
        }
    }

    fn next(&mut self) {
        let len = self.filter.len() as i16;
        if let Some(pos) = self.list_state.selected() {
            if pos < (len - 1) as usize {
                self.list_state.select(Some(pos + 1));
            }
        }
    }

    fn prev(&mut self) {
        if let Some(pos) = self.list_state.selected() {
            if pos > 0 {
                self.list_state.select(Some(pos - 1));
            }
        }
    }

    fn update(&mut self) {
        let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();
        let mut matched = Vec::new();
        for (i, path) in self.paths.iter().enumerate() {
            let path_str = path.to_str().unwrap();
            if let Some((_, indices)) = matcher.fuzzy_indices(path_str, &self.user_input) {
                matched.push((false, i, indices));
            }
        }
        self.filter = matched;
    }
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<(), std::io::Error> {
    app.update();
    app.list_state.select(Some(0));
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(KeyEvent {
            code,
            modifiers,
            kind: _,
            state: _,
        }) = event::read()?
        {
            match (code, modifiers) {
                (event::KeyCode::Char(c), event::KeyModifiers::NONE) => {
                    app.user_input.push(c);
                    app.list_state.select(Some(0));
                    app.update();
                }
                (event::KeyCode::Backspace, event::KeyModifiers::NONE) => {
                    _ = app.user_input.pop();
                    app.update();
                }
                (event::KeyCode::Enter, event::KeyModifiers::NONE) => {
                    if app.filter.is_empty() {
                        continue;
                    }
                    let path = app.paths[app.filter[app.list_state.selected().unwrap()].1].clone();
                    tmux::run(path)?;
                    app.active = tmux::sessions();
                    return Ok(());
                }
                (event::KeyCode::Esc, event::KeyModifiers::NONE) => return Ok(()),
                (event::KeyCode::Char('c'), event::KeyModifiers::CONTROL) => return Ok(()),
                (event::KeyCode::Char('j'), event::KeyModifiers::CONTROL) => app.next(),
                (event::KeyCode::Char('k'), event::KeyModifiers::CONTROL) => app.prev(),
                _ => {}
            }
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let size = f.size();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(3), Constraint::Percentage(100)].as_ref())
        .split(size);

    let total = format!("{}/{}", app.filter.len(), app.paths.len());
    let len = Paragraph::new(Span::styled(
        &total,
        Style::default().fg(Color::Gray).add_modifier(Modifier::DIM),
    ))
    .style(Style::default().fg(Color::Gray));

    let user_input: &str = app.user_input.as_ref();
    let search_icon = format!(" {}  ", nerd_font_symbols::fa::FA_SEARCH);
    let input = Paragraph::new(Spans::from(vec![
        Span::styled(search_icon, Style::default().fg(Color::Red)),
        Span::raw(user_input),
    ]))
    .style(Style::default().fg(Color::White))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::Red))
            .title(Span::styled(
                "   Input   ",
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::Red)
                    .fg(Color::Black),
            )),
    );

    for paths in app.filter.iter_mut() {
        let dir_name = app.paths[paths.1].file_name().unwrap().to_str().unwrap();
        paths.0 = app.active.contains(&dir_name.to_string());
    }

    app.filter.sort_by_key(|&(contains, _, _)| !contains);

    let paths_sorted: Vec<ListItem> = app
        .filter
        .iter()
        .map(|(contains, path, highlight)| {
            let path = app.paths[*path].to_str().unwrap();
            let colored = color_fzf(path, highlight);
            let items = {
                if *contains {
                    let icon = format!("{} ", nerd_font_symbols::md::MD_STAR_CIRCLE);
                    let active = Span::styled(icon, Style::default().fg(Color::Green));
                    ListItem::new(Spans::from(
                        vec![active]
                            .into_iter()
                            .chain(colored)
                            .collect::<Vec<Span>>(),
                    ))
                } else {
                    ListItem::new(Spans::from(colored))
                }
            };
            items
        })
        .collect();

    let arrow = nerd_font_symbols::fa::FA_CHEVRON_RIGHT;
    let list = List::new(paths_sorted)
        .block(
            Block::default().title(Span::styled(
                "  Results  ",
                Style::default()
                    .bg(Color::Green)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )),
        )
        .highlight_symbol(arrow)
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    f.render_widget(input, chunks[0]);
    f.set_cursor(
        chunks[0].x + app.user_input.len() as u16 + 4,
        chunks[0].y + 1,
    );
    f.render_widget(
        len,
        Rect::new(
            chunks[0].width - total.len() as u16,
            chunks[0].y + 1,
            total.len() as u16,
            1,
        ),
    );
    f.render_stateful_widget(list, chunks[1], &mut app.list_state)
}

fn color_fzf<'a>(input: &'a str, indices: &[usize]) -> Vec<Span<'a>> {
    input
        .chars()
        .enumerate()
        .map(|(i, c)| {
            if !indices.contains(&i) {
                Span::styled(c.to_string(), Style::default().fg(Color::White))
            } else {
                Span::styled(
                    c.to_string(),
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                )
            }
        })
        .collect::<Vec<Span>>()
}
