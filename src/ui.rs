use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::app::{App, Focus};

pub fn ui(f: &mut ratatui::Frame, app: &App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1), Constraint::Length(1)])
        .split(f.area());

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

    let zone_items: Vec<ListItem> = app.zones
        .iter()
        .map(|z| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<12}", z.name), Style::default().fg(Color::White)),
                Span::styled(z.path.clone(), Style::default().fg(Color::DarkGray)),
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

    let all_zones_hint = if app.is_all_zone_selected() {
        let names: Vec<&str> = app.zones.iter()
            .filter(|z| z.path != "*")
            .map(|z| z.name.as_str())
            .collect();
        Some(format!("All zones: {}", names.join(", ")))
    } else {
        None
    };
    let status = all_zones_hint.as_deref()
        .or(app.status.as_deref())
        .unwrap_or("");
    f.render_widget(
        Paragraph::new(status).style(Style::default().fg(Color::DarkGray)),
        rows[1],
    );

    f.render_widget(Paragraph::new(shortcuts_hint(app)), rows[2]);
}

pub fn shortcuts_hint(app: &App) -> Line<'static> {
    let key_style = Style::default().fg(Color::Yellow);
    let dim_style = Style::default().fg(Color::DarkGray);

    let mut parts: Vec<(&'static str, &'static str)> = vec![];

    if app.ffmpeg_child.is_some() {
        parts.push(("[x]", "cancel encoding"));
    }

    match app.focus {
        Focus::Zones => {
            parts.push(("[h/j/k/l/←/↓/↑/→]", "Navigate"));
            parts.push(("[Enter/f]", "Run search on this zone"));
        }
        Focus::Actions => {
            parts.push(("[h/j/k/l/←/↓/↑/→]", "Navigate"));
            if app.selected_file.is_some() {
                for a in app.current_actions() {
                    parts.push((key_label(a.key), a.label));
                }
            } else {
                parts.push(("[f]", "Run search on this zone"));
            }
            parts.push(("[Tab/h]", "→ zones"));
        }
    }

    parts.push(("[Z]", "Configure zones"));

    let mut spans: Vec<Span<'static>> = vec![];
    for (i, (key, label)) in parts.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", dim_style));
        }
        spans.push(Span::styled(*key, key_style));
        spans.push(Span::styled(format!(" {}", label), dim_style));
    }
    Line::from(spans)
}

fn key_label(k: char) -> &'static str {
    match k {
        'o' => "[o]",
        'p' => "[p]",
        'e' => "[e]",
        'd' => "[d]",
        'c' => "[c]",
        'n' => "[n]",
        'r' => "[r]",
        _ => "[?]",
    }
}
