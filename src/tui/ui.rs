use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

use super::{App, Pane, Screen};
use crate::labels::fill;
use crate::model::{azure_types, azure_values};

pub fn render(frame: &mut Frame<'_>, app: &App) {
    match app.screen {
        Screen::Snapshots => render_snapshots(frame, app),
        Screen::Estate => render_estate(frame, app),
        Screen::Findings => render_findings(frame, app),
    }
}

fn severity_color(severity: &str) -> Color {
    match severity {
        "high" => Color::Red,
        "medium" => Color::Yellow,
        "low" => Color::LightYellow,
        _ => Color::Blue,
    }
}

fn pane_block(title: &str, focused: bool) -> Block<'_> {
    let style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };
    Block::default()
        .borders(Borders::ALL)
        .border_style(style)
        .title(title)
}

fn render_snapshots(frame: &mut Frame<'_>, app: &App) {
    let [body, footer] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(frame.area());
    let labels = &app.labels;
    let units = &labels.units;
    let items: Vec<ListItem> = app
        .snapshots
        .iter()
        .map(|entry| {
            // Column widths are layout, so the format stays here; only the
            // unit words come from the labels.
            ListItem::new(format!(
                "{}  {}  {:9}  {:4} {}  {:6} {}  {:4} {}",
                &entry.snapshot.id[..8],
                entry.snapshot.created_at.format("%Y-%m-%d %H:%M"),
                entry.snapshot.status.as_str(),
                entry.subscriptions,
                units.subscriptions,
                entry.resources,
                units.resources,
                entry.findings,
                units.findings,
            ))
        })
        .collect();
    let mut state = ListState::default().with_selected(Some(app.snapshot_index));
    frame.render_stateful_widget(
        List::new(items)
            .block(pane_block(&labels.panes.snapshots, true))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED)),
        body,
        &mut state,
    );
    frame.render_widget(
        Paragraph::new(labels.keys.snapshots.as_str()).style(Style::default().fg(Color::DarkGray)),
        footer,
    );
}

fn render_estate(frame: &mut Frame<'_>, app: &App) {
    let [body, footer] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(frame.area());
    let [tree_area, list_area, detail_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(24),
            Constraint::Percentage(38),
            Constraint::Percentage(38),
        ])
        .areas(body);

    render_tree(frame, app, tree_area);
    render_resource_list(frame, app, list_area);
    render_detail(frame, app, detail_area);

    let keys = &app.labels.keys;
    let filter_hint = if app.filtering {
        fill(&keys.filtering, &[("filter", &app.filter)])
    } else if app.filter.is_empty() {
        keys.estate.clone()
    } else {
        fill(&keys.filtered, &[("filter", &app.filter)])
    };
    frame.render_widget(
        Paragraph::new(filter_hint).style(Style::default().fg(Color::DarkGray)),
        footer,
    );
}

fn render_tree(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .estate
        .as_ref()
        .map(|estate| {
            estate
                .tree
                .iter()
                .map(|entry| ListItem::new(entry.label.clone()))
                .collect()
        })
        .unwrap_or_default();
    let mut state = ListState::default().with_selected(Some(app.tree_index));
    frame.render_stateful_widget(
        List::new(items)
            .block(pane_block(&app.labels.panes.estate, app.pane == Pane::Tree))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED)),
        area,
        &mut state,
    );
}

fn render_resource_list(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let resources = app.visible_resources();
    let title = fill(&app.labels.panes.resources, &[("count", &resources.len())]);
    let items: Vec<ListItem> = resources
        .iter()
        .map(|r| {
            ListItem::new(Line::from(vec![
                Span::raw(format!("{:32.32}  ", r.name)),
                Span::styled(
                    azure_types::display_name(&r.azure_type).to_owned(),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect();
    let mut state = ListState::default().with_selected(Some(app.list_index));
    frame.render_stateful_widget(
        List::new(items)
            .block(pane_block(&title, app.pane == Pane::List))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED)),
        area,
        &mut state,
    );
}

fn render_detail(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();
    let words = &app.labels;
    let fields = &words.fields;
    if let Some(resource) = app.selected_resource() {
        let field = |label: &str, value: String| {
            Line::from(vec![
                Span::styled(format!("{label:<10}"), Style::default().fg(Color::Cyan)),
                Span::raw(value),
            ])
        };
        lines.push(field(&fields.name, resource.name.clone()));
        lines.push(field(&fields.r#type, resource.azure_type.clone()));
        if let Some(kind) = &resource.kind {
            lines.push(field(
                &fields.kind,
                azure_values::display_kind(&resource.azure_type, kind).into_owned(),
            ));
        }
        if let Some(location) = &resource.location {
            lines.push(field(
                &fields.location,
                azure_values::display_location(location).into_owned(),
            ));
        }
        lines.push(field(
            &fields.subscription,
            resource.subscription_id.clone(),
        ));
        lines.push(field(&fields.id, resource.display_id.clone()));
        if let Some(tags) = &resource.tags {
            lines.push(field(&fields.tags, tags.to_string()));
        }

        if let Some(estate) = &app.estate {
            let related: Vec<&crate::model::Edge> = estate
                .edges
                .iter()
                .filter(|e| e.source_id == resource.id || e.target_id == resource.id)
                .collect();
            if !related.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::styled(
                    fields.related.as_str(),
                    Style::default().add_modifier(Modifier::BOLD),
                ));
                for edge in related {
                    let (arrow, other) = if edge.source_id == resource.id {
                        ("→", &edge.target_id)
                    } else {
                        ("←", &edge.source_id)
                    };
                    let name = crate::model::short_name(other);
                    lines.push(Line::from(format!(
                        "  {arrow} {name} ({})",
                        edge.kind.as_str()
                    )));
                }
            }
        }

        if let Some(properties) = &resource.properties {
            lines.push(Line::from(""));
            lines.push(Line::styled(
                fields.properties.as_str(),
                Style::default().add_modifier(Modifier::BOLD),
            ));
            let pretty =
                serde_json::to_string_pretty(properties).unwrap_or_else(|_| properties.to_string());
            lines.extend(pretty.lines().map(|l| Line::from(l.to_owned())));
        }
    } else {
        lines.push(Line::from(words.messages.no_selection.as_str()));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(pane_block(&words.panes.detail, app.pane == Pane::Detail))
            .wrap(Wrap { trim: false })
            .scroll((app.detail_scroll, 0)),
        area,
    );
}

fn render_findings(frame: &mut Frame<'_>, app: &App) {
    let [body, footer] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(frame.area());
    let items: Vec<ListItem> = app
        .estate
        .as_ref()
        .map(|estate| {
            estate
                .findings
                .iter()
                .map(|f| {
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!("{:7}", f.severity.as_str()),
                            Style::default()
                                .fg(severity_color(f.severity.as_str()))
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(format!(" {:12.12} ", f.category)),
                        Span::raw(f.title.clone()),
                    ]))
                })
                .collect()
        })
        .unwrap_or_default();
    let count = items.len();
    let mut state = ListState::default().with_selected(Some(app.findings_index));
    frame.render_stateful_widget(
        List::new(items)
            .block(pane_block(
                &fill(&app.labels.panes.findings, &[("count", &count)]),
                true,
            ))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED)),
        body,
        &mut state,
    );
    frame.render_widget(
        Paragraph::new(app.labels.keys.findings.as_str())
            .style(Style::default().fg(Color::DarkGray)),
        footer,
    );
}
