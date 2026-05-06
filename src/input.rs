use crossterm::event::{Event, KeyCode, KeyEventKind, MouseButton, MouseEvent, MouseEventKind};

use crate::app::{App, Focus};

pub enum AppCommand {
    Quit,
    OpenPicker,
    EditConfig,
    NavigateDown,
    NavigateUp,
    SwitchFocus,
    RunSelectedAction,
    RunActionByKey(char),
    CancelEncoding,
    StartDrag,
    MouseDrag(u16),
    MouseRelease,
    HoverGutter(bool),
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
    match code {
        KeyCode::Char('x') if app.ffmpeg_child.is_some() => AppCommand::CancelEncoding,
        KeyCode::Char('q') => AppCommand::Quit,
        KeyCode::Char('Z') => AppCommand::EditConfig,
        KeyCode::Tab | KeyCode::Char('h') | KeyCode::Char('l') => AppCommand::SwitchFocus,
        KeyCode::Char('f') => AppCommand::OpenPicker,
        KeyCode::Down | KeyCode::Char('j') => AppCommand::NavigateDown,
        KeyCode::Up | KeyCode::Char('k') => AppCommand::NavigateUp,
        KeyCode::Enter => match app.focus {
            Focus::Zones => AppCommand::OpenPicker,
            Focus::Actions => AppCommand::RunSelectedAction,
        },
        KeyCode::Char(c) if app.focus == Focus::Actions => AppCommand::RunActionByKey(c),
        _ => AppCommand::None,
    }
}

fn handle_mouse(mouse: MouseEvent, app: &App) -> AppCommand {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) if mouse.column == app.zone_width => AppCommand::StartDrag,
        MouseEventKind::Drag(MouseButton::Left) if app.dragging => AppCommand::MouseDrag(mouse.column),
        MouseEventKind::Up(MouseButton::Left) => AppCommand::MouseRelease,
        MouseEventKind::Moved => AppCommand::HoverGutter(mouse.column == app.zone_width),
        _ => AppCommand::None,
    }
}
