use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap};

use crate::app::{App, EventView, SessionView};

use ennio_core::event::EventPriority;
use ennio_core::session::SessionStatus;

fn status_color(status: SessionStatus) -> Color {
    match status {
        SessionStatus::Spawning => Color::Cyan,
        SessionStatus::Working => Color::Green,
        SessionStatus::PrOpen | SessionStatus::PrDraft => Color::Blue,
        SessionStatus::CiPassing => Color::Green,
        SessionStatus::CiFailed | SessionStatus::CiFixFailed => Color::Yellow,
        SessionStatus::CiFixSent => Color::Cyan,
        SessionStatus::ReviewPending | SessionStatus::ChangesRequested => Color::Yellow,
        SessionStatus::Approved => Color::Green,
        SessionStatus::MergeConflicts => Color::Red,
        SessionStatus::Merged | SessionStatus::Done => Color::Green,
        SessionStatus::Exited => Color::Gray,
        SessionStatus::Killed => Color::Red,
    }
}

fn priority_color(priority: EventPriority) -> Color {
    match priority {
        EventPriority::Info => Color::Gray,
        EventPriority::Action => Color::Blue,
        EventPriority::Urgent => Color::Yellow,
        EventPriority::Critical => Color::Red,
    }
}

fn format_elapsed(view: &SessionView) -> String {
    let elapsed = chrono::Utc::now()
        .signed_duration_since(view.last_activity_at)
        .num_seconds();
    if elapsed < 60 {
        format!("{elapsed}s ago")
    } else if elapsed < 3600 {
        format!("{}m ago", elapsed / 60)
    } else {
        format!("{}h ago", elapsed / 3600)
    }
}

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),
            Constraint::Length(10),
            Constraint::Length(1),
        ])
        .split(frame.area());

    if app.show_detail {
        let upper = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[0]);
        draw_session_list(frame, upper[0], app);
        draw_session_detail(frame, upper[1], app);
    } else {
        draw_session_list(frame, chunks[0], app);
    }

    draw_event_log(frame, chunks[1], app);
    draw_status_bar(frame, chunks[2], app);
}

pub fn draw_session_list(frame: &mut Frame, area: Rect, app: &App) {
    let header_cells = ["ID", "Project", "Status", "Agent", "Branch", "Activity"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow)));
    let header = Row::new(header_cells)
        .style(Style::default().add_modifier(Modifier::BOLD))
        .height(1);

    let rows = app.sessions.iter().enumerate().map(|(i, s)| {
        let color = status_color(s.status);
        let style = if i == app.selected_index {
            Style::default().fg(color).add_modifier(Modifier::REVERSED)
        } else {
            Style::default().fg(color)
        };

        let activity_str = s.activity.map_or("-", |a| activity_label(a));

        Row::new(vec![
            Cell::from(s.id.as_str()),
            Cell::from(s.project_id.as_str()),
            Cell::from(s.status_label()),
            Cell::from(s.agent_name.as_deref().unwrap_or("-")),
            Cell::from(s.branch.as_deref().unwrap_or("-")),
            Cell::from(activity_str),
        ])
        .style(style)
    });

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(18),
            Constraint::Percentage(15),
            Constraint::Percentage(22),
            Constraint::Percentage(15),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(" Sessions "));

    frame.render_widget(table, area);
}

pub fn draw_session_detail(frame: &mut Frame, area: Rect, app: &App) {
    let content = match app.selected_session() {
        Some(s) => {
            let elapsed = format_elapsed(s);
            let lines = vec![
                Line::from(vec![
                    Span::styled("ID: ", Style::default().fg(Color::Yellow)),
                    Span::raw(s.id.as_str()),
                ]),
                Line::from(vec![
                    Span::styled("Project: ", Style::default().fg(Color::Yellow)),
                    Span::raw(s.project_id.as_str()),
                ]),
                Line::from(vec![
                    Span::styled("Status: ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        s.status_label(),
                        Style::default().fg(status_color(s.status)),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Agent: ", Style::default().fg(Color::Yellow)),
                    Span::raw(s.agent_name.as_deref().unwrap_or("-")),
                ]),
                Line::from(vec![
                    Span::styled("Branch: ", Style::default().fg(Color::Yellow)),
                    Span::raw(s.branch.as_deref().unwrap_or("-")),
                ]),
                Line::from(vec![
                    Span::styled("PR: ", Style::default().fg(Color::Yellow)),
                    Span::raw(s.pr_url.as_deref().unwrap_or("-")),
                ]),
                Line::from(vec![
                    Span::styled("Last Activity: ", Style::default().fg(Color::Yellow)),
                    Span::raw(elapsed),
                ]),
            ];
            Paragraph::new(lines)
        }
        None => Paragraph::new("No session selected"),
    };

    let block = Block::default().borders(Borders::ALL).title(" Detail ");

    frame.render_widget(content.block(block).wrap(Wrap { trim: true }), area);
}

pub fn draw_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let active_count = app
        .sessions
        .iter()
        .filter(|s| !s.status.is_terminal())
        .count();
    let total = app.sessions.len();

    let bar = Paragraph::new(Line::from(vec![
        Span::styled(
            " j/k",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(": navigate  "),
        Span::styled(
            "Enter",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(": detail  "),
        Span::styled(
            "q",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(": quit  "),
        Span::styled(
            format!(" {active_count}/{total} active"),
            Style::default().fg(Color::Green),
        ),
    ]))
    .style(Style::default().bg(Color::DarkGray));

    frame.render_widget(bar, area);
}

pub fn draw_event_log(frame: &mut Frame, area: Rect, app: &App) {
    let lines: Vec<Line<'_>> = app.events.iter().rev().map(|e| event_line(e)).collect();

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" Events "))
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

fn event_line<'a>(event: &'a EventView) -> Line<'a> {
    let ts = event.timestamp.format("%H:%M:%S").to_string();
    let color = priority_color(event.priority);

    Line::from(vec![
        Span::styled(ts, Style::default().fg(Color::DarkGray)),
        Span::raw(" "),
        Span::styled(
            event.event_type.to_string(),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(event.session_id.as_str(), Style::default().fg(Color::Cyan)),
        Span::raw(" "),
        Span::raw(event.message.as_str()),
    ])
}

fn activity_label(state: ennio_core::session::ActivityState) -> &'static str {
    match state {
        ennio_core::session::ActivityState::Active => "active",
        ennio_core::session::ActivityState::Ready => "ready",
        ennio_core::session::ActivityState::Idle => "idle",
        ennio_core::session::ActivityState::WaitingInput => "waiting",
        ennio_core::session::ActivityState::Blocked => "blocked",
        ennio_core::session::ActivityState::Exited => "exited",
    }
}
