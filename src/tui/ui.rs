//! TUI UI rendering

use std::sync::Arc;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Row, Table, Tabs},
    Frame,
};
use crate::tui::app::{App, VERSION};

/// Draw the UI
pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(f.area());

    // Draw title bar with tabs
    let titles = vec!["系统状态", "会话管理", "配置"];
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("TinyClaw TUI"))
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
    let help = Paragraph::new(" [Tab/1-3] 切换标签 | [q/ESC] 退出 ")
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL));
    let help_rect = Rect::new(chunks[1].x, chunks[1].y + chunks[1].height - 3, chunks[1].width, 3);
    f.render_widget(help, help_rect);
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

/// Draw sessions tab
fn draw_sessions_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let sessions = app.session_manager.list();

    if sessions.is_empty() {
        let empty = Paragraph::new("暂无会话")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).title("会话列表"));
        f.render_widget(empty, area);
        return;
    }

    let rows: Vec<Row> = sessions
        .iter()
        .map(|s: &Arc<parking_lot::RwLock<crate::gateway::session::Session>>| {
            let session = s.read();
            Row::new(vec![
                session.id[..8].to_string(),
                session.label.clone().unwrap_or_else(|| "-".to_string()),
                format!("{:?}", session.kind),
                session.created_at.format("%H:%M:%S").to_string(),
            ])
        })
        .collect();

    let table = Table::new(rows, [Constraint::Length(10), Constraint::Length(20), Constraint::Length(15), Constraint::Length(12)])
        .block(Block::default().borders(Borders::ALL).title("会话列表"))
        .header(Row::new(vec!["ID", "标签", "类型", "创建时间"]).style(Style::default().fg(Color::Cyan)))
        .widths([
            Constraint::Length(10),
            Constraint::Length(20),
            Constraint::Length(15),
            Constraint::Length(12),
        ]);

    f.render_widget(table, area);
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
  工作区: {}"#,
        config.gateway.bind,
        if config.gateway.verbose { "启用" } else { "禁用" },
        config.gateway.data_dir.as_deref().unwrap_or("-"),
        config.agent.model,
        config.agent.api_base,
        config.agent.workspace.as_deref().unwrap_or("-"),
    );

    let config_paragraph = Paragraph::new(config_text)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL).title("当前配置"));

    f.render_widget(config_paragraph, area);
}
