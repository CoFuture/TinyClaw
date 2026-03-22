//! TUI UI components

use crate::tui::state::{AppState, AgentActivityType};
use ratatui::{
    layout::Alignment,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

/// Draw the session list panel
pub fn draw_sessions_panel(f: &mut Frame<'_>, area: Rect, state: &AppState) {
    let connection_indicator = if state.connected { "●" } else { "○" };
    let block = Block::default()
        .title(format!(" Sessions {} ", connection_indicator))
        .borders(Borders::ALL);

    let items: Vec<ListItem> = if state.sessions.is_empty() {
        vec![ListItem::new(" No sessions ")]
    } else {
        state.sessions.iter()
            .map(|s| {
                let prefix = if Some(s.as_str()) == state.current_session_id.as_deref() {
                    "● "
                } else {
                    "  "
                };
                let display_name = if s == "main" { "Main" } else { s.as_str() };
                let style = if Some(s.as_str()) == state.current_session_id.as_deref() {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                };
                ListItem::new(format!("{}{}", prefix, display_name)).style(style)
            })
            .collect()
    };

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

/// Get the activity indicator line based on agent activity state
fn get_activity_indicator(state: &AppState) -> Option<Line<'static>> {
    if !state.loading {
        return None;
    }
    match state.agent_activity.activity_type {
        AgentActivityType::Thinking => {
            Some(Line::from(vec![
                Span::styled("🤔 ", Style::default().fg(Color::Yellow)),
                Span::styled("thinking...", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
            ]))
        }
        AgentActivityType::UsingTool => {
            let tool_name = state.agent_activity.tool_name.as_deref().unwrap_or("unknown");
            Some(Line::from(vec![
                Span::styled("🔧 ", Style::default().fg(Color::Magenta)),
                Span::styled(format!("using tool: {}", tool_name), Style::default().fg(Color::Magenta).add_modifier(Modifier::ITALIC)),
            ]))
        }
        AgentActivityType::Waiting | AgentActivityType::Idle => {
            Some(Line::from(vec![
                Span::styled("⏳ ", Style::default().fg(Color::DarkGray)),
                Span::styled("waiting...", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
            ]))
        }
    }
}

/// Draw the message view panel with enhanced formatting
pub fn draw_messages_panel(f: &mut Frame<'_>, area: Rect, state: &AppState) {
    let title_text = state.current_session_id.as_deref()
        .map(|s| if s == "main" { " Main Session " } else { s })
        .unwrap_or(" Messages - No session ");
    
    let block = Block::default()
        .title(title_text)
        .borders(Borders::ALL);

    let messages = state.get_current_messages();
    
    let lines: Vec<Line> = if messages.is_empty() {
        vec![Line::from(vec![
            Span::raw("No messages yet. "),
            Span::styled("Select a session", Style::default().fg(Color::Cyan)),
            Span::raw(" and start chatting!"),
        ])]
    } else {
        let mut result: Vec<Line> = Vec::new();
        
        for msg in messages.iter().skip(state.scroll_offset) {
            // Get timestamp - clone to avoid lifetime issues
            let ts = msg.timestamp.format("%H:%M:%S").to_string();
            
            // Get role styling
            match msg.role {
                crate::types::Role::User => {
                    let content = msg.content.trim();
                    let content_lines: Vec<&str> = content.split('\n').collect();
                    let first_line = content_lines.first().unwrap_or(&"").to_string();
                    result.push(Line::from(vec![
                        Span::raw("["),
                        Span::styled(ts.clone(), Style::default().fg(Color::DarkGray)),
                        Span::raw("] "),
                        Span::styled("User", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                        Span::raw(": "),
                        Span::raw(first_line),
                    ]));
                    for line in content_lines.iter().skip(1) {
                        result.push(Line::from(vec![
                            Span::raw("          "),
                            Span::raw(line.to_string()),
                        ]));
                    }
                }
                crate::types::Role::Assistant => {
                    let content = msg.content.trim();
                    let content_lines: Vec<&str> = content.split('\n').collect();
                    let first_line = content_lines.first().unwrap_or(&"").to_string();
                    result.push(Line::from(vec![
                        Span::raw("["),
                        Span::styled(ts.clone(), Style::default().fg(Color::DarkGray)),
                        Span::raw("] "),
                        Span::styled("Assistant", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                        Span::raw(": "),
                        Span::raw(first_line),
                    ]));
                    for line in content_lines.iter().skip(1) {
                        result.push(Line::from(vec![
                            Span::raw("          "),
                            Span::raw(line.to_string()),
                        ]));
                    }
                }
                crate::types::Role::System => {
                    let content = msg.content.trim();
                    let content_lines: Vec<&str> = content.split('\n').collect();
                    let first_line = content_lines.first().unwrap_or(&"").to_string();
                    result.push(Line::from(vec![
                        Span::raw("["),
                        Span::styled(ts.clone(), Style::default().fg(Color::DarkGray)),
                        Span::raw("] "),
                        Span::styled("System", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                        Span::raw(": "),
                        Span::raw(first_line),
                    ]));
                    for line in content_lines.iter().skip(1) {
                        result.push(Line::from(vec![
                            Span::raw("          "),
                            Span::raw(line.to_string()),
                        ]));
                    }
                }
                crate::types::Role::Tool => {
                    let tool_name = msg.tool_name.as_deref().unwrap_or("tool").to_string();
                    let content = msg.content.trim();
                    // Show first line of content, truncate if long
                    let first_line = if content.len() > 100 {
                        format!("{}...", &content[..100])
                    } else {
                        content.to_string()
                    };
                    result.push(Line::from(vec![
                        Span::raw("["),
                        Span::styled(ts, Style::default().fg(Color::DarkGray)),
                        Span::raw("] "),
                        Span::styled("Tool", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
                        Span::raw(" "),
                        Span::styled(tool_name, Style::default().fg(Color::Magenta)),
                        Span::raw(" → "),
                        Span::raw(first_line),
                    ]));
                }
            }
        }
        
        // Add loading indicator if active
        if let Some(indicator) = get_activity_indicator(state) {
            result.push(indicator);
        }
        
        result
    };

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .alignment(Alignment::Left)
        .scroll((state.scroll_offset as u16, 0));

    f.render_widget(paragraph, area);
}

/// Draw the notes panel overlay
pub fn draw_notes_panel(f: &mut Frame<'_>, area: Rect, state: &AppState) {
    let title_text = if let Some(ref sid) = state.notes_session_id {
        if sid == "main" {
            " Notes - Main Session ".to_string()
        } else {
            format!(" Notes - {} ", sid)
        }
    } else {
        " Notes ".to_string()
    };
    
    let block = Block::default()
        .title(title_text)
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Rgb(20, 20, 35)));

    let content = state.notes_content.as_deref()
        .unwrap_or("Loading notes...\n\nPress :note or :pin again to exit.");

    let lines: Vec<Line> = content.lines()
        .map(Line::from)
        .collect();

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .alignment(Alignment::Left)
        .scroll((state.scroll_offset as u16, 0));

    f.render_widget(paragraph, area);
}

/// Draw the instructions panel (session agent instructions editor)
pub fn draw_instructions_panel(f: &mut Frame<'_>, area: Rect, state: &AppState) {
    let title_text = if let Some(ref sid) = state.instructions_session_id {
        if sid == "main" {
            " Instructions - Main Session ".to_string()
        } else {
            format!(" Instructions - {} ", sid)
        }
    } else {
        " Instructions ".to_string()
    };
    
    let block = Block::default()
        .title(title_text)
        .title_style(Style::default().fg(Color::Cyan))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let instructions = state.current_instructions.as_deref().unwrap_or("");
    
    let intro = vec![
        Line::from(vec![
            Span::styled("Session Instructions", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("Set instructions that modify how the AI behaves in this session."),
        ]),
        Line::from(vec![
            Span::raw("Example: "),
            Span::styled("\"You are a code reviewer. Focus on security.\"", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Current instructions:", Style::default().fg(Color::Yellow)),
        ]),
    ];
    
    let current: Vec<Line> = if instructions.is_empty() {
        vec![Line::from(vec![
            Span::styled("(none - AI uses default behavior)", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
        ])]
    } else {
        instructions.lines()
            .map(|l| Line::from(vec![Span::raw(l.to_string())]))
            .collect()
    };
    
    let footer = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Type new instructions below and press Enter to save.", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("Clear all text and Enter to remove instructions.", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("Press Esc to cancel.", Style::default().fg(Color::DarkGray)),
        ]),
    ];
    
    let all_lines: Vec<Line> = intro.into_iter().chain(current).chain(footer).collect();

    let paragraph = Paragraph::new(Text::from(all_lines))
        .block(block)
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

/// Draw the input panel with enhanced features
pub fn draw_input_panel(f: &mut Frame<'_>, area: Rect, state: &AppState) {
    let title = if state.instructions_mode {
        " Instructions "
    } else if state.input_buffer.starts_with(':') {
        " Command "
    } else {
        " Input "
    };
    
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL);

    let display_text: String;
    let hint: Option<String>;
    
    if state.rename_mode {
        // Rename mode - prompt for new session name
        display_text = if state.input_buffer.is_empty() {
            " Enter new session name... ".to_string()
        } else {
            format!(" New name: {}", state.input_buffer)
        };
        hint = Some("Press Enter to rename, Esc to cancel".to_string());
    } else if state.current_session_id.is_none() {
        display_text = " (select a session first) ".to_string();
        hint = None;
    } else if state.input_buffer.is_empty() {
        display_text = " Type your message... ".to_string();
        hint = None;
    } else {
        display_text = if state.input_buffer.starts_with(':') {
            format!(":{}", &state.input_buffer[1..])
        } else {
            state.input_buffer.clone()
        };
        
        hint = if state.completion.active && state.completion.candidates.len() > 1 {
            let all_candidates: Vec<String> = state.completion.candidates.iter()
                .enumerate()
                .map(|(i, c)| if i == state.completion.index { 
                    format!("[{}]", c) 
                } else { 
                    c.clone() 
                })
                .collect();
            Some(all_candidates.join(" "))
        } else if state.is_navigating_history() {
            // Show input history navigation hint
            state.input_history_position().map(|pos| {
                format!("↑↓ {} (Enter to select, any key to cancel)", pos)
            })
        } else {
            None
        };
    };

    let paragraph = Paragraph::new(display_text.as_str())
        .block(block);

    f.render_widget(paragraph, area);

    // Show completion hint if active
    if let Some(hint_text) = hint {
        let hint_area = Rect {
            y: area.y + area.height,
            x: area.x + 2,
            width: area.width.saturating_sub(4),
            height: 1,
        };
        if hint_area.y < f.area().height.saturating_sub(1) {
            let hint_paragraph = Paragraph::new(hint_text.as_str())
                .alignment(Alignment::Left)
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(hint_paragraph, hint_area);
        }
    }
    
    // Show char count in bottom-right of input area
    let char_count = state.input_buffer.len();
    if char_count > 0 {
        let count_text = format!("{} chars", char_count);
        let count_area = Rect {
            x: area.x + area.width.saturating_sub(count_text.len() as u16 + 2),
            y: area.y + area.height - 1,
            width: count_text.len() as u16 + 2,
            height: 1,
        };
        let count_paragraph = Paragraph::new(count_text.as_str())
            .alignment(Alignment::Right)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(count_paragraph, count_area);
    }
}

/// Draw the help bar at the bottom
pub fn draw_help_bar(f: &mut Frame<'_>, area: Rect) {
    let help_text = " ↑↓ Navigate | Tab Complete | Enter Send | :q Quit | :h Help | :ren Rename | :rc Reconnect | 🤔 Thinking | 🔧 Tool active ";
    
    let paragraph = Paragraph::new(help_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));

    f.render_widget(paragraph, area);
}
