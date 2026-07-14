use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};
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

use crate::diff::{DiffEngine, DiffEntry, DiffKind, DiffReport, EntryInfo, EntryKind, human_bytes};

const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const SIDE_BY_SIDE_MIN_WIDTH: u16 = 72;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Pane {
    Left,
    Right,
}

struct VisibleRow<'a> {
    entry: &'a DiffEntry,
    depth: usize,
    collapsed_children: usize,
}

struct App {
    report: DiffReport,
    selected: usize,
    active_pane: Pane,
    collapsed: BTreeSet<PathBuf>,
    show_identical: bool,
    paused: bool,
    spinner: usize,
    last_error: Option<String>,
}

impl App {
    fn visible_rows(&self) -> Vec<VisibleRow<'_>> {
        let candidates = self
            .report
            .entries
            .iter()
            .map(|entry| self.is_candidate(entry))
            .collect::<Vec<_>>();

        self.report
            .entries
            .iter()
            .enumerate()
            .filter(|(index, entry)| {
                candidates[*index] && !self.has_collapsed_ancestor(&entry.path)
            })
            .map(|(_, entry)| VisibleRow {
                entry,
                depth: entry.path.components().count().saturating_sub(1),
                collapsed_children: if self.collapsed.contains(&entry.path) {
                    self.report
                        .entries
                        .iter()
                        .enumerate()
                        .filter(|(index, child)| {
                            candidates[*index]
                                && child.path != entry.path
                                && child.path.starts_with(&entry.path)
                        })
                        .count()
                } else {
                    0
                },
            })
            .collect()
    }

    fn is_candidate(&self, entry: &DiffEntry) -> bool {
        self.show_identical
            || entry.kind != DiffKind::Identical
            || (is_directory(entry)
                && self.report.entries.iter().any(|child| {
                    child.path != entry.path
                        && child.path.starts_with(&entry.path)
                        && child.kind != DiffKind::Identical
                }))
    }

    fn has_collapsed_ancestor(&self, path: &Path) -> bool {
        path.ancestors()
            .skip(1)
            .any(|ancestor| !ancestor.as_os_str().is_empty() && self.collapsed.contains(ancestor))
    }

    fn visible_len(&self) -> usize {
        self.visible_rows().len()
    }

    fn selected_path(&self) -> Option<PathBuf> {
        self.visible_rows()
            .get(self.selected)
            .map(|row| row.entry.path.clone())
    }

    fn select_path(&mut self, path: &Path) {
        if let Some(index) = self
            .visible_rows()
            .iter()
            .position(|row| row.entry.path == path)
        {
            self.selected = index;
        } else {
            self.clamp_selection();
        }
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

    fn toggle_selected_directory(&mut self) {
        let Some(path) = self.selected_path() else {
            return;
        };
        let is_selected_directory = self
            .report
            .entries
            .iter()
            .find(|entry| entry.path == path)
            .is_some_and(is_directory);
        if !is_selected_directory {
            return;
        }
        if !self.collapsed.remove(&path) {
            self.collapsed.insert(path);
        }
        self.clamp_selection();
    }

    fn expand_selected_directory(&mut self) {
        if let Some(path) = self.selected_path() {
            self.collapsed.remove(&path);
        }
    }

    fn collapse_or_select_parent(&mut self) {
        let Some(path) = self.selected_path() else {
            return;
        };
        let is_selected_directory = self
            .report
            .entries
            .iter()
            .find(|entry| entry.path == path)
            .is_some_and(is_directory);
        if is_selected_directory && !self.collapsed.contains(&path) {
            self.collapsed.insert(path);
            self.clamp_selection();
            return;
        }
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            self.select_path(parent);
        }
    }

    fn switch_pane(&mut self) {
        self.active_pane = match self.active_pane {
            Pane::Left => Pane::Right,
            Pane::Right => Pane::Left,
        };
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
        active_pane: Pane::Left,
        collapsed: BTreeSet::new(),
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
                KeyCode::Tab => app.switch_pane(),
                KeyCode::Char('h') | KeyCode::Left => app.collapse_or_select_parent(),
                KeyCode::Char('l') | KeyCode::Right => app.expand_selected_directory(),
                KeyCode::Enter => app.toggle_selected_directory(),
                KeyCode::Char(' ') => app.paused = !app.paused,
                KeyCode::Char('a') => {
                    let selected_path = app.selected_path();
                    app.show_identical = !app.show_identical;
                    if let Some(path) = selected_path {
                        app.select_path(&path);
                    } else {
                        app.clamp_selection();
                    }
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
    let selected_path = app.selected_path();
    match engine.scan() {
        Ok(report) => {
            let directories = report
                .entries
                .iter()
                .filter(|entry| is_directory(entry))
                .map(|entry| entry.path.clone())
                .collect::<BTreeSet<_>>();
            app.report = report;
            app.collapsed.retain(|path| directories.contains(path));
            app.last_error = None;
            if let Some(path) = selected_path {
                app.select_path(&path);
            } else {
                app.clamp_selection();
            }
        }
        Err(error) => app.last_error = Some(format!("{error:#}")),
    }
}

fn render(frame: &mut ratatui::Frame<'_>, app: &App) {
    let [header_area, panes_area, detail_area, footer_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(8),
        Constraint::Length(4),
        Constraint::Length(2),
    ])
    .areas(frame.area());

    render_header(frame, header_area, app);
    render_panes(frame, panes_area, app);
    render_detail(frame, detail_area, app);
    render_footer(frame, footer_area, app);
}

fn render_header(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let state = if app.paused { "PAUSED" } else { "LIVE" };
    let summary = &app.report.summary;
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
    let stats = Line::from(vec![
        stat("◀", summary.left_only, "left", Color::Blue),
        Span::raw("   "),
        stat("≠", summary.modified, "changed", Color::Yellow),
        Span::raw("   "),
        stat("⇄", summary.type_changed, "type", Color::Magenta),
        Span::raw("   "),
        stat("▶", summary.right_only, "right", Color::Green),
        Span::raw("   "),
        stat("=", summary.identical, "same", Color::DarkGray),
    ]);
    frame.render_widget(
        Paragraph::new(stats).block(Block::default().borders(Borders::ALL).title(title)),
        area,
    );
}

fn stat(symbol: &'static str, count: usize, label: &'static str, color: Color) -> Span<'static> {
    Span::styled(
        format!("{symbol} {count} {label}"),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )
}

fn render_panes(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let [left_area, right_area] = if area.width >= SIDE_BY_SIDE_MIN_WIDTH {
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(area)
    } else {
        Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(area)
    };
    render_pane(frame, left_area, app, Pane::Left);
    render_pane(frame, right_area, app, Pane::Right);
}

fn render_pane(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App, pane: Pane) {
    let visible = app.visible_rows();
    let show_metadata = area.width >= 36;
    let rows = visible.iter().map(|row| {
        let info = pane_info(row.entry, pane);
        let name = tree_name(row, info.is_some(), app.collapsed.contains(&row.entry.path));
        let marker = pane_marker(row.entry.kind, pane);
        let cells = match (pane, show_metadata) {
            (Pane::Left, true) => vec![
                Cell::from(name),
                Cell::from(info.map(pane_metadata).unwrap_or_default()),
                Cell::from(marker),
            ],
            (Pane::Right, true) => vec![
                Cell::from(marker),
                Cell::from(name),
                Cell::from(info.map(pane_metadata).unwrap_or_default()),
            ],
            (Pane::Left, false) => vec![Cell::from(name), Cell::from(marker)],
            (Pane::Right, false) => vec![Cell::from(marker), Cell::from(name)],
        };
        Row::new(cells).style(status_style(row.entry.kind))
    });

    let header = match (pane, show_metadata) {
        (Pane::Left, true) => Row::new(["NAME", "SIZE/TYPE", ""]),
        (Pane::Right, true) => Row::new(["", "NAME", "SIZE/TYPE"]),
        (Pane::Left, false) => Row::new(["NAME", ""]),
        (Pane::Right, false) => Row::new(["", "NAME"]),
    }
    .style(
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    );
    let widths = match (pane, show_metadata) {
        (Pane::Left, true) => vec![
            Constraint::Min(10),
            Constraint::Length(10),
            Constraint::Length(2),
        ],
        (Pane::Right, true) => vec![
            Constraint::Length(2),
            Constraint::Min(10),
            Constraint::Length(10),
        ],
        (Pane::Left, false) => vec![Constraint::Min(6), Constraint::Length(2)],
        (Pane::Right, false) => vec![Constraint::Length(2), Constraint::Min(6)],
    };
    let active = app.active_pane == pane;
    let root = match pane {
        Pane::Left => &app.report.left_root,
        Pane::Right => &app.report.right_root,
    };
    let label = match pane {
        Pane::Left => " LEFT ",
        Pane::Right => " RIGHT ",
    };
    let title = Line::from(vec![
        Span::styled(
            if active { "◆" } else { "◇" },
            Style::default().fg(if active { Color::Cyan } else { Color::DarkGray }),
        ),
        Span::styled(label, Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(
            root.display().to_string(),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw(" "),
    ]);
    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(if active {
            Style::default()
                .fg(Color::White)
                .bg(Color::Rgb(20, 55, 82))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White).bg(Color::Rgb(42, 46, 54))
        })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(if active {
                    Color::Cyan
                } else {
                    Color::DarkGray
                }))
                .title(title),
        );
    let mut state =
        TableState::default().with_selected((!visible.is_empty()).then_some(app.selected));
    frame.render_stateful_widget(table, area, &mut state);
}

fn tree_name(row: &VisibleRow<'_>, exists: bool, collapsed: bool) -> String {
    let indent = "  ".repeat(row.depth);
    if !exists {
        return format!("{indent}—");
    }
    let name = row
        .entry
        .path
        .file_name()
        .unwrap_or_else(|| row.entry.path.as_os_str())
        .to_string_lossy();
    let glyph = if is_directory(row.entry) {
        if collapsed { "▸ " } else { "▾ " }
    } else {
        "  "
    };
    let hidden = if row.collapsed_children > 0 {
        format!(" (+{})", row.collapsed_children)
    } else {
        String::new()
    };
    format!("{indent}{glyph}{name}{hidden}")
}

fn pane_info(entry: &DiffEntry, pane: Pane) -> Option<&EntryInfo> {
    match pane {
        Pane::Left => entry.left.as_ref(),
        Pane::Right => entry.right.as_ref(),
    }
}

fn pane_metadata(info: &EntryInfo) -> String {
    match info.kind {
        EntryKind::File | EntryKind::Other => human_bytes(info.len),
        EntryKind::Directory => "<DIR>".to_owned(),
        EntryKind::Symlink => "<LINK>".to_owned(),
    }
}

fn pane_marker(kind: DiffKind, pane: Pane) -> &'static str {
    match (kind, pane) {
        (DiffKind::LeftOnly, Pane::Left) => "◀",
        (DiffKind::RightOnly, Pane::Right) => "▶",
        (DiffKind::Modified, _) => "≠",
        (DiffKind::TypeChanged, _) => "⇄",
        (DiffKind::Identical, _) => "=",
        _ => "",
    }
}

fn render_detail(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let visible = app.visible_rows();
    let lines = if let Some(row) = visible.get(app.selected) {
        vec![
            Line::from(vec![
                Span::styled(
                    format!("{:<10}", row.entry.kind.plain_label()),
                    status_style(row.entry.kind).add_modifier(Modifier::BOLD),
                ),
                Span::raw(row.entry.path.to_string_lossy().into_owned()),
            ]),
            Line::from(vec![
                detail_span("LEFT", row.entry.left.as_ref()),
                Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
                detail_span("RIGHT", row.entry.right.as_ref()),
            ]),
        ]
    } else {
        vec![Line::from("No visible differences")]
    };
    frame.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" Selection ")),
        area,
    );
}

fn detail_span(label: &'static str, info: Option<&EntryInfo>) -> Span<'static> {
    Span::raw(format!(
        "{label} {}",
        info.map(EntryInfo::description)
            .unwrap_or_else(|| "—".to_owned())
    ))
}

fn render_footer(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    if let Some(error) = &app.last_error {
        frame.render_widget(
            Paragraph::new(Line::styled(
                format!("scan failed · {error} · r retry · q quit"),
                Style::default().fg(Color::Red),
            )),
            area,
        );
        return;
    }

    let scanned = SystemTime::now()
        .duration_since(app.report.scanned_at)
        .map(|duration| match duration.as_secs() {
            0 => "now".to_owned(),
            seconds => format!("{seconds}s ago"),
        })
        .unwrap_or_else(|_| "now".to_owned());
    let controls = if area.width >= 96 {
        "Tab pane   ↑↓/jk move   ←→/hl tree   Enter open   a same   r refresh   Space pause   q quit"
    } else {
        "Tab pane  ↑↓ move  ←→ tree  Enter open  a same  r scan  q quit"
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(controls, Style::default().fg(Color::Cyan)),
            Line::styled(
                format!("{} items · scanned {scanned}", app.visible_len()),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        area,
    );
}

fn is_directory(entry: &DiffEntry) -> bool {
    entry
        .left
        .as_ref()
        .into_iter()
        .chain(entry.right.as_ref())
        .any(|info| info.kind == EntryKind::Directory)
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

    fn app_with_report(report: DiffReport) -> App {
        App {
            report,
            selected: 0,
            active_pane: Pane::Left,
            collapsed: BTreeSet::new(),
            show_identical: false,
            paused: false,
            spinner: 0,
            last_error: None,
        }
    }

    fn screen(terminal: &Terminal<TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    }

    #[test]
    fn visible_rows_keep_identical_parent_and_collapse_its_differences() {
        let left = tempdir().unwrap();
        let right = tempdir().unwrap();
        fs::create_dir(left.path().join("folder")).unwrap();
        fs::create_dir(right.path().join("folder")).unwrap();
        fs::write(left.path().join("folder/changed.txt"), "left").unwrap();
        fs::write(right.path().join("folder/changed.txt"), "rght").unwrap();
        let report = DiffEngine::new(left.path(), right.path())
            .unwrap()
            .scan()
            .unwrap();
        let mut app = app_with_report(report);

        let paths = app
            .visible_rows()
            .iter()
            .map(|row| row.entry.path.as_path())
            .collect::<Vec<_>>();
        assert_eq!(
            paths,
            [Path::new("folder"), Path::new("folder/changed.txt")]
        );

        app.toggle_selected_directory();
        let rows = app.visible_rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].collapsed_children, 1);
    }

    #[test]
    fn refresh_keeps_the_selected_path_when_order_changes() {
        let left = tempdir().unwrap();
        let right = tempdir().unwrap();
        fs::write(left.path().join("b.txt"), "left").unwrap();
        let mut engine = DiffEngine::new(left.path(), right.path()).unwrap();
        let report = engine.scan().unwrap();
        let mut app = app_with_report(report);
        assert_eq!(app.selected_path().as_deref(), Some(Path::new("b.txt")));

        fs::write(left.path().join("a.txt"), "left").unwrap();
        refresh(&mut engine, &mut app);

        assert_eq!(app.selected_path().as_deref(), Some(Path::new("b.txt")));
    }

    #[test]
    fn wide_frame_contains_synchronized_commander_panes() {
        let left = tempdir().unwrap();
        let right = tempdir().unwrap();
        fs::write(left.path().join("changed.txt"), "left").unwrap();
        fs::write(right.path().join("changed.txt"), "rght").unwrap();
        let report = DiffEngine::new(left.path(), right.path())
            .unwrap()
            .scan()
            .unwrap();
        let app = app_with_report(report);
        let backend = TestBackend::new(120, 28);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|frame| render(frame, &app)).unwrap();
        let screen = screen(&terminal);

        assert!(screen.contains("fdiff"));
        assert!(screen.contains("LIVE"));
        assert!(screen.contains("LEFT"));
        assert!(screen.contains("RIGHT"));
        assert_eq!(screen.matches("changed.txt").count(), 3);
        assert!(screen.contains("Selection"));
    }

    #[test]
    fn narrow_frame_keeps_both_panes_readable() {
        let left = tempdir().unwrap();
        let right = tempdir().unwrap();
        fs::write(left.path().join("left.txt"), "left").unwrap();
        fs::write(right.path().join("right.txt"), "right").unwrap();
        let report = DiffEngine::new(left.path(), right.path())
            .unwrap()
            .scan()
            .unwrap();
        let app = app_with_report(report);
        let backend = TestBackend::new(60, 28);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|frame| render(frame, &app)).unwrap();
        let screen = screen(&terminal);

        assert!(screen.contains("LEFT"));
        assert!(screen.contains("RIGHT"));
        assert!(screen.contains("left.txt"));
        assert!(screen.contains("right.txt"));
    }
}
