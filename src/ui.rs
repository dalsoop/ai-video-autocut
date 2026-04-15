use crate::app::{App, View};
use crate::util::{compute_viewport, truncate_left, truncate_right};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App) {
    let size = f.area();
    let vsplit = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5), Constraint::Length(3)])
        .split(size);
    draw_header(f, vsplit[0], app);
    match app.view {
        View::Projects => draw_projects(f, vsplit[1], app),
        View::Files => draw_files_with_preview(f, vsplit[1], app),
        View::Subtitles => draw_subtitles(f, vsplit[1], app),
    }
    draw_footer(f, vsplit[2], app);
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let proj = app.active_project.as_deref().unwrap_or("(미선택)");
    let engine = &app.engine;
    let title = format!(" autocut-tui │ project: {proj} │ engine: {engine} │ lang: {} ", app.lang);
    let p = Paragraph::new(title).style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(p, area);
}

fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    let footer = if let Some((_, pct, msg)) = &app.job_progress {
        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL)
                .title(format!(" {msg}  [ESC로 취소] ")))
            .gauge_style(Style::default().fg(Color::Green))
            .percent((*pct).min(100) as u16);
        f.render_widget(gauge, area);
        return;
    } else {
        match app.view {
            View::Projects => "[↑/↓ 이동] [Enter 선택] [q 종료]".to_string(),
            View::Files => "[↑/↓ 이동] [Enter 열기] [t 자막추출] [p 프로젝트] [r 새로고침] [q 종료]".into(),
            View::Subtitles => "[↑/↓ 이동] [Space 토글] [a 모두유지] [i 반전] [c 컷] [b 뒤로] [q 종료]".into(),
        }
    };
    let mut lines = vec![Line::from(footer)];
    if !app.status.is_empty() {
        lines.push(Line::from(Span::styled(&app.status, Style::default().fg(Color::Yellow))));
    }
    let p = Paragraph::new(lines).block(Block::default().borders(Borders::ALL));
    f.render_widget(p, area);
}

fn draw_projects(f: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app.projects.iter().enumerate().map(|(i, p)| {
        let marker = if Some(p) == app.active_project.as_ref() { "★ " } else { "  " };
        let style = if i == app.project_cursor {
            Style::default().bg(Color::DarkGray).fg(Color::White).add_modifier(Modifier::BOLD)
        } else { Style::default() };
        ListItem::new(format!("{marker}{p}")).style(style)
    }).collect();
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" 활성 프로젝트 선택 "));
    let mut state = ListState::default();
    state.select(Some(app.project_cursor));
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_files_with_preview(f: &mut Frame, area: Rect, app: &App) {
    let hsplit = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    // 왼쪽: 파일 리스트 (unicode-safe truncate + viewport 스크롤)
    let name_w = (hsplit[0].width as usize).saturating_sub(12);
    let items: Vec<ListItem> = app.files.iter().enumerate().map(|(i, f)| {
        let s = if f.has_output { "✂ " } else if f.has_subtitle { "✓ " } else { "  " };
        let style = if i == app.file_cursor {
            Style::default().bg(Color::DarkGray).fg(Color::White).add_modifier(Modifier::BOLD)
        } else { Style::default() };
        let short = truncate_left(&f.name, name_w);
        ListItem::new(format!("{s}{} ({})", short, human_size(f.size))).style(style)
    }).collect();
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL)
            .title(format!(" 파일 ({}) ", app.files.len())));
    let mut state = ListState::default();
    state.select(Some(app.file_cursor));
    let visible = (hsplit[0].height as usize).saturating_sub(2);
    *state.offset_mut() = compute_viewport(app.file_cursor, visible, app.files.len());
    f.render_stateful_widget(list, hsplit[0], &mut state);

    // 오른쪽: 결과물
    let outs: Vec<ListItem> = app.outputs.iter().map(|o| {
        let label = o.source.as_deref().unwrap_or(&o.name);
        ListItem::new(format!("{} ({})", label, human_size(o.size)))
    }).collect();
    let olist = List::new(outs)
        .block(Block::default().borders(Borders::ALL)
            .title(format!(" 편집본 ({}) ", app.outputs.len())));
    f.render_widget(olist, hsplit[1]);
}

fn draw_subtitles(f: &mut Frame, area: Rect, app: &App) {
    let Some(sub) = &app.subtitle else {
        let p = Paragraph::new("자막이 없습니다").block(Block::default().borders(Borders::ALL));
        f.render_widget(p, area);
        return;
    };
    let (kept_cnt, kept_dur) = app.kept_count();
    let title = format!(" {} │ {}/{} 유지 │ {:.1}s/{:.1}s ",
        app.selected_file.as_ref().map(|f| f.name.as_str()).unwrap_or(""),
        kept_cnt, sub.lines.len(), kept_dur, sub.total_duration);

    let text_w = (area.width as usize).saturating_sub(18);
    let items: Vec<ListItem> = sub.lines.iter().enumerate().map(|(i, l)| {
        let mark = if l.kept { "[✓]" } else { "[ ]" };
        let t_start = fmt_time(l.start);
        let text = truncate_right(&l.text, text_w);
        let base = format!("{mark} {t_start} │ {} ({:.1}s)", text, l.duration);
        let mut style = if l.kept {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::CROSSED_OUT)
        };
        if i == app.sub_cursor {
            style = style.bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD);
        }
        ListItem::new(Line::from(base)).style(style)
    }).collect();
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title));
    let mut state = ListState::default();
    state.select(Some(app.sub_cursor));
    let visible = (area.height as usize).saturating_sub(2);
    *state.offset_mut() = compute_viewport(app.sub_cursor, visible, sub.lines.len());
    f.render_stateful_widget(list, area, &mut state);
}

fn human_size(b: u64) -> String {
    if b < 1024 { format!("{}B", b) }
    else if b < 1_000_000 { format!("{:.1}K", b as f64 / 1024.0) }
    else if b < 1_000_000_000 { format!("{:.1}M", b as f64 / 1e6) }
    else { format!("{:.2}G", b as f64 / 1e9) }
}

fn fmt_time(s: f64) -> String {
    let m = (s as u64) / 60;
    let sec = (s as u64) % 60;
    format!("{:02}:{:02}", m, sec)
}
