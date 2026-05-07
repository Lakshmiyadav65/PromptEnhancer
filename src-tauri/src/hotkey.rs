use std::str::FromStr;

use anyhow::{anyhow, Result};
use tauri::{AppHandle, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::{clipboard, enhance};

pub const DEFAULT_HOTKEY: &str = "CommandOrControl+Alt+E";

pub fn register<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let shortcut = Shortcut::from_str(DEFAULT_HOTKEY).map_err(|e| {
        tauri::Error::Anyhow(anyhow::anyhow!("invalid hotkey {DEFAULT_HOTKEY:?}: {e}"))
    })?;

    app.global_shortcut()
        .on_shortcut(shortcut, |app_handle, _shortcut, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }
            println!("[hotkey] pressed");
            let app = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = run_enhancement_pipeline(&app).await {
                    println!("[pipeline] failed: {e:#}");
                }
            });
        })
        .map_err(|e| tauri::Error::Anyhow(anyhow::anyhow!("failed to register hotkey: {e}")))?;

    println!("[hotkey] registered: {DEFAULT_HOTKEY}");
    Ok(())
}

async fn run_enhancement_pipeline<R: Runtime>(app: &AppHandle<R>) -> Result<()> {
    let input = clipboard::capture_selection(app)
        .await
        .map_err(|e| anyhow!("capture failed: {e}"))?;
    let input_chars = input.chars().count();
    println!("[capture] {input_chars} chars captured");

    if input.trim().is_empty() {
        return Err(anyhow!("captured selection is empty"));
    }

    let enhanced = enhance::enhance_prompt(app, &input)
        .await
        .map_err(|e| anyhow!("enhance failed: {e}"))?;
    let output_chars = enhanced.chars().count();
    println!("[enhance] {input_chars} chars -> {output_chars} chars");

    clipboard::replace_selection(app, &enhanced)
        .await
        .map_err(|e| anyhow!("replace failed: {e}"))?;
    println!("[replace] selection replaced with enhanced prompt");

    Ok(())
}
