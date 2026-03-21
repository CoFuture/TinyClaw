//! TUI UI components

use ratatui::{
    layout::Alignment,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use crate::tui::state::AppState;

/// Draw the session list panel
pub fn draw_sessions_panel(f: &mut ratatui::Frame<'_>, area: ratatui::layout::Rect, state: &AppState) {
    let block = Block::default()
        .title(" Sessions ")
        .borders(Borders::ALL);

    let items: Vec<ListItem> = if state.sessions.is_empty() {
        vec![ListItem::new(" No sessions ")]
    } else {
        state.sessions.iter().map(|s| {
            let prefix = if Some(s.as_str()) == state.current_session_id.as_deref() {
                "● "
            } else {
                "  "
            };
            let display_name = if s == "main" { "Main" } else { s.as_str() };
            ListItem::new(format!("{}{}", prefix, display_name))
        }).collect()
    };

    let list = List::new(items)
        .block(block);

    f.render_widget(list, area);
}

/// Draw the message view panel
pub fn draw_messages_panel(f: &mut ratatui::Frame<'_>, area: ratatui::layout::Rect, state: &AppState) {
    let title_text = state.current_session_id.as_deref().map(|s| {
        if s == "main" { " Main Session " } else { s }
    }).unwrap_or(" Messages - No session ");
    
    let block = Block::default()
        .title(title_text)
        .borders(Borders::ALL);

    let messages = state.get_current_messages();
    
    let content: String = if messages.is_empty() {
        "No messages yet. Select a session and start chatting!".to_string()
    } else {
        messages.iter().skip(state.scroll_offset).map(|msg| {
            let role_str = match msg.role {
                crate::types::Role::User => "User",
                crate::types::Role::Assistant => "Assistant",
                crate::types::Role::System => "System",
                crate::types::Role::Tool => "Tool",
            };
            let content_str = msg.content.trim();
            if content_str.is_empty() {
                format!("[{}] (empty)", role_str)
            } else {
                format!("[{}] {}", role_str, content_str)
            }
        }).collect::<Vec<_>>().join("\n")
    };

    let paragraph = Paragraph::new(content.as_str())
        .block(block)
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

/// Draw the input panel
pub fn draw_input_panel(f: &mut ratatui::Frame<'_>, area: ratatui::layout::Rect, state: &AppState) {
    let block = Block::default()
        .title(" Input ")
        .borders(Borders::ALL);

    let text = if state.current_session_id.is_none() {
        " (select a session first) "
    } else if state.input_buffer.is_empty() {
        " Type your message... "
    } else {
        state.input_buffer.as_str()
    };

    let paragraph = Paragraph::new(text)
        .block(block);

    f.render_widget(paragraph, area);
}

/// Draw the help bar at the bottom
pub fn draw_help_bar(f: &mut ratatui::Frame<'_>, area: ratatui::layout::Rect) {
    let help_text = " ↑↓ Navigate | Tab Switch panel | Enter Send | :q Quit | :h Help ";
    
    let paragraph = Paragraph::new(help_text)
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}
