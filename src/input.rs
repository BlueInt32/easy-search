use crossterm::event::{Event, KeyCode, KeyEventKind, MouseEvent};

use crate::actions::FFMPEG_SUBACTIONS;
use crate::app::{App, Focus};

pub enum AppCommand {
    Quit,
    OpenPicker,
    EditConfig,
    NavigateDown,
    NavigateUp,
    FocusNext,
    FocusPrev,
    RunSelectedAction,
    RunActionByKey(char),
    AskDeleteEntry,
    AskClearAll,
    ConfirmHistoryAction,
    CancelHistoryAction,
    SelectHistoryItem,
    RunFfmpegSubaction(char),
    FfmpegSubmenuDown,
    FfmpegSubmenuUp,
    CloseFfmpegSubmenu,
    None,
}

pub fn handle_event(event: Event, app: &App) -> AppCommand {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => handle_key(key.code, app),
        Event::Mouse(mouse) => handle_mouse(mouse, app),
        _ => AppCommand::None,
    }
}

fn handle_key(code: KeyCode, app: &App) -> AppCommand {
    if app.history_confirm.is_some() {
        return match code {
            KeyCode::Char('y') | KeyCode::Enter => AppCommand::ConfirmHistoryAction,
            _ => AppCommand::CancelHistoryAction,
        };
    }
    if app.ffmpeg_submenu {
        return match code {
            KeyCode::Esc => AppCommand::CloseFfmpegSubmenu,
            KeyCode::Char('q') => AppCommand::Quit,
            KeyCode::Down | KeyCode::Char('j') => AppCommand::FfmpegSubmenuDown,
            KeyCode::Up | KeyCode::Char('k') => AppCommand::FfmpegSubmenuUp,
            KeyCode::Enter => {
                if let Some(action) = FFMPEG_SUBACTIONS.get(app.ffmpeg_submenu_idx) {
                    AppCommand::RunFfmpegSubaction(action.key)
                } else {
                    AppCommand::None
                }
            }
            KeyCode::Char(c) => AppCommand::RunFfmpegSubaction(c),
            _ => AppCommand::None,
        };
    }
    match code {
        KeyCode::Char('x') if app.focus == Focus::History => AppCommand::AskDeleteEntry,
        KeyCode::Char('X') if app.focus == Focus::History => AppCommand::AskClearAll,
        KeyCode::Char('q') => AppCommand::Quit,
        KeyCode::Char('Z') => AppCommand::EditConfig,
        KeyCode::Tab | KeyCode::Char('l') | KeyCode::Right => AppCommand::FocusNext,
        KeyCode::Char('h') | KeyCode::Left => AppCommand::FocusPrev,
        KeyCode::Char('f') => AppCommand::OpenPicker,
        KeyCode::Down | KeyCode::Char('j') => AppCommand::NavigateDown,
        KeyCode::Up | KeyCode::Char('k') => AppCommand::NavigateUp,
        KeyCode::Enter => match app.focus {
            Focus::Zones => AppCommand::OpenPicker,
            Focus::History => AppCommand::SelectHistoryItem,
            Focus::Actions => AppCommand::RunSelectedAction,
        },
        KeyCode::Char(c) if app.focus == Focus::Actions || app.focus == Focus::History => AppCommand::RunActionByKey(c),
        _ => AppCommand::None,
    }
}

fn handle_mouse(_mouse: MouseEvent, _app: &App) -> AppCommand {
    AppCommand::None
}

