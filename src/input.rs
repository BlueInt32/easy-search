use crossterm::event::{Event, KeyCode, KeyEventKind, MouseButton, MouseEvent, MouseEventKind};

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
    CancelEncoding,
    AskDeleteEntry,
    AskClearAll,
    ConfirmHistoryAction,
    CancelHistoryAction,
    SelectHistoryItem,
    StartDrag,
    StartDragRight,
    MouseDrag(u16),
    MouseDragRight(u16),
    MouseRelease,
    HoverGutter(bool, bool),
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
    match code {
        KeyCode::Char('x') if app.ffmpeg_child.is_some() => AppCommand::CancelEncoding,
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
        KeyCode::Char(c) if app.focus == Focus::Actions => AppCommand::RunActionByKey(c),
        _ => AppCommand::None,
    }
}

fn handle_mouse(mouse: MouseEvent, app: &App) -> AppCommand {
    let left_col = app.zone_width;
    let right_col = app.zone_width + 1 + app.history_width;
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) if mouse.column == left_col => AppCommand::StartDrag,
        MouseEventKind::Down(MouseButton::Left) if mouse.column == right_col => AppCommand::StartDragRight,
        MouseEventKind::Drag(MouseButton::Left) if app.dragging => AppCommand::MouseDrag(mouse.column),
        MouseEventKind::Drag(MouseButton::Left) if app.dragging_right => AppCommand::MouseDragRight(mouse.column),
        MouseEventKind::Up(MouseButton::Left) => AppCommand::MouseRelease,
        MouseEventKind::Moved => AppCommand::HoverGutter(
            mouse.column == left_col,
            mouse.column == right_col,
        ),
        _ => AppCommand::None,
    }
}

