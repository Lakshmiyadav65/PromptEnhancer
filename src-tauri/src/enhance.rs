use std::path::PathBuf;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Manager, Runtime};

const KEYRING_SERVICE: &str = "PromptForge";
const KEYRING_ACCOUNT: &str = "groq_api_key";
const ENV_VAR: &str = "GROQ_API_KEY";
const GROQ_API_URL: &str = "https://api.groq.com/openai/v1/chat/completions";
const MODEL: &str = "llama-3.3-70b-versatile";
const MAX_TOKENS: u32 = 1024;
const REQUEST_TIMEOUT_SECS: u64 = 30;

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    messages: Vec<Message<'a>>,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: Option<String>,
}

pub async fn enhance_prompt<R: Runtime>(app: &AppHandle<R>, input: &str) -> Result<String> {
    let api_key = load_api_key()?;
    let system_prompt = load_meta_prompt(app)?;

    let body = ChatRequest {
        model: MODEL,
        max_tokens: MAX_TOKENS,
        messages: vec![
            Message {
                role: "system",
                content: &system_prompt,
            },
            Message {
                role: "user",
                content: input,
            },
        ],
    };

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .context("could not build HTTP client")?;

    let response = client
        .post(GROQ_API_URL)
        .bearer_auth(&api_key)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .context("Groq API request failed (network or DNS issue)")?;

    let status = response.status();
    if !status.is_success() {
        let err_body = response.text().await.unwrap_or_default();
        return Err(anyhow!("Groq API returned {status}: {err_body}"));
    }

    let parsed: ChatResponse = response
        .json()
        .await
        .context("failed to parse Groq response as JSON")?;

    parsed
        .choices
        .into_iter()
        .next()
        .and_then(|c| c.message.content)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Groq response had no text content"))
}

pub(crate) fn load_api_key() -> Result<String> {
    // Prefer env var (read from shell or loaded from .env at startup).
    if let Ok(key) = std::env::var(ENV_VAR) {
        let trimmed = key.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    // Fall back to OS keychain (populated by Settings window in Phase 6).
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
        .context("could not access OS keychain")?;
    match entry.get_password() {
        Ok(key) => Ok(key),
        Err(keyring::Error::NoEntry) => Err(anyhow!(
            "Groq API key not found. Add {ENV_VAR}=... to your .env file at the project root, or save a key via Settings (Phase 6)."
        )),
        Err(e) => Err(anyhow!("keychain error: {e}")),
    }
}

fn load_meta_prompt<R: Runtime>(app: &AppHandle<R>) -> Result<String> {
    let path = resolve_prompt_path(app)?;
    std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read meta-prompt from {}", path.display()))
}

fn resolve_prompt_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf> {
    app.path()
        .resolve(
            "prompts/enhancer-system-prompt.md",
            BaseDirectory::Resource,
        )
        .context("failed to resolve meta-prompt resource path")
}
