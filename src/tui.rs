use std::io;
use std::time::{Duration, Instant, SystemTime};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};

use crate::diff::{DiffEngine, DiffEntry, DiffKind, DiffReport};

const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

struct App {
    report: DiffReport,
    selected: usize,
    show_identical: bool,
    paused: bool,
    spinner: usize,
    last_error: Option<String>,
}

impl App {
    fn visible_entries(&self) -> Vec<&DiffEntry> {
        self.report
            .entries
            .iter()
            .filter(|entry| self.show_identical || entry.kind != DiffKind::Identical)
            .collect()
    }

    fn visible_len(&self) -> usize {
        self.report
            .entries
            .iter()
            .filter(|entry| self.show_identical || entry.kind != DiffKind::Identical)
            .count()
    }

    fn select_next(&mut self) {
        let len = self.visible_len();
        if len > 0 {
            self.selected = (self.selected + 1).min(len - 1);
        }
    }

    fn select_previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    fn select_last(&mut self) {
        self.selected = self.visible_len().saturating_sub(1);
    }

    fn clamp_selection(&mut self) {
        self.selected = self.selected.min(self.visible_len().saturating_sub(1));
    }
}

pub fn run(
    engine: &mut DiffEngine,
    report: DiffReport,
    scan_interval: Duration,
    show_identical: bool,
) -> Result<()> {
    let guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = App {
        report,
        selected: 0,
        show_identical,
        paused: false,
        spinner: 0,
        last_error: None,
    };
    let result = run_loop(&mut terminal, engine, &mut app, scan_interval);

    drop(terminal);
    drop(guard);
    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    engine: &mut DiffEngine,
    app: &mut App,
    scan_interval: Duration,
) -> Result<()> {
    let mut last_scan = Instant::now();
    let mut last_animation = Instant::now();

    loop {
        terminal.draw(|frame| render(frame, app))?;

        if event::poll(Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Char('j') | KeyCode::Down => app.select_next(),
                KeyCode::Char('k') | KeyCode::Up => app.select_previous(),
                KeyCode::Char('g') | KeyCode::Home => app.selected = 0,
                KeyCode::Char('G') | KeyCode::End => app.select_last(),
                KeyCode::Char(' ') => app.paused = !app.paused,
                KeyCode::Char('a') => {
                    app.show_identical = !app.show_identical;
                    app.clamp_selection();
                }
                KeyCode::Char('r') => refresh(engine, app),
                _ => {}
            }
        }

        if last_animation.elapsed() >= Duration::from_millis(100) {
            app.spinner = app.spinner.wrapping_add(1);
            last_animation = Instant::now();
        }
        if !app.paused && last_scan.elapsed() >= scan_interval {
            refresh(engine, app);
            last_scan = Instant::now();
        }
    }
}

fn refresh(engine: &mut DiffEngine, app: &mut App) {
    match engine.scan() {
        Ok(report) => {
            app.report = report;
            app.last_error = None;
            app.clamp_selection();
        }
        Err(error) => app.last_error = Some(format!("{error:#}")),
    }
}

fn render(frame: &mut ratatui::Frame<'_>, app: &App) {
    let [
        header_area,
        stats_area,
        table_area,
        detail_area,
        footer_area,
    ] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(6),
        Constraint::Length(5),
        Constraint::Length(2),
    ])
    .areas(frame.area());

    render_header(frame, header_area, app);
    render_stats(frame, stats_area, app);
    render_table(frame, table_area, app);
    render_detail(frame, detail_area, app);
    render_footer(frame, footer_area, app);
}

fn render_header(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let state = if app.paused { "paused" } else { "live" };
    let title = Line::from(vec![
        Span::styled(
            format!(" {} ", SPINNER[app.spinner % SPINNER.len()]),
            Style::default().fg(Color::Cyan),
        ),
        Span::styled("fdiff", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(
            format!(" · {state} "),
            Style::default().fg(if app.paused {
                Color::Yellow
            } else {
                Color::Green
            }),
        ),
    ]);
    let paths = Line::from(vec![
        Span::styled(
            app.report.left_root.display().to_string(),
            Style::default().fg(Color::Blue),
        ),
        Span::styled("  ↔  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            app.report.right_root.display().to_string(),
            Style::default().fg(Color::Green),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(paths).block(Block::default().borders(Borders::ALL).title(title)),
        area,
    );
}

fn render_stats(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let summary = &app.report.summary;
    let line = Line::from(vec![
        stat("←", summary.left_only, "left", Color::Blue),
        Span::raw("   "),
        stat("≠", summary.modified, "changed", Color::Yellow),
        Span::raw("   "),
        stat("⇄", summary.type_changed, "type", Color::Magenta),
        Span::raw("   "),
        stat("→", summary.right_only, "right", Color::Green),
        Span::raw("   "),
        stat("=", summary.identical, "same", Color::DarkGray),
    ]);
    let title = format!(" Summary · {} paths ", summary.total());
    frame.render_widget(
        Paragraph::new(line).block(Block::default().borders(Borders::ALL).title(title)),
        area,
    );
}

fn stat(symbol: &'static str, count: usize, label: &'static str, color: Color) -> Span<'static> {
    Span::styled(
        format!("{symbol} {count} {label}"),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )
}

fn render_table(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let visible = app.visible_entries();
    let rows = visible.iter().map(|entry| {
        let style = status_style(entry.kind);
        Row::new(vec![
            Cell::from(entry.kind.plain_label()),
            Cell::from(entry.path.to_string_lossy().into_owned()),
            Cell::from(
                entry
                    .left
                    .as_ref()
                    .map(|info| info.description())
                    .unwrap_or_else(|| "—".to_owned()),
            ),
            Cell::from(
                entry
                    .right
                    .as_ref()
                    .map(|info| info.description())
                    .unwrap_or_else(|| "—".to_owned()),
            ),
        ])
        .style(style)
    });
    let header = Row::new(["status", "relative path", "left", "right"])
        .style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(1);
    let table = Table::new(
        rows,
        [
            Constraint::Length(11),
            Constraint::Min(20),
            Constraint::Length(20),
            Constraint::Length(20),
        ],
    )
    .header(header)
    .column_spacing(2)
    .row_highlight_style(
        Style::default()
            .bg(Color::Rgb(35, 43, 55))
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▸ ")
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(if app.show_identical {
                " Paths · all "
            } else {
                " Paths · differences "
            }),
    );
    let mut state =
        TableState::default().with_selected((!visible.is_empty()).then_some(app.selected));
    frame.render_stateful_widget(table, area, &mut state);
}

fn render_detail(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let visible = app.visible_entries();
    let lines = if let Some(entry) = visible.get(app.selected) {
        vec![
            Line::from(vec![
                Span::styled("path   ", Style::default().fg(Color::DarkGray)),
                Span::raw(entry.path.to_string_lossy().into_owned()),
            ]),
            detail_line("left", entry.left.as_ref().map(|info| info.description())),
            detail_line("right", entry.right.as_ref().map(|info| info.description())),
        ]
    } else {
        vec![Line::from("No visible differences")]
    };
    frame.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" Detail ")),
        area,
    );
}

fn detail_line(label: &'static str, value: Option<String>) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<7}"), Style::default().fg(Color::DarkGray)),
        Span::raw(value.unwrap_or_else(|| "—".to_owned())),
    ])
}

fn render_footer(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let line = if let Some(error) = &app.last_error {
        Line::styled(
            format!("scan failed · {error} · r retry · q quit"),
            Style::default().fg(Color::Red),
        )
    } else {
        let scanned = SystemTime::now()
            .duration_since(app.report.scanned_at)
            .map(|duration| match duration.as_secs() {
                0 => "now".to_owned(),
                seconds => format!("{seconds}s ago"),
            })
            .unwrap_or_else(|_| "now".to_owned());
        Line::from(vec![
            Span::styled("↑↓/jk", Style::default().fg(Color::Cyan)),
            Span::raw(" move  "),
            Span::styled("space", Style::default().fg(Color::Cyan)),
            Span::raw(" pause  "),
            Span::styled("a", Style::default().fg(Color::Cyan)),
            Span::raw(" same  "),
            Span::styled("r", Style::default().fg(Color::Cyan)),
            Span::raw(" refresh  "),
            Span::styled("q", Style::default().fg(Color::Cyan)),
            Span::raw(format!(" quit  · scanned {scanned}")),
        ])
    };
    frame.render_widget(Paragraph::new(line), area);
}

fn status_style(kind: DiffKind) -> Style {
    match kind {
        DiffKind::LeftOnly => Style::default().fg(Color::Blue),
        DiffKind::RightOnly => Style::default().fg(Color::Green),
        DiffKind::Modified => Style::default().fg(Color::Yellow),
        DiffKind::TypeChanged => Style::default().fg(Color::Magenta),
        DiffKind::Identical => Style::default().fg(Color::DarkGray),
    }
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> Result<Self> {
        enable_raw_mode()?;
        if let Err(error) = execute!(io::stdout(), EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(error.into());
        }
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn frame_contains_live_diff_dashboard() {
        let left = tempdir().unwrap();
        let right = tempdir().unwrap();
        fs::write(left.path().join("changed.txt"), "left").unwrap();
        fs::write(right.path().join("changed.txt"), "rght").unwrap();
        let report = DiffEngine::new(left.path(), right.path())
            .unwrap()
            .scan()
            .unwrap();
        let app = App {
            report,
            selected: 0,
            show_identical: false,
            paused: false,
            spinner: 0,
            last_error: None,
        };
        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|frame| render(frame, &app)).unwrap();
        let screen = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(screen.contains("fdiff"));
        assert!(screen.contains("live"));
        assert!(screen.contains("changed.txt"));
        assert!(screen.contains("Summary"));
        assert!(screen.contains("Detail"));
    }
}
