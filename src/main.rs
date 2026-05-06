mod actions;
mod app;
mod ffmpeg;
mod input;
mod ui;
mod zones;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, process::Command, time::Duration};

use app::{App, Focus};
use ffmpeg::ffmpeg_last_line;
use input::AppCommand;
use ui::ui;
use zones::config_path;

fn open_picker(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App, cmd: &str) -> Result<()> {
    let _ = std::fs::remove_file("/tmp/ratafzf_result");
    if std::env::var("TMUX").is_ok() {
        let full_cmd = format!("{}; tmux wait-for -S ratafzf-done", cmd);
        Command::new("tmux")
            .args(["split-window", "-v", "-l", "70%", "sh", "-c", &full_cmd])
            .status()?;
        app.fzf_running = true;
        terminal.draw(|f| ui(f, app))?;
        Command::new("tmux").args(["wait-for", "ratafzf-done"]).status()?;
        app.fzf_running = false;
    } else {
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
        Command::new("sh").args(["-c", cmd]).status()?;
        enable_raw_mode()?;
        execute!(terminal.backend_mut(), EnterAlternateScreen, EnableMouseCapture)?;
    }
    app.apply_fzf_result();
    terminal.clear()?;
    Ok(())
}

fn edit_config(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    let path = config_path();
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".into());
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    Command::new(&editor).arg(&path).status()?;
    enable_raw_mode()?;
    execute!(terminal.backend_mut(), EnterAlternateScreen, EnableMouseCapture)?;
    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        if let Some(ref log_path) = app.ffmpeg_log.clone() {
            if let Some(line) = ffmpeg_last_line(log_path) {
                app.status = Some(line);
            }
        }

        terminal.draw(|f| ui(f, app))?;

        if !event::poll(Duration::from_millis(250))? {
            continue;
        }

        match input::handle_event(event::read()?, app) {
            AppCommand::Quit => return Ok(()),
            AppCommand::OpenPicker => {
                let cmd = app.fzf_cmd();
                open_picker(terminal, app, &cmd)?;
            }
            AppCommand::EditConfig => {
                edit_config(terminal)?;
                app.reload_zones();
                terminal.clear()?;
            }
            AppCommand::SwitchFocus => {
                app.focus = match app.focus {
                    Focus::Zones => Focus::Actions,
                    Focus::Actions => Focus::Zones,
                };
            }
            AppCommand::NavigateDown => match app.focus {
                Focus::Zones => {
                    let i = app.zone_state.selected().unwrap_or(0);
                    app.zone_state.select(Some((i + 1) % app.zones.len()));
                }
                Focus::Actions => {
                    let n = app.current_actions().len();
                    if n > 0 {
                        let i = app.action_state.selected().unwrap_or(0);
                        app.action_state.select(Some((i + 1) % n));
                    }
                }
            },
            AppCommand::NavigateUp => match app.focus {
                Focus::Zones => {
                    let i = app.zone_state.selected().unwrap_or(0);
                    app.zone_state
                        .select(Some(if i == 0 { app.zones.len() - 1 } else { i - 1 }));
                }
                Focus::Actions => {
                    let n = app.current_actions().len();
                    if n > 0 {
                        let i = app.action_state.selected().unwrap_or(0);
                        app.action_state.select(Some(i.saturating_sub(1)));
                    }
                }
            },
            AppCommand::RunSelectedAction => {
                let idx = app.action_state.selected().unwrap_or(0);
                let actions = app.current_actions().to_vec();
                if let Some(action) = actions.get(idx) {
                    let _ = app.run_action(action);
                }
                if app.cd_target.is_some() { return Ok(()); }
            }
            AppCommand::RunActionByKey(c) => {
                let actions = app.current_actions().to_vec();
                if let Some(action) = actions.iter().find(|a| a.key == c) {
                    let _ = app.run_action(action);
                }
                if app.cd_target.is_some() { return Ok(()); }
            }
            AppCommand::CancelEncoding => {
                if let Some(mut child) = app.ffmpeg_child.take() {
                    let _ = child.kill();
                }
                app.ffmpeg_log = None;
                app.status = Some("Encodage annulé".to_string());
            }
            AppCommand::StartDrag => app.dragging = true,
            AppCommand::MouseDrag(col) => app.zone_width = col.max(10),
            AppCommand::MouseRelease => app.dragging = false,
            AppCommand::HoverGutter(hover) => app.hover_gutter = hover,
            AppCommand::None => {}
        }
    }
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
