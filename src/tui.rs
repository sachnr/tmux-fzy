use crate::config::Config;
use crate::tmux;
use crossterm::{
    event::{self, Event, KeyEvent},
    execute,
    terminal::{self, disable_raw_mode, enable_raw_mode, EnterAlternateScreen},
};
use fuzzy_matcher::FuzzyMatcher;
use std::{collections::VecDeque, path::PathBuf};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, Paragraph},
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

    let mut app = App::new(config.expand());
    app.filter();

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
    filter: VecDeque<(PathBuf, Vec<usize>)>,
    active: Vec<String>,
    curr: i16,
    rect: i16,
}

impl App {
    fn new(paths: Vec<PathBuf>) -> Self {
        Self {
            user_input: String::new(),
            filter: VecDeque::new(),
            active: tmux::sessions(),
            paths,
            curr: 0,
            rect: 0,
        }
    }

    fn filter(&mut self) {
        self.filter = self
            .paths
            .iter()
            .map(|pathbuf| (pathbuf.clone(), vec![]))
            .collect();
    }

    fn next(&mut self) {
        self.curr += 1;
        let len = self.filter.len() as i16;
        match (self.curr, self.rect) {
            (curr, size) if curr > size => {
                let first = self.filter.pop_front().unwrap();
                self.filter.push_back(first);
                self.curr -= 1;
            }
            (curr, _) if curr >= len => {
                self.curr = len - 1;
            }
            _ => {}
        }
    }

    fn prev(&mut self) {
        self.curr -= 1;
        let len = self.filter.len() as i16;
        match (self.curr, self.rect) {
            (curr, size) if curr < 0 && len > size => {
                let last = self.filter.pop_back().unwrap();
                self.filter.push_front(last);
                self.curr += 1;
            }
            (curr, _) if curr < 0 => self.curr = 0,
            _ => {}
        }
    }

    fn update(&mut self) {
        let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();
        let mut matched = VecDeque::new();
        for path in self.paths.clone() {
            let path_str = path.to_str().unwrap();
            if let Some((_, indices)) = matcher.fuzzy_indices(path_str, &self.user_input) {
                matched.push_front((path, indices));
            }
        }
        self.filter = matched;
    }
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<(), std::io::Error> {
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
                    app.update();
                    app.curr = 0;
                }
                (event::KeyCode::Backspace, event::KeyModifiers::NONE) => {
                    _ = app.user_input.pop();
                    app.update();
                }
                (event::KeyCode::Enter, event::KeyModifiers::NONE) => {
                    if app.filter.is_empty() {
                        continue;
                    }
                    let path = app.filter[app.curr as usize].0.clone();
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

    app.rect = (size.height - 7) as i16;

    let input = Paragraph::new(Spans::from(vec![
        Span::styled("   ", Style::default().fg(Color::Red)),
        Span::raw(app.user_input.clone()),
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

    let paths: Vec<ListItem> = app
        .filter
        .iter()
        .enumerate()
        .map(|(i, paths)| {
            let (paths, indices) = paths;
            let active = || -> bool {
                let path = paths.file_name().unwrap().to_str().unwrap();
                for session in &app.active {
                    if path.eq(session) {
                        return true;
                    }
                }
                false
            };
            let sign = {
                if active() {
                    " [Active]"
                } else {
                    ""
                }
            };

            let content = if i == app.curr as usize {
                let colored = color_fzf_bold(paths.to_str().unwrap(), indices);
                Spans::from(
                    vec![Span::styled("❯ ", Style::default().fg(Color::Green))]
                        .into_iter()
                        .chain(colored)
                        .chain(vec![Span::styled(sign, Style::default().fg(Color::Green))])
                        .collect::<Vec<Span>>(),
                )
            } else {
                let colored = color_fzf(paths.to_str().unwrap(), indices);
                Spans::from(
                    colored
                        .into_iter()
                        .chain(vec![Span::styled(sign, Style::default().fg(Color::Green))])
                        .collect::<Vec<Span>>(),
                )
            };
            ListItem::new(content)
        })
        .collect();

    let list = List::new(paths).block(
        Block::default().title(Span::styled(
            "  Results  ",
            Style::default()
                .bg(Color::Green)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )),
    );

    f.render_widget(input, chunks[0]);
    f.set_cursor(
        chunks[0].x + app.user_input.len() as u16 + 4,
        chunks[0].y + 1,
    );
    f.render_widget(list, chunks[1]);
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

fn color_fzf_bold<'a>(input: &'a str, indices: &[usize]) -> Vec<Span<'a>> {
    input
        .chars()
        .enumerate()
        .map(|(i, c)| {
            if !indices.contains(&i) {
                Span::styled(
                    c.to_string(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(
                    c.to_string(),
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                )
            }
        })
        .collect::<Vec<Span>>()
}
