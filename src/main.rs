mod actions;
mod app;
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

use app::{App, Focus, HistoryConfirm};
use input::AppCommand;
use ui::ui;
use zones::config_path;

fn open_picker(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App, cmd: &str) -> Result<()> {
    let _ = std::fs::remove_file("/tmp/easy-search_result");
    if std::env::var("TMUX").is_ok() {
        let full_cmd = format!("{}; tmux wait-for -S easy-search-done", cmd);
        Command::new("tmux")
            .args(["split-window", "-v", "-l", "70%", "sh", "-c", &full_cmd])
            .status()?;
        app.fzf_running = true;
        terminal.draw(|f| ui(f, app))?;
        Command::new("tmux").args(["wait-for", "easy-search-done"]).status()?;
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

fn edit_file(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, path: &std::path::Path) -> Result<()> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nvim".into());
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    Command::new(&editor).arg(path).status()?;
    enable_raw_mode()?;
    execute!(terminal.backend_mut(), EnterAlternateScreen, EnableMouseCapture)?;
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
        terminal.draw(|f| ui(f, app))?;
        app.flash_action = None;

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
            AppCommand::FocusNext => {
                app.status = None;
                app.focus = match app.focus {
                    Focus::Zones => Focus::History,
                    Focus::History => Focus::Actions,
                    Focus::Actions => Focus::Zones,
                };
                if app.focus == Focus::History {
                    app.sync_selected_from_history();
                }
            }
            AppCommand::FocusPrev => {
                app.status = None;
                app.focus = match app.focus {
                    Focus::Zones => Focus::Actions,
                    Focus::History => Focus::Zones,
                    Focus::Actions => Focus::History,
                };
                if app.focus == Focus::History {
                    app.sync_selected_from_history();
                }
            }
            AppCommand::AskDeleteEntry => app.history_confirm = Some(HistoryConfirm::DeleteEntry),
            AppCommand::AskClearAll => app.history_confirm = Some(HistoryConfirm::ClearAll),
            AppCommand::ConfirmHistoryAction => {
                match app.history_confirm.take() {
                    Some(HistoryConfirm::DeleteEntry) => app.delete_selected_history_entry(),
                    Some(HistoryConfirm::ClearAll) => app.clear_history(),
                    None => {}
                }
            }
            AppCommand::CancelHistoryAction => { app.history_confirm = None; }
            AppCommand::SelectHistoryItem => {
                app.select_history_item();
                if app.cd_target.is_some() { return Ok(()); }
            }
            AppCommand::NavigateDown => {
                app.status = None;
                match app.focus {
                Focus::Zones => {
                    let i = app.zone_state.selected().unwrap_or(0);
                    app.zone_state.select(Some((i + 1) % app.zones.len()));
                }
                Focus::History => {
                    let n = app.history.len();
                    if n > 0 {
                        let i = app.history_state.selected().unwrap_or(0);
                        app.history_state.select(Some((i + 1) % n));
                        app.sync_selected_from_history();
                    }
                }
                Focus::Actions => {
                    let n = app.current_actions().len();
                    if n > 0 {
                        let i = app.action_state.selected().unwrap_or(0);
                        app.action_state.select(Some((i + 1) % n));
                    }
                }
            }},
            AppCommand::NavigateUp => {
                app.status = None;
                match app.focus {
                Focus::Zones => {
                    let i = app.zone_state.selected().unwrap_or(0);
                    app.zone_state
                        .select(Some(if i == 0 { app.zones.len() - 1 } else { i - 1 }));
                }
                Focus::History => {
                    let n = app.history.len();
                    if n > 0 {
                        let i = app.history_state.selected().unwrap_or(0);
                        app.history_state.select(Some(i.saturating_sub(1)));
                        app.sync_selected_from_history();
                    }
                }
                Focus::Actions => {
                    let n = app.current_actions().len();
                    if n > 0 {
                        let i = app.action_state.selected().unwrap_or(0);
                        app.action_state.select(Some(i.saturating_sub(1)));
                    }
                }
            }},
            AppCommand::RunSelectedAction => {
                let idx = app.action_state.selected().unwrap_or(0);
                let actions = app.current_actions().to_vec();
                if let Some(action) = actions.get(idx) {
                    let _ = app.run_action(action);
                }
                if app.cd_target.is_some() { return Ok(()); }
                if let Some(path) = app.edit_target.take() { edit_file(terminal, &path)?; }
            }
            AppCommand::RunActionByKey(c) => {
                app.flash_action = Some(c);
                terminal.draw(|f| ui(f, app))?;
                let actions = app.current_actions().to_vec();
                if let Some(action) = actions.iter().find(|a| a.key == c) {
                    let _ = app.run_action(action);
                }
                if app.cd_target.is_some() { return Ok(()); }
                if let Some(path) = app.edit_target.take() { edit_file(terminal, &path)?; }
            }
            AppCommand::RunFfmpegSubaction(c) => {
                let _ = app.run_ffmpeg_subaction(c);
            }
            AppCommand::FfmpegSubmenuDown => {
                let n = crate::actions::FFMPEG_SUBACTIONS.len();
                if n > 0 {
                    app.ffmpeg_submenu_idx = (app.ffmpeg_submenu_idx + 1) % n;
                }
            }
            AppCommand::FfmpegSubmenuUp => {
                let n = crate::actions::FFMPEG_SUBACTIONS.len();
                if n > 0 {
                    app.ffmpeg_submenu_idx = app.ffmpeg_submenu_idx.saturating_sub(1);
                }
            }
            AppCommand::CloseFfmpegSubmenu => {
                app.ffmpeg_submenu = false;
            }
            AppCommand::None => {}
        }
    }
}

struct TerminalGuard;
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
    }
}

fn main() -> Result<()> {
    enable_raw_mode()?;
    let _guard = TerminalGuard;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;

    if let Some(ref target) = app.cd_target {
        std::fs::write("/tmp/easy-search_lastdir", target.to_string_lossy().as_bytes())?;
    }

    result
}
