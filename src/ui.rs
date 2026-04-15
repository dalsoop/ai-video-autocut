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
        View::Settings => draw_settings(f, vsplit[1], app),
    }
    draw_footer(f, vsplit[2], app);
    if app.show_help { draw_help(f, size); }
    if app.confirm_cut { draw_confirm_cut(f, size, app); }
    if app.editing_line { draw_edit_modal(f, size, app); }
    if app.label_mode { draw_label_modal(f, size, app); }
}

fn draw_label_modal(f: &mut Frame, area: Rect, app: &App) {
    let w = 60.min(area.width);
    let h: u16 = 7;
    let popup = Rect {
        x: (area.width - w) / 2, y: (area.height - h) / 2,
        width: w, height: h,
    };
    f.render_widget(ratatui::widgets::Clear, popup);
    let (cnt, dur) = app.kept_count();
    let lines = vec![
        Line::from(Span::styled("컷 편집 라벨 (선택)", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(format!("유지 {} 라인 / {:.1}초", cnt, dur), Style::default().fg(Color::DarkGray))),
        Line::from(""),
        Line::from(Span::styled(format!("라벨> {}█", app.label_buffer), Style::default().fg(Color::White))),
        Line::from(Span::styled("  [Enter 실행]   [Esc 취소]   (비워두면 타임스탬프만)", Style::default().fg(Color::Cyan))),
    ];
    let p = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).style(Style::default().bg(Color::Rgb(30,20,30))))
        .wrap(Wrap { trim: false });
    f.render_widget(p, popup);
}

fn draw_edit_modal(f: &mut Frame, area: Rect, app: &App) {
    let w = (area.width * 4 / 5).min(100).max(40);
    let h: u16 = 7;
    let popup = Rect {
        x: (area.width - w) / 2,
        y: (area.height - h) / 2,
        width: w, height: h,
    };
    f.render_widget(ratatui::widgets::Clear, popup);
    let orig = app.subtitle.as_ref()
        .and_then(|s| s.lines.get(app.sub_cursor))
        .map(|l| l.text.as_str()).unwrap_or("");
    let lines = vec![
        Line::from(Span::styled("자막 라인 편집", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(format!("원본: {}", orig), Style::default().fg(Color::DarkGray))),
        Line::from(""),
        Line::from(Span::styled(format!("편집> {}█", app.edit_buffer), Style::default().fg(Color::White))),
        Line::from(Span::styled("  [Enter 저장]   [Esc 취소]   [Backspace 지우기]", Style::default().fg(Color::Cyan))),
    ];
    let p = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL)
            .style(Style::default().bg(Color::Rgb(20, 30, 20))))
        .wrap(Wrap { trim: false });
    f.render_widget(p, popup);
}

fn draw_confirm_cut(f: &mut Frame, area: Rect, app: &App) {
    let w = 50.min(area.width);
    let h = 8.min(area.height);
    let popup = Rect {
        x: (area.width - w) / 2,
        y: (area.height - h) / 2,
        width: w, height: h,
    };
    f.render_widget(ratatui::widgets::Clear, popup);
    let (cnt, dur) = app.kept_count();
    let filename = app.selected_file.as_ref().map(|f| f.name.as_str()).unwrap_or("");
    let lines = vec![
        Line::from(Span::styled("컷 편집 실행", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(format!("파일: {}", truncate_right(filename, (w as usize).saturating_sub(6)))),
        Line::from(format!("유지: {} 라인 / {:.1}초", cnt, dur)),
        Line::from(""),
        Line::from(Span::styled("  [Y/Enter 실행]   [N/Esc 취소]", Style::default().fg(Color::Cyan))),
    ];
    let p = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).style(Style::default().bg(Color::Rgb(30,20,20))));
    f.render_widget(p, popup);
}

fn draw_help(f: &mut Frame, area: Rect) {
    let w = (area.width * 4 / 5).min(60);
    let h = (area.height * 4 / 5).min(24);
    let popup = Rect {
        x: (area.width - w) / 2,
        y: (area.height - h) / 2,
        width: w, height: h,
    };
    f.render_widget(ratatui::widgets::Clear, popup);
    let lines = vec![
        Line::from(Span::styled("ai-video-autocut 키바인드", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::styled("  공통", Style::default().fg(Color::Yellow))),
        Line::from("    ?           도움말 토글"),
        Line::from("    q           종료 / Ctrl+C 강제 종료"),
        Line::from("    r           새로고침"),
        Line::from(""),
        Line::from(Span::styled("  파일 화면", Style::default().fg(Color::Yellow))),
        Line::from("    j/k ↑/↓     이동, PgUp/PgDn 10씩, g/G 처음/끝"),
        Line::from("    Enter       파일 열기"),
        Line::from("    /           파일명 검색"),
        Line::from("    t           자막 추출"),
        Line::from("    B           미추출 전체 배치 추출"),
        Line::from("    e           엔진 토글 / l 언어 토글"),
        Line::from("    s           설정 / p 프로젝트 변경"),
        Line::from(""),
        Line::from(Span::styled("  자막 편집", Style::default().fg(Color::Yellow))),
        Line::from("    Space       라인 토글  / a 모두 유지 / n 모두 제거 / i 반전"),
        Line::from("    E           라인 텍스트 편집"),
        Line::from("    S           라인 split / M 다음과 merge"),
        Line::from("    /           자막 내 검색"),
        Line::from("    c           컷 실행  / t 재추출"),
        Line::from("    Esc, b      파일 뷰로"),
        Line::from(""),
        Line::from(Span::styled("  작업 진행 중", Style::default().fg(Color::Yellow))),
        Line::from("    Esc         취소"),
    ];
    let p = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL)
            .title(" ? 로 닫기 ")
            .style(Style::default().bg(Color::Rgb(20, 20, 30))))
        .wrap(Wrap { trim: false });
    f.render_widget(p, popup);
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let proj = app.active_project.as_deref().unwrap_or("(미선택)");
    let engine = &app.engine;
    let title = format!(" ai-video-autocut │ project: {proj} │ engine: {engine} │ lang: {} ", app.lang);
    let p = Paragraph::new(title).style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(p, area);
}

fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    let footer = if let Some(q) = &app.sub_search {
        let p = Paragraph::new(format!("/{}", q))
            .block(Block::default().borders(Borders::ALL).title(" 자막 내 검색 (Enter, Esc) "))
            .style(Style::default().fg(Color::Cyan));
        f.render_widget(p, area); return;
    } else if let Some((_, pct, msg)) = &app.job_progress {
        let elapsed = app.job_started.map(|t| format!(" • {}초", t.elapsed().as_secs())).unwrap_or_default();
        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL)
                .title(format!(" {msg}{elapsed}  [ESC로 취소] ")))
            .gauge_style(Style::default().fg(Color::Green))
            .percent((*pct).min(100) as u16);
        f.render_widget(gauge, area);
        return;
    } else if app.search_mode {
        let p = Paragraph::new(format!("/ {}", app.search_query))
            .block(Block::default().borders(Borders::ALL).title(" 검색 (Enter 확정, Esc 취소) "))
            .style(Style::default().fg(Color::Cyan));
        f.render_widget(p, area);
        return;
    } else {
        match app.view {
            View::Projects => "[↑/↓ 이동] [Enter 선택] [q 종료]".to_string(),
            View::Files => format!("[↑/↓] [Enter] [t 추출] [B 배치({}대기)] [/ 검색] [s 설정] [p 프로젝트] [q]", app.pending_count),
            View::Subtitles => "[↑/↓] [Space] [a/n/i] [E 편집] [S split] [M merge] [/ 검색] [c 컷] [t 재추출] [b] [q]".into(),
            View::Settings => "[↑/↓ 이동] [←/→ 값변경] [Enter 저장] [Esc 취소]".into(),
        }
    };
    let mut lines = vec![Line::from(footer)];
    if !app.status.is_empty() {
        lines.push(Line::from(Span::styled(&app.status, Style::default().fg(Color::Yellow))));
    }
    let p = Paragraph::new(lines).block(Block::default().borders(Borders::ALL));
    f.render_widget(p, area);
}

fn draw_settings(f: &mut Frame, area: Rect, app: &App) {
    let fields = [
        ("기본 엔진", app.settings.default_engine.as_str()),
        ("기본 언어", app.settings.default_lang.as_str()),
        ("Whisper 모델", app.settings.default_whisper_model.as_str()),
        ("Qwen3 디바이스", app.settings.qwen3_device.as_str()),
    ];
    let items: Vec<ListItem> = fields.iter().enumerate().map(|(i, (k, v))| {
        let mut style = Style::default();
        if i == app.settings_cursor {
            style = Style::default().bg(Color::DarkGray).fg(Color::White).add_modifier(Modifier::BOLD);
        }
        ListItem::new(format!("  {:<18} {}", k, v)).style(style)
    }).collect();
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL)
            .title(" 설정 (←→ 값 변경, Enter 저장, Esc 취소) "));
    let mut state = ListState::default();
    state.select(Some(app.settings_cursor));
    f.render_stateful_widget(list, area, &mut state);
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

    // 오른쪽: 자막 미리보기 + 결과물
    let rsplit = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(hsplit[1]);

    draw_preview(f, rsplit[0], app);

    let outs: Vec<ListItem> = app.outputs.iter().map(|o| {
        let label = o.source.as_deref().unwrap_or(&o.name);
        let tw = (rsplit[1].width as usize).saturating_sub(4);
        ListItem::new(truncate_right(&format!("{} ({})", label, human_size(o.size)), tw))
    }).collect();
    let olist = List::new(outs)
        .block(Block::default().borders(Borders::ALL)
            .title(format!(" 편집본 ({}) ", app.outputs.len())));
    f.render_widget(olist, rsplit[1]);
}

fn draw_preview(f: &mut Frame, area: Rect, app: &App) {
    let tw = (area.width as usize).saturating_sub(12);
    let (title, lines) = match (&app.preview, app.files.get(app.file_cursor)) {
        (_, None) => (" 미리보기 ".to_string(), vec![Line::from("파일 없음")]),
        (None, Some(f)) if !f.has_subtitle => (
            format!(" 미리보기 (자막 없음) "),
            vec![
                Line::from(Span::styled("자막이 아직 없습니다.", Style::default().fg(Color::DarkGray))),
                Line::from(""),
                Line::from(Span::styled("  [t] 자막 추출", Style::default().fg(Color::Cyan))),
            ],
        ),
        (None, Some(_)) => (
            " 미리보기 ".into(),
            vec![Line::from(Span::styled("로딩…", Style::default().fg(Color::DarkGray)))],
        ),
        (Some(sub), Some(_)) => {
            let (kept, _) = app.kept_count_for(sub);
            let mut ls: Vec<Line> = vec![
                Line::from(Span::styled(
                    format!("{} 라인 • {} 유지 • {:.1}초", sub.lines.len(), kept, sub.total_duration),
                    Style::default().fg(Color::Yellow),
                )),
                Line::from(""),
            ];
            for l in sub.lines.iter().take(((area.height as usize).saturating_sub(4)).max(3)) {
                let mark = if l.kept { "✓" } else { "·" };
                let t = fmt_time(l.start);
                let text = truncate_right(&l.text, tw);
                let style = if l.kept { Style::default().fg(Color::Green) } else { Style::default().fg(Color::DarkGray) };
                ls.push(Line::from(vec![
                    Span::styled(format!("{mark} "), style),
                    Span::styled(format!("{t} ", ), Style::default().fg(Color::DarkGray)),
                    Span::styled(text, style),
                ]));
            }
            (" 자막 미리보기 ".into(), ls)
        }
    };
    let p = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(p, area);
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
