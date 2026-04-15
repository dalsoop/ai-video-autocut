use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub size: u64,
    #[serde(rename = "hasSubtitle")]
    pub has_subtitle: bool,
    #[serde(rename = "hasOutput")]
    pub has_output: bool,
    #[serde(default)]
    pub outputs: Vec<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(rename = "createdAt", default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileListResponse {
    pub input: Vec<FileInfo>,
    pub output: Vec<FileInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubtitleLine {
    pub index: u32,
    pub start: f64,
    pub end: f64,
    pub duration: f64,
    pub text: String,
    pub kept: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubtitleData {
    pub filename: String,
    pub lines: Vec<SubtitleLine>,
    #[serde(rename = "totalDuration")]
    pub total_duration: f64,
    #[serde(rename = "hasSrt")]
    pub has_srt: bool,
    #[serde(rename = "hasMd")]
    pub has_md: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Job {
    pub id: String,
    #[serde(rename = "type")]
    pub job_type: String,
    pub filename: String,
    pub status: String,
    #[serde(default)]
    pub progress: u32,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TranscribeRequest<'a> {
    pub filename: &'a str,
    pub engine: &'a str,
    #[serde(rename = "whisperModel", skip_serializing_if = "Option::is_none")]
    pub whisper_model: Option<&'a str>,
    pub lang: &'a str,
}

#[derive(Debug, Serialize)]
pub struct CutRequest<'a> {
    pub filename: &'a str,
    #[serde(rename = "keepIndices")]
    pub keep_indices: Vec<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Clone)]
pub struct Client {
    base: String,
    http: reqwest::Client,
}

impl Client {
    pub fn new(base: impl Into<String>) -> Self {
        Self {
            base: base.into(),
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap(),
        }
    }

    pub async fn projects(&self) -> Result<Vec<String>> {
        Ok(self.http.get(format!("{}/api/projects", self.base)).send().await?.json().await?)
    }

    pub async fn get_config(&self) -> Result<serde_json::Value> {
        Ok(self.http.get(format!("{}/api/config", self.base)).send().await?.json().await?)
    }

    pub async fn set_project(&self, project: &str) -> Result<()> {
        self.patch_config(&serde_json::json!({ "activeProject": project })).await
    }

    pub async fn patch_config(&self, patch: &serde_json::Value) -> Result<()> {
        self.http
            .post(format!("{}/api/config", self.base))
            .json(patch)
            .send().await?.error_for_status()?;
        Ok(())
    }

    pub async fn files(&self) -> Result<FileListResponse> {
        Ok(self.http.get(format!("{}/api/files", self.base)).send().await?.json().await?)
    }

    pub async fn subtitle(&self, filename: &str) -> Result<SubtitleData> {
        let enc = encode_path(filename);
        Ok(self.http.get(format!("{}/api/subtitle/{}", self.base, enc)).send().await?.json().await?)
    }

    pub async fn transcribe(&self, req: &TranscribeRequest<'_>) -> Result<Job> {
        Ok(self.http.post(format!("{}/api/jobs/transcribe", self.base))
            .json(req).send().await?.json().await?)
    }

    pub async fn cut(&self, req: &CutRequest<'_>) -> Result<Job> {
        Ok(self.http.post(format!("{}/api/jobs/cut", self.base))
            .json(req).send().await?.json().await?)
    }

    pub async fn job(&self, id: &str) -> Result<Job> {
        Ok(self.http.get(format!("{}/api/jobs/{}", self.base, id)).send().await?.json().await?)
    }

    pub async fn cancel(&self, id: &str) -> Result<()> {
        self.http.post(format!("{}/api/jobs/{}/cancel", self.base, id))
            .send().await?.error_for_status()?;
        Ok(())
    }

    pub async fn edit_lines(&self, filename: &str, edits: serde_json::Value) -> Result<()> {
        let enc = encode_path(filename);
        self.http.patch(format!("{}/api/subtitle/{}", self.base, enc))
            .json(&serde_json::json!({ "edits": edits }))
            .send().await?.error_for_status()?;
        Ok(())
    }

    pub async fn split_line(&self, filename: &str, index: u32) -> Result<()> {
        let enc = encode_path(filename);
        self.http.patch(format!("{}/api/subtitle/{}", self.base, enc))
            .json(&serde_json::json!({ "action": "split", "index": index }))
            .send().await?.error_for_status()?;
        Ok(())
    }

    pub async fn merge_line(&self, filename: &str, index: u32) -> Result<()> {
        let enc = encode_path(filename);
        self.http.patch(format!("{}/api/subtitle/{}", self.base, enc))
            .json(&serde_json::json!({ "action": "merge", "index": index }))
            .send().await?.error_for_status()?;
        Ok(())
    }

    pub async fn batch_transcribe(&self) -> Result<serde_json::Value> {
        Ok(self.http.post(format!("{}/api/jobs/transcribe-batch", self.base))
            .send().await?.json().await?)
    }

    pub async fn pending(&self) -> Result<Vec<String>> {
        Ok(self.http.get(format!("{}/api/pending", self.base))
            .send().await?.json().await?)
    }
}

fn encode_path(p: &str) -> String {
    p.split('/')
        .map(|seg| urlencoding_encode(seg))
        .collect::<Vec<_>>()
        .join("/")
}

fn urlencoding_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}
