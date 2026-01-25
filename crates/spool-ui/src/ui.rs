use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, Focus};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header
            Constraint::Min(0),    // Main content
            Constraint::Length(1), // Footer
        ])
        .split(f.area());

    draw_header(f, chunks[0], app);
    draw_main(f, chunks[1], app);
    draw_footer(f, chunks[2]);
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let title = format!(
        " spool  {} tasks ({})",
        app.tasks.len(),
        app.status_filter.label()
    );
    let header = Paragraph::new(title).style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(header, area);
}

fn draw_main(f: &mut Frame, area: Rect, app: &App) {
    if app.show_detail {
        // Split into list and detail
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        draw_task_list(f, chunks[0], app);
        draw_task_detail(f, chunks[1], app);
    } else {
        draw_task_list(f, area, app);
    }
}

fn draw_task_list(f: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .tasks
        .iter()
        .enumerate()
        .map(|(i, task)| {
            let priority = task.priority.as_deref().unwrap_or("--");
            let priority_style = match priority {
                "p0" => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                "p1" => Style::default().fg(Color::Yellow),
                "p2" => Style::default().fg(Color::Blue),
                _ => Style::default().fg(Color::DarkGray),
            };

            let assignee = task
                .assignee
                .as_deref()
                .map(|a| format!(" {}", a))
                .unwrap_or_default();

            let line = Line::from(vec![
                Span::styled(format!("{:4} ", priority), priority_style),
                Span::raw(&task.title),
                Span::styled(assignee, Style::default().fg(Color::DarkGray)),
            ]);

            let style = if i == app.selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let border_style = if app.focus == Focus::TaskList {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(" Tasks "),
    );

    f.render_widget(list, area);
}

fn draw_task_detail(f: &mut Frame, area: Rect, app: &App) {
    let border_style = if app.focus == Focus::Detail {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let content = if let Some(task) = app.selected_task() {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("ID: ", Style::default().fg(Color::DarkGray)),
                Span::raw(&task.id),
            ]),
            Line::from(vec![
                Span::styled("Title: ", Style::default().fg(Color::DarkGray)),
                Span::styled(&task.title, Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::DarkGray)),
                Span::raw(format!("{:?}", task.status)),
            ]),
        ];

        if let Some(priority) = &task.priority {
            lines.push(Line::from(vec![
                Span::styled("Priority: ", Style::default().fg(Color::DarkGray)),
                Span::raw(priority),
            ]));
        }

        if let Some(assignee) = &task.assignee {
            lines.push(Line::from(vec![
                Span::styled("Assignee: ", Style::default().fg(Color::DarkGray)),
                Span::raw(assignee),
            ]));
        }

        if !task.tags.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Tags: ", Style::default().fg(Color::DarkGray)),
                Span::raw(task.tags.join(", ")),
            ]));
        }

        if let Some(desc) = &task.description {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "Description:",
                Style::default().fg(Color::DarkGray),
            )]));
            lines.push(Line::from(desc.as_str()));
        }

        lines
    } else {
        vec![Line::from("No task selected")]
    };

    let detail = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(" Detail "),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(detail, area);
}

fn draw_footer(f: &mut Frame, area: Rect) {
    let help = " q: quit  j/k: navigate  Enter: toggle detail  Tab: switch focus ";
    let footer = Paragraph::new(help).style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, area);
}
