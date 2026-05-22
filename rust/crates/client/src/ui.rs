use crate::app::{
    AddField, AddModal, App, ConfirmModal, EditField, EditModal, Modal, Screen, TypeManagerMode,
    TypeManagerModal,
};
use media_elo_core::{is_rankable, Row, STATUSES};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row as TableRow, Table, TableState, Wrap},
    Frame,
};

const LIST_KEYS: &[(&str, &str)] = &[
    ("Tab", "Compare"),
    ("a", "Add"),
    ("N", "Manage types"),
    ("d", "Cycle status"),
    ("x", "Delete"),
    ("y", "Yank"),
    ("/", "Search"),
    ("p", "Pending"),
    ("t", "Cycle type"),
    ("o", "Sort"),
    ("?", "Stats"),
    ("r", "Reload"),
    ("H", "Hide help"),
    ("q", "Quit"),
];

const COMPARE_KEYS: &[(&str, &str)] = &[
    ("Tab", "List"),
    ("1/2", "Vote"),
    ("s", "Skip"),
    ("u", "Undo"),
    ("H", "Hide help"),
    ("q", "Quit"),
];

const STATS_KEYS: &[(&str, &str)] = &[
    ("?/q/Esc", "Close"),
    ("j/k", "Scroll"),
    ("d/u", "Page"),
    ("g/G", "Top/Bot"),
    ("H", "Hide help"),
];

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();

    let main = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    draw_header(f, main[0], app);
    match app.screen {
        Screen::List => draw_list(f, main[1], app),
        Screen::Compare => draw_compare(f, main[1], app),
        Screen::Stats => draw_stats(f, main[1], app),
    }

    if app.screen == Screen::List && app.list.search_active {
        draw_search_input(f, main[2], app);
    } else if app.show_help {
        draw_footer(f, main[2], app);
    } else {
        draw_state_line(f, main[2], app);
    }

    match &app.modal {
        Modal::None => {}
        Modal::Add(m) => draw_add_modal(f, area, m, &app.types),
        Modal::Edit(m) => draw_edit_modal(f, area, m, &app.types),
        Modal::Confirm(m) => draw_confirm_modal(f, area, m),
        Modal::TypeManager(m) => draw_type_manager_modal(f, area, m, app),
    }
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let muted = Style::default().fg(Color::DarkGray);
    let screen = match app.screen {
        Screen::List => "list",
        Screen::Compare => "compare",
        Screen::Stats => "stats",
    };
    let spans = vec![
        Span::styled("media-elo", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(screen, muted),
    ];
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_state_line(f: &mut Frame, area: Rect, app: &App) {
    let label = Style::default().fg(Color::Gray);
    let value = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    let on_value = Style::default()
        .fg(Color::LightYellow)
        .add_modifier(Modifier::BOLD);

    let mut chunks: Vec<Vec<Span<'static>>> = Vec::new();

    let push = |chunks: &mut Vec<Vec<Span<'static>>>, k: &str, v: Span<'static>| {
        chunks.push(vec![
            Span::styled(format!("{k}: "), label),
            v,
        ]);
    };

    match app.screen {
        Screen::List => {
            let type_label = app
                .list
                .type_filter
                .clone()
                .unwrap_or_else(|| "all".to_string());
            push(&mut chunks, "type", Span::styled(type_label, value));
            push(
                &mut chunks,
                "sort",
                Span::styled(app.list.sort.label().to_string(), value),
            );
            if app.list.pending_only {
                chunks.push(vec![Span::styled("pending only", on_value)]);
            }
            if !app.list.search_query.is_empty() {
                push(
                    &mut chunks,
                    "search",
                    Span::styled(format!("/{}", app.list.search_query), on_value),
                );
            }
            push(
                &mut chunks,
                "count",
                Span::styled(app.list.visible.len().to_string(), value),
            );
        }
        Screen::Compare => {
            let type_label = app
                .list
                .type_filter
                .clone()
                .unwrap_or_else(|| "all".to_string());
            push(&mut chunks, "type", Span::styled(type_label, value));
            let pending = app.undo_stack.len();
            if pending > 0 {
                push(
                    &mut chunks,
                    "undo",
                    Span::styled(pending.to_string(), value),
                );
            }
        }
        Screen::Stats => {
            chunks.push(vec![Span::styled("press ? to close", label)]);
        }
    }

    if let Some(err) = &app.last_error {
        chunks.push(vec![Span::styled(
            err.clone(),
            Style::default()
                .fg(Color::LightRed)
                .add_modifier(Modifier::BOLD),
        )]);
    }

    let sep = Span::styled("  •  ", Style::default().fg(Color::DarkGray));
    let mut out = Vec::with_capacity(chunks.len() * 4);
    for (i, c) in chunks.into_iter().enumerate() {
        if i > 0 {
            out.push(sep.clone());
        }
        out.extend(c);
    }
    f.render_widget(Paragraph::new(Line::from(out)), area);
}

fn draw_search_input(f: &mut Frame, area: Rect, app: &App) {
    let line = Line::from(vec![
        Span::styled("/", Style::default().fg(Color::Yellow)),
        Span::raw(app.list.search_query.clone()),
        Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK)),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    let keys = match app.screen {
        Screen::List => LIST_KEYS,
        Screen::Compare => COMPARE_KEYS,
        Screen::Stats => STATS_KEYS,
    };
    let key_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(Color::Gray);
    let sep_style = Style::default().fg(Color::DarkGray);

    let mut spans = Vec::with_capacity(keys.len() * 4);
    for (i, (k, label)) in keys.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", sep_style));
        }
        spans.push(Span::styled(*k, key_style));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(*label, label_style));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn status_mark(status: &str) -> Span<'static> {
    match status {
        "done" => Span::styled("x", Style::default().fg(Color::Green)),
        "in progress" => Span::styled(">", Style::default().fg(Color::Yellow)),
        "on hold" => Span::styled("||", Style::default().fg(Color::Cyan)),
        "dropped" => Span::styled("-", Style::default().fg(Color::Red)),
        "backlog" => Span::styled(".", Style::default().fg(Color::DarkGray)),
        _ => Span::styled("?", Style::default().fg(Color::DarkGray)),
    }
}

fn format_elo(v: f64) -> String {
    let rounded = (v * 100.0).round() / 100.0;
    if rounded.fract() == 0.0 {
        format!("{}", rounded as i64)
    } else {
        let s = format!("{:.2}", rounded);
        if let Some(stripped) = s.strip_suffix('0') {
            stripped.to_string()
        } else {
            s
        }
    }
}

fn draw_list(f: &mut Frame, area: Rect, app: &App) {
    let header = TableRow::new(vec![" ", "Title", "Type", "Elo", "M", "Added"]).style(
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<TableRow> = app
        .list
        .visible
        .iter()
        .map(|&i| {
            let r: &Row = &app.rows[i];
            TableRow::new(vec![
                Cell::from(Line::from(status_mark(&r.status))),
                Cell::from(r.title.clone()),
                Cell::from(r.type_.clone()),
                Cell::from(format_elo(r.elo)),
                Cell::from(r.matches.to_string()),
                Cell::from(r.date_added.clone()),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(2),
        Constraint::Percentage(50),
        Constraint::Length(14),
        Constraint::Length(8),
        Constraint::Length(4),
        Constraint::Length(10),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("");

    let mut state = TableState::default();
    if !app.list.visible.is_empty() {
        state.select(Some(app.list.cursor));
    }
    f.render_stateful_widget(table, area, &mut state);
}

fn draw_compare(f: &mut Frame, area: Rect, app: &App) {
    let muted = Style::default().fg(Color::DarkGray);
    let pair_line = |n: &str, title: String, matches: Option<u32>| {
        let mut spans = vec![
            Span::styled(format!("{n}. "), Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(title),
        ];
        if let Some(m) = matches {
            spans.push(Span::styled(format!("  ({m}m)"), muted));
        }
        Line::from(spans).alignment(Alignment::Center)
    };

    let pair = app
        .compare
        .current_pair
        .and_then(|(ai, bi)| {
            let a = app.rows.iter().find(|r| r.id == ai)?;
            let b = app.rows.iter().find(|r| r.id == bi)?;
            Some((a, b))
        });

    let (type_label, line_a, line_b) = match pair {
        Some((a, b)) => (
            a.type_.clone(),
            pair_line("1", a.title.clone(), Some(a.matches)),
            pair_line("2", b.title.clone(), Some(b.matches)),
        ),
        None => (
            String::new(),
            pair_line("1", "Not enough entries to compare.".to_string(), None),
            Line::raw(""),
        ),
    };

    let lines = vec![
        Line::from(Span::styled(type_label, muted.italic()))
            .alignment(Alignment::Center),
        Line::raw(""),
        line_a,
        Line::from(Span::styled("vs", muted)).alignment(Alignment::Center),
        line_b,
    ];

    let box_h: u16 = 9;
    let box_w: u16 = 70.min(area.width.saturating_sub(4));
    let x = area.x + area.width.saturating_sub(box_w) / 2;
    let y = area.y + area.height.saturating_sub(box_h) / 2;
    let rect = Rect {
        x,
        y,
        width: box_w,
        height: box_h,
    };

    f.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL))
            .wrap(Wrap { trim: false }),
        rect,
    );

    if let Some(res) = &app.compare.last_result {
        if rect.y >= area.y + 2 {
            let win_style = Style::default()
                .fg(Color::LightGreen)
                .add_modifier(Modifier::BOLD);
            let lose_style = Style::default().fg(Color::LightRed);
            let result_line = Line::from(vec![
                Span::styled(res.winner_title.clone(), win_style),
                Span::styled(format!("  {:+.1}", res.delta_w), win_style),
                Span::raw("      "),
                Span::styled(res.loser_title.clone(), lose_style),
                Span::styled(format!("  {:+.1}", res.delta_l), lose_style),
            ])
            .alignment(Alignment::Center);
            let line_rect = Rect {
                x: area.x,
                y: rect.y.saturating_sub(2),
                width: area.width,
                height: 1,
            };
            f.render_widget(Paragraph::new(result_line), line_rect);
        }
    }
}

fn draw_stats(f: &mut Frame, area: Rect, app: &App) {
    let total = app.rows.len();
    let total_matches: u32 = app.rows.iter().map(|r| r.matches).sum();
    let rankable: Vec<&Row> = app.rows.iter().filter(|r| is_rankable(&r.status)).collect();
    let rankable_matches: u32 = rankable.iter().map(|r| r.matches).sum();
    let avg = if rankable.is_empty() {
        0.0
    } else {
        rankable_matches as f64 / rankable.len() as f64
    };

    let bold_cyan = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let bold = Style::default().add_modifier(Modifier::BOLD);
    let muted = Style::default().fg(Color::DarkGray);
    let yellow = Style::default().fg(Color::Yellow);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled("Global", bold_cyan)));
    lines.push(Line::from(format!("  Total entries:           {total}")));
    lines.push(Line::from(format!(
        "  Rankable (done/dropped): {}",
        rankable.len()
    )));
    lines.push(Line::from(format!(
        "  Total matches:           {total_matches}"
    )));
    lines.push(Line::from(format!(
        "  Avg matches / rankable:  {avg:.1}"
    )));
    lines.push(Line::raw(""));

    lines.push(Line::from(Span::styled("By status", bold_cyan)));
    for s in STATUSES {
        let count = app.rows.iter().filter(|r| &r.status == s).count();
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::raw(format!("{s:<14}")),
            Span::styled(count.to_string(), yellow),
        ]));
    }
    lines.push(Line::raw(""));

    let mut ordered: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for t in &app.types {
        if rankable.iter().any(|r| &r.type_ == t) {
            seen.insert(t.clone());
            ordered.push(t.clone());
        }
    }
    for r in &rankable {
        if !seen.contains(&r.type_) {
            seen.insert(r.type_.clone());
            ordered.push(r.type_.clone());
        }
    }

    if !ordered.is_empty() {
        let (lo, hi) = elo_range(&rankable);

        lines.push(Line::from(Span::styled("Top per type", bold_cyan)));
        for t in &ordered {
            let mut entries: Vec<&&Row> =
                rankable.iter().filter(|r| &r.type_ == t).collect();
            entries.sort_by(|a, b| {
                b.elo
                    .partial_cmp(&a.elo)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            lines.push(Line::from(Span::styled(format!("  {t}"), bold)));
            let spark = sparkline(entries.iter().map(|r| r.elo), lo, hi, 24);
            lines.push(Line::from(vec![
                Span::styled(format!("    {:.0} ", lo), muted),
                Span::styled(spark, Style::default().fg(Color::Cyan)),
                Span::styled(format!(" {:.0}", hi), muted),
                Span::styled(format!("   n={}", entries.len()), muted),
            ]));
            for (i, r) in entries.iter().take(5).enumerate() {
                let title = truncate(&r.title, 40);
                lines.push(Line::from(vec![
                    Span::raw(format!("    {}. ", i + 1)),
                    Span::raw(format!("{title:<40}  ")),
                    Span::styled(format_elo(r.elo), yellow),
                    Span::styled(format!("  ({}m)", r.matches), muted),
                ]));
            }
        }
    }

    let content_h = lines.len() as u16;
    app.stats_content_height.set(content_h);
    let max_scroll = content_h.saturating_sub(area.height);
    let scroll = app.stats_scroll.min(max_scroll);

    f.render_widget(Paragraph::new(lines).scroll((scroll, 0)), area);

    if max_scroll > 0 && area.width > 12 {
        let pct = if max_scroll == 0 {
            100
        } else {
            (scroll as u32 * 100 / max_scroll as u32).min(100) as u16
        };
        let txt = format!(" {pct:>3}% ");
        let w = txt.len() as u16;
        let indicator = Rect {
            x: area.x + area.width.saturating_sub(w),
            y: area.y,
            width: w,
            height: 1,
        };
        f.render_widget(
            Paragraph::new(txt).style(Style::default().fg(Color::DarkGray)),
            indicator,
        );
    }
}

fn elo_range(rows: &[&Row]) -> (f64, f64) {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for r in rows {
        lo = lo.min(r.elo);
        hi = hi.max(r.elo);
    }
    if !lo.is_finite() || !hi.is_finite() || (hi - lo).abs() < 1.0 {
        return (1200.0, 2000.0);
    }
    (lo.floor(), hi.ceil())
}

fn sparkline(values: impl IntoIterator<Item = f64>, lo: f64, hi: f64, width: usize) -> String {
    const LEVELS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    if width == 0 || hi <= lo {
        return String::new();
    }
    let mut buckets = vec![0u32; width];
    for v in values {
        let clamped = v.clamp(lo, hi);
        let pos = ((clamped - lo) / (hi - lo)) * width as f64;
        let idx = (pos as usize).min(width - 1);
        buckets[idx] += 1;
    }
    let max = *buckets.iter().max().unwrap_or(&0);
    if max == 0 {
        return " ".repeat(width);
    }
    buckets
        .iter()
        .map(|&c| {
            if c == 0 {
                ' '
            } else {
                let level = ((c as f64 / max as f64) * (LEVELS.len() - 1) as f64).round() as usize;
                LEVELS[level.min(LEVELS.len() - 1)]
            }
        })
        .collect()
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

fn centered_rect(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width.saturating_sub(2));
    let h = h.min(area.height.saturating_sub(2));
    Rect {
        x: area.x + (area.width.saturating_sub(w)) / 2,
        y: area.y + (area.height.saturating_sub(h)) / 2,
        width: w,
        height: h,
    }
}

fn select_line(label: &str, value: &str, focused: bool) -> Line<'static> {
    let arrow_style = if focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let val_style = if focused {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    Line::from(vec![
        Span::raw(format!("{label}: ")),
        Span::styled("< ", arrow_style),
        Span::styled(value.to_string(), val_style),
        Span::styled(" >", arrow_style),
    ])
}

fn input_line(label: &str, value: &str, focused: bool) -> Line<'static> {
    let style = if focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let mut spans = vec![
        Span::raw(format!("{label}: ")),
        Span::styled("[", style),
        Span::raw(value.to_string()),
    ];
    if focused {
        spans.push(Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK)));
    }
    spans.push(Span::styled("]", style));
    Line::from(spans)
}

fn draw_add_modal(f: &mut Frame, area: Rect, m: &AddModal, types: &[String]) {
    let rect = centered_rect(area, 60, 12);
    f.render_widget(Clear, rect);

    let type_label = types.get(m.type_idx).map(|s| s.as_str()).unwrap_or("(none)");

    let body = vec![
        Line::from(Span::styled(
            "Add entry",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        select_line("Type", type_label, m.focus == AddField::Type),
        input_line("Title", &m.title, m.focus == AddField::Title),
        input_line("Rating (1-10)", &m.rating, m.focus == AddField::Rating),
        select_line("Status", STATUSES[m.status_idx], m.focus == AddField::Status),
        Line::raw(""),
        Line::from(Span::styled(
            "Tab next  h/l change select  Enter save  Esc cancel",
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        )),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Add ")
        .border_style(Style::default().fg(Color::Cyan));
    f.render_widget(Paragraph::new(body).block(block), rect);
}

fn draw_edit_modal(f: &mut Frame, area: Rect, m: &EditModal, types: &[String]) {
    let rect = centered_rect(area, 60, 12);
    f.render_widget(Clear, rect);

    let type_label = types.get(m.type_idx).map(|s| s.as_str()).unwrap_or("(none)");

    let body = vec![
        Line::from(Span::styled(
            "Edit entry",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        select_line("Type", type_label, m.focus == EditField::Type),
        input_line("Title", &m.title, m.focus == EditField::Title),
        select_line("Status", STATUSES[m.status_idx], m.focus == EditField::Status),
        Line::raw(""),
        Line::from(Span::styled(
            format!("Elo: {}  Matches: {}", format_elo(m.display_elo), m.display_matches),
            Style::default().fg(Color::White),
        )),
        Line::raw(""),
        Line::from(Span::styled(
            "Tab next  h/l change select  Enter save  Esc cancel",
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        )),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Edit ")
        .border_style(Style::default().fg(Color::Cyan));
    f.render_widget(Paragraph::new(body).block(block), rect);
}

fn draw_type_manager_modal(f: &mut Frame, area: Rect, m: &TypeManagerModal, app: &App) {
    let row_count = (app.types.len() as u16).max(1);
    let height = (row_count + 6).min(area.height.saturating_sub(2)).max(8);
    let rect = centered_rect(area, 60, height);
    f.render_widget(Clear, rect);

    let mut body: Vec<Line> = Vec::new();
    if app.types.is_empty() {
        body.push(Line::from(Span::styled(
            "(no types defined)",
            Style::default().fg(Color::DarkGray).italic(),
        )));
    } else {
        for (i, t) in app.types.iter().enumerate() {
            let in_use = app
                .rows
                .iter()
                .filter(|r| r.type_.eq_ignore_ascii_case(t))
                .count();
            let selected = i == m.cursor && m.mode == TypeManagerMode::Browse;
            let marker = if selected { "> " } else { "  " };
            let row_style = if selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let line = format!("{marker}{t}  ({in_use} rows)");
            body.push(Line::from(Span::styled(line, row_style)));
        }
    }

    body.push(Line::raw(""));
    match m.mode {
        TypeManagerMode::Browse => {
            if let Some(err) = &m.error {
                body.push(Line::from(Span::styled(
                    err.clone(),
                    Style::default().fg(Color::Red),
                )));
                body.push(Line::raw(""));
            }
            body.push(Line::from(Span::styled(
                "a add  r rename  x delete  K/J reorder  Esc close",
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            )));
        }
        TypeManagerMode::AddInput => {
            body.push(input_line("New", &m.buffer, true));
            if let Some(err) = &m.error {
                body.push(Line::from(Span::styled(
                    err.clone(),
                    Style::default().fg(Color::Red),
                )));
            }
            body.push(Line::raw(""));
            body.push(Line::from(Span::styled(
                "Enter save  Esc back",
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            )));
        }
        TypeManagerMode::RenameInput => {
            body.push(input_line("Rename to", &m.buffer, true));
            if let Some(err) = &m.error {
                body.push(Line::from(Span::styled(
                    err.clone(),
                    Style::default().fg(Color::Red),
                )));
            }
            body.push(Line::raw(""));
            body.push(Line::from(Span::styled(
                "Enter save  Esc back",
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            )));
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Types ")
        .border_style(Style::default().fg(Color::Cyan));
    f.render_widget(Paragraph::new(body).block(block), rect);
}

fn draw_confirm_modal(f: &mut Frame, area: Rect, m: &ConfirmModal) {
    let rect = centered_rect(area, 50, 6);
    f.render_widget(Clear, rect);

    let body = vec![
        Line::from(m.message.as_str()),
        Line::raw(""),
        Line::from(Span::styled(
            "y=confirm, n/Esc=cancel",
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        )),
    ];
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Confirm ")
        .border_style(Style::default().fg(Color::Red));
    f.render_widget(Paragraph::new(body).block(block), rect);
}
