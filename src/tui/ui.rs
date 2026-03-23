//! TUI rendering.

use super::App;
use crate::scanner::PortEntry;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState},
    Frame,
};

/// Draw the main UI.
pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Header
            Constraint::Min(10),    // Table
            Constraint::Length(5),  // Detail
            Constraint::Length(1),  // Footer
        ])
        .split(f.area());
    
    // Header
    let header = Paragraph::new(format!(
        " portpilot - {} ports listening                    [H]elp [Q]uit",
        app.entries.len()
    ))
    .style(Style::default().bg(Color::Blue).fg(Color::White));
    f.render_widget(header, chunks[0]);
    
    // Port table
    draw_table(f, app, chunks[1]);
    
    // Detail panel
    draw_detail(f, app, chunks[2]);
    
    // Footer
    let footer_text = if app.filter_mode {
        format!("Filter: {}_", app.filter_input)
    } else if let Some(ref msg) = app.status_message {
        msg.clone()
    } else {
        "[K] Kill    [S] SIGTERM    [/] Filter    [R] Refresh    [E] External    [L] Local".to_string()
    };
    let footer = Paragraph::new(footer_text)
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));
    f.render_widget(footer, chunks[3]);
    
    // Popups (in order of priority)
    if app.confirm_kill {
        draw_kill_confirm(f, app);
    } else if app.show_detail {
        draw_detail_popup(f, app);
    } else if app.show_help {
        draw_help(f);
    }
}

fn draw_table(f: &mut Frame, app: &App, area: Rect) {
    let header_cells = ["PORT", "PROTO", "PROCESS", "PID", "MEM", "ADDRESS"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1);
    
    let rows = app.entries.iter().map(|entry| {
        let cells = vec![
            Cell::from(entry.port.to_string()),
            Cell::from(entry.protocol.to_string()),
            Cell::from(entry.process_display().to_string()),
            Cell::from(entry.pid.map(|p| p.to_string()).unwrap_or_else(|| "-".to_string())),
            Cell::from(entry.memory_display()),
            Cell::from(format_address(entry)),
        ];
        Row::new(cells)
    });
    
    let widths = [
        Constraint::Length(7),
        Constraint::Length(6),
        Constraint::Length(16),
        Constraint::Length(8),
        Constraint::Length(8),
        Constraint::Min(15),
    ];
    
    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL))
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("> ");
    
    let mut state = TableState::default();
    state.select(Some(app.selected));
    
    f.render_stateful_widget(table, area, &mut state);
}

fn draw_detail(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default().borders(Borders::ALL);
    
    let content = if let Some(entry) = app.selected_entry() {
        let process = entry.process_name.as_deref().unwrap_or("unknown");
        let cmd = entry.command.as_deref().unwrap_or("-");
        let user = entry.user.as_deref().unwrap_or("-");
        let cpu = entry.cpu_percent.map(|c| format!("{:.1}%", c)).unwrap_or_else(|| "-".to_string());
        let started = entry.started_at
            .map(format_time_ago_short)
            .unwrap_or_else(|| "-".to_string());
        
        vec![
            Line::from(format!("Process: {}    Command: {}", process, cmd)),
            Line::from(format!(
                "User: {}    Started: {}    Memory: {}    CPU: {}",
                user,
                started,
                entry.memory_display(),
                cpu
            )),
        ]
    } else {
        vec![Line::from("No port selected")]
    };
    
    let paragraph = Paragraph::new(content).block(block);
    f.render_widget(paragraph, area);
}

/// Format time ago in short form (e.g., "2h ago").
fn format_time_ago_short(time: chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(time);
    
    if duration.num_days() > 0 {
        format!("{}d ago", duration.num_days())
    } else if duration.num_hours() > 0 {
        format!("{}h ago", duration.num_hours())
    } else if duration.num_minutes() > 0 {
        format!("{}m ago", duration.num_minutes())
    } else {
        "now".to_string()
    }
}

fn draw_help(f: &mut Frame) {
    let area = centered_rect(60, 60, f.area());
    
    let help_text = vec![
        Line::from("Keybindings:"),
        Line::from(""),
        Line::from("  j/Down    Move down"),
        Line::from("  k/Up      Move up"),
        Line::from("  g         Go to top"),
        Line::from("  G         Go to bottom"),
        Line::from("  Enter     Show full detail"),
        Line::from("  K         Kill process (confirm)"),
        Line::from("  S         Send SIGTERM only"),
        Line::from("  /         Filter by process"),
        Line::from("  e         Toggle external only"),
        Line::from("  l         Toggle localhost only"),
        Line::from("  s         Cycle sort field"),
        Line::from("  r/R       Refresh"),
        Line::from("  ?/h       Show this help"),
        Line::from("  q/Esc     Quit"),
        Line::from(""),
        Line::from("Press any key to close"),
    ];
    
    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray));
    
    let paragraph = Paragraph::new(help_text).block(block);
    
    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}

fn format_address(entry: &PortEntry) -> String {
    if entry.is_external() {
        "0.0.0.0".to_string()
    } else {
        entry.address.to_string()
    }
}

fn draw_detail_popup(f: &mut Frame, app: &App) {
    let area = centered_rect(70, 60, f.area());
    
    let content = if let Some(entry) = app.selected_entry() {
        let process = entry.process_name.as_deref().unwrap_or("unknown");
        let cmd = entry.command.as_deref().unwrap_or("-");
        let user = entry.user.as_deref().unwrap_or("-");
        let cpu = entry.cpu_percent.map(|c| format!("{:.1}%", c)).unwrap_or_else(|| "-".to_string());
        let exposure = if entry.is_external() {
            "externally accessible"
        } else if entry.is_localhost() {
            "localhost only"
        } else {
            ""
        };
        
        let mut lines = vec![
            Line::from(format!("PORT {} is in use", entry.port)),
            Line::from(""),
            Line::from(format!("Process:    {}", process)),
        ];
        
        if let Some(pid) = entry.pid {
            lines.push(Line::from(format!("PID:        {}", pid)));
        }
        
        lines.push(Line::from(format!("Command:    {}", cmd)));
        lines.push(Line::from(format!("User:       {}", user)));
        lines.push(Line::from(format!("Memory:     {}", entry.memory_display())));
        lines.push(Line::from(format!("CPU:        {}", cpu)));
        lines.push(Line::from(format!("Listening:  {}:{} ({})", entry.address, entry.port, exposure)));
        
        if let (Some(ppid), Some(ref pname)) = (entry.parent_pid, &entry.parent_name) {
            lines.push(Line::from(""));
            lines.push(Line::from(format!("Parent:     {} (PID {})", pname, ppid)));
        }
        
        lines.push(Line::from(""));
        lines.push(Line::from("Press any key to close"));
        
        lines
    } else {
        vec![Line::from("No port selected")]
    };
    
    let block = Block::default()
        .title(" Port Detail ")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray));
    
    let paragraph = Paragraph::new(content).block(block);
    
    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}

fn draw_kill_confirm(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 30, f.area());
    
    let content = if let Some(entry) = app.selected_entry() {
        let process = entry.process_name.as_deref().unwrap_or("unknown");
        let pid = entry.pid.map(|p| p.to_string()).unwrap_or_else(|| "?".to_string());
        
        vec![
            Line::from(""),
            Line::from(format!("Kill {} (PID {})?", process, pid)),
            Line::from(""),
            Line::from("This will send SIGTERM, then SIGKILL if needed."),
            Line::from(""),
            Line::from("[Y] Yes, kill    [N/Esc] Cancel"),
        ]
    } else {
        vec![Line::from("No process selected")]
    };
    
    let block = Block::default()
        .title(" Confirm Kill ")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Red).fg(Color::White));
    
    let paragraph = Paragraph::new(content)
        .block(block)
        .style(Style::default().fg(Color::White));
    
    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
