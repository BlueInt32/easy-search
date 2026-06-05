use crossterm::event::{Event, KeyCode, KeyEventKind, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

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
    RetriggerHistoryEntry,
    RunFfmpegSubaction(char),
    FfmpegSubmenuDown,
    FfmpegSubmenuUp,
    CloseFfmpegSubmenu,
    OpenZoneFolder,
    CdToZone,
    ClickZone(usize),
    ClickHistory(usize),
    ClickAction(usize),
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
        KeyCode::Char('x') if app.focus == Focus::History || app.focus == Focus::Actions => AppCommand::AskDeleteEntry,
        KeyCode::Char('X') if app.focus == Focus::History || app.focus == Focus::Actions => AppCommand::AskClearAll,
        KeyCode::Char('o') if app.focus == Focus::Zones => AppCommand::OpenZoneFolder,
        KeyCode::Char('d') if app.focus == Focus::Zones => AppCommand::CdToZone,
        KeyCode::Char('q') => AppCommand::Quit,
        KeyCode::Char('Z') => AppCommand::EditConfig,
        KeyCode::Tab | KeyCode::Char('l') | KeyCode::Right => AppCommand::FocusNext,
        KeyCode::Char('h') | KeyCode::Left => AppCommand::FocusPrev,
        KeyCode::Char('f') if app.focus == Focus::History => AppCommand::RetriggerHistoryEntry,
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

fn in_rect(rect: Rect, col: u16, row: u16) -> bool {
    col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
}

fn handle_mouse(mouse: MouseEvent, app: &App) -> AppCommand {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {}
        _ => return AppCommand::None,
    }
    let (col, row) = (mouse.column, mouse.row);
    if in_rect(app.zone_rect, col, row) {
        let idx = row.saturating_sub(app.zone_rect.y + 1) as usize;
        return AppCommand::ClickZone(idx);
    }
    if in_rect(app.history_rect, col, row) {
        let idx = row.saturating_sub(app.history_rect.y + 1) as usize;
        return AppCommand::ClickHistory(idx);
    }
    if in_rect(app.actions_rect, col, row) {
        let idx = row.saturating_sub(app.actions_rect.y + 1) as usize;
        return AppCommand::ClickAction(idx);
    }
    AppCommand::None
}

