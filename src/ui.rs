use chrono::TimeZone;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table},
};

use crate::actions::FFMPEG_SUBACTIONS;
use crate::app::{App, Focus, HistoryConfirm};

pub fn ui(f: &mut ratatui::Frame, app: &mut App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1), Constraint::Length(1)])
        .split(f.area());

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(app.zone_panel_width()),
            Constraint::Min(0),
            Constraint::Length(App::actions_panel_width()),
        ])
        .split(rows[0]);

    let zone_items: Vec<ListItem> = app.zones
        .iter()
        .map(|z| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<12}", z.name), Style::default().fg(Color::White)),
                Span::styled(z.path.clone(), Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();

    let zones_focus = app.focus == Focus::Zones && !app.fzf_running;
    let zones_border_style = if zones_focus {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    f.render_stateful_widget(
        List::new(zone_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(Line::from(vec![
                        Span::styled("─", zones_border_style),
                        Span::styled("Zones", if zones_focus {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default()
                        }),
                    ]))
                    .border_style(zones_border_style),
            )
            .highlight_style(if app.fzf_running {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else if zones_focus {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default().bg(Color::Rgb(55, 55, 55))
            }),
        cols[0],
        &mut app.zone_state,
    );

    let history_focus = app.focus == Focus::History && !app.fzf_running;
    let history_border_style = if history_focus {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let history_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(Line::from(vec![
            Span::styled("─", history_border_style),
            Span::styled("History", if history_focus {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            }),
        ]))
        .border_style(history_border_style);

    if app.history.is_empty() {
        f.render_widget(
            List::new(vec![
                ListItem::new(Line::default()),
                ListItem::new(Span::styled(
                    "press [f] to search files",
                    Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
                )),
            ])
            .block(history_block),
            cols[1],
        );
    } else {
        let history_rows: Vec<Row> = app.history
            .iter()
            .map(|e| {
                let filename = e.path.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let parent = e.path.parent()
                    .map(|par| par.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let path_chars: Vec<char> = parent.chars().collect();
                let path_display = if path_chars.len() > 45 {
                    format!("…{}", path_chars[path_chars.len() - 44..].iter().collect::<String>())
                } else {
                    parent
                };
                Row::new(vec![
                    Cell::from(format_datetime(e.added_at)).style(Style::default().fg(Color::DarkGray)),
                    Cell::from(filename).style(Style::default().fg(Color::White)),
                    Cell::from(path_display).style(Style::default().fg(Color::DarkGray)),
                ])
            })
            .collect();
        f.render_stateful_widget(
            Table::new(history_rows, [
                Constraint::Length(11),
                Constraint::Min(0),
                Constraint::Length(45),
            ])
            .block(history_block)
            .column_spacing(1)
            .row_highlight_style(if history_focus {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default().bg(Color::Rgb(55, 55, 55))
            }),
            cols[1],
            &mut app.history_state,
        );
    }

    let actions_focus = app.focus == Focus::Actions && !app.fzf_running;
    let actions_border_style = if actions_focus {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
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
            let flash = app.flash_action == Some(a.key);
            if no_file {
                ListItem::new(Line::from(vec![
                    Span::styled(format!("[{}] ", a.key), Style::default().fg(Color::DarkGray)),
                    Span::styled(a.label, Style::default().fg(Color::DarkGray)),
                ]))
            } else if flash {
                ListItem::new(Line::from(vec![
                    Span::styled(format!("[{}] ", a.key), Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::styled(a.label, Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)),
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

    f.render_stateful_widget(
        List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(Line::from(vec![
                        Span::styled("─", actions_border_style),
                        Span::styled(panel_title, if actions_focus {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default()
                        }),
                    ]))
                    .border_style(actions_border_style),
            )
            .highlight_style(if actions_focus && !no_file {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            }),
        cols[2],
        &mut app.action_state,
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
    let status = app.status.as_deref()
        .or(all_zones_hint.as_deref())
        .unwrap_or("");
    f.render_widget(
        Paragraph::new(status).style(Style::default().fg(Color::DarkGray)),
        rows[1],
    );

    f.render_widget(Paragraph::new(shortcuts_hint(app)), rows[2]);

    if app.ffmpeg_submenu {
        let preview = app.ffmpeg_subaction_preview().unwrap_or("");
        let popup_h = FFMPEG_SUBACTIONS.len() as u16 + 8;
        let mut area = centered_rect(90, popup_h, f.area());
        area.y = area.y.saturating_sub(4);
        f.render_widget(Clear, area);
        let mut lines = vec![Line::default()];
        for (i, action) in FFMPEG_SUBACTIONS.iter().enumerate() {
            let selected = i == app.ffmpeg_submenu_idx;
            let label_style = if selected {
                Style::default().fg(Color::White).add_modifier(Modifier::REVERSED)
            } else {
                Style::default().fg(Color::White)
            };
            lines.push(Line::from(vec![
                Span::styled(format!(" [{}] ", action.key), Style::default().fg(Color::Yellow)),
                Span::styled(action.label, label_style),
            ]));
        }
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(" copies to clipboard:", Style::default().fg(Color::DarkGray))));
        lines.push(Line::from(Span::styled(format!("   {}", preview), Style::default().fg(Color::Cyan))));
        lines.push(Line::default());
        let key_style = Style::default().fg(Color::Yellow);
        let dim_style = Style::default().fg(Color::DarkGray);
        lines.push(Line::from(vec![
            Span::raw(" "),
            Span::styled("[j/k]", key_style),
            Span::styled(" Navigate", dim_style),
            Span::styled("  ", dim_style),
            Span::styled("[Enter]", key_style),
            Span::styled(" Copy to clipboard", dim_style),
            Span::styled("  ", dim_style),
            Span::styled("[Esc]", key_style),
            Span::styled(" Cancel", dim_style),
        ]));
        f.render_widget(
            Paragraph::new(lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(Span::styled(" ffmpeg — copy to clipboard ", Style::default().fg(Color::Yellow))),
            ),
            area,
        );
    }

    if let Some(ref confirm) = app.history_confirm {
        let question = match confirm {
            HistoryConfirm::DeleteEntry => " Delete this history entry?",
            HistoryConfirm::ClearAll => " Clear all history?",
        };
        let mut area = centered_rect(56, 6, f.area());
        area.y = area.y.saturating_sub(4);
        f.render_widget(Clear, area);
        f.render_widget(
            Paragraph::new(vec![
                Line::default(),
                Line::from(Span::styled(question, Style::default().fg(Color::White))),
                Line::default(),
                Line::from(vec![
                    Span::styled(" y or Enter", Style::default().fg(Color::Yellow)),
                    Span::styled(" to confirm  ·  ", Style::default().fg(Color::DarkGray)),
                    Span::styled("any other key", Style::default().fg(Color::Yellow)),
                    Span::styled(" to cancel", Style::default().fg(Color::DarkGray)),
                ]),
            ])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            ),
            area,
        );
    }
}

fn format_datetime(ts: u64) -> String {
    if ts == 0 {
        return String::new();
    }
    chrono::Local
        .timestamp_opt(ts as i64, 0)
        .single()
        .map(|dt| dt.format("%d/%m %H:%M").to_string())
        .unwrap_or_default()
}

fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let x = r.x + r.width.saturating_sub(width) / 2;
    let y = r.y + r.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(r.width), height.min(r.height))
}

fn shortcuts_hint(app: &App) -> Line<'static> {
    let key_style = Style::default().fg(Color::Yellow);
    let dim_style = Style::default().fg(Color::DarkGray);

    let mut parts: Vec<(&'static str, &'static str)> = vec![];

    match app.focus {
        Focus::Zones => {
            parts.push(("[j/k]", "Navigate"));
            parts.push(("[Enter/f]", "Run search on this zone"));
            parts.push(("[Tab/l]", "→ history"));
        }
        Focus::History => {
            parts.push(("[j/k]", "Navigate"));
            if !app.history.is_empty() {
                parts.push(("[Enter]", "Select"));
                parts.push(("[x]", "Delete entry"));
                parts.push(("[X]", "Clear all"));
            }
            parts.push(("[Tab/l]", "→ actions"));
            parts.push(("[h]", "→ zones"));
        }
        Focus::Actions => {
            parts.push(("[j/k]", "Navigate"));
            if app.selected_file.is_some() {
                for a in app.current_actions() {
                    parts.push((key_label(a.key), a.label));
                }
            } else {
                parts.push(("[f]", "Run search on this zone"));
            }
            parts.push(("[Tab/l]", "→ zones"));
            parts.push(("[h]", "→ history"));
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
