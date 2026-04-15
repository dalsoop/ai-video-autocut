use crate::api::{Client, FileInfo, SubtitleData, SubtitleLine};
#[allow(unused_imports)]
use crate::api::*;
use crate::config::Config;

#[derive(Clone, Copy, PartialEq)]
pub enum View {
    Projects,
    Files,
    Subtitles,
    Settings,
}

#[derive(Clone, Default)]
pub struct RemoteSettings {
    pub default_engine: String,
    pub default_lang: String,
    pub default_whisper_model: String,
    pub qwen3_device: String,
}

pub struct App {
    pub client: Client,
    pub config: Config,
    pub view: View,
    pub projects: Vec<String>,
    pub active_project: Option<String>,
    pub project_cursor: usize,
    pub files: Vec<FileInfo>,
    pub outputs: Vec<FileInfo>,
    pub file_cursor: usize,
    pub selected_file: Option<FileInfo>,
    pub subtitle: Option<SubtitleData>,
    pub sub_cursor: usize,
    pub job_progress: Option<(String, u32, String)>,  // id, %, message
    pub job_started: Option<std::time::Instant>,
    pub status: String,
    pub should_quit: bool,
    pub engine: String,
    pub lang: String,
    pub show_help: bool,
    pub search_mode: bool,
    pub search_query: String,
    pub confirm_cut: bool,
    pub preview: Option<SubtitleData>,
    pub preview_for: Option<String>,
    pub settings: RemoteSettings,
    pub settings_cursor: usize,
}

impl App {
    pub fn new(client: Client, config: Config) -> Self {
        let engine = config.defaults.engine.clone();
        let lang = config.defaults.lang.clone();
        Self {
            client, config,
            view: View::Projects,
            projects: vec![], active_project: None, project_cursor: 0,
            files: vec![], outputs: vec![], file_cursor: 0, selected_file: None,
            subtitle: None, sub_cursor: 0,
            job_progress: None, job_started: None,
            status: "loading...".into(),
            should_quit: false,
            engine, lang,
            show_help: false,
            search_mode: false, search_query: String::new(),
            confirm_cut: false,
            preview: None, preview_for: None,
            settings: RemoteSettings::default(),
            settings_cursor: 0,
        }
    }

    pub async fn open_settings(&mut self) {
        if let Ok(cfg) = self.client.get_config().await {
            self.settings = RemoteSettings {
                default_engine: cfg.get("defaultEngine").and_then(|v| v.as_str()).unwrap_or("qwen3").into(),
                default_lang: cfg.get("defaultLang").and_then(|v| v.as_str()).unwrap_or("Korean").into(),
                default_whisper_model: cfg.get("defaultWhisperModel").and_then(|v| v.as_str()).unwrap_or("medium").into(),
                qwen3_device: cfg.get("qwen3Device").and_then(|v| v.as_str()).unwrap_or("cuda:0").into(),
            };
        }
        self.settings_cursor = 0;
        self.view = View::Settings;
    }

    pub fn settings_cycle(&mut self, delta: i32) {
        let field = self.settings_cursor;
        let cycle = |v: &mut String, opts: &[&str]| {
            let i = opts.iter().position(|o| o == &v.as_str()).unwrap_or(0) as i32;
            let n = opts.len() as i32;
            let new = ((i + delta).rem_euclid(n)) as usize;
            *v = opts[new].to_string();
        };
        match field {
            0 => cycle(&mut self.settings.default_engine, &["qwen3", "whisper"]),
            1 => cycle(&mut self.settings.default_lang, &["Korean", "English", "Japanese", "zh"]),
            2 => cycle(&mut self.settings.default_whisper_model, &["tiny","base","small","medium","large-v3-turbo","large-v3"]),
            3 => cycle(&mut self.settings.qwen3_device, &["cuda:0","cuda:1","cpu"]),
            _ => {}
        }
    }

    pub async fn save_settings(&mut self) {
        let patch = serde_json::json!({
            "defaultEngine": self.settings.default_engine,
            "defaultLang": self.settings.default_lang,
            "defaultWhisperModel": self.settings.default_whisper_model,
            "qwen3Device": self.settings.qwen3_device,
        });
        match self.client.patch_config(&patch).await {
            Ok(_) => {
                self.engine = self.settings.default_engine.clone();
                self.lang = self.settings.default_lang.clone();
                self.status = "✓ 설정 저장됨".into();
                self.view = View::Files;
            }
            Err(e) => self.status = format!("설정 저장 실패: {e}"),
        }
    }

    pub async fn refresh_preview(&mut self) {
        let Some(f) = self.files.get(self.file_cursor).cloned() else {
            self.preview = None; self.preview_for = None; return;
        };
        if self.preview_for.as_deref() == Some(&f.name) { return; }
        self.preview_for = Some(f.name.clone());
        if !f.has_subtitle { self.preview = None; return; }
        match self.client.subtitle(&f.name).await {
            Ok(s) => self.preview = Some(s),
            Err(_) => self.preview = None,
        }
    }

    pub fn filtered_files(&self) -> Vec<(usize, &crate::api::FileInfo)> {
        if self.search_query.is_empty() {
            self.files.iter().enumerate().collect()
        } else {
            let q = self.search_query.to_lowercase();
            self.files.iter().enumerate()
                .filter(|(_, f)| f.name.to_lowercase().contains(&q))
                .collect()
        }
    }

    pub async fn refresh_projects(&mut self) {
        match self.client.projects().await {
            Ok(p) => { self.projects = p; }
            Err(e) => self.status = format!("projects 실패: {e}"),
        }
        if let Ok(cfg) = self.client.get_config().await {
            if let Some(ap) = cfg.get("activeProject").and_then(|v| v.as_str()) {
                if !ap.is_empty() { self.active_project = Some(ap.into()); }
            }
        }
    }

    pub async fn refresh_files(&mut self) {
        match self.client.files().await {
            Ok(r) => { self.files = r.input; self.outputs = r.output; }
            Err(e) => self.status = format!("files 실패: {e}"),
        }
    }

    pub async fn select_project(&mut self) {
        if let Some(p) = self.projects.get(self.project_cursor).cloned() {
            if let Err(e) = self.client.set_project(&p).await {
                self.status = format!("프로젝트 변경 실패: {e}");
                return;
            }
            self.active_project = Some(p.clone());
            self.status = format!("프로젝트: {p}");
            self.refresh_files().await;
            self.view = View::Files;
            self.file_cursor = 0;
        }
    }

    pub async fn select_file(&mut self) {
        if let Some(f) = self.files.get(self.file_cursor).cloned() {
            self.selected_file = Some(f.clone());
            if f.has_subtitle {
                match self.client.subtitle(&f.name).await {
                    Ok(s) => {
                        self.subtitle = Some(s);
                        self.sub_cursor = 0;
                        self.view = View::Subtitles;
                    }
                    Err(e) => self.status = format!("subtitle 실패: {e}"),
                }
            } else {
                self.status = format!("{}: 자막 없음 (t로 추출)", f.name);
            }
        }
    }

    pub async fn transcribe(&mut self) {
        let f = match &self.selected_file {
            Some(f) => f.clone(),
            None => match self.files.get(self.file_cursor).cloned() {
                Some(f) => { self.selected_file = Some(f.clone()); f }
                None => return,
            },
        };
        let req = crate::api::TranscribeRequest {
            filename: &f.name,
            engine: &self.engine,
            whisper_model: if self.engine == "whisper" {
                Some(&self.config.defaults.whisper_model)
            } else { None },
            lang: &self.lang,
        };
        match self.client.transcribe(&req).await {
            Ok(job) => {
                self.job_progress = Some((job.id.clone(), 0, "자막 추출 중...".into()));
                self.job_started = Some(std::time::Instant::now());
                self.status = format!("{} 시작 ({})", job.job_type, self.engine);
            }
            Err(e) => self.status = format!("transcribe 실패: {e}"),
        }
    }

    pub fn request_cut(&mut self) {
        if self.subtitle.is_none() { self.status = "자막 없음".into(); return; }
        let kept = self.kept_count();
        if kept.0 == 0 { self.status = "유지할 라인이 없음".into(); return; }
        self.confirm_cut = true;
    }

    pub async fn do_cut(&mut self) {
        self.confirm_cut = false;
        let f = match &self.selected_file { Some(f) => f.clone(), None => return };
        let keep: Vec<u32> = self.subtitle.as_ref()
            .map(|s| s.lines.iter().filter(|l| l.kept).map(|l| l.index).collect())
            .unwrap_or_default();
        if keep.is_empty() { self.status = "유지할 라인이 없음".into(); return; }
        let req = crate::api::CutRequest { filename: &f.name, keep_indices: keep };
        match self.client.cut(&req).await {
            Ok(job) => {
                self.job_progress = Some((job.id, 0, "컷 편집 중...".into()));
                self.job_started = Some(std::time::Instant::now());
                self.status = "컷 시작".into();
            }
            Err(e) => self.status = format!("cut 실패: {e}"),
        }
    }

    pub async fn poll_job(&mut self) {
        let id = match &self.job_progress { Some((id, _, _)) => id.clone(), None => return };
        if let Ok(job) = self.client.job(&id).await {
            let msg = job.message.clone().unwrap_or_default();
            self.job_progress = Some((job.id.clone(), job.progress, msg));
            if job.status == "done" {
                let elapsed = self.job_started.map(|t| t.elapsed().as_secs()).unwrap_or(0);
                self.status = format!("✓ {} 완료 ({}초)", job.job_type, elapsed);
                self.job_progress = None;
                self.job_started = None;
                self.refresh_files().await;
                if let Some(f) = &self.selected_file.clone() {
                    if let Ok(s) = self.client.subtitle(&f.name).await {
                        if s.has_srt { self.subtitle = Some(s); self.view = View::Subtitles; }
                    }
                }
            } else if job.status == "failed" {
                self.status = format!("✗ 실패: {}", job.message.unwrap_or_default());
                self.job_progress = None;
                self.job_started = None;
            }
        }
    }

    pub fn toggle_line(&mut self) {
        if let Some(s) = &mut self.subtitle {
            if let Some(l) = s.lines.get_mut(self.sub_cursor) {
                l.kept = !l.kept;
            }
        }
    }

    pub fn select_all_lines(&mut self, keep: bool) {
        if let Some(s) = &mut self.subtitle {
            for l in s.lines.iter_mut() { l.kept = keep; }
        }
    }

    pub fn invert_lines(&mut self) {
        if let Some(s) = &mut self.subtitle {
            for l in s.lines.iter_mut() { l.kept = !l.kept; }
        }
    }

    pub async fn cancel_job(&mut self) {
        let id = match &self.job_progress { Some((id, _, _)) => id.clone(), None => return };
        match self.client.cancel(&id).await {
            Ok(_) => {
                self.status = "작업 취소 요청".into();
                self.job_progress = None;
            }
            Err(e) => self.status = format!("취소 실패: {e}"),
        }
    }

    pub fn kept_count(&self) -> (usize, f64) {
        if let Some(s) = &self.subtitle {
            let kept: Vec<&SubtitleLine> = s.lines.iter().filter(|l| l.kept).collect();
            (kept.len(), kept.iter().map(|l| l.duration).sum())
        } else { (0, 0.0) }
    }

    pub fn kept_count_for(&self, s: &SubtitleData) -> (usize, f64) {
        let kept: Vec<&SubtitleLine> = s.lines.iter().filter(|l| l.kept).collect();
        (kept.len(), kept.iter().map(|l| l.duration).sum())
    }
}
