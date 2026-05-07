use std::str::FromStr;

use anyhow::{anyhow, Result};
use tauri::{AppHandle, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::{clipboard, enhance, status_window};

pub const DEFAULT_HOTKEY: &str = "CommandOrControl+Alt+E";

pub fn register<R: Runtime>(app: &AppHandle<R>, combo: &str) -> tauri::Result<()> {
    let shortcut = Shortcut::from_str(combo)
        .map_err(|e| tauri::Error::Anyhow(anyhow::anyhow!("invalid hotkey {combo:?}: {e}")))?;

    app.global_shortcut()
        .on_shortcut(shortcut, |app_handle, _shortcut, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }
            println!("[hotkey] pressed");
            let app = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                let result = run_enhancement_pipeline(&app).await;
                let _ = status_window::hide(&app);
                if let Err(e) = result {
                    println!("[pipeline] failed: {e:#}");
                }
            });
        })
        .map_err(|e| tauri::Error::Anyhow(anyhow::anyhow!("failed to register hotkey: {e}")))?;

    println!("[hotkey] registered: {combo}");
    Ok(())
}

pub fn reregister<R: Runtime>(app: &AppHandle<R>, combo: &str) -> Result<()> {
    // Clear any previously registered shortcuts first; we only ever register one.
    let _ = app.global_shortcut().unregister_all();
    register(app, combo).map_err(|e| anyhow!("{e}"))
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

    if let Err(e) = status_window::show_near_cursor(app) {
        println!("[status] could not show indicator: {e}");
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
