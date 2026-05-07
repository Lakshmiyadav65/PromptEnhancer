use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Manager, Runtime};

use crate::{enhance, hotkey};

const KEYRING_SERVICE: &str = "PromptForge";
const KEYRING_ACCOUNT: &str = "groq_api_key";
const SETTINGS_FILE: &str = "settings.json";
const ENV_VAR: &str = "GROQ_API_KEY";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSettings {
    pub hotkey: String,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            hotkey: hotkey::DEFAULT_HOTKEY.to_string(),
        }
    }
}

fn settings_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf> {
    let dir = app
        .path()
        .app_config_dir()
        .context("could not resolve app config dir")?;
    std::fs::create_dir_all(&dir).context("could not create app config dir")?;
    Ok(dir.join(SETTINGS_FILE))
}

pub fn load<R: Runtime>(app: &AppHandle<R>) -> UserSettings {
    let Ok(path) = settings_path(app) else {
        return UserSettings::default();
    };
    match std::fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str::<UserSettings>(&s).unwrap_or_default(),
        Err(_) => UserSettings::default(),
    }
}

fn save<R: Runtime>(app: &AppHandle<R>, settings: &UserSettings) -> Result<()> {
    let path = settings_path(app)?;
    let json = serde_json::to_string_pretty(settings).context("serialize settings")?;
    std::fs::write(&path, json).context("write settings.json")?;
    Ok(())
}

// ---------- Tauri commands ----------

#[derive(Serialize)]
pub struct ApiKeyStatus {
    pub from_env: bool,
    pub from_keychain: bool,
}

#[tauri::command]
pub fn api_key_status() -> ApiKeyStatus {
    let from_env = std::env::var(ENV_VAR)
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);

    let from_keychain = keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
        .ok()
        .and_then(|e| e.get_password().ok())
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);

    ApiKeyStatus { from_env, from_keychain }
}

#[tauri::command]
pub fn save_api_key(key: String) -> std::result::Result<(), String> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err("API key cannot be empty".into());
    }
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
        .map_err(|e| format!("could not access OS keychain: {e}"))?;
    entry
        .set_password(trimmed)
        .map_err(|e| format!("could not save key to keychain: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn clear_api_key() -> std::result::Result<(), String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
        .map_err(|e| format!("could not access OS keychain: {e}"))?;
    match entry.delete_credential() {
        Ok(_) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("could not delete key from keychain: {e}")),
    }
}

#[tauri::command]
pub fn get_hotkey<R: Runtime>(app: AppHandle<R>) -> String {
    load(&app).hotkey
}

#[tauri::command]
pub fn save_hotkey<R: Runtime>(
    app: AppHandle<R>,
    combo: String,
) -> std::result::Result<(), String> {
    let trimmed = combo.trim().to_string();
    if trimmed.is_empty() {
        return Err("hotkey combo cannot be empty".into());
    }
    // Validate by re-registering. If parse/registration fails, surface that error
    // and don't persist the bad combo.
    hotkey::reregister(&app, &trimmed).map_err(|e| format!("{e:#}"))?;

    let mut settings = load(&app);
    settings.hotkey = trimmed;
    save(&app, &settings).map_err(|e| format!("{e:#}"))?;
    Ok(())
}

#[derive(Serialize)]
pub struct ConnectionTest {
    pub ok: bool,
    pub latency_ms: u128,
    pub message: String,
}

#[tauri::command]
pub async fn test_connection() -> ConnectionTest {
    let api_key = match enhance::load_api_key() {
        Ok(k) => k,
        Err(e) => {
            return ConnectionTest {
                ok: false,
                latency_ms: 0,
                message: format!("{e:#}"),
            }
        }
    };

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return ConnectionTest {
                ok: false,
                latency_ms: 0,
                message: format!("could not build HTTP client: {e}"),
            }
        }
    };

    let body = json!({
        "model": "llama-3.3-70b-versatile",
        "max_tokens": 8,
        "messages": [{ "role": "user", "content": "ping" }]
    });

    let start = Instant::now();
    let response = match client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .bearer_auth(&api_key)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return ConnectionTest {
                ok: false,
                latency_ms: start.elapsed().as_millis(),
                message: format!("network error: {e}"),
            }
        }
    };
    let latency_ms = start.elapsed().as_millis();

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return ConnectionTest {
            ok: false,
            latency_ms,
            message: format!("Groq returned {status}: {body}"),
        };
    }

    ConnectionTest {
        ok: true,
        latency_ms,
        message: "ok".into(),
    }
}

#[tauri::command]
pub fn open_settings<R: Runtime>(app: AppHandle<R>) -> std::result::Result<(), String> {
    let window = app
        .get_webview_window("settings")
        .ok_or_else(|| "settings window not found".to_string())?;
    window.show().map_err(|e| format!("show: {e}"))?;
    window.set_focus().map_err(|e| format!("focus: {e}"))?;
    Ok(())
}
