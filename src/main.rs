mod api;
mod app;
mod config;
mod keys;
mod ui;
mod util;

use anyhow::Result;
use app::{App, View};
use clap::Parser;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::stdout;
use std::time::Duration;

#[derive(Parser)]
#[command(
    name = "ai-video-autocut",
    version = env!("CARGO_PKG_VERSION"),
    about = "autocut TUI — Qwen3-ASR + 컷편집",
)]
struct Cli {
    /// autocut-web endpoint (기본: config 또는 http://localhost:8080)
    #[arg(long)]
    endpoint: Option<String>,

    /// 사용 가능 키바인드 출력 후 종료
    #[arg(long)]
    keys: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.keys {
        print_keybinds();
        return Ok(());
    }
    let (mut cfg, cfg_err) = match config::load() {
        Ok(c) => (c, None),
        Err(e) => (config::Config::default(), Some(format!("config 로드 실패: {e} — 기본값 사용"))),
    };
    if let Some(e) = cli.endpoint { cfg.endpoint = e; }

    let client = api::Client::new(&cfg.endpoint);
    let mut app = App::new(client, cfg);
    if let Some(e) = cfg_err { app.status = e; }
    app.refresh_projects().await;
    if app.active_project.is_some() {
        app.refresh_files().await;
        app.view = View::Files;
    }

    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(out);
    let mut term = Terminal::new(backend)?;

    let res = run(&mut term, &mut app).await;

    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    res
}

async fn run<B: ratatui::backend::Backend>(
    term: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    let mut last_poll = std::time::Instant::now();
    let mut last_cursor = usize::MAX;
    let mut cursor_changed_at = std::time::Instant::now();
    loop {
        term.draw(|f| ui::draw(f, app))?;

        if app.job_progress.is_some() && last_poll.elapsed() > Duration::from_millis(1500) {
            app.poll_job().await;
            last_poll = std::time::Instant::now();
        }

        // 파일 뷰에서 커서가 잠시 멈추면 미리보기 로드
        if matches!(app.view, app::View::Files) {
            if app.file_cursor != last_cursor {
                last_cursor = app.file_cursor;
                cursor_changed_at = std::time::Instant::now();
            } else if cursor_changed_at.elapsed() > Duration::from_millis(250)
                && app.preview_for.as_deref() != app.files.get(app.file_cursor).map(|f| f.name.as_str())
            {
                app.refresh_preview().await;
            }
        }

        if event::poll(Duration::from_millis(if app.job_progress.is_some() { 500 } else { 200 }))? {
            if let Event::Key(k) = event::read()? {
                if k.kind != KeyEventKind::Press { continue; }
                // Ctrl+C 종료 (어디서든)
                if matches!(k.code, KeyCode::Char('c')) && k.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                    if app.job_progress.is_some() { app.cancel_job().await; }
                    app.should_quit = true;
                    break;
                }
                // 작업 중이면 ESC로 취소
                if app.job_progress.is_some() && matches!(k.code, KeyCode::Esc) {
                    app.cancel_job().await;
                    continue;
                }
                // 검색 모드 우선
                if app.search_mode {
                    handle_search(app, k.code);
                    continue;
                }
                // 컷 확인 다이얼로그 우선
                if app.confirm_cut {
                    match k.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                            app.do_cut().await;
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            app.confirm_cut = false;
                        }
                        _ => {}
                    }
                    continue;
                }
                // 도움말 모달 오픈 상태
                if app.show_help {
                    if matches!(k.code, KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q')) {
                        app.show_help = false;
                    }
                    continue;
                }
                // 전역 키
                if matches!(k.code, KeyCode::Char('?')) { app.show_help = true; continue; }
                match app.view {
                    View::Projects => handle_projects(app, k.code).await,
                    View::Files => handle_files(app, k.code).await,
                    View::Subtitles => handle_subs(app, k.code).await,
                    View::Settings => handle_settings(app, k.code).await,
                }
                if app.should_quit { break; }
            }
        }
    }
    Ok(())
}

fn print_keybinds() {
    println!("autocut-tui v{} — 키바인드", env!("CARGO_PKG_VERSION"));
    println!();
    println!("[공통]");
    println!("  ?           도움말 토글");
    println!("  q           종료");
    println!("  Ctrl+C      강제 종료 (진행중 작업 취소 포함)");
    println!("  r           새로고침");
    println!();
    println!("[프로젝트 화면]");
    println!("  ↑/k ↓/j     이동");
    println!("  Enter       선택");
    println!();
    println!("[파일 화면]");
    println!("  ↑/k ↓/j     이동, PgUp/PgDn 10개씩, g/G 맨위/아래");
    println!("  Enter       파일 열기 (자막 있으면 편집 뷰)");
    println!("  /           파일 이름 검색");
    println!("  t           자막 추출");
    println!("  e           엔진 토글 (qwen3 ↔ whisper)");
    println!("  l           언어 토글");
    println!("  p           프로젝트 선택으로");
    println!();
    println!("[자막 편집 화면]");
    println!("  Space       라인 토글");
    println!("  a           모두 유지");
    println!("  n           모두 제거");
    println!("  i           반전");
    println!("  c           컷 실행");
    println!("  Esc / b     파일 뷰로");
    println!();
    println!("[작업 중]");
    println!("  Esc         작업 취소");
}

fn handle_search(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => { app.search_mode = false; app.search_query.clear(); }
        KeyCode::Enter => { app.search_mode = false; }
        KeyCode::Backspace => { app.search_query.pop(); }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            // 검색 매칭 첫 항목으로 커서 이동
            let q = app.search_query.to_lowercase();
            if let Some((i, _)) = app.files.iter().enumerate()
                .find(|(_, f)| f.name.to_lowercase().contains(&q)) {
                app.file_cursor = i;
            }
        }
        _ => {}
    }
}

async fn handle_projects(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Up | KeyCode::Char('k') => {
            if app.project_cursor > 0 { app.project_cursor -= 1; }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.project_cursor + 1 < app.projects.len() { app.project_cursor += 1; }
        }
        KeyCode::Enter => app.select_project().await,
        KeyCode::Char('r') => app.refresh_projects().await,
        _ => {}
    }
}

async fn handle_files(app: &mut App, code: KeyCode) {
    let kb = app.config.keybinds.clone();
    if keys::matches(code, &kb.quit) || matches!(code, KeyCode::Esc) {
        app.should_quit = true; return;
    }
    if keys::matches(code, &kb.search) {
        app.search_mode = true; app.search_query.clear(); return;
    }
    if keys::matches(code, &kb.transcribe) { app.transcribe().await; return; }
    if keys::matches(code, &kb.project) { app.view = View::Projects; return; }
    if keys::matches(code, &kb.refresh) { app.refresh_files().await; return; }
    if matches!(code, KeyCode::Char('s')) { app.open_settings().await; return; }
    if keys::matches(code, &kb.engine_toggle) {
        app.engine = if app.engine == "qwen3" { "whisper".into() } else { "qwen3".into() };
        app.status = format!("engine → {}", app.engine);
        return;
    }
    if keys::matches(code, &kb.lang_toggle) {
        app.lang = match app.lang.as_str() {
            "Korean" => "English".into(), "English" => "Japanese".into(),
            "Japanese" => "zh".into(), _ => "Korean".into(),
        };
        app.status = format!("lang → {}", app.lang);
        return;
    }
    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            if app.file_cursor > 0 { app.file_cursor -= 1; }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.file_cursor + 1 < app.files.len() { app.file_cursor += 1; }
        }
        KeyCode::PageUp => app.file_cursor = app.file_cursor.saturating_sub(10),
        KeyCode::PageDown => {
            app.file_cursor = (app.file_cursor + 10).min(app.files.len().saturating_sub(1));
        }
        KeyCode::Char('g') => app.file_cursor = 0,
        KeyCode::Char('G') => app.file_cursor = app.files.len().saturating_sub(1),
        KeyCode::Enter => app.select_file().await,
        _ => {}
    }
}

async fn handle_settings(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => app.view = View::Files,
        KeyCode::Up | KeyCode::Char('k') => {
            if app.settings_cursor > 0 { app.settings_cursor -= 1; }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.settings_cursor + 1 < 4 { app.settings_cursor += 1; }
        }
        KeyCode::Left | KeyCode::Char('h') => app.settings_cycle(-1),
        KeyCode::Right | KeyCode::Char('l') | KeyCode::Char(' ') => app.settings_cycle(1),
        KeyCode::Enter => app.save_settings().await,
        _ => {}
    }
}

async fn handle_subs(app: &mut App, code: KeyCode) {
    let kb = app.config.keybinds.clone();
    if keys::matches(code, &kb.quit) { app.should_quit = true; return; }
    if matches!(code, KeyCode::Esc) || keys::matches(code, &kb.back) {
        app.view = View::Files; return;
    }
    if keys::matches(code, &kb.toggle_line) { app.toggle_line(); return; }
    if keys::matches(code, &kb.keep_all) { app.select_all_lines(true); return; }
    if keys::matches(code, &kb.keep_none) { app.select_all_lines(false); return; }
    if keys::matches(code, &kb.invert) { app.invert_lines(); return; }
    if keys::matches(code, &kb.cut) { app.request_cut(); return; }
    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            if app.sub_cursor > 0 { app.sub_cursor -= 1; }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(s) = &app.subtitle {
                if app.sub_cursor + 1 < s.lines.len() { app.sub_cursor += 1; }
            }
        }
        KeyCode::PageUp => app.sub_cursor = app.sub_cursor.saturating_sub(10),
        KeyCode::PageDown => {
            if let Some(s) = &app.subtitle {
                app.sub_cursor = (app.sub_cursor + 10).min(s.lines.len().saturating_sub(1));
            }
        }
        KeyCode::Char('g') => app.sub_cursor = 0,
        KeyCode::Char('G') => {
            if let Some(s) = &app.subtitle {
                app.sub_cursor = s.lines.len().saturating_sub(1);
            }
        }
        _ => {}
    }
}
