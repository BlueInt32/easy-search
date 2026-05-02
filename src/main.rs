use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    event::EnableMouseCapture,
    event::DisableMouseCapture,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
use std::{
    io,
    path::PathBuf,
    process::{Command, Stdio},
    time::Duration,
};

#[derive(Debug, Clone)]
struct Action {
    key: char,
    label: &'static str,
}

const ACTIONS_FILE: &[Action] = &[
    Action { key: 'o', label: "open" },
    Action { key: 'e', label: "edit with kate" },
    Action { key: 'd', label: "cd" },
    Action { key: 'c', label: "copy path" },
    Action { key: 'n', label: "copy filename" },
    Action { key: 'r', label: "reenc" },
    Action { key: 'p', label: "open parent folder" },
];

const ACTIONS_DIR: &[Action] = &[
    Action { key: 'o', label: "open" },
    Action { key: 'd', label: "cd" },
    Action { key: 'c', label: "copy path" },
    Action { key: 'n', label: "copy filename" },
];

struct Zone {
    name: &'static str,
    path: &'static str,
}

const ZONES: &[Zone] = &[
    Zone { name: "home",    path: "/home/user" },
    Zone { name: "musique", path: "/mnt/WIN_E/Musique/" },
];

#[derive(PartialEq)]
enum Focus {
    Zones,
    Actions,
}

struct App {
    focus: Focus,
    zone_state: ListState,
    selected_file: Option<PathBuf>,
    action_state: ListState,
    history: Vec<PathBuf>,
    status: Option<String>,
    ffmpeg_log: Option<String>,
    ffmpeg_child: Option<std::process::Child>,
    cd_target: Option<PathBuf>,
    zone_width: u16,
    dragging: bool,
    hover_gutter: bool,
}

impl App {
    fn new() -> Self {
        let mut zone_state = ListState::default();
        zone_state.select(Some(0));
        let mut action_state = ListState::default();
        action_state.select(Some(0));
        Self {
            focus: Focus::Zones,
            zone_state,
            selected_file: None,
            action_state,
            history: vec![],
            status: None,
            ffmpeg_log: None,
            ffmpeg_child: None,
            cd_target: None,
            zone_width: 30,
            dragging: false,
            hover_gutter: false,
        }
    }

    fn current_zone_path(&self) -> &'static str {
        let i = self.zone_state.selected().unwrap_or(0);
        ZONES.get(i).map(|z| z.path).unwrap_or("/home/user")
    }

    fn current_actions(&self) -> &[Action] {
        match &self.selected_file {
            Some(p) if p.is_dir() => ACTIONS_DIR,
            Some(_) | None => ACTIONS_FILE,
        }
    }

    fn fzf_cmd(&self) -> String {
        format!(
            "fdfind --hidden --no-ignore --search-path '{}' \
             -E .wine -E .java -E .thunderbird -E .mozilla -E .git -E node_modules -E obj \
             | fzf --border rounded --border-label ' Pick ' --border-label-pos 2 \
                   --preview '~/.config/fzf/preview.sh {{}}' --preview-window=right:50%:border-left \
             > /tmp/ratafzf_result",
            self.current_zone_path()
        )
    }

    fn apply_fzf_result(&mut self) {
        if let Ok(content) = std::fs::read_to_string("/tmp/ratafzf_result") {
            let path_str = content.trim().to_string();
            if !path_str.is_empty() {
                let path = PathBuf::from(&path_str);
                self.selected_file = Some(path.clone());
                self.history.push(path);
                self.action_state.select(Some(0));
                self.focus = Focus::Actions;
                self.status = None;
            }
        }
    }

    fn run_action(&mut self, action: &Action) -> Result<()> {
        let path = match &self.selected_file {
            Some(p) => p.clone(),
            None => return Ok(()),
        };

        match action.key {
            'o' => {
                Command::new("xdg-open").arg(&path)
                    .stdout(Stdio::null()).stderr(Stdio::null()).spawn()?;
                self.status = Some(format!("Opened {}", path.display()));
            }
            'e' => {
                Command::new("kate").arg(&path)
                    .stdout(Stdio::null()).stderr(Stdio::null()).spawn()?;
                self.status = Some(format!("Editing {}", path.display()));
            }
            'd' => {
                let target = if path.is_dir() { path.clone() } else { path.parent().unwrap_or(&path).to_path_buf() };
                self.cd_target = Some(target);
            }
            'c' => {
                copy_to_clipboard(&path.to_string_lossy())?;
                self.status = Some("Chemin copié".to_string());
            }
            'n' => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                copy_to_clipboard(&name)?;
                self.status = Some("Nom copié".to_string());
            }
            'r' => {
                let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                let output = path.with_file_name(format!("{}-reenc.mp4", stem));
                let log_path = "/tmp/ratafzf_ffmpeg.log";
                let log_file = std::fs::File::create(log_path)?;
                let child = Command::new("ffmpeg")
                    .args([
                        "-i", &path.to_string_lossy(),
                        "-c:v", "libx264", "-crf", "18", "-preset", "slow",
                        "-c:a", "copy", &output.to_string_lossy(),
                    ])
                    .stdout(Stdio::null()).stderr(log_file)
                    .spawn()?;
                self.ffmpeg_child = Some(child);
                self.ffmpeg_log = Some(log_path.to_string());
                self.status = Some(format!("Encoding → {}", output.display()));
            }
            'p' => {
                let parent = path.parent().unwrap_or(&path).to_path_buf();
                Command::new("xdg-open").arg(&parent)
                    .stdout(Stdio::null()).stderr(Stdio::null()).spawn()?;
                self.status = Some(format!("Opened {}", parent.display()));
            }
            _ => {}
        }
        Ok(())
    }
}

fn copy_to_clipboard(s: &str) -> Result<()> {
    use std::io::Write;
    let mut child = Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(Stdio::piped())
        .spawn()?;
    child.stdin.as_mut().unwrap().write_all(s.as_bytes())?;
    child.wait()?;
    Ok(())
}

fn ffmpeg_last_line(log_path: &str) -> Option<String> {
    let content = std::fs::read_to_string(log_path).ok()?;
    content.split('\r').filter(|s| !s.trim().is_empty()).last()
        .map(|s| s.trim().to_string())
}


fn open_picker(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    let _ = std::fs::remove_file("/tmp/ratafzf_result");
    if std::env::var("TMUX").is_ok() {
        let cmd = format!("{}; tmux wait-for -S ratafzf-done", app.fzf_cmd());
        Command::new("tmux")
            .args(["split-window", "-v", "-l", "70%", "sh", "-c", &cmd])
            .status()?;
        Command::new("tmux").args(["wait-for", "ratafzf-done"]).status()?;
    } else {
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
        Command::new("sh").args(["-c", &app.fzf_cmd()]).status()?;
        enable_raw_mode()?;
        execute!(terminal.backend_mut(), EnterAlternateScreen, EnableMouseCapture)?;
    }
    app.apply_fzf_result();
    terminal.clear()?;
    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        if let Some(ref log_path) = app.ffmpeg_log.clone() {
            if let Some(line) = ffmpeg_last_line(log_path) {
                app.status = Some(format!("{}  [x] annuler", line));
            }
        }

        terminal.draw(|f| ui(f, app))?;

        if !event::poll(Duration::from_millis(250))? {
            continue;
        }

        match event::read()? {
            Event::Mouse(mouse) => {
                match mouse.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        if mouse.column == app.zone_width {
                            app.dragging = true;
                        }
                    }
                    MouseEventKind::Drag(MouseButton::Left) => {
                        if app.dragging {
                            app.zone_width = mouse.column.max(10);
                        }
                    }
                    MouseEventKind::Up(MouseButton::Left) => {
                        app.dragging = false;
                    }
                    MouseEventKind::Moved => {
                        app.hover_gutter = mouse.column == app.zone_width;
                    }
                    _ => {}
                }
                continue;
            }
            Event::Key(key) => {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                KeyCode::Char('x') if app.ffmpeg_child.is_some() => {
                    if let Some(mut child) = app.ffmpeg_child.take() {
                        let _ = child.kill();
                    }
                    app.ffmpeg_log = None;
                    app.status = Some("Encodage annulé".to_string());
                }
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Tab | KeyCode::Char('h') | KeyCode::Char('l') => {
                    app.focus = match app.focus {
                        Focus::Zones => Focus::Actions,
                        Focus::Actions => Focus::Zones,
                    };
                }
                KeyCode::Char('f') => {
                    open_picker(terminal, app)?;
                }
                KeyCode::Down | KeyCode::Char('j') => match app.focus {
                    Focus::Zones => {
                        let i = app.zone_state.selected().unwrap_or(0);
                        app.zone_state.select(Some((i + 1) % ZONES.len()));
                    }
                    Focus::Actions => {
                        let n = app.current_actions().len();
                        if n > 0 {
                            let i = app.action_state.selected().unwrap_or(0);
                            app.action_state.select(Some((i + 1) % n));
                        }
                    }
                },
                KeyCode::Up | KeyCode::Char('k') => match app.focus {
                    Focus::Zones => {
                        let i = app.zone_state.selected().unwrap_or(0);
                        app.zone_state
                            .select(Some(if i == 0 { ZONES.len() - 1 } else { i - 1 }));
                    }
                    Focus::Actions => {
                        let n = app.current_actions().len();
                        if n > 0 {
                            let i = app.action_state.selected().unwrap_or(0);
                            app.action_state.select(Some(i.saturating_sub(1)));
                        }
                    }
                },
                KeyCode::Enter => match app.focus {
                    Focus::Zones => {
                        open_picker(terminal, app)?;
                    }
                    Focus::Actions => {
                        let idx = app.action_state.selected().unwrap_or(0);
                        let actions = app.current_actions().to_vec();
                        if let Some(action) = actions.get(idx) {
                            let _ = app.run_action(action);
                        }
                        if app.cd_target.is_some() { return Ok(()); }
                    }
                },
                KeyCode::Char(c) if app.focus == Focus::Actions => {
                    let actions = app.current_actions().to_vec();
                    if let Some(action) = actions.iter().find(|a| a.key == c) {
                        let _ = app.run_action(action);
                    }
                    if app.cd_target.is_some() { return Ok(()); }
                }
                _ => {}
            }
            }
            _ => {}
        }
    }
}

fn ui(f: &mut ratatui::Frame, app: &App) {
    // Split vertically first: main area + statusbar
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(f.area());

    // Split main area: zones | gutter (1 col) | actions
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(app.zone_width),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(rows[0]);

    let gutter_style = if app.hover_gutter || app.dragging {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    // Gutter widget — caractères de jointure avec les bordures hautes/basses
    let h = cols[1].height as usize;
    let mut gutter_lines = Vec::with_capacity(h);
    if h > 0 {
        gutter_lines.push(Line::from(Span::styled("┬", gutter_style)));
        for _ in 1..h.saturating_sub(1) {
            gutter_lines.push(Line::from(Span::styled("│", gutter_style)));
        }
        if h > 1 {
            gutter_lines.push(Line::from(Span::styled("┴", gutter_style)));
        }
    }
    f.render_widget(Paragraph::new(gutter_lines), cols[1]);

    // Zones panel — pas de bordure droite (la goutière la remplace)
    let zone_items: Vec<ListItem> = ZONES
        .iter()
        .map(|z| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<12}", z.name), Style::default().fg(Color::White)),
                Span::styled(z.path, Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();

    let zones_focus = app.focus == Focus::Zones;
    let mut zone_state = app.zone_state.clone();
    f.render_stateful_widget(
        List::new(zone_items)
            .block(
                Block::default()
                    .borders(Borders::LEFT | Borders::TOP | Borders::BOTTOM)
                    .title(Span::styled("Zones", if zones_focus {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    }))
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .highlight_style(if zones_focus {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            }),
        cols[0],
        &mut zone_state,
    );

    // Actions panel — pas de bordure gauche (la goutière la remplace)
    let actions_focus = app.focus == Focus::Actions;
    let panel_title = app
        .selected_file
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "Actions".to_string());

    let no_file = app.selected_file.is_none();
    let mut items: Vec<ListItem> = app
        .current_actions()
        .iter()
        .map(|a| {
            if no_file {
                ListItem::new(Line::from(vec![
                    Span::styled(format!("[{}] ", a.key), Style::default().fg(Color::DarkGray)),
                    Span::styled(a.label, Style::default().fg(Color::DarkGray)),
                ]))
            } else {
                ListItem::new(Line::from(vec![
                    Span::styled(format!("[{}] ", a.key), Style::default().fg(Color::Yellow)),
                    Span::raw(a.label),
                ]))
            }
        })
        .collect();
    if no_file {
        items.push(ListItem::new(Line::default()));
        items.push(ListItem::new(Span::styled(
            "press [f] to select a file first",
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
        )));
    }

    let mut action_state = app.action_state.clone();
    f.render_stateful_widget(
        List::new(items)
            .block(
                Block::default()
                    .borders(Borders::RIGHT | Borders::TOP | Borders::BOTTOM)
                    .title(Span::styled(panel_title, if actions_focus {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    }))
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .highlight_style(if actions_focus && !no_file {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            }),
        cols[2],
        &mut action_state,
    );

    // Status bar
    let status = app
        .status
        .as_deref()
        .unwrap_or("[Tab] changer focus  [f/Entrée] pick  [q] quit");
    f.render_widget(
        Paragraph::new(status).style(Style::default().fg(Color::DarkGray)),
        rows[1],
    );
}


fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;

    if let Some(ref target) = app.cd_target {
        std::fs::write("/tmp/ratafzf_lastdir", target.to_string_lossy().as_bytes())?;
    }

    result
}
