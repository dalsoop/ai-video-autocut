mod api;
mod app;
mod config;
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
#[command(about = "autocut TUI — Qwen3-ASR + 컷편집")]
struct Cli {
    /// autocut-web endpoint (기본: config 또는 http://localhost:8080)
    #[arg(long)]
    endpoint: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
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
    loop {
        term.draw(|f| ui::draw(f, app))?;

        if app.job_progress.is_some() && last_poll.elapsed() > Duration::from_millis(1500) {
            app.poll_job().await;
            last_poll = std::time::Instant::now();
        }

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(k) = event::read()? {
                if k.kind != KeyEventKind::Press { continue; }
                match app.view {
                    View::Projects => handle_projects(app, k.code).await,
                    View::Files => handle_files(app, k.code).await,
                    View::Subtitles => handle_subs(app, k.code).await,
                }
                if app.should_quit { break; }
            }
        }
    }
    Ok(())
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
    match code {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
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
        KeyCode::Char('t') => app.transcribe().await,
        KeyCode::Char('p') => app.view = View::Projects,
        KeyCode::Char('r') => app.refresh_files().await,
        KeyCode::Char('e') => {
            app.engine = if app.engine == "qwen3" { "whisper".into() } else { "qwen3".into() };
            app.status = format!("engine → {}", app.engine);
        }
        KeyCode::Char('l') => {
            app.lang = match app.lang.as_str() {
                "Korean" => "English".into(),
                "English" => "Japanese".into(),
                "Japanese" => "zh".into(),
                _ => "Korean".into(),
            };
            app.status = format!("lang → {}", app.lang);
        }
        _ => {}
    }
}

async fn handle_subs(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Esc | KeyCode::Char('b') => app.view = View::Files,
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
        KeyCode::Char(' ') => app.toggle_line(),
        KeyCode::Char('a') => app.select_all_lines(true),
        KeyCode::Char('n') => app.select_all_lines(false),
        KeyCode::Char('i') => app.invert_lines(),
        KeyCode::Char('c') => app.do_cut().await,
        _ => {}
    }
}
