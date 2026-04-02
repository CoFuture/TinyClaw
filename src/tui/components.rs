//! TUI UI components

use crate::tui::markdown::{contains_markdown, is_markdown_heavy, parse_markdown};
use crate::tui::state::{AppState, AgentActivityType};
use ratatui::{
    layout::Alignment,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
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

                    // Use markdown rendering for assistant messages with markdown content
                    if is_markdown_heavy(content) {
                        // Full markdown rendering for rich content
                        let prefix = vec![
                            Span::raw("["),
                            Span::styled(ts.clone(), Style::default().fg(Color::DarkGray)),
                            Span::raw("] "),
                            Span::styled("Assistant", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                            Span::raw(": "),
                        ];
                        let md_lines = parse_markdown(content);
                        for (i, md_line) in md_lines.into_iter().enumerate() {
                            if i == 0 {
                                // First line: prefix + first content
                                let mut combined: Vec<Span> = prefix.clone();
                                combined.extend(md_line.spans);
                                result.push(Line::from(combined));
                            } else {
                                // Continuation lines: indent
                                let indent = Span::raw("                        ");
                                let mut continued: Vec<Span> = vec![indent];
                                continued.extend(md_line.spans);
                                result.push(Line::from(continued));
                            }
                        }
                    } else if contains_markdown(content) {
                        // Light markdown: inline formatting only
                        let content_lines: Vec<&str> = content.split('\n').collect();
                        let first_line = content_lines.first().unwrap_or(&"").to_string();
                        let styled_line = crate::tui::markdown::render_inline(&first_line);
                        result.push(Line::from(vec![
                            Span::raw("["),
                            Span::styled(ts.clone(), Style::default().fg(Color::DarkGray)),
                            Span::raw("] "),
                            Span::styled("Assistant", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                            Span::raw(": "),
                        ].into_iter().chain(styled_line.spans).collect::<Vec<_>>()));
                        for line in content_lines.iter().skip(1) {
                            let styled = crate::tui::markdown::render_inline(line);
                            result.push(Line::from(vec![
                                Span::raw("          "),
                            ].into_iter().chain(styled.spans).collect::<Vec<_>>()));
                        }
                    } else {
                        // Plain text - original behavior
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
        
        // Add streaming text indicator if streaming
        if state.is_streaming && !state.partial_text.is_empty() {
            // Show partial text with blinking cursor indicator
            let partial = &state.partial_text;
            // Truncate if too long for display
            let display_text = if partial.len() > 200 {
                format!("{}...", &partial[..200])
            } else {
                partial.clone()
            };
            result.push(Line::from(vec![
                Span::raw("["),
                Span::styled("streaming".to_string(), Style::default().fg(Color::DarkGray)),
                Span::raw("] "),
                Span::styled("Assistant", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(": "),
                Span::raw(display_text),
                // Blinking cursor character to indicate streaming
                Span::styled("▊", Style::default().fg(Color::Cyan).add_modifier(Modifier::SLOW_BLINK)),
            ]));
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

/// Draw the action confirmation panel (waiting for user to confirm/deny tool execution)
pub fn draw_confirm_panel(f: &mut Frame<'_>, area: Rect, state: &AppState) {
    let block = Block::default()
        .title(" ⚠️ Action Confirmation ")
        .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("Agent plans to execute the following tools:", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
    ];

    // List each tool
    for (i, tool) in state.confirm_tools.iter().enumerate() {
        let input_str = if let Some(obj) = tool.input.as_object() {
            serde_json::to_string_pretty(obj).unwrap_or_default()
        } else {
            tool.input.to_string()
        };
        // Truncate long input
        let input_preview = if input_str.len() > 100 {
            format!("{}...", &input_str[..100])
        } else {
            input_str
        };

        lines.push(Line::from(vec![
            Span::styled(format!("{}. ", i + 1), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(&tool.name, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(vec![
            Span::raw("   Input: "),
            Span::styled(input_preview, Style::default().fg(Color::DarkGray)),
        ]));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(vec![
        Span::styled("Type ", Style::default().fg(Color::White)),
        Span::styled(":confirm", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::styled(" or ", Style::default().fg(Color::White)),
        Span::styled(":y", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::styled(" to allow, ", Style::default().fg(Color::White)),
        Span::styled(":deny", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        Span::styled(" or ", Style::default().fg(Color::White)),
        Span::styled(":n", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        Span::raw(" to cancel."),
    ]));

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

/// Draw the summarizer panel (config, stats, and history viewer)
pub fn draw_summarizer_panel(f: &mut Frame<'_>, area: Rect, state: &AppState) {
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::{Line, Span, Text};
    use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
    
    let title_text = " 📊 Summarizer ".to_string();
    
    let block = Block::default()
        .title(title_text.as_str())
        .title_style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta))
        .style(Style::default().bg(Color::Rgb(20, 20, 35)));

    let mut lines: Vec<Line> = Vec::new();
    
    // Title
    lines.push(Line::from(vec![
        Span::styled("Context Summarizer", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(""));
    
    // Configuration section
    lines.push(Line::from(vec![
        Span::styled("Configuration:", Style::default().fg(Color::Cyan).add_modifier(Modifier::UNDERLINED)),
    ]));
    
    if let Some(ref config) = state.summarizer_config {
        // Try to parse and display nicely
        if let Ok(config_obj) = serde_json::from_str::<serde_json::Value>(config) {
            let enabled = config_obj.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
            let min_messages = config_obj.get("minMessages").and_then(|v| v.as_u64()).unwrap_or(0);
            let token_threshold = config_obj.get("tokenThreshold").and_then(|v| v.as_u64()).unwrap_or(0);
            
            lines.push(Line::from(vec![
                Span::raw("  • Enabled: "),
                Span::styled(if enabled { "Yes" } else { "No" }, 
                    Style::default().fg(if enabled { Color::Green } else { Color::Red }).add_modifier(Modifier::BOLD)),
            ]));
            lines.push(Line::from(vec![
                Span::raw("  • Min Messages: "),
                Span::styled(format!("{}", min_messages), Style::default().fg(Color::White)),
            ]));
            lines.push(Line::from(vec![
                Span::raw("  • Token Threshold: "),
                Span::styled(format!("{}", token_threshold), Style::default().fg(Color::White)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled(config, Style::default().fg(Color::DarkGray)),
            ]));
        }
    } else {
        lines.push(Line::from(vec![
            Span::styled("  (loading...)", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
        ]));
    }
    
    lines.push(Line::from(""));
    
    // Statistics section
    lines.push(Line::from(vec![
        Span::styled("Statistics:", Style::default().fg(Color::Cyan).add_modifier(Modifier::UNDERLINED)),
    ]));
    
    if let Some(ref stats) = state.summarizer_stats {
        if let Ok(stats_obj) = serde_json::from_str::<serde_json::Value>(stats) {
            let total_summaries = stats_obj.get("totalSummaries").and_then(|v| v.as_u64()).unwrap_or(0);
            let total_messages = stats_obj.get("totalMessagesSummarized").and_then(|v| v.as_u64()).unwrap_or(0);
            let avg_compression = stats_obj.get("avgCompressionRatio").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let session_count = stats_obj.get("sessionCount").and_then(|v| v.as_u64()).unwrap_or(0);
            
            lines.push(Line::from(vec![
                Span::raw("  • Total Summaries: "),
                Span::styled(format!("{}", total_summaries), Style::default().fg(Color::White)),
            ]));
            lines.push(Line::from(vec![
                Span::raw("  • Messages Summarized: "),
                Span::styled(format!("{}", total_messages), Style::default().fg(Color::White)),
            ]));
            lines.push(Line::from(vec![
                Span::raw("  • Avg Compression: "),
                Span::styled(format!("{:.1}%", avg_compression * 100.0), Style::default().fg(Color::Yellow)),
            ]));
            lines.push(Line::from(vec![
                Span::raw("  • Sessions: "),
                Span::styled(format!("{}", session_count), Style::default().fg(Color::White)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled(stats, Style::default().fg(Color::DarkGray)),
            ]));
        }
    } else {
        lines.push(Line::from(vec![
            Span::styled("  (loading...)", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
        ]));
    }
    
    lines.push(Line::from(""));
    
    // History section (last 5 entries)
    lines.push(Line::from(vec![
        Span::styled("Recent History:", Style::default().fg(Color::Cyan).add_modifier(Modifier::UNDERLINED)),
    ]));
    
    // Try to parse and display history - extract owned data first to avoid borrow issues
    let history_displayed = if let Some(ref history) = state.summarizer_history {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(history) {
            // Extract entries and convert to owned data
            let entries = val.get("entries").and_then(|v| v.as_array());
            let owned_entries: Vec<(String, u64, f64, String)> = entries
                .map(|arr| {
                    arr.iter().map(|entry| {
                        let session = entry.get("sessionId").and_then(|v| v.as_str()).unwrap_or("?").to_string();
                        let msgs = entry.get("messagesSummarized").and_then(|v| v.as_u64()).unwrap_or(0);
                        let ratio = entry.get("compressionRatio").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let created = entry.get("createdAt").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        (session, msgs, ratio, created)
                    }).collect()
                })
                .unwrap_or_default();
            
            if owned_entries.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("  (no history)", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
                ]));
            } else {
                // Show last 5 entries
                let start = if owned_entries.len() > 5 { owned_entries.len() - 5 } else { 0 };
                for (session, msgs, ratio, created) in owned_entries.into_iter().skip(start) {
                    let ratio_color = if ratio > 0.5 {
                        Color::Green
                    } else if ratio > 0.2 {
                        Color::Yellow
                    } else {
                        Color::Red
                    };
                    
                    lines.push(Line::from(vec![
                        Span::raw("  • "),
                        Span::styled(session, Style::default().fg(Color::White)),
                        Span::raw(": "),
                        Span::styled(format!("{} msgs", msgs), Style::default().fg(Color::DarkGray)),
                        Span::raw(" → "),
                        Span::styled(format!("{:.0}%", ratio * 100.0), Style::default().fg(ratio_color)),
                        Span::raw(" "),
                        Span::styled(created, Style::default().fg(Color::DarkGray)),
                    ]));
                }
            }
            true
        } else {
            false
        }
    } else {
        false
    };
    
    if !history_displayed {
        if state.summarizer_history.is_some() {
            lines.push(Line::from(vec![
                Span::styled("  (no history)", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("  (loading...)", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
            ]));
        }
    }
    
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Press Esc to close", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
    ]));

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

/// Draw the summarizer config editing panel
pub fn draw_sumcfg_panel(f: &mut Frame<'_>, area: Rect, state: &AppState) {
    let block = Block::default()
        .title(" ⚙ Summarizer Config ")
        .title_style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta))
        .style(Style::default().bg(Color::Rgb(20, 20, 35)));

    let mut lines: Vec<Line> = Vec::new();

    // Title
    lines.push(Line::from(vec![
        Span::styled("Edit Summarizer Configuration", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(""));

    // Current config display
    lines.push(Line::from(vec![
        Span::styled("Current Settings:", Style::default().fg(Color::Cyan).add_modifier(Modifier::UNDERLINED)),
    ]));

    if let Some(ref config) = state.summarizer_config {
        if let Ok(config_obj) = serde_json::from_str::<serde_json::Value>(config) {
            let enabled = config_obj.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
            let min_messages = config_obj.get("minMessages").and_then(|v| v.as_u64()).unwrap_or(0);
            let token_threshold = config_obj.get("tokenThreshold").and_then(|v| v.as_u64()).unwrap_or(0);

            lines.push(Line::from(vec![
                Span::raw("  • enabled: "),
                Span::styled(if enabled { "true" } else { "false" },
                    Style::default().fg(if enabled { Color::Green } else { Color::Red }).add_modifier(Modifier::BOLD)),
            ]));
            lines.push(Line::from(vec![
                Span::raw("  • minMessages: "),
                Span::styled(format!("{}", min_messages), Style::default().fg(Color::White)),
            ]));
            lines.push(Line::from(vec![
                Span::raw("  • tokenThreshold: "),
                Span::styled(format!("{}", token_threshold), Style::default().fg(Color::White)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled(config, Style::default().fg(Color::DarkGray)),
            ]));
        }
    } else {
        lines.push(Line::from(vec![
            Span::styled("  (loading...)", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
        ]));
    }

    lines.push(Line::from(""));

    // Format hint
    lines.push(Line::from(vec![
        Span::styled("Input Format:", Style::default().fg(Color::Cyan).add_modifier(Modifier::UNDERLINED)),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("minMessages=N", Style::default().fg(Color::Yellow)),
        Span::raw(", "),
        Span::styled("tokenThreshold=N", Style::default().fg(Color::Yellow)),
        Span::raw(", "),
        Span::styled("enabled=true|false", Style::default().fg(Color::Yellow)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  All fields optional — only specified fields are updated.", Style::default().fg(Color::DarkGray)),
    ]));

    lines.push(Line::from(""));

    // Examples
    lines.push(Line::from(vec![
        Span::styled("Examples:", Style::default().fg(Color::Cyan).add_modifier(Modifier::UNDERLINED)),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("minMessages=20", Style::default().fg(Color::DarkGray)),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("enabled=false", Style::default().fg(Color::DarkGray)),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("minMessages=15,enabled=true", Style::default().fg(Color::DarkGray)),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Type config below and press Enter to save.", Style::default().fg(Color::DarkGray)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Press Esc to cancel.", Style::default().fg(Color::DarkGray)),
    ]));

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

/// Draw the input panel with enhanced features
pub fn draw_input_panel(f: &mut Frame<'_>, area: Rect, state: &AppState) {
    let title = if state.confirm_mode {
        " Confirm "
    } else if state.instructions_mode {
        " Instructions "
    } else if state.quality_mode {
        " Quality "
    } else if state.eval_mode {
        " Evaluations "
    } else if state.summarizer_mode {
        " Summarizer "
    } else if state.sumcfg_mode {
        " sumcfg "
    } else if state.safety_mode {
        " Safety "
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
    } else if state.sumcfg_mode {
        // Summarizer config editing mode
        display_text = if state.input_buffer.is_empty() {
            " minMessages=N,tokenThreshold=N,enabled=true|false ".to_string()
        } else {
            state.input_buffer.clone()
        };
        hint = Some("Press Enter to save, Esc to cancel".to_string());
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
pub fn draw_help_bar(f: &mut Frame<'_>, area: Rect, state: &AppState) {
    // Build help text with token usage
    let token_usage = state.formatted_token_usage();
    
    let help_text = if state.confirm_mode {
        format!(" ⚠️ :confirm/:y Allow | :deny/:n Cancel | Esc Cancel | 📊 {} ", token_usage)
    } else {
        format!(" ↑↓ Navigate | Tab Complete | Enter Send | :q Quit | :h Help | 📊 {} ", token_usage)
    };
    
    let paragraph = Paragraph::new(help_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));

    f.render_widget(paragraph, area);
}

/// Draw the session quality panel overlay
pub fn draw_quality_panel(f: &mut Frame<'_>, area: Rect, state: &AppState) {
    let title_text = " 📊 Session Quality Analysis ";
    
    let block = Block::default()
        .title(title_text)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta))
        .style(Style::default().bg(Color::Rgb(25, 15, 35)));

    let mut lines: Vec<Line> = Vec::new();
    
    if let Some(ref q) = state.quality_data {
        // Rating display
        let stars = "★".repeat(q.rating as usize) + &"☆".repeat(5 - q.rating as usize);
        lines.push(Line::from(vec![
            Span::styled("Session: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&q.session_id, Style::default().fg(Color::Cyan)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Rating: ", Style::default().fg(Color::DarkGray)),
            Span::styled(stars, Style::default().fg(Color::Yellow)),
            Span::styled(format!(" ({:.0}%)", q.quality_score * 100.0), Style::default().fg(Color::Yellow)),
        ]));
        lines.push(Line::from(""));
        
        // Metrics
        lines.push(Line::from(vec![
            Span::styled("Metrics:", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  Turn Count:        "),
            Span::styled(format!("{}", q.turn_count), Style::default().fg(Color::Cyan)),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  Task Completion:   "),
            Span::styled(format!("{:.0}%", q.task_completion_rate * 100.0), 
                if q.task_completion_rate >= 0.8 { Color::Green } 
                else if q.task_completion_rate >= 0.5 { Color::Yellow } 
                else { Color::Red }),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  Tool Success:      "),
            Span::styled(format!("{:.0}%", q.tool_success_rate * 100.0),
                if q.tool_success_rate >= 0.8 { Color::Green } 
                else if q.tool_success_rate >= 0.5 { Color::Yellow } 
                else { Color::Red }),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  Issues Detected:   "),
            Span::styled(format!("{}", q.issue_count),
                if q.issue_count == 0 { Color::Green } 
                else if q.issue_count <= 2 { Color::Yellow } 
                else { Color::Red }),
        ]));
        
        // Suggestions
        if !q.suggestions.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Suggestions:", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]));
            for s in q.suggestions.iter().take(5) {
                lines.push(Line::from(vec![
                    Span::styled("  • ", Style::default().fg(Color::DarkGray)),
                    Span::raw(s.clone()),
                ]));
            }
        }
    } else {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("No quality data available yet.", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from("Quality data will appear after agent turns."));
        lines.push(Line::from(""));
        lines.push(Line::from("Press :quality or :qly again to exit."));
    }
    
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Press Esc or :quality to exit", Style::default().fg(Color::DarkGray)),
    ]));

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .alignment(Alignment::Left)
        .scroll((state.scroll_offset as u16, 0));

    f.render_widget(paragraph, area);
}

/// Draw the self-evaluation panel overlay
pub fn draw_eval_panel(f: &mut Frame<'_>, area: Rect, state: &AppState) {
    let title_text = " 📈 Agent Self-Evaluations ";
    
    let block = Block::default()
        .title(title_text)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Rgb(15, 25, 35)));

    let mut lines: Vec<Line> = Vec::new();
    
    if let Some(ref evals) = state.eval_data {
        if evals.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("No evaluations available yet.", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from("Evaluations will appear after agent turns."));
        } else {
            lines.push(Line::from(vec![
                Span::styled(format!("{} recent evaluations", evals.len()), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]));
            lines.push(Line::from(""));
            
            for (idx, e) in evals.iter().take(10).enumerate() {
                // Overall score
                let score_color = if e.overall_score >= 0.7 { Color::Green } 
                    else if e.overall_score >= 0.4 { Color::Yellow } 
                    else { Color::Red };
                
                lines.push(Line::from(vec![
                    Span::styled(format!("#{} ", idx + 1), Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("Score: {:.0}%", e.overall_score * 100.0), Style::default().fg(score_color).add_modifier(Modifier::BOLD)),
                    Span::styled(format!(" (turn: {})", &e.turn_id[..8]), Style::default().fg(Color::DarkGray)),
                ]));
                
                // Dimension scores
                for (dim, score) in e.dimension_scores.iter().take(4) {
                    let dim_score_color = if *score >= 0.7 { Color::Green } 
                        else if *score >= 0.4 { Color::Yellow } 
                        else { Color::Red };
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(dim.clone(), Style::default().fg(Color::DarkGray)),
                        Span::raw(": "),
                        Span::styled(format!("{:.0}%", score * 100.0), Style::default().fg(dim_score_color)),
                    ]));
                }
                
                // Strengths (show first)
                if let Some(s) = e.strengths.first() {
                    lines.push(Line::from(vec![
                        Span::styled("  ✓ ", Style::default().fg(Color::Green)),
                        Span::styled(s.clone(), Style::default().fg(Color::DarkGray)),
                    ]));
                }
                
                // Weaknesses (show first)
                if let Some(w) = e.weaknesses.first() {
                    lines.push(Line::from(vec![
                        Span::styled("  ✗ ", Style::default().fg(Color::Red)),
                        Span::styled(w.clone(), Style::default().fg(Color::DarkGray)),
                    ]));
                }
                
                lines.push(Line::from(""));
            }
        }
    } else {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("No evaluation data available yet.", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from("Evaluations will appear after agent turns."));
    }
    
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Press Esc or :eval to exit", Style::default().fg(Color::DarkGray)),
    ]));

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .alignment(Alignment::Left)
        .scroll((state.scroll_offset as u16, 0));

    f.render_widget(paragraph, area);
}

/// Draw skill recommendations panel
pub fn draw_recommendations_panel(f: &mut Frame<'_>, area: Rect, state: &AppState) {
    let title_text = " 💡 Skill Recommendations ";
    
    let block = Block::default()
        .title(title_text)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta))
        .style(Style::default().bg(Color::Rgb(15, 25, 35)));

    let mut lines: Vec<Line> = Vec::new();
    
    if let Some(ref recommendations) = state.recommendations_data {
        if recommendations.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("No skill recommendations available yet.", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from("Recommendations appear based on conversation context."));
        } else {
            lines.push(Line::from(vec![
                Span::styled(format!("{} recommended skills", recommendations.len()), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]));
            lines.push(Line::from(""));
            
            for (idx, rec) in recommendations.iter().enumerate() {
                // Confidence color
                let conf_color = if rec.confidence >= 0.7 { Color::Green }
                    else if rec.confidence >= 0.4 { Color::Yellow }
                    else { Color::DarkGray };
                
                // Already enabled badge
                let enabled_badge = if rec.already_enabled {
                    Span::styled(" [enabled]", Style::default().fg(Color::Green))
                } else {
                    Span::raw("")
                };
                
                lines.push(Line::from(vec![
                    Span::styled(format!("#{} ", idx + 1), Style::default().fg(Color::DarkGray)),
                    Span::styled(rec.skill_name.clone(), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::raw(" "),
                    Span::styled(format!("{:.0}%", rec.confidence * 100.0), Style::default().fg(conf_color)),
                    enabled_badge,
                ]));
                
                // Description
                if !rec.description.is_empty() && rec.description != rec.skill_name {
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(rec.description.clone(), Style::default().fg(Color::DarkGray)),
                    ]));
                }
                
                // Reasons (show first 2)
                for (i, reason) in rec.reasons.iter().take(2).enumerate() {
                    let bullet = if i == 0 { "→" } else { " " };
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(bullet, Style::default().fg(Color::Magenta)),
                        Span::styled(" ", Style::default()),
                        Span::styled(reason.clone(), Style::default().fg(Color::DarkGray)),
                    ]));
                }
                
                // Triggered keywords (show first 3)
                if !rec.triggered_keywords.is_empty() {
                    let keywords: Vec<String> = rec.triggered_keywords.iter().take(3).cloned().collect();
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled("keywords: ", Style::default().fg(Color::DarkGray)),
                        Span::styled(keywords.join(", "), Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
                    ]));
                }
                
                lines.push(Line::from(""));
            }
            
            lines.push(Line::from(vec![
                Span::styled("Use WebUI or API to enable recommended skills", Style::default().fg(Color::DarkGray)),
            ]));
        }
    } else {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Loading recommendations...", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from("Recommendations will appear based on conversation context."));
    }
    
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Press Esc or :rec to exit", Style::default().fg(Color::DarkGray)),
    ]));

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .alignment(Alignment::Left)
        .scroll((state.scroll_offset as u16, 0));

    f.render_widget(paragraph, area);
}

/// Draw the execution safety panel
pub fn draw_safety_panel(f: &mut Frame<'_>, area: Rect, state: &AppState) {
    let border_color = if state.safety_halted { Color::Red } else { Color::Yellow };
    let title_text = if state.safety_halted { " 🛑 Execution Safety - HALTED " } else { " ⚠️ Execution Safety " };
    
    let block = Block::default()
        .title(title_text)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(Color::Rgb(15, 25, 35)));

    let mut lines: Vec<Line> = Vec::new();
    
    // Show last warning/halt message if available
    if let Some(ref warning) = state.last_safety_warning {
        let warning_color = if state.safety_halted { Color::Red } else { Color::Yellow };
        lines.push(Line::from(vec![
            Span::styled(warning.clone(), Style::default().fg(warning_color).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(""));
    }
    
    // Safety stats info
    lines.push(Line::from(vec![
        Span::styled("Execution Safety System", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(""));
    
    // Circuit breaker status
    lines.push(Line::from(vec![
        Span::styled("AI Circuit Breaker: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            match state.circuit_breaker_state.as_str() {
                "closed" => "🟢 Closed (healthy)",
                "open" => "🔴 Open (failing)",
                "half_open" => "🟡 Half-Open (testing)",
                _ => "⚪ Unknown",
            },
            Style::default().fg(match state.circuit_breaker_state.as_str() {
                "closed" => Color::Green,
                "open" => Color::Red,
                "half_open" => Color::Yellow,
                _ => Color::DarkGray,
            }),
        ),
    ]));
    lines.push(Line::from(""));
    
    // Safety halted status
    if state.safety_halted {
        lines.push(Line::from(vec![
            Span::styled("⚠️ ", Style::default().fg(Color::Red)),
            Span::styled("Agent execution has been halted due to excessive tool calls.", Style::default().fg(Color::Red)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("The agent will wait for user confirmation before continuing.", Style::default().fg(Color::DarkGray)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Use :confirm or :y to allow, :deny or :n to stop.", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled("✓ ", Style::default().fg(Color::Green)),
            Span::styled("Agent execution is within safe limits.", Style::default().fg(Color::DarkGray)),
        ]));
    }
    
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("About Execution Safety:", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Monitors consecutive tool-call turns to prevent runaway loops.", Style::default().fg(Color::DarkGray)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Warning at 75% of limit, halt at 100%.", Style::default().fg(Color::DarkGray)),
    ]));
    
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Press Esc or :safety to exit", Style::default().fg(Color::DarkGray)),
    ]));

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .alignment(Alignment::Left)
        .scroll((state.scroll_offset as u16, 0));

    f.render_widget(paragraph, area);
}

/// Draw the performance insights panel
pub fn draw_perf_panel(f: &mut Frame<'_>, area: Rect, state: &AppState) {
    let title_text = " 📈 Performance Insights ";
    
    let block = Block::default()
        .title(title_text)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta))
        .style(Style::default().bg(Color::Rgb(20, 15, 30)));

    let mut lines: Vec<Line> = Vec::new();
    
    if let Some(ref perf) = state.perf_data {
        // Summary stats
        lines.push(Line::from(vec![
            Span::styled("Turns Analyzed: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", perf.turns_analyzed), Style::default().fg(Color::Cyan)),
            Span::raw("  |  "),
            Span::styled("Avg Tools/Turn: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{:.1}", perf.avg_tools_per_turn), Style::default().fg(Color::Cyan)),
        ]));
        lines.push(Line::from(""));
        
        // Trend
        let trend_arrow = if perf.trend_direction == "improving" { "↑" }
            else if perf.trend_direction == "declining" { "↓" }
            else { "→" };
        let trend_color = if perf.trend_direction == "improving" { Color::Green }
            else if perf.trend_direction == "declining" { Color::Red }
            else { Color::Yellow };
        lines.push(Line::from(vec![
            Span::styled("Quality Trend: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{} {}", trend_arrow, perf.trend_direction), Style::default().fg(trend_color)),
            Span::raw("  "),
            Span::styled(format!("({:.1}% change)", perf.trend_magnitude), Style::default().fg(Color::DarkGray)),
        ]));
        lines.push(Line::from(""));
        
        // Tool efficiency
        lines.push(Line::from(vec![
            Span::styled("Tool Efficiency:", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]));
        
        if let Some(ref most) = perf.most_efficient_tool {
            lines.push(Line::from(vec![
                Span::styled("  🟢 Most Efficient: ", Style::default().fg(Color::DarkGray)),
                Span::styled(most, Style::default().fg(Color::Green)),
            ]));
        }
        if let Some(ref least) = perf.least_efficient_tool {
            lines.push(Line::from(vec![
                Span::styled("  🔴 Least Efficient: ", Style::default().fg(Color::DarkGray)),
                Span::styled(least, Style::default().fg(Color::Red)),
            ]));
        }
        if !perf.problematic_tools.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("  ⚠️ Problematic: ", Style::default().fg(Color::DarkGray)),
                Span::styled(perf.problematic_tools.join(", "), Style::default().fg(Color::Yellow)),
            ]));
        }
        
        // Insights
        if !perf.insights.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Insights:", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]));
            
            for insight in perf.insights.iter().take(6) {
                let severity_color = match insight.severity.as_str() {
                    "high" => Color::Red,
                    "medium" => Color::Yellow,
                    "low" => Color::Green,
                    _ => Color::DarkGray,
                };
                let severity_icon = match insight.severity.as_str() {
                    "high" => "🔴",
                    "medium" => "🟡",
                    "low" => "🟢",
                    _ => "⚪",
                };
                lines.push(Line::from(vec![
                    Span::styled(format!("  {} ", severity_icon), Style::default().fg(severity_color)),
                    Span::styled(format!("[{}] ", insight.category), Style::default().fg(Color::DarkGray)),
                    Span::styled(&insight.title, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(&insight.description, Style::default().fg(Color::DarkGray)),
                ]));
                if !insight.suggestions.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("    💡 ", Style::default().fg(Color::Cyan)),
                        Span::styled(&insight.suggestions[0], Style::default().fg(Color::Cyan)),
                    ]));
                }
            }
        }
    } else {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("No performance data available yet.", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from("Performance data will appear after agent turns."));
        lines.push(Line::from(""));
        lines.push(Line::from("Press :perf or :performance again to exit."));
    }
    
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Press Esc or :perf to exit", Style::default().fg(Color::DarkGray)),
    ]));

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .alignment(Alignment::Left)
        .scroll((state.scroll_offset as u16, 0));

    f.render_widget(paragraph, area);
}

/// Draw context advisor panel
pub fn draw_advisor_panel(f: &mut Frame<'_>, area: Rect, state: &AppState) {
    use ratatui::{widgets::{Block, Borders, Paragraph}, text::{Text, Line, Span}, style::{Color, Modifier, Style}, layout::Alignment};
    
    let title_text = " 💡 Context Advisor ";
    
    let block = Block::default()
        .title(title_text)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .style(Style::default().bg(Color::Rgb(25, 20, 15)));

    let mut lines: Vec<Line> = Vec::new();
    
    if let Some(ref advisor) = state.advisor_data {
        // Session and summary
        lines.push(Line::from(vec![
            Span::styled("Session: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&advisor.session_id, Style::default().fg(Color::Cyan)),
            Span::raw("  |  "),
            Span::styled("Advice: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", advisor.advice_count), Style::default().fg(if advisor.advice_count > 0 { Color::Yellow } else { Color::Green })),
        ]));
        
        // Suggest new session indicator
        if advisor.should_suggest_new_session {
            lines.push(Line::from(vec![
                Span::styled("⚠️ ", Style::default().fg(Color::Red)),
                Span::styled("建议开启新会话以改善上下文管理", Style::default().fg(Color::Red)),
            ]));
        }
        lines.push(Line::from(""));
        
        // Advisor stats
        lines.push(Line::from(vec![
            Span::styled("Statistics:", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Turns: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", advisor.turn_count), Style::default().fg(Color::Cyan)),
            Span::raw("  |  "),
            Span::styled("Tokens: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", advisor.total_tokens_processed), Style::default().fg(Color::Cyan)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Compressions: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", advisor.compression_count), Style::default().fg(Color::Yellow)),
            Span::raw("  |  "),
            Span::styled("Utilization: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{:.1}%", advisor.current_utilization), Style::default().fg(if advisor.current_utilization >= 80.0 { Color::Red } else { Color::Green })),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Active Patterns: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", advisor.active_patterns), Style::default().fg(if advisor.active_patterns > 0 { Color::Yellow } else { Color::Green })),
        ]));
        
        // Advice items
        if !advisor.advice.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Advice:", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]));
            
            for item in advisor.advice.iter().take(8) {
                let severity_icon = match item.severity {
                    3 => "🔴",
                    2 => "🟡",
                    _ => "🟢",
                };
                let severity_color = match item.severity {
                    3 => Color::Red,
                    2 => Color::Yellow,
                    _ => Color::Green,
                };
                lines.push(Line::from(vec![
                    Span::styled(format!("  {} ", severity_icon), Style::default().fg(severity_color)),
                    Span::styled(format!("[{}] ", item.category), Style::default().fg(Color::DarkGray)),
                    Span::styled(&item.title, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    Span::styled(if item.is_urgent { " ⚡URGENT" } else { "" }, Style::default().fg(Color::Red)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(&item.explanation, Style::default().fg(Color::DarkGray)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("    💡 ", Style::default().fg(Color::Cyan)),
                    Span::styled(&item.suggestion, Style::default().fg(Color::Cyan)),
                ]));
                lines.push(Line::from(""));
            }
        }
    } else {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("No context advisor data available.", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from("Context advisor analyzes context patterns and generates"));
        lines.push(Line::from("actionable advice to optimize context management."));
        lines.push(Line::from(""));
        lines.push(Line::from("Press :advisor or :advice again to exit."));
    }
    
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Press Esc or :advisor to exit", Style::default().fg(Color::DarkGray)),
    ]));

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .alignment(Alignment::Left)
        .scroll((state.scroll_offset as u16, 0));

    f.render_widget(paragraph, area);
}

/// Draw context health panel
pub fn draw_context_health_panel(f: &mut Frame<'_>, area: Rect, state: &AppState) {
    use ratatui::{widgets::{Block, Borders, Paragraph}, text::{Text, Line, Span}, style::{Color, Modifier, Style}, layout::Alignment};
    
    let title_text = " 🧠 Context Health ";
    
    let block = Block::default()
        .title(title_text)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Rgb(20, 15, 30)));

    let mut lines: Vec<Line> = Vec::new();
    
    if let Some(ref health) = state.context_health_data {
        let level_emoji = match health.health_level.as_str() {
            "Healthy" => "🟢",
            "Warning" => "🟡",
            "Critical" => "🟠",
            "Emergency" => "🔴",
            _ => "⚪",
        };
        let level_color = match health.health_level.as_str() {
            "Healthy" => Color::Green,
            "Warning" => Color::Yellow,
            "Critical" => Color::LightYellow,
            "Emergency" => Color::Red,
            _ => Color::Gray,
        };
        lines.push(Line::from(vec![
            Span::styled("Health: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{} {}", level_emoji, health.health_level), Style::default().fg(level_color)),
            Span::raw("  |  "),
            Span::styled("Score: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}/100", health.health_score), Style::default().fg(Color::Cyan)),
        ]));
        lines.push(Line::from(""));
        
        let util_color = if health.utilization_pct >= 80.0 { Color::Red }
            else if health.utilization_pct >= 60.0 { Color::Yellow }
            else { Color::Green };
        lines.push(Line::from(vec![
            Span::styled("Context Usage: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{:.1}%", health.utilization_pct), Style::default().fg(util_color)),
            Span::raw("  "),
            Span::styled(format!("({} / {} tokens)", health.total_tokens, health.max_tokens), Style::default().fg(Color::DarkGray)),
        ]));
        lines.push(Line::from(""));
        
        lines.push(Line::from(vec![
            Span::styled("Statistics:", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Total Turns: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", health.total_turns), Style::default().fg(Color::Cyan)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Truncations: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", health.truncation_count), Style::default().fg(Color::Yellow)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Summarizations: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", health.summarization_count), Style::default().fg(Color::Yellow)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Peak Utilization: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{:.1}%", health.peak_utilization_pct), Style::default().fg(Color::Cyan)),
        ]));
        
        if health.recommendations_count > 0 {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("⚠️ ", Style::default().fg(Color::Yellow)),
                Span::styled(format!("{} recommendations", health.recommendations_count), Style::default().fg(Color::Yellow)),
            ]));
        }
    } else {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("No context health data yet.", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from("Press :context or :ctx to view."));
    }
    
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Press Esc to exit", Style::default().fg(Color::DarkGray)),
    ]));

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .alignment(Alignment::Left)
        .scroll((state.scroll_offset as u16, 0));

    f.render_widget(paragraph, area);
}

/// Draw the scheduled tasks panel
pub fn draw_scheduled_tasks_panel(f: &mut Frame<'_>, area: Rect, state: &AppState) {
    let title_text = " ⏰ Scheduled Tasks ";
    
    let block = Block::default()
        .title(title_text)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Rgb(20, 15, 30)));

    let mut lines: Vec<Line> = Vec::new();
    
    if let Some(ref tasks) = state.scheduled_tasks_data {
        if tasks.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("No scheduled tasks yet.", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from("Use the admin panel to create scheduled tasks."));
        } else {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled(format!("{} task(s)", tasks.len()), Style::default().fg(Color::Cyan)),
                Span::raw(" total"),
            ]));
            lines.push(Line::from(""));
            
            for task in tasks.iter() {
                // Task name and status
                let status_icon = if task.paused { "⏸" } else if task.enabled { "✅" } else { "❌" };
                let status_color = if task.paused { Color::Yellow } else if task.enabled { Color::Green } else { Color::Red };
                
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(status_icon, Style::default().fg(status_color)),
                    Span::raw(" "),
                    Span::styled(&task.name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                ]));
                
                // Schedule info
                lines.push(Line::from(vec![
                    Span::styled("     Schedule: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&task.schedule_display, Style::default().fg(Color::Cyan)),
                ]));
                
                // Task description
                if !task.task_description.is_empty() {
                    let desc_preview = if task.task_description.len() > 50 {
                        format!("{}...", &task.task_description[..50])
                    } else {
                        task.task_description.clone()
                    };
                    lines.push(Line::from(vec![
                        Span::styled("     Task: ", Style::default().fg(Color::DarkGray)),
                        Span::styled(desc_preview, Style::default().fg(Color::LightGreen)),
                    ]));
                }
                
                // Next run
                if let Some(ref next) = task.next_run_at {
                    lines.push(Line::from(vec![
                        Span::styled("     Next: ", Style::default().fg(Color::DarkGray)),
                        Span::styled(next, Style::default().fg(Color::Yellow)),
                    ]));
                }
                
                // Stats
                lines.push(Line::from(vec![
                    Span::styled("     Runs: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{}", task.run_count), Style::default().fg(Color::White)),
                ]));
                if let Some(ref last) = task.last_run_at {
                    lines.push(Line::from(vec![
                        Span::styled("     Last: ", Style::default().fg(Color::DarkGray)),
                        Span::styled(last, Style::default().fg(Color::DarkGray)),
                    ]));
                }
                
                lines.push(Line::from(""));
            }
        }
    } else {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Loading scheduled tasks...", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
        ]));
    }
    
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Press Esc to exit", Style::default().fg(Color::DarkGray)),
    ]));

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .alignment(Alignment::Left)
        .scroll((state.scroll_offset as u16, 0));

    f.render_widget(paragraph, area);
}

/// Draw the accomplishments panel
pub fn draw_accomplishments_panel(f: &mut Frame<'_>, area: Rect, state: &AppState) {
    let title_text = " 🏆 Session Accomplishments ";

    let block = Block::default()
        .title(title_text)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .style(Style::default().bg(Color::Rgb(20, 15, 30)));

    let mut lines: Vec<Line> = Vec::new();

    if let Some(ref text) = state.accomplishments_data {
        if text.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("No accomplishments recorded yet.", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from("Accomplishments are automatically tracked as you work."));
        } else {
            // Display the text summary line by line
            for line in text.lines() {
                if line.is_empty() {
                    lines.push(Line::from(""));
                } else if line.starts_with("📋") || (!line.starts_with("├─") && (line.contains("Files") || line.contains("Tasks") || line.contains("Problems") || line.contains("Tools"))) {
                    // Header lines
                    lines.push(Line::from(vec![
                        Span::styled(line, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    ]));
                } else {
                    // Regular content
                    lines.push(Line::from(vec![
                        Span::styled(line, Style::default().fg(Color::Cyan)),
                    ]));
                }
            }
        }
    } else {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Loading accomplishments...", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Press Esc or :acc again to exit", Style::default().fg(Color::DarkGray)),
    ]));

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .alignment(Alignment::Left)
        .scroll((state.scroll_offset as u16, 0));

    f.render_widget(paragraph, area);
}

/// Draw the status/dashboard panel - a compact overview of agent health
pub fn draw_status_panel(f: &mut Frame<'_>, area: Rect, state: &AppState) {
    let title_text = " 📊 Agent Status ";

    let block = Block::default()
        .title(title_text)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Rgb(20, 15, 30)));

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    // Circuit Breaker Status
    let cb_state = &state.circuit_breaker_state;
    let cb_color = match cb_state.as_str() {
        "open" => Color::Red,
        "half_open" => Color::Yellow,
        _ => Color::Green,
    };
    let cb_indicator = match cb_state.as_str() {
        "open" => "🔴 OPEN",
        "half_open" => "🟡 HALF-OPEN",
        _ => "🟢 CLOSED",
    };
    lines.push(Line::from(vec![
        Span::styled("Circuit Breaker: ", Style::default().fg(Color::DarkGray)),
        Span::styled(cb_indicator, Style::default().fg(cb_color).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(""));

    // Context Health
    let health_level = &state.context_health_level;
    let health_color = match health_level.as_str() {
        "Emergency" => Color::Red,
        "Critical" => Color::Red,
        "Warning" => Color::Yellow,
        _ => Color::Green,
    };
    let health_indicator = match health_level.as_str() {
        "Emergency" => "🛑 EMERGENCY",
        "Critical" => "🔴 CRITICAL",
        "Warning" => "🟡 WARNING",
        "Healthy" => "🟢 HEALTHY",
        _ => "⚪ UNKNOWN",
    };
    lines.push(Line::from(vec![
        Span::styled("Context Health: ", Style::default().fg(Color::DarkGray)),
        Span::styled(health_indicator, Style::default().fg(health_color).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(""));

    // Context Utilization
    if let Some(util_pct) = state.context_utilization_pct {
        let util_color = if util_pct >= 90.0 {
            Color::Red
        } else if util_pct >= 75.0 {
            Color::Yellow
        } else {
            Color::Green
        };
        lines.push(Line::from(vec![
            Span::styled("Context Usage: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{:.1}%", util_pct), Style::default().fg(util_color).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(""));
    }

    // Safety Status
    if state.safety_halted {
        lines.push(Line::from(vec![
            Span::styled("⚠️ Safety Stopped", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(""));
    } else if let Some(ref warning) = state.last_safety_warning {
        lines.push(Line::from(vec![
            Span::styled("⚠️ Safety Warning: ", Style::default().fg(Color::Yellow)),
            Span::styled(warning, Style::default().fg(Color::DarkGray)),
        ]));
        lines.push(Line::from(""));
    }

    // Connection Status
    let conn_indicator = if state.connected { "🟢 Connected" } else { "🔴 Disconnected" };
    let conn_color = if state.connected { Color::Green } else { Color::Red };
    lines.push(Line::from(vec![
        Span::styled("Connection: ", Style::default().fg(Color::DarkGray)),
        Span::styled(conn_indicator, Style::default().fg(conn_color)),
    ]));
    lines.push(Line::from(""));

    // Session Info
    if let Some(ref session_id) = state.current_session_id {
        lines.push(Line::from(vec![
            Span::styled("Session: ", Style::default().fg(Color::DarkGray)),
            Span::styled(session_id.as_str(), Style::default().fg(Color::Cyan)),
        ]));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(vec![
        Span::styled("Press Esc or :status again to exit", Style::default().fg(Color::DarkGray)),
    ]));

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .alignment(Alignment::Left)
        .scroll((state.scroll_offset as u16, 0));

    f.render_widget(paragraph, area);
}

/// Draw the turn summaries panel
pub fn draw_turn_summary_panel(f: &mut Frame<'_>, area: Rect, state: &AppState) {
    let title_text = " 📋 Turn Summaries ";

    let block = Block::default()
        .title(title_text)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Rgb(20, 15, 30)));

    let mut lines: Vec<Line> = Vec::new();

    if let Some(ref summaries) = state.turn_summary_data {
        if summaries.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("No turn summaries yet.", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from("Turn summaries are generated after each agent turn."));
        } else {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled(format!("{} summary(s)", summaries.len()), Style::default().fg(Color::Cyan)),
                Span::raw(" total"),
            ]));
            lines.push(Line::from(""));

            for summary in summaries.iter() {
                // Turn ID and success status
                let status_icon = if summary.success { "✅" } else { "❌" };
                let status_color = if summary.success { Color::Green } else { Color::Red };

                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(status_icon, Style::default().fg(status_color)),
                    Span::raw(" "),
                    Span::styled(format!("Turn {}", &summary.turn_id[..8.min(summary.turn_id.len())]), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    Span::raw(" - "),
                    Span::styled(format!("{} tool(s)", summary.tool_count), Style::default().fg(Color::Cyan)),
                    Span::raw(" in "),
                    Span::styled(format!("{}ms", summary.total_duration_ms), Style::default().fg(Color::Yellow)),
                ]));

                // Session ID
                lines.push(Line::from(vec![
                    Span::styled("     Session: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&summary.session_id[..16.min(summary.session_id.len())], Style::default().fg(Color::DarkGray)),
                ]));

                // Accomplishment
                if !summary.accomplishment.is_empty() {
                    let acc_preview = if summary.accomplishment.len() > 60 {
                        format!("{}...", &summary.accomplishment[..60])
                    } else {
                        summary.accomplishment.clone()
                    };
                    lines.push(Line::from(vec![
                        Span::styled("     ", Style::default().fg(Color::DarkGray)),
                        Span::styled("📝 ", Style::default().fg(Color::LightGreen)),
                        Span::styled(acc_preview, Style::default().fg(Color::LightGreen)),
                    ]));
                }

                // Affected resources
                if !summary.affected_resources.is_empty() {
                    let resources_preview = if summary.affected_resources.len() > 3 {
                        let first = summary.affected_resources[..3].join(", ");
                        format!("{}... (+{} more)", first, summary.affected_resources.len() - 3)
                    } else {
                        summary.affected_resources.join(", ")
                    };
                    lines.push(Line::from(vec![
                        Span::styled("     ", Style::default().fg(Color::DarkGray)),
                        Span::styled("📁 ", Style::default().fg(Color::LightBlue)),
                        Span::styled(resources_preview, Style::default().fg(Color::LightBlue)),
                    ]));
                }

                // Tool execution summaries
                if !summary.tool_summaries.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("     ", Style::default().fg(Color::DarkGray)),
                        Span::styled("🔧 ", Style::default().fg(Color::Magenta)),
                        Span::styled("Tools:", Style::default().fg(Color::Magenta)),
                    ]));
                    for (i, tool) in summary.tool_summaries.iter().enumerate() {
                        let prefix = if i == summary.tool_summaries.len() - 1 { "       └─ " } else { "       ├─ " };
                        let tool_status = if tool.success { "✅" } else { "❌" };
                        let tool_status_color = if tool.success { Color::Green } else { Color::Red };
                        let tool_summary_preview = if tool.summary.len() > 50 {
                            format!("{}...", &tool.summary[..50])
                        } else {
                            tool.summary.clone()
                        };
                        lines.push(Line::from(vec![
                            Span::styled(prefix, Style::default().fg(Color::DarkGray)),
                            Span::styled(&tool.tool_name, Style::default().fg(Color::Cyan)),
                            Span::raw(" "),
                            Span::styled(tool_status, Style::default().fg(tool_status_color)),
                            Span::raw(" "),
                            Span::styled(format!("({}ms)", tool.duration_ms), Style::default().fg(Color::DarkGray)),
                            Span::raw(" "),
                            Span::styled(tool_summary_preview, Style::default().fg(Color::White)),
                        ]));
                    }
                }

                lines.push(Line::from(""));
            }
        }
    } else {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("No turn summaries collected yet.", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Turn summaries are generated after each agent turn.", Style::default().fg(Color::DarkGray)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Press Esc to exit", Style::default().fg(Color::DarkGray)),
    ]));

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .alignment(Alignment::Left)
        .scroll((state.scroll_offset as u16, 0));

    f.render_widget(paragraph, area);
}
