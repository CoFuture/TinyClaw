//! TUI UI rendering

use std::sync::Arc;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Row, Table, Tabs, Wrap},
    Frame,
};
use crate::tui::app::{App, VERSION};

/// Draw the UI
pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)].as_ref())
        .split(f.area());

    // Draw title bar with tabs
    let titles = vec!["系统状态", "会话管理", "配置"];
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(format!("TinyClaw TUI v{}", VERSION)))
        .select(app.current_tab)
        .style(Style::default().fg(Color::Cyan));
    f.render_widget(tabs, chunks[0]);

    // Draw content based on current tab
    match app.current_tab {
        0 => draw_status_tab(f, app, chunks[1]),
        1 => draw_sessions_tab(f, app, chunks[1]),
        2 => draw_config_tab(f, app, chunks[1]),
        _ => {}
    }

    // Draw help bar
    let help_text = if app.current_tab == 1 {
        if app.show_details {
            " [Esc] 关闭详情 | [q] 退出 "
        } else {
            " [↑/↓] 选择会话 | [Enter] 查看详情 | [Tab/1-3] 切换标签 | [q/ESC] 退出 "
        }
    } else {
        " [Tab/1-3] 切换标签 | [q/ESC] 退出 "
    };
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[2]);
}

/// Draw system status tab
fn draw_status_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(area);

    let config = app.config.read();

    // Version
    let version = Paragraph::new(format!("版本: {}", VERSION))
        .style(Style::default().fg(Color::Green))
        .block(Block::default().borders(Borders::ALL).title("TinyClaw"));
    f.render_widget(version, chunks[0]);

    // Model
    let model = Paragraph::new(format!("当前模型: {}", config.agent.model))
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL).title("模型"));
    f.render_widget(model, chunks[1]);

    // Sessions count
    let sessions = app.session_manager.list();
    let session_count = Paragraph::new(format!("会话数量: {}", sessions.len()))
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("会话"));
    f.render_widget(session_count, chunks[2]);

    // Gateway
    let gateway = Paragraph::new(format!("网关地址: {}", config.gateway.bind))
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL).title("网关"));
    f.render_widget(gateway, chunks[3]);

    // Status
    let status = Paragraph::new("状态: 运行中 🟢")
        .style(Style::default().fg(Color::Green))
        .block(Block::default().borders(Borders::ALL).title("系统状态"));
    f.render_widget(status, chunks[4]);
}

/// Draw sessions tab with interactive selection
fn draw_sessions_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let sessions = app.session_manager.list();

    if app.show_details {
        // Show session details
        draw_session_details(f, app, area, &sessions);
        return;
    }

    if sessions.is_empty() {
        let empty = Paragraph::new("暂无会话\n\n按 [q] 退出")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).title("会话列表"));
        f.render_widget(empty, area);
        return;
    }

    // Build table with selection highlight
    let header = Row::new(vec!["ID", "标签", "类型", "创建时间"])
        .style(Style::default().fg(Color::Cyan));
    
    let rows: Vec<Row> = sessions
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let session = s.read();
            let style = if i == app.selected_session {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Row::new(vec![
                session.id[..8.min(session.id.len())].to_string(),
                session.label.clone().unwrap_or_else(|| "-".to_string()),
                format!("{:?}", session.kind),
                session.created_at.format("%H:%M:%S").to_string(),
            ]).style(style)
        })
        .collect();

    let table = Table::new(rows, [Constraint::Length(10), Constraint::Length(20), Constraint::Length(15), Constraint::Length(12)])
        .block(Block::default().borders(Borders::ALL).title("会话列表 (↑/↓ 选择, Enter 查看)"))
        .header(header)
        .widths([
            Constraint::Length(10),
            Constraint::Length(20),
            Constraint::Length(15),
            Constraint::Length(12),
        ]);

    f.render_widget(table, area);
}

/// Draw session details panel
fn draw_session_details(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    sessions: &[Arc<parking_lot::RwLock<crate::gateway::session::Session>>],
) {
    if sessions.is_empty() || app.selected_session >= sessions.len() {
        return;
    }

    let session = sessions[app.selected_session].read();
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(0)].as_ref())
        .split(area);

    // Session info
    let info_text = format!(
        "会话 ID: {}\n标签: {}\n类型: {:?}\n创建时间: {}\n最后活跃: {}",
        session.id,
        session.label.as_deref().unwrap_or("-"),
        session.kind,
        session.created_at.format("%Y-%m-%d %H:%M:%S"),
        session.last_active.format("%Y-%m-%d %H:%M:%S"),
    );
    let info = Paragraph::new(info_text)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL).title("会话详情"));
    f.render_widget(info, chunks[0]);

    // Messages (if available)
    let messages_text = if let Some(history_manager) = &app.history_manager {
        if let Some(history) = history_manager.get(&session.id) {
            let history = history.read();
            if history.messages.is_empty() {
                "暂无消息".to_string()
            } else {
                history.messages
                    .iter()
                    .map(|m| format!("[{:?}] {}", m.role, m.content))
                    .collect::<Vec<_>>()
                    .join("\n\n")
            }
        } else {
            "暂无消息".to_string()
        }
    } else {
        "消息预览需要 HistoryManager".to_string()
    };

    let messages = Paragraph::new(messages_text)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL).title("消息历史"))
        .wrap(Wrap { trim: true });
    f.render_widget(messages, chunks[1]);
}

/// Draw config tab
fn draw_config_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let config = app.config.read();

    let config_text = format!(
        r#"网关配置:
  绑定地址: {}
  调试模式: {}
  数据目录: {}

Agent 配置:
  模型: {}
  API 地址: {}
  工作区: {}

工具配置:
  Exec 工具: {}"#,
        config.gateway.bind,
        if config.gateway.verbose { "启用" } else { "禁用" },
        config.gateway.data_dir.as_deref().unwrap_or("-"),
        config.agent.model,
        config.agent.api_base,
        config.agent.workspace.as_deref().unwrap_or("-"),
        if config.tools.exec_enabled { "启用" } else { "禁用" },
    );

    let config_paragraph = Paragraph::new(config_text)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL).title("当前配置"));

    f.render_widget(config_paragraph, area);
}
