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
