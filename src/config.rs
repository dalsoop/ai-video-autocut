use anyhow::{anyhow, Result};
use nickel_lang_core::error::report::ErrorFormat;
use nickel_lang_core::error::NullReporter;
use nickel_lang_core::eval::cache::CacheImpl;
use nickel_lang_core::program::Program;
use serde::Deserialize;
use std::io::Cursor;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_endpoint")]
    pub endpoint: String,
    #[serde(default)]
    pub defaults: Defaults,
    #[serde(default)]
    pub keybinds: Keybinds,
    #[serde(default)]
    pub theme: Theme,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Defaults {
    #[serde(default = "default_engine")]
    pub engine: String,
    #[serde(default = "default_lang")]
    pub lang: String,
    #[serde(default = "default_whisper_model")]
    pub whisper_model: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Keybinds {
    #[serde(default = "default_quit")]    pub quit: String,
    #[serde(default = "default_help")]    pub help: String,
    #[serde(default = "default_search")]  pub search: String,
    #[serde(default = "default_refresh")] pub refresh: String,
    #[serde(default = "default_toggle")]  pub toggle_line: String,
    #[serde(default = "default_transcribe")] pub transcribe: String,
    #[serde(default = "default_cut")]     pub cut: String,
    #[serde(default = "default_engine_toggle")] pub engine_toggle: String,
    #[serde(default = "default_lang_toggle")]   pub lang_toggle: String,
    #[serde(default = "default_project")] pub project: String,
    #[serde(default = "default_back")]    pub back: String,
    #[serde(default = "default_all")]     pub keep_all: String,
    #[serde(default = "default_none")]    pub keep_none: String,
    #[serde(default = "default_invert")]  pub invert: String,
}

impl Default for Keybinds {
    fn default() -> Self { Self {
        quit: default_quit(), help: default_help(), search: default_search(),
        refresh: default_refresh(), toggle_line: default_toggle(),
        transcribe: default_transcribe(), cut: default_cut(),
        engine_toggle: default_engine_toggle(), lang_toggle: default_lang_toggle(),
        project: default_project(), back: default_back(),
        keep_all: default_all(), keep_none: default_none(), invert: default_invert(),
    } }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Theme {
    #[serde(default)]
    pub accent: Option<String>,
}

fn default_quit() -> String { "q".into() }
fn default_help() -> String { "?".into() }
fn default_search() -> String { "/".into() }
fn default_refresh() -> String { "r".into() }
fn default_toggle() -> String { " ".into() }
fn default_transcribe() -> String { "t".into() }
fn default_cut() -> String { "c".into() }
fn default_engine_toggle() -> String { "e".into() }
fn default_lang_toggle() -> String { "l".into() }
fn default_project() -> String { "p".into() }
fn default_back() -> String { "b".into() }
fn default_all() -> String { "a".into() }
fn default_none() -> String { "n".into() }
fn default_invert() -> String { "i".into() }

fn default_endpoint() -> String { "http://localhost:8080".into() }
fn default_engine() -> String { "qwen3".into() }
fn default_lang() -> String { "Korean".into() }
fn default_whisper_model() -> String { "medium".into() }

impl Default for Config {
    fn default() -> Self {
        Self {
            endpoint: default_endpoint(),
            defaults: Defaults {
                engine: default_engine(),
                lang: default_lang(),
                whisper_model: default_whisper_model(),
            },
            keybinds: Keybinds::default(),
            theme: Theme::default(),
        }
    }
}

pub fn config_path() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("autocut/config.ncl")
}

pub fn load() -> Result<Config> {
    let path = config_path();
    if !path.exists() {
        return Ok(Config::default());
    }
    let source = std::fs::read_to_string(&path)?;
    let mut prog: Program<CacheImpl> = Program::new_from_source(
        Cursor::new(source.as_bytes()),
        path.to_string_lossy().as_ref(),
        std::io::stderr(),
        NullReporter {},
    )?;
    let _ = ErrorFormat::Text;
    let term = prog.eval_full_for_export()
        .map_err(|e| anyhow!("nickel eval failed: {:?}", e))?;
    let json = nickel_lang_core::serialize::to_string(
        nickel_lang_core::serialize::ExportFormat::Json,
        &term,
    ).map_err(|e| anyhow!("nickel export failed: {:?}", e))?;
    let cfg: Config = serde_json::from_str(&json)?;
    Ok(cfg)
}
