use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Row, Table, Wrap},
    Frame,
};

use crate::app::{App, Mode};

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(frame.area());

    render_header(frame, app, chunks[0]);
    render_table(frame, app, chunks[1]);
    render_footer(frame, app, chunks[2]);

    match &app.mode {
        Mode::ConfirmKill {
            pid,
            name,
            container_id: _,
        } => {
            render_confirm_modal(frame, pid, name);
        }
        _ => {
            if let Some(msg) = &app.message {
                render_message_popup(frame, msg);
            }
        }
    }
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let search_text = if matches!(app.mode, Mode::Search) {
        format!("Search: {}", app.search_query)
    } else if !app.search_query.is_empty() {
        format!("Search: {} (press / to edit)", app.search_query)
    } else {
        "Press / to search".to_string()
    };

    let header = Paragraph::new(search_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title("Port Manager")
                .title_alignment(Alignment::Center),
        )
        .style(if matches!(app.mode, Mode::Search) {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });

    frame.render_widget(header, area);
}

fn render_table(frame: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec!["PORT", "PID", "NAME", "PATH"]).style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = app
        .filtered
        .iter()
        .enumerate()
        .map(|(idx, &port_idx)| {
            let port = &app.ports[port_idx];
            let is_selected = app.selected == idx;
            let style = if is_selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Row::new(vec![
                port.port.to_string(),
                port.pid.to_string(),
                truncate(&port.process_name, 15),
                truncate(&port.process_path, 35),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Min(15),
            Constraint::Min(35),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(format!(
        "Open Ports (showing {} of {})",
        app.filtered.len(),
        app.ports.len()
    )));

    frame.render_widget(table, area);

    if !app.filtered.is_empty() {
        if let Mode::Normal = app.mode {
            let highlight_idx = app.selected;
            let first_visible = 0usize;
            let visible_count = area.height.saturating_sub(3) as usize;

            let offset = if highlight_idx < first_visible + visible_count / 2 {
                0
            } else {
                highlight_idx.saturating_sub(visible_count / 2)
            };

            let _ = offset;
        }
    }
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let hints = match app.mode {
        Mode::Normal => "q:quit /:search j/k:navigate Enter:kill r:refresh",
        Mode::Search => "Esc:cancel Enter:done",
        Mode::ConfirmKill { .. } => "y:confirm n:cancel",
    };

    let footer = Paragraph::new(hints)
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);

    frame.render_widget(footer, area);
}

fn render_message_popup(frame: &mut Frame, msg: &str) {
    let area = centered_rect(60, 20, frame.area());
    let popup = Paragraph::new(Text::from(vec![
        Line::from(msg),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to continue",
            Style::default().fg(Color::Gray),
        )),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    )
    .alignment(Alignment::Center)
    .wrap(Wrap { trim: true });
    frame.render_widget(Clear, area);
    frame.render_widget(popup, area);
}

fn render_confirm_modal(frame: &mut Frame, pid: &u32, name: &str) {
    let area = centered_rect(50, 20, frame.area());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .title("Confirm Kill")
        .title_alignment(Alignment::Center);

    let text = Text::from(vec![
        Line::from(Span::styled(
            format!("Kill process '{}'?", name),
            Style::default().fg(Color::Yellow),
        )),
        Line::from(Span::styled(
            format!("PID: {}", pid),
            Style::default().fg(Color::White),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "[Y]es  [N]o",
            Style::default().add_modifier(Modifier::BOLD),
        )),
    ]);

    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(Clear, area);
    frame.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
